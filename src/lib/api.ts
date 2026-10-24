import { invoke } from "@tauri-apps/api/core";
import type { HostInfo, IsoInfo, VmConfig, VmSummary, DiskFormat } from "./types";

export function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return String(e);
}

export const api = {
  hostInfo: () => invoke<HostInfo>("host_info"),
  listVms: () => invoke<VmSummary[]>("list_vms"),
  inspectIso: (path: string) => invoke<IsoInfo>("inspect_iso", { path }),
  createFromIso: (req: {
    isoPath: string;
    name: string;
    cpuCores: number;
    ramMb: number;
    diskGb: number;
  }) => invoke<VmConfig>("create_vm_from_iso", { req }),
  save: (config: VmConfig) => invoke<void>("save_vm", { config }),
  remove: (id: string, deleteFiles: boolean) => invoke<void>("delete_vm", { id, deleteFiles }),
  start: (id: string) => invoke<string[]>("start_vm", { id }),
  stop: (id: string, force: boolean) => invoke<void>("stop_vm", { id, force }),
  pause: (id: string) => invoke<void>("pause_vm", { id }),
  resume: (id: string) => invoke<void>("resume_vm", { id }),
  createDisk: (path: string, format: DiskFormat, sizeGb: number) =>
    invoke<void>("create_disk", { path, format, sizeGb }),
};
