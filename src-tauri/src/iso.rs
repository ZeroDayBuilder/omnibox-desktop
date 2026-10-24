//! ISO inspection: reads the ISO-9660 volume label plus the file name,
//! guesses the guest OS and proposes sensible hardware.

use crate::models::{Arch, OsFamily};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::Path};

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Recommended {
    pub cpu_cores: u32,
    pub ram_mb: u32,
    pub disk_gb: u32,
    pub vram_mb: u32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IsoInfo {
    pub path: String,
    pub file_name: String,
    pub size_mb: u64,
    pub volume_label: String,
    pub os_family: OsFamily,
    pub os_name: String,
    pub os_version: String,
    pub arch: Arch,
    pub needs_tpm: bool,
    pub supported: bool,
    pub recommended: Recommended,
    pub warnings: Vec<String>,
}

fn volume_label(path: &Path) -> String {
    let Ok(mut f) = File::open(path) else { return String::new() };
    let mut buf = [0u8; 2048];
    // Primary Volume Descriptor lives at sector 16.
    if f.seek(SeekFrom::Start(16 * 2048)).is_err() || f.read_exact(&mut buf).is_err() {
        return String::new();
    }
    if &buf[1..6] != b"CD001" { return String::new(); }
    String::from_utf8_lossy(&buf[40..72]).trim().to_string()
}

const DISTROS: &[(&str, &str)] = &[
    ("endeavouros", "EndeavourOS"), ("manjaro", "Manjaro"), ("archlinux", "Arch Linux"),
    ("arch_2", "Arch Linux"), ("linuxmint", "Linux Mint"), ("pop-os", "Pop!_OS"), ("pop_os", "Pop!_OS"),
    ("kali", "Kali Linux"), ("ubuntu", "Ubuntu"), ("debian", "Debian"), ("fedora", "Fedora"),
    ("opensuse", "openSUSE"), ("tumbleweed", "openSUSE"), ("leap", "openSUSE"),
    ("centos", "CentOS"), ("rocky", "Rocky Linux"), ("almalinux", "AlmaLinux"),
    ("nixos", "NixOS"), ("gentoo", "Gentoo"), ("alpine", "Alpine Linux"),
    ("elementary", "elementary OS"), ("zorin", "Zorin OS"), ("tails", "Tails"),
    ("garuda", "Garuda Linux"), ("solus", "Solus"), ("void", "Void Linux"),
    ("slackware", "Slackware"), ("mx-linux", "MX Linux"), ("mxlinux", "MX Linux"),
    ("deepin", "Deepin"), ("nobara", "Nobara"), ("cachyos", "CachyOS"),
];

fn first_version(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || (c == '.' && !out.is_empty()) { out.push(c); }
        else if !out.is_empty() { break; }
    }
    out.trim_end_matches('.').to_string()
}

pub fn detect(file_name: &str, label: &str) -> (OsFamily, String, String, Arch) {
    let hay = format!("{} {}", file_name, label).to_lowercase().replace(['_', '-'], " ");
    let raw = format!("{} {}", file_name, label).to_lowercase();
    let arch = if hay.contains("arm64") || hay.contains("aarch64") || hay.contains("a64fre") { Arch::Aarch64 } else { Arch::X86_64 };

    if ["macos", "mac os", "osx", "sequoia", "sonoma", "ventura", "monterey", "big sur"].iter().any(|k| hay.contains(k)) {
        return (OsFamily::Macos, "macOS".into(), first_version(file_name), arch);
    }
    let is_windows = hay.contains("windows") || hay.contains("win11") || hay.contains("win10") || hay.contains("win7")
        || hay.contains("cccoma") || hay.contains("ccsa") || hay.contains("x64fre") || hay.contains("a64fre")
        || hay.contains("x86fre") || hay.contains("winpe") || hay.contains("sw_dvd") || hay.contains("client");
    if is_windows {
        let (name, ver) = if hay.contains("server") || hay.contains("srv") { ("Windows Server", "Server") }
            else if hay.contains("win11") || hay.contains("windows 11") || hay.contains("24h2") || hay.contains("23h2") { ("Windows 11", "11") }
            else if hay.contains("win10") || hay.contains("windows 10") || hay.contains("22h2") || hay.contains("21h2") { ("Windows 10", "10") }
            else if hay.contains("win7") || hay.contains("windows 7") || hay.contains("winxp") { ("Windows 7", "7") }
            else { ("Windows 10/11", "10/11") };
        return (OsFamily::Windows, name.into(), ver.into(), arch);
    }
    for (key, name) in DISTROS {
        if raw.contains(key) { return (OsFamily::Linux, (*name).into(), first_version(file_name), arch); }
    }
    for (key, name) in [("freebsd", "FreeBSD"), ("openbsd", "OpenBSD"), ("netbsd", "NetBSD")] {
        if hay.contains(key) { return (OsFamily::Bsd, name.into(), first_version(file_name), arch); }
    }
    if hay.contains("linux") { return (OsFamily::Linux, "Linux".into(), first_version(file_name), arch); }
    (OsFamily::Other, "Unknown OS".into(), String::new(), arch)
}

pub fn recommend(family: OsFamily, version: &str, host_cores: usize, host_ram_mb: u64) -> Recommended {
    let (cpu, ram, disk, vram) = match (family, version) {
        (OsFamily::Windows, "7") => (2, 2048, 40, 64),
        (OsFamily::Windows, "10") => (2, 4096, 64, 128),
        (OsFamily::Windows, "Server") => (4, 8192, 80, 128),
        (OsFamily::Windows, _) => (4, 8192, 80, 128), // Windows 11 / unknown modern
        (OsFamily::Linux, _) => (2, 4096, 40, 64),
        (OsFamily::Bsd, _) => (2, 2048, 20, 16),
        _ => (2, 2048, 20, 32),
    };
    // Leave the host some breathing room: half the logical cores (min 1) and half the RAM,
    // snapped down to a 256 MB boundary so the slider stays on nice values.
    let max_cpu = (host_cores / 2).max(1) as u32;
    let max_ram = ((host_ram_mb / 2) / 256 * 256).max(1024) as u32;
    Recommended { cpu_cores: cpu.min(max_cpu), ram_mb: ram.min(max_ram), disk_gb: disk, vram_mb: vram }
}

pub fn inspect(path: &Path, host_cores: usize, host_ram_mb: u64) -> Result<IsoInfo> {
    let meta = std::fs::metadata(path).map_err(|e| anyhow!("Can't read {}: {e}", path.display()))?;
    let file_name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    if !file_name.to_lowercase().ends_with(".iso") && !file_name.to_lowercase().ends_with(".img") {
        return Err(anyhow!("{file_name} isn't an ISO image."));
    }
    let label = volume_label(path);
    let (family, name, version, arch) = detect(&file_name, &label);
    let mut warnings = Vec::new();
    let mut supported = true;
    match family {
        OsFamily::Macos => {
            supported = false;
            warnings.push("macOS guests aren't supported: Apple's licence limits macOS to Apple hardware and QEMU can't boot it reliably.".into());
        }
        OsFamily::Other => warnings.push("Couldn't identify this OS. Generic settings were applied; adjust them in Settings if it doesn't boot.".into()),
        OsFamily::Windows if version == "10/11" => warnings.push("Windows version not detected. TPM 2.0 and UEFI are enabled so both Windows 10 and 11 can install.".into()),
        _ => {}
    }
    Ok(IsoInfo {
        path: path.to_string_lossy().into_owned(),
        file_name,
        size_mb: meta.len() / 1_048_576,
        volume_label: label,
        os_family: family,
        needs_tpm: family == OsFamily::Windows && (version == "11" || version == "10/11" || version == "Server"),
        recommended: recommend(family, &version, host_cores, host_ram_mb),
        os_name: name,
        os_version: version,
        arch,
        supported,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_windows_11() {
        let (f, n, v, _) = detect("Win11_24H2_English_x64.iso", "CCCOMA_X64FRE_EN-US_DV9");
        assert_eq!((f, n.as_str(), v.as_str()), (OsFamily::Windows, "Windows 11", "11"));
    }
    #[test]
    fn detects_ubuntu_with_version() {
        let (f, n, v, a) = detect("ubuntu-24.04.1-desktop-amd64.iso", "");
        assert_eq!((f, n.as_str(), v.as_str(), a), (OsFamily::Linux, "Ubuntu", "24.04.1", Arch::X86_64));
    }
    #[test]
    fn detects_arm() {
        assert_eq!(detect("Fedora-Workstation-Live-aarch64-41.iso", "").3, Arch::Aarch64);
    }
    #[test]
    fn caps_to_host() {
        let r = recommend(OsFamily::Windows, "11", 4, 8192);
        assert_eq!((r.cpu_cores, r.ram_mb), (2, 4096));
    }
}
