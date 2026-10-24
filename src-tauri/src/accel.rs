//! Host probing: OS/arch, hardware accelerator, and locating QEMU, swtpm and UEFI firmware.

use crate::models::Arch;
use crate::qemu::{FirmwarePaths, HostEnv, HostOs};
use std::path::{Path, PathBuf};

pub fn host_os() -> HostOs {
    if cfg!(target_os = "windows") { HostOs::Windows }
    else if cfg!(target_os = "macos") { HostOs::Macos }
    else { HostOs::Linux }
}

pub fn host_arch() -> Arch {
    if cfg!(target_arch = "aarch64") { Arch::Aarch64 } else { Arch::X86_64 }
}

/// Name of the hardware accelerator for this host, or "tcg" when none is usable.
pub fn accel_name() -> &'static str {
    match host_os() {
        HostOs::Linux => if Path::new("/dev/kvm").exists() { "KVM" } else { "Software (TCG)" },
        HostOs::Macos => "Hypervisor.framework",
        HostOs::Windows => "WHPX",
    }
}

fn extra_dirs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if let Ok(d) = std::env::var("OMNIBOX_QEMU_DIR") { v.push(d.into()); }
    match host_os() {
        HostOs::Windows => {
            v.push(r"C:\Program Files\qemu".into());
            v.push(r"C:\Program Files (x86)\qemu".into());
        }
        HostOs::Macos => { v.push("/opt/homebrew/bin".into()); v.push("/usr/local/bin".into()); }
        HostOs::Linux => { v.push("/usr/bin".into()); v.push("/usr/local/bin".into()); v.push("/usr/libexec".into()); }
    }
    v
}

pub fn find_tool(name: &str) -> Option<PathBuf> {
    for d in extra_dirs().iter().take(1) {
        if let Some(p) = probe(d, name) { return Some(p); }
    }
    if let Ok(p) = which::which(name) { return Some(p); }
    extra_dirs().iter().find_map(|d| probe(d, name))
}

fn probe(dir: &Path, name: &str) -> Option<PathBuf> {
    let file = if host_os() == HostOs::Windows { format!("{name}.exe") } else { name.to_string() };
    let p = dir.join(file);
    p.exists().then_some(p)
}

fn firmware_candidates(arch: Arch) -> Vec<(&'static str, &'static str)> {
    match (host_os(), arch) {
        (HostOs::Linux, Arch::X86_64) => vec![
            ("/usr/share/OVMF/OVMF_CODE_4M.ms.fd", "/usr/share/OVMF/OVMF_VARS_4M.ms.fd"),
            ("/usr/share/OVMF/OVMF_CODE_4M.secboot.fd", "/usr/share/OVMF/OVMF_VARS_4M.ms.fd"),
            ("/usr/share/OVMF/OVMF_CODE_4M.secboot.fd", "/usr/share/OVMF/OVMF_VARS_4M.secboot.fd"),
            ("/usr/share/OVMF/OVMF_CODE.secboot.fd", "/usr/share/OVMF/OVMF_VARS.secboot.fd"),
            ("/usr/share/OVMF/OVMF_CODE.ms.fd", "/usr/share/OVMF/OVMF_VARS.ms.fd"),
            ("/usr/share/edk2/ovmf/OVMF_CODE.secboot.fd", "/usr/share/edk2/ovmf/OVMF_VARS.secboot.fd"),
            ("/usr/share/edk2-ovmf/x64/OVMF_CODE.secboot.fd", "/usr/share/edk2-ovmf/x64/OVMF_VARS.secboot.fd"),
            ("/usr/share/OVMF/OVMF_CODE_4M.fd", "/usr/share/OVMF/OVMF_VARS_4M.fd"),
            ("/usr/share/OVMF/OVMF_CODE.fd", "/usr/share/OVMF/OVMF_VARS.fd"),
            ("/usr/share/edk2/ovmf/OVMF_CODE.fd", "/usr/share/edk2/ovmf/OVMF_VARS.fd"),
            ("/usr/share/edk2/x64/OVMF_CODE.4m.fd", "/usr/share/edk2/x64/OVMF_VARS.4m.fd"),
            ("/usr/share/edk2-ovmf/x64/OVMF_CODE.fd", "/usr/share/edk2-ovmf/x64/OVMF_VARS.fd"),
            ("/usr/share/qemu/edk2-x86_64-code.fd", "/usr/share/qemu/edk2-i386-vars.fd"),
        ],
        (HostOs::Linux, Arch::Aarch64) => vec![
            ("/usr/share/AAVMF/AAVMF_CODE.fd", "/usr/share/AAVMF/AAVMF_VARS.fd"),
            ("/usr/share/edk2/aarch64/QEMU_EFI-pflash.raw", "/usr/share/edk2/aarch64/vars-template-pflash.raw"),
            ("/usr/share/qemu/edk2-aarch64-code.fd", "/usr/share/qemu/edk2-arm-vars.fd"),
        ],
        (HostOs::Macos, Arch::X86_64) => vec![
            ("/opt/homebrew/share/qemu/edk2-x86_64-code.fd", "/opt/homebrew/share/qemu/edk2-i386-vars.fd"),
            ("/usr/local/share/qemu/edk2-x86_64-code.fd", "/usr/local/share/qemu/edk2-i386-vars.fd"),
        ],
        (HostOs::Macos, Arch::Aarch64) => vec![
            ("/opt/homebrew/share/qemu/edk2-aarch64-code.fd", "/opt/homebrew/share/qemu/edk2-arm-vars.fd"),
            ("/usr/local/share/qemu/edk2-aarch64-code.fd", "/usr/local/share/qemu/edk2-arm-vars.fd"),
        ],
        (HostOs::Windows, Arch::X86_64) => vec![
            (r"C:\Program Files\qemu\share\edk2-x86_64-code.fd", r"C:\Program Files\qemu\share\edk2-i386-vars.fd"),
        ],
        (HostOs::Windows, Arch::Aarch64) => vec![
            (r"C:\Program Files\qemu\share\edk2-aarch64-code.fd", r"C:\Program Files\qemu\share\edk2-arm-vars.fd"),
        ],
    }
}

fn is_secure_pair(code: &Path, vars: &Path) -> bool {
    let s = format!("{} {}", code.to_string_lossy(), vars.to_string_lossy()).to_lowercase();
    s.contains("secboot") || s.contains(".ms.") || s.contains("_ms.") || s.contains("-ms.")
}

pub fn find_firmware(arch: Arch) -> Option<FirmwarePaths> {
    firmware_candidates(arch).into_iter().find_map(|(c, v)| {
        let (c, v) = (PathBuf::from(c), PathBuf::from(v));
        (c.exists() && v.exists()).then_some(FirmwarePaths { code: c, vars_template: v })
    })
}

/// Prefer Secure Boot / MS-enrolled OVMF (required for Windows 11 Setup checks).
pub fn find_firmware_secure(arch: Arch) -> Option<FirmwarePaths> {
    // 1) Known path pairs that exist
    for (c, v) in firmware_candidates(arch) {
        let (c, v) = (PathBuf::from(c), PathBuf::from(v));
        if c.exists() && v.exists() && is_secure_pair(&c, &v) {
            return Some(FirmwarePaths { code: c, vars_template: v });
        }
    }
    // 2) Scan /usr/share/OVMF for any *secboot* / *.ms.* CODE+VARS pair
    if host_os() == HostOs::Linux && arch == Arch::X86_64 {
        let dir = Path::new("/usr/share/OVMF");
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut codes = Vec::new();
            let mut vars = Vec::new();
            for e in rd.flatten() {
                let n = e.file_name().to_string_lossy().to_lowercase();
                let p = e.path();
                if !n.ends_with(".fd") { continue; }
                if n.contains("code") && (n.contains("secboot") || n.contains(".ms.") || n.contains("_ms")) {
                    codes.push(p);
                } else if n.contains("vars") && (n.contains("secboot") || n.contains(".ms.") || n.contains("_ms")) {
                    vars.push(p);
                }
            }
            // Prefer 4M secboot, then 4M ms, then anything
            codes.sort_by(|a, b| {
                let score = |p: &Path| {
                    let n = p.to_string_lossy().to_lowercase();
                    let mut s = 0;
                    if n.contains("4m") { s += 10; }
                    if n.contains("ms") { s += 8; } // MS keys enrolled — Win11 Setup likes this
                    if n.contains("secboot") { s += 5; }
                    if n.contains("strict") { s -= 5; }
                    s
                };
                score(b).cmp(&score(a))
            });
            for c in &codes {
                // same "family" vars: 4m with 4m
                let c_4m = c.to_string_lossy().to_lowercase().contains("4m");
                if let Some(v) = vars.iter().find(|v| {
                    let vn = v.to_string_lossy().to_lowercase();
                    vn.contains("4m") == c_4m
                }) {
                    return Some(FirmwarePaths { code: c.clone(), vars_template: v.clone() });
                }
                if let Some(v) = vars.first() {
                    return Some(FirmwarePaths { code: c.clone(), vars_template: v.clone() });
                }
            }
        }
    }
    None
}

pub fn detect_env(guest: Arch) -> HostEnv {
    HostEnv {
        os: host_os(),
        arch: host_arch(),
        kvm: Path::new("/dev/kvm").exists(),
        qemu: find_tool(&format!("qemu-system-{}", guest.qemu_suffix())),
        swtpm: find_tool("swtpm"),
        firmware: find_firmware(guest),
    }
}
