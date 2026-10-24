export type OsFamily = "windows" | "linux" | "macos" | "bsd" | "other";
export type Arch = "x86_64" | "aarch64";
export type Firmware = "auto" | "uefi" | "bios";
export type DiskFormat = "qcow2" | "vdi" | "raw";
export type StorageBus = "auto" | "virtio" | "sata";
export type NetMode = "nat" | "bridged";
export type Accel = "auto" | "software";
export type VmStatus = "running" | "paused" | "stopped";

export interface DiskConfig {
  path: string;
  format: DiskFormat;
  sizeGb: number;
}

export interface UsbId {
  vendorId: string;
  productId: string;
}

export interface SharedFolder {
  name: string;
  hostPath: string;
}

export interface VmConfig {
  id: string;
  name: string;
  osFamily: OsFamily;
  osName: string;
  osVersion: string;
  arch: Arch;
  baseFolder: string;
  firmware: Firmware;
  tpm: boolean;
  ramMb: number;
  cpuCores: number;
  acceleration: Accel;
  vramMb: number;
  gl3d: boolean;
  width: number;
  height: number;
  disk: DiskConfig;
  isoPath: string | null;
  driversIso: string | null;
  storageBus: StorageBus;
  network: NetMode;
  bridgeIface: string;
  usbDevices: UsbId[];
  sharedFolders: SharedFolder[];
}

export interface VmSummary {
  config: VmConfig;
  status: VmStatus;
  cpuPercent: number;
  memMb: number;
  diskCreated: boolean;
}

export interface HostInfo {
  os: string;
  arch: Arch;
  cores: number;
  ramMb: number;
  accel: string;
  qemu: string | null;
  swtpm: boolean;
  uefi: boolean;
  driversIso: boolean;
  dataDir: string;
  defaultBase: string;
}

export interface Recommended {
  cpuCores: number;
  ramMb: number;
  diskGb: number;
  vramMb: number;
}

export interface IsoInfo {
  path: string;
  fileName: string;
  sizeMb: number;
  volumeLabel: string;
  osFamily: OsFamily;
  osName: string;
  osVersion: string;
  arch: Arch;
  needsTpm: boolean;
  supported: boolean;
  recommended: Recommended;
  warnings: string[];
}
