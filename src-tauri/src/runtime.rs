//! Process lifecycle: spawn QEMU (+ swtpm), control it over QMP, and sample resource usage.

use crate::{accel, models::*, qemu, store};
use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Mutex,
    time::Duration,
};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::{tcp::OwnedReadHalf, TcpStream},
    process::{Child, Command},
    time::timeout,
};

pub struct RunningVm {
    pub child: Child,
    pub swtpm: Option<Child>,
    pub qmp_port: u16,
    pub paused: bool,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub running: Mutex<HashMap<String, RunningVm>>,
    pub sys: Mutex<System>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir, running: Mutex::new(HashMap::new()), sys: Mutex::new(System::new()) }
    }
}

fn quiet(cmd: &mut Command) -> &mut Command {
    // Hide the console window on Windows (tokio::process::Command has this inherent method).
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

fn free_port() -> Result<u16> {
    let l = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(l.local_addr()?.port())
}

fn tail(s: &str, n: usize) -> String {
    let t = s.trim();
    let start = t.char_indices().rev().nth(n).map(|(i, _)| i).unwrap_or(0);
    t[start..].to_string()
}

/// Drop VMs whose process has exited (guest shut down, crashed or was closed).
pub fn reap(state: &AppState) {
    let mut r = state.running.lock().unwrap();
    let dead: Vec<String> = r
        .iter_mut()
        .filter_map(|(id, v)| matches!(v.child.try_wait(), Ok(Some(_))).then(|| id.clone()))
        .collect();
    for id in dead {
        if let Some(mut v) = r.remove(&id) {
            if let Some(t) = v.swtpm.as_mut() { let _ = t.start_kill(); }
        }
    }
}

pub fn shutdown_all(state: &AppState) {
    if let Ok(mut r) = state.running.lock() {
        for (_, v) in r.iter_mut() {
            let _ = v.child.start_kill();
            if let Some(t) = v.swtpm.as_mut() { let _ = t.start_kill(); }
        }
    }
}

pub async fn create_disk(path: &Path, format: DiskFormat, size_gb: u32) -> Result<()> {
    let img = accel::find_tool("qemu-img").ok_or_else(|| anyhow!("qemu-img wasn't found. Install QEMU and restart OmniBox."))?;
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await?; }
    let mut cmd = Command::new(img);
    // qcow2 is sparse by default (grows as the guest writes) — like VirtualBox "dynamically allocated".
    // Explicit preallocation=off so the host does not reserve the full size_gb up front.
    quiet(&mut cmd);
    match format {
        crate::models::DiskFormat::Qcow2 => {
            cmd.args(["create", "-f", "qcow2", "-o", "preallocation=off"]).arg(path).arg(format!("{size_gb}G"));
        }
        crate::models::DiskFormat::Vdi => {
            // QEMU can create VDI; still sparse-friendly when the host FS supports holes
            cmd.args(["create", "-f", "vdi"]).arg(path).arg(format!("{size_gb}G"));
        }
        crate::models::DiskFormat::Raw => {
            // raw may still use full size on some filesystems — prefer qcow2 in the UI
            cmd.args(["create", "-f", "raw", "-o", "preallocation=off"]).arg(path).arg(format!("{size_gb}G"));
        }
    }
    let out = cmd.output().await.context("running qemu-img")?;
    if !out.status.success() {
        return Err(anyhow!("qemu-img failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(())
}

pub async fn start_vm(state: &AppState, cfg: &VmConfig) -> Result<Vec<String>> {
    reap(state);
    if state.running.lock().unwrap().contains_key(&cfg.id) {
        return Err(anyhow!("{} is already running.", cfg.name));
    }
    // Clone so we can auto-eject the installer ISO like VirtualBox/VMware once the disk
    // looks installed — next boot goes straight to GRUB/the OS, not the installer menu.
    let mut cfg = cfg.clone();
    let mut auto_notes: Vec<String> = Vec::new();
    let disk_bytes = std::fs::metadata(&cfg.disk.path).map(|m| m.len()).unwrap_or(0);
    if cfg.iso_path.is_some() && disk_bytes > 2 * 1024 * 1024 {
        cfg.iso_path = None;
        let _ = store::save(&state.data_dir, &cfg);
        auto_notes.push(
            "Installer ISO was removed automatically (disk already has an OS). Booting from the hard disk — like VirtualBox after install."
                .into(),
        );
    }

    let vm_dir = store::vm_dir(&cfg);
    tokio::fs::create_dir_all(&vm_dir).await?;
    if !Path::new(&cfg.disk.path).exists() {
        create_disk(Path::new(&cfg.disk.path), cfg.disk.format, cfg.disk.size_gb).await?;
    }
    for iso in cfg.iso_path.iter().chain(cfg.drivers_iso.iter()) {
        if !Path::new(iso).exists() { return Err(anyhow!("Disc image not found: {iso}")); }
    }

    let mut env = accel::detect_env(cfg.arch);
    if cfg.os_family == crate::models::OsFamily::Windows {
        if let Some(sb) = accel::find_firmware_secure(cfg.arch) {
            env.firmware = Some(sb);
        }
    }
    let qmp_port = free_port()?;
    let plan = qemu::build_plan(&cfg, &vm_dir, qmp_port, &env).map_err(|e| anyhow!(e))?;

    if let Some((template, dst)) = &plan.vars_copy {
        if !dst.exists() { tokio::fs::copy(template, dst).await.context("copying UEFI variable store")?; }
    }

    let mut swtpm_child = None;
    if let Some(t) = &plan.swtpm {
        tokio::fs::create_dir_all(&t.state_dir).await?;
        let _ = tokio::fs::remove_file(&t.socket).await;
        let mut cmd = Command::new(&t.program);
        quiet(&mut cmd).args(&t.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true);
        let child = cmd.spawn().context("starting swtpm")?;
        for _ in 0..50 {
            if t.socket.exists() { break; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if !t.socket.exists() {
            let mut c = child; let _ = c.start_kill();
            return Err(anyhow!("swtpm didn't start. Check that swtpm is installed and the machine folder is writable."));
        }
        swtpm_child = Some(child);
    }

    let log_path = vm_dir.join("qemu.log");
    let log = std::fs::File::create(&log_path)?;
    let mut cmd = Command::new(&plan.program);
    quiet(&mut cmd).args(&plan.args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::from(log));
    let mut child = cmd.spawn().with_context(|| format!("starting {}", plan.program.display()))?;

    // Catch immediate failures (bad accelerator, missing firmware...) and show QEMU's own message.
    tokio::time::sleep(Duration::from_millis(1200)).await;
    if let Some(status) = child.try_wait()? {
        if let Some(mut t) = swtpm_child { let _ = t.start_kill(); }
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        return Err(anyhow!("QEMU exited immediately ({status}). {}", tail(&log, 700)));
    }

    state.running.lock().unwrap().insert(cfg.id.clone(), RunningVm { child, swtpm: swtpm_child, qmp_port, paused: false });
    let mut notes = auto_notes;
    notes.extend(plan.notes);
    Ok(notes)
}

fn port_of(state: &AppState, id: &str) -> Result<u16> {
    reap(state);
    state.running.lock().unwrap().get(id).map(|r| r.qmp_port).ok_or_else(|| anyhow!("That machine isn't running."))
}

pub async fn stop_vm(state: &AppState, id: &str, force: bool) -> Result<()> {
    let port = match port_of(state, id) {
        Ok(p) => p,
        Err(e) => {
            reap(state);
            return Err(e);
        }
    };
    let cmd = if force { "quit" } else { "system_powerdown" };
    match qmp(port, cmd).await {
        Ok(()) => {
            if force {
                tokio::time::sleep(Duration::from_millis(500)).await;
                reap(state);
            }
            Ok(())
        }
        Err(e) if force => {
            if let Some(mut v) = state.running.lock().unwrap().remove(id) {
                let _ = v.child.start_kill();
                if let Some(t) = v.swtpm.as_mut() { let _ = t.start_kill(); }
            }
            Err(anyhow!("QMP failed ({e}); the process was killed."))
        }
        Err(e) => Err(e),
    }
}

pub async fn pause_vm(state: &AppState, id: &str) -> Result<()> {
    let port = port_of(state, id)?;
    // QEMU accepts "stop" even if already paused; always mark our flag.
    qmp(port, "stop").await?;
    if let Some(r) = state.running.lock().unwrap().get_mut(id) { r.paused = true; }
    Ok(())
}

pub async fn resume_vm(state: &AppState, id: &str) -> Result<()> {
    let port = port_of(state, id)?;
    // Always send cont — fixes desync when QEMU was paused from its own menu
    // while OmniBox still thought the VM was running.
    qmp(port, "cont").await?;
    if let Some(r) = state.running.lock().unwrap().get_mut(id) { r.paused = false; }
    Ok(())
}

/// Read one QMP reply line that contains "return" or "error". Returns the raw line.
async fn read_reply_line(lines: &mut Lines<BufReader<OwnedReadHalf>>) -> Result<String> {
    loop {
        let line = timeout(Duration::from_secs(5), lines.next_line()).await??.ok_or_else(|| anyhow!("QMP connection closed"))?;
        if line.contains("\"return\"") || line.contains("\"error\"") {
            if line.contains("\"error\"") {
                return Err(anyhow!("QEMU rejected the command: {line}"));
            }
            return Ok(line);
        }
        // async event — keep reading
    }
}

async fn qmp(port: u16, execute: &str) -> Result<()> {
    let _ = qmp_raw(port, execute).await?;
    Ok(())
}

async fn qmp_raw(port: u16, execute: &str) -> Result<String> {
    let stream = timeout(Duration::from_secs(3), TcpStream::connect(("127.0.0.1", port))).await??;
    let (r, mut w) = stream.into_split();
    let mut lines = BufReader::new(r).lines();
    let _ = timeout(Duration::from_secs(3), lines.next_line()).await??; // greeting
    w.write_all(b"{\"execute\":\"qmp_capabilities\"}\n").await?;
    read_reply_line(&mut lines).await?;
    w.write_all(format!("{{\"execute\":\"{execute}\"}}\n").as_bytes()).await?;
    read_reply_line(&mut lines).await
}

/// Ask QEMU whether the VM is paused. Falls back to our cached flag on QMP failure.
async fn qmp_is_paused(port: u16) -> Option<bool> {
    let reply = qmp_raw(port, "query-status").await.ok()?;
    // Typical: {"return":{"status":"running","singlestep":false,"running":true}}
    // or status "paused" / "running": false
    let lower = reply.to_lowercase();
    if lower.contains("\"status\":\"paused\"") || lower.contains("\"status\": \"paused\"") {
        return Some(true);
    }
    if lower.contains("\"running\":false") || lower.contains("\"running\": false") {
        return Some(true);
    }
    if lower.contains("\"status\":\"running\"") || lower.contains("\"running\":true") {
        return Some(false);
    }
    None
}

/// Sync each running VM's paused flag from live QMP status (fixes menu/keyboard pause desync).
pub async fn sync_pause_flags(state: &AppState) {
    reap(state);
    let ports: Vec<(String, u16)> = state
        .running
        .lock()
        .unwrap()
        .iter()
        .map(|(id, v)| (id.clone(), v.qmp_port))
        .collect();
    for (id, port) in ports {
        if let Some(paused) = qmp_is_paused(port).await {
            if let Some(r) = state.running.lock().unwrap().get_mut(&id) {
                r.paused = paused;
            }
        }
    }
}


/// Fallback CPU% from /proc when sysinfo still reports 0 (common right after spawn).
/// Returns percent of **one** core (100 = fully busy core), then caller scales to host.
fn proc_stat_ticks(pid: u32) -> Option<(u64, u64)> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // after comm in parens: fields 14=utime 15=stime (1-based in man proc)
    let rpar = s.rfind(')')?;
    let rest = s[rpar + 1..].split_whitespace().collect::<Vec<_>>();
    if rest.len() < 13 { return None; }
    let utime: u64 = rest[11].parse().ok()?;
    let stime: u64 = rest[12].parse().ok()?;
    Some((utime, stime))
}

pub async fn summaries(state: &AppState, configs: Vec<VmConfig>) -> Vec<VmSummary> {
    sync_pause_flags(state).await;
    reap(state);

    // Snapshot /proc ticks, wait, snapshot again → real CPU% (sysinfo alone often stays 0 here).
    let pid_list: Vec<(String, u32, bool)> = {
        let running = state.running.lock().unwrap();
        running
            .iter()
            .filter_map(|(id, r)| r.child.id().map(|p| (id.clone(), p, r.paused)))
            .collect()
    };
    let ticks1: std::collections::HashMap<u32, (u64, u64)> = pid_list
        .iter()
        .filter_map(|(_, pid, _)| proc_stat_ticks(*pid).map(|t| (*pid, t)))
        .collect();
    tokio::time::sleep(Duration::from_millis(250)).await;
    let ticks2: std::collections::HashMap<u32, (u64, u64)> = pid_list
        .iter()
        .filter_map(|(_, pid, _)| proc_stat_ticks(*pid).map(|t| (*pid, t)))
        .collect();

    let hz = {
        // USER_HZ is almost always 100 on Linux
        100.0_f32
    };
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f32;
    let sample_secs = 0.25_f32;

    let mut cpu_map: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    for (id, pid, paused) in &pid_list {
        if *paused {
            cpu_map.insert(id.clone(), 0.0);
            continue;
        }
        let pct = match (ticks1.get(pid), ticks2.get(pid)) {
            (Some((u1, s1)), Some((u2, s2))) if *u2 >= *u1 && *s2 >= *s1 => {
                let delta = (u2 - u1 + s2 - s1) as f32;
                // fraction of one core during sample window, then as % of whole host
                let one_core = (delta / hz / sample_secs).clamp(0.0, cores);
                ((one_core / cores) * 100.0).clamp(0.0, 100.0)
            }
            _ => 0.0,
        };
        cpu_map.insert(id.clone(), pct);
    }

    let running = state.running.lock().unwrap();
    let pids: Vec<Pid> = running.values().filter_map(|r| r.child.id()).map(Pid::from_u32).collect();
    let mut sys = state.sys.lock().unwrap();
    sys.refresh_processes(ProcessesToUpdate::Some(&pids), true);

    configs
        .into_iter()
        .map(|cfg| {
            let (status, cpu, mem) = match running.get(&cfg.id) {
                Some(r) => {
                    let proc_ = r.child.id().and_then(|pid| sys.process(Pid::from_u32(pid)));
                    let mem = proc_.map(|p| p.memory() / 1_048_576).unwrap_or(0);
                    let mut cpu = cpu_map.get(&cfg.id).copied().unwrap_or(0.0);
                    // Fallback to sysinfo if /proc sample failed
                    if cpu < 0.05 {
                        if let Some(p) = proc_ {
                            let raw = p.cpu_usage();
                            if raw > 0.0 {
                                cpu = (raw / cores).clamp(0.0, 100.0);
                            }
                        }
                    }
                    (if r.paused { VmStatus::Paused } else { VmStatus::Running }, cpu, mem)
                }
                None => (VmStatus::Stopped, 0.0, 0),
            };
            let disk_created = Path::new(&cfg.disk.path).exists();
            VmSummary { config: cfg, status, cpu_percent: cpu, mem_mb: mem, disk_created }
        })
        .collect()
}
