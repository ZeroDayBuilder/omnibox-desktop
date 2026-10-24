use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OsFamily { Windows, Linux, Macos, Bsd, Other }

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Arch { X86_64, Aarch64 }

impl Arch {
    pub fn qemu_suffix(self) -> &'static str {
        match self { Arch::X86_64 => "x86_64", Arch::Aarch64 => "aarch64" }
    }
}

/// Auto: UEFI when available and sensible, Legacy BIOS for very old guests.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Firmware { Auto, Uefi, Bios }

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DiskFormat { Qcow2, Vdi, Raw }

impl DiskFormat {
    pub fn qemu(self) -> &'static str {
        match self { DiskFormat::Qcow2 => "qcow2", DiskFormat::Vdi => "vdi", DiskFormat::Raw => "raw" }
    }
}

/// Auto: SATA for Windows (boots without extra drivers), virtio for everything else.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StorageBus { Auto, Virtio, Sata }

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum NetMode { Nat, Bridged }

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Accel { Auto, Software }

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum VmStatus { Running, Paused, Stopped }

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiskConfig {
    pub path: String,
    pub format: DiskFormat,
    pub size_gb: u32,
}

/// Hex ids without the 0x prefix, e.g. "046d" / "c52b".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsbId {
    pub vendor_id: String,
    pub product_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SharedFolder {
    pub name: String,
    pub host_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VmConfig {
    pub id: String,
    pub name: String,
    pub os_family: OsFamily,
    pub os_name: String,
    pub os_version: String,
    pub arch: Arch,
    pub base_folder: String,
    pub firmware: Firmware,
    pub tpm: bool,
    pub ram_mb: u32,
    pub cpu_cores: u32,
    pub acceleration: Accel,
    pub vram_mb: u32,
    pub gl_3d: bool,
    pub width: u32,
    pub height: u32,
    pub disk: DiskConfig,
    pub iso_path: Option<String>,
    pub drivers_iso: Option<String>,
    pub storage_bus: StorageBus,
    pub network: NetMode,
    #[serde(default)]
    pub bridge_iface: String,
    #[serde(default)]
    pub usb_devices: Vec<UsbId>,
    #[serde(default)]
    pub shared_folders: Vec<SharedFolder>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VmSummary {
    pub config: VmConfig,
    pub status: VmStatus,
    pub cpu_percent: f32,
    pub mem_mb: u64,
    pub disk_created: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub os: String,
    pub arch: Arch,
    pub cores: usize,
    pub ram_mb: u64,
    pub accel: String,
    pub qemu: Option<String>,
    pub swtpm: bool,
    pub uefi: bool,
    pub drivers_iso: bool,
    pub data_dir: String,
    pub default_base: String,
}
