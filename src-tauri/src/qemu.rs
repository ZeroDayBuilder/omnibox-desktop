//! Translates a `VmConfig` into a QEMU command line. Pure and host-injected so it can be unit-tested.
//!
//! Compatibility rules baked in here:
//!  * Windows guests get SATA + Intel e1000e + plain VGA so Setup boots without virtio drivers
//!    (avoids INACCESSIBLE_BOOT_DEVICE). If a virtio-win ISO is present it is attached as a second CD.
//!  * Windows 11 forces UEFI and a software TPM 2.0 (swtpm).
//!  * Linux/BSD guests get virtio disk, NIC and GPU.
//!  * aarch64 guests always use UEFI (no BIOS exists on the `virt` machine).

use crate::models::*;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HostOs { Linux, Macos, Windows }

#[derive(Clone, Debug)]
pub struct FirmwarePaths { pub code: PathBuf, pub vars_template: PathBuf }

#[derive(Clone, Debug)]
pub struct HostEnv {
    pub os: HostOs,
    pub arch: Arch,
    pub kvm: bool,
    pub qemu: Option<PathBuf>,
    pub swtpm: Option<PathBuf>,
    pub firmware: Option<FirmwarePaths>,
}

#[derive(Clone, Debug)]
pub struct SwtpmPlan { pub program: PathBuf, pub args: Vec<String>, pub socket: PathBuf, pub state_dir: PathBuf }

#[derive(Clone, Debug)]
pub struct LaunchPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub swtpm: Option<SwtpmPlan>,
    /// (template, per-VM copy) of the UEFI variable store, copied on first boot.
    pub vars_copy: Option<(PathBuf, PathBuf)>,
    pub notes: Vec<String>,
}

fn p(path: &Path) -> String { path.to_string_lossy().into_owned() }

/// Commas must be doubled inside QEMU option values.
fn esc(s: &str) -> String { s.replace(',', ",,") }

fn legacy_os(cfg: &VmConfig) -> bool { cfg.os_family == OsFamily::Windows && cfg.os_version == "7" }

pub fn build_plan(cfg: &VmConfig, vm_dir: &Path, qmp_port: u16, env: &HostEnv) -> Result<LaunchPlan, String> {
    let program = env.qemu.clone().ok_or_else(|| format!(
        "QEMU (qemu-system-{}) wasn't found. Install QEMU, then restart OmniBox.", cfg.arch.qemu_suffix()))?;
    if cfg.os_family == OsFamily::Macos {
        return Err("macOS guests aren't supported.".into());
    }
    let mut notes: Vec<String> = Vec::new();
    let mut a: Vec<String> = Vec::new();
    let mut push = |items: &[&str]| a.extend(items.iter().map(|s| s.to_string()));
    let is_win = cfg.os_family == OsFamily::Windows;
    let x86 = cfg.arch == Arch::X86_64;

    // ---- Acceleration ----------------------------------------------------------------------
    let native = cfg.arch == env.arch;
    let hw = cfg.acceleration == Accel::Auto && native && (env.os != HostOs::Linux || env.kvm);
    let accel = if hw {
        match env.os { HostOs::Linux => "kvm", HostOs::Macos => "hvf", HostOs::Windows => "whpx,kernel-irqchip=off" }
    } else {
        if cfg.acceleration == Accel::Auto {
            notes.push(if native {
                "Hardware acceleration isn't available (is /dev/kvm accessible?). Running in slow software emulation.".into()
            } else {
                format!("Guest architecture ({}) differs from the host, so hardware acceleration can't be used. Expect slow performance.", cfg.arch.qemu_suffix())
            });
        }
        "tcg,thread=multi"
    };
    push(&["-name", &cfg.name]);
    push(&["-accel", accel]);
    // Windows (esp. 11) Secure Boot needs SMM on q35.
    let machine = if x86 {
        if is_win { "q35,smm=on" } else { "q35" }
    } else {
        "virt"
    };
    push(&["-machine", machine]);
    push(&["-cpu", if hw && !accel.starts_with("whpx") { "host" } else { "max" }]);
    push(&["-smp", &cfg.cpu_cores.to_string(), "-m", &cfg.ram_mb.to_string()]);
    if is_win { push(&["-rtc", "base=localtime"]); }

    // ---- Firmware: UEFI vs legacy BIOS -----------------------------------------------------
    // Win7 → legacy BIOS. Win8/10/Server → UEFI preferred. Win11 → UEFI + Secure Boot firmware.
    let win11 = is_win && (cfg.os_version.contains("11") || cfg.os_version.contains("10/11"));
    let required_uefi = !x86 || win11;
    let want_uefi = match cfg.firmware {
        Firmware::Uefi => true,
        Firmware::Bios => false,
        Firmware::Auto => !legacy_os(cfg),
    };
    let mut vars_copy = None;
    if required_uefi || want_uefi {
        // Firmware comes only from HostEnv (injected by start_vm / tests) so build_plan stays pure.
        match env.firmware.clone() {
            Some(fw) => {
                let code_s = fw.code.to_string_lossy().to_lowercase();
                let secure = code_s.contains("secboot") || code_s.contains(".ms.") || code_s.contains("_ms");
                // Separate vars file for SB so an old non-SB NVRAM store is not reused.
                let vars = vm_dir.join(if secure { "uefi-vars-sb.fd" } else { "uefi-vars.fd" });
                push(&["-drive", &format!("if=pflash,format=raw,unit=0,readonly=on,file={}", esc(&p(&fw.code)))]);
                push(&["-drive", &format!("if=pflash,format=raw,unit=1,file={}", esc(&p(&vars)))]);
                if secure {
                    // Tell the pflash device this image is a secure-boot capable firmware.
                    push(&["-global", "driver=cfi.pflash01,property=secure,value=on"]);
                    notes.push(format!("UEFI Secure Boot firmware: {}", fw.code.display()));
                } else if is_win {
                    notes.push(
                        "Secure Boot OVMF (*.secboot.fd / *.ms.fd) was not selected. Windows 11 Setup may still block. In Setup press Shift+F10 → type: reg import (see README LabConfig) or create HKLM\\SYSTEM\\Setup\\LabConfig DWORD BypassSecureBootCheck=1 and BypassTPMCheck=1.".into(),
                    );
                }
                vars_copy = Some((fw.vars_template.clone(), vars));
            }
            None if required_uefi || cfg.firmware == Firmware::Uefi => {
                return Err("UEFI firmware (OVMF/edk2) wasn't found. Install ovmf (Linux) and try again.".into());
            }
            None => notes.push("UEFI firmware not found; falling back to legacy BIOS.".into()),
        }
    }
    if cfg.firmware == Firmware::Bios && required_uefi {
        notes.push("This guest needs UEFI, so the BIOS setting was ignored.".into());
    }

    // ---- TPM 2.0 ---------------------------------------------------------------------------
    let mut swtpm = None;
    if cfg.tpm {
        match (&env.swtpm, env.os) {
            (Some(bin), HostOs::Linux | HostOs::Macos) => {
                let state_dir = vm_dir.join("tpm");
                let socket = state_dir.join("swtpm.sock");
                swtpm = Some(SwtpmPlan {
                    program: bin.clone(),
                    args: vec![
                        "socket".into(), "--tpm2".into(),
                        "--tpmstate".into(), format!("dir={}", p(&state_dir)),
                        "--ctrl".into(), format!("type=unixio,path={}", p(&socket)),
                        "--flags".into(), "not-need-init,startup-clear".into(),
                    ],
                    socket: socket.clone(), state_dir,
                });
                push(&["-chardev", &format!("socket,id=chrtpm,path={}", esc(&p(&socket)))]);
                push(&["-tpmdev", "emulator,id=tpm0,chardev=chrtpm"]);
                push(&["-device", if x86 { "tpm-tis,tpmdev=tpm0" } else { "tpm-tis-device,tpmdev=tpm0" }]);
            }
            (None, _) => notes.push("TPM 2.0 is enabled but swtpm isn't installed. Install swtpm (Linux: apt/dnf/pacman, macOS: brew install swtpm) or Windows 11 Setup will reject this machine.".into()),
            (Some(_), HostOs::Windows) => notes.push("swtpm doesn't run on Windows hosts, so TPM 2.0 can't be emulated here. Use the Windows 11 LabConfig bypass described in the README.".into()),
        }
    }

    // ---- Graphics --------------------------------------------------------------------------
    let gl = cfg.gl_3d && env.os == HostOs::Linux && cfg.os_family == OsFamily::Linux;
    if cfg.gl_3d && !gl {
        notes.push("3D acceleration is only available for Linux guests on Linux hosts; it was turned off.".into());
    }
    // Do not pin xres/yres on virtio — many live ISOs (Kodachi, some Kali/Ubuntu) only fill
    // part of the window when the host forces a fixed framebuffer size.
    let gpu = match (x86, is_win) {
        (true, true) => format!("VGA,vgamem_mb={}", cfg.vram_mb.next_power_of_two().clamp(16, 256)),
        (true, false) => if gl { "virtio-vga-gl".into() } else { "virtio-vga".into() },
        (false, true) => "ramfb".to_string(),
        (false, false) => if gl { "virtio-gpu-gl-pci".into() } else { "virtio-gpu-pci".into() },
    };
    push(&["-device", &gpu]);
    // Display backends: cocoa on macOS; gtk on Linux with zoom-to-fit so small guest
    // resolutions (and half-screen installers) still fill the window.
    let backend = match env.os {
        HostOs::Macos => "cocoa".to_string(),
        HostOs::Windows => if gl { "gtk,gl=on,zoom-to-fit=on".into() } else { "sdl".into() },
        HostOs::Linux => if gl { "gtk,gl=on,zoom-to-fit=on".into() } else { "gtk,zoom-to-fit=on".into() },
    };
    push(&["-display", &backend]);

    // ---- Input & USB -----------------------------------------------------------------------
    push(&["-device", "qemu-xhci,id=xhci", "-device", "usb-kbd", "-device", "usb-tablet"]);
    for u in &cfg.usb_devices {
        if u.vendor_id.trim().is_empty() || u.product_id.trim().is_empty() { continue; }
        let hex = |s: &str| s.trim_start_matches("0x").chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>();
        push(&["-device", &format!("usb-host,vendorid=0x{},productid=0x{}", hex(&u.vendor_id), hex(&u.product_id))]);
    }

    // ---- Audio -----------------------------------------------------------------------------
    let audio = match env.os { HostOs::Linux => "pa", HostOs::Macos => "coreaudio", HostOs::Windows => "dsound" };
    if x86 {
        push(&["-audiodev", &format!("{audio},id=snd0"), "-device", "intel-hda", "-device", "hda-duplex,audiodev=snd0"]);
    } else {
        // virt machine: use virtio-sound when available; fall back to nothing rather than broken HDA
        push(&["-audiodev", &format!("{audio},id=snd0")]);
        push(&["-device", "virtio-sound-pci,audiodev=snd0"]);
    }

    // ---- Storage ---------------------------------------------------------------------------
    // Auto: SATA for x86 Windows (Setup sees the disk without virtio drivers); virtio elsewhere.
    // aarch64 virt has no ICH9 AHCI; always use virtio-blk and SCSI CD for ISOs.
    let bus = match cfg.storage_bus {
        StorageBus::Auto => if is_win && x86 { StorageBus::Sata } else { StorageBus::Virtio },
        b => b,
    };
    push(&["-drive", &format!("file={},if=none,id=hd0,format={},cache=writeback,discard=unmap", esc(&cfg.disk.path), cfg.disk.format.qemu())]);
    let has_iso = cfg.iso_path.is_some();
    // Empty qcow2 is a few hundred KB. After a real OS install the file grows well past 32 MB.
    // Prefer the hard disk once it looks installed so reboot does not land back in the ISO installer.
    let disk_has_os = std::fs::metadata(&cfg.disk.path).map(|m| m.len() > 2 * 1024 * 1024).unwrap_or(false);
    let (disk_boot, cd_boot) = if has_iso && !disk_has_os {
        (2u32, 1u32) // fresh install: boot the ISO first
    } else if has_iso && disk_has_os {
        notes.push("Installer ISO is still attached, but the disk already has an OS — booting the disk first. Eject the ISO in Settings → Storage when you are done installing.".into());
        (1u32, 2u32)
    } else {
        (1u32, 2u32)
    };
    if x86 {
        push(&["-device", "ich9-ahci,id=ahci"]);
        match bus {
            StorageBus::Sata => push(&["-device", &format!("ide-hd,drive=hd0,bus=ahci.0,bootindex={disk_boot}")]),
            _ => push(&["-device", &format!("virtio-blk-pci,drive=hd0,bootindex={disk_boot}")]),
        }
        if let Some(iso) = &cfg.iso_path {
            push(&["-drive", &format!("file={},media=cdrom,if=none,id=cd0,readonly=on", esc(iso))]);
            push(&["-device", &format!("ide-cd,drive=cd0,bus=ahci.1,bootindex={cd_boot}")]);
        }
        if let Some(drv) = cfg.drivers_iso.as_ref().filter(|_| is_win) {
            push(&["-drive", &format!("file={},media=cdrom,if=none,id=cd1,readonly=on", esc(drv))]);
            push(&["-device", "ide-cd,drive=cd1,bus=ahci.2"]);
            notes.push("virtio-win drivers are attached as a second CD. Switch the disk to virtio after installing them.".into());
        }
    } else {
        // aarch64: virtio-blk + virtio-scsi for CDs
        push(&["-device", &format!("virtio-blk-pci,drive=hd0,bootindex={disk_boot}")]);
        push(&["-device", "virtio-scsi-pci,id=scsi0"]);
        if let Some(iso) = &cfg.iso_path {
            push(&["-drive", &format!("file={},media=cdrom,if=none,id=cd0,readonly=on", esc(iso))]);
            push(&["-device", &format!("scsi-cd,drive=cd0,bus=scsi0.0,bootindex={cd_boot}")]);
        }
    }

    // ---- Network & shared folders ----------------------------------------------------------
    let nic = if is_win { "e1000e" } else { "virtio-net-pci" };
    let mut shared = cfg.shared_folders.clone();
    match cfg.network {
        NetMode::Nat => {
            let mut nd = "user,id=net0,dns=8.8.8.8".to_string();
            if env.os != HostOs::Linux {
                if let Some(first) = shared.first() {
                    nd.push_str(&format!(",smb={}", esc(&first.host_path)));
                    if shared.len() > 1 { notes.push("Only the first shared folder is exposed on this host (SMB limit).".into()); }
                    notes.push("Shared folder is served over SMB at \\\\10.0.2.4\\qemu inside the guest (requires Samba on the host).".into());
                }
                shared.clear();
            }
            push(&["-netdev", &nd]);
        }
        NetMode::Bridged => {
            if cfg.bridge_iface.trim().is_empty() {
                return Err("Bridged networking needs a host interface name (for example br0, en0 or a TAP adapter).".into());
            }
            let nd = match env.os {
                HostOs::Linux => format!("bridge,id=net0,br={}", esc(&cfg.bridge_iface)),
                HostOs::Macos => format!("vmnet-bridged,id=net0,ifname={}", esc(&cfg.bridge_iface)),
                HostOs::Windows => format!("tap,id=net0,ifname={}", esc(&cfg.bridge_iface)),
            };
            push(&["-netdev", &nd]);
            if env.os != HostOs::Linux { shared.clear(); }
        }
    }
    push(&["-device", &format!("{nic},netdev=net0")]);
    for (i, f) in shared.iter().enumerate() {
        push(&["-virtfs", &format!("local,id=fs{i},path={},mount_tag={},security_model=mapped-xattr", esc(&f.host_path), esc(&f.name))]);
    }

    // ---- Control channel -------------------------------------------------------------------
    push(&["-boot", "menu=on,strict=off"]);
    push(&["-qmp", &format!("tcp:127.0.0.1:{qmp_port},server=on,wait=off")]);

    Ok(LaunchPlan { program, args: a, swtpm, vars_copy, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(family: OsFamily, version: &str) -> VmConfig {
        VmConfig {
            id: "abc".into(), name: "t".into(), os_family: family, os_name: "x".into(), os_version: version.into(),
            arch: Arch::X86_64, base_folder: "/tmp".into(), firmware: Firmware::Auto, tpm: version == "11",
            ram_mb: 4096, cpu_cores: 2, acceleration: Accel::Auto, vram_mb: 64, gl_3d: false, width: 1920, height: 1080,
            disk: DiskConfig { path: "/tmp/abc/disk.qcow2".into(), format: DiskFormat::Qcow2, size_gb: 40 },
            iso_path: Some("/tmp/a.iso".into()), drivers_iso: None, storage_bus: StorageBus::Auto, network: NetMode::Nat,
            bridge_iface: String::new(), usb_devices: vec![], shared_folders: vec![],
        }
    }
    fn env() -> HostEnv {
        HostEnv {
            os: HostOs::Linux, arch: Arch::X86_64, kvm: true,
            qemu: Some("/usr/bin/qemu-system-x86_64".into()), swtpm: Some("/usr/bin/swtpm".into()),
            firmware: Some(FirmwarePaths { code: "/fw/code.fd".into(), vars_template: "/fw/vars.fd".into() }),
        }
    }
    fn joined(p: &LaunchPlan) -> String { p.args.join(" ") }

    #[test]
    fn windows_11_gets_uefi_tpm_and_sata() {
        let plan = build_plan(&cfg(OsFamily::Windows, "11"), Path::new("/tmp/abc"), 4444, &env()).unwrap();
        let s = joined(&plan);
        assert!(s.contains("if=pflash") && s.contains("tpm-tis") && s.contains("ide-hd") && s.contains("e1000e"));
        assert!(!s.contains("virtio-blk") && plan.swtpm.is_some());
    }
    #[test]
    fn linux_gets_virtio_and_kvm() {
        let plan = build_plan(&cfg(OsFamily::Linux, "24.04"), Path::new("/tmp/abc"), 4444, &env()).unwrap();
        let s = joined(&plan);
        assert!(s.contains("-accel kvm") && s.contains("virtio-blk-pci") && s.contains("virtio-net-pci") && s.contains("virtio-vga"));
    }
    #[test]
    fn windows_7_uses_legacy_bios() {
        let plan = build_plan(&cfg(OsFamily::Windows, "7"), Path::new("/tmp/abc"), 4444, &env()).unwrap();
        assert!(!joined(&plan).contains("pflash"));
    }
    #[test]
    fn missing_firmware_is_an_error_for_win11() {
        let mut e = env(); e.firmware = None;
        assert!(build_plan(&cfg(OsFamily::Windows, "11"), Path::new("/tmp/abc"), 4444, &e).is_err());
    }
    #[test]
    fn falls_back_to_tcg_without_kvm() {
        let mut e = env(); e.kvm = false;
        let plan = build_plan(&cfg(OsFamily::Linux, "24.04"), Path::new("/tmp/abc"), 4444, &e).unwrap();
        assert!(joined(&plan).contains("tcg") && !plan.notes.is_empty());
    }
    #[test]
    fn aarch64_uses_virt_machine_and_virtio() {
        let mut c = cfg(OsFamily::Linux, "24.04");
        c.arch = Arch::Aarch64;
        let mut e = env();
        e.arch = Arch::Aarch64;
        e.qemu = Some("/usr/bin/qemu-system-aarch64".into());
        e.firmware = Some(FirmwarePaths { code: "/fw/aa64-code.fd".into(), vars_template: "/fw/aa64-vars.fd".into() });
        let plan = build_plan(&c, Path::new("/tmp/abc"), 4444, &e).unwrap();
        let s = joined(&plan);
        assert!(s.contains("-machine virt") && s.contains("virtio-blk") && s.contains("pflash"));
        assert!(!s.contains("ich9-ahci"));
    }

}
