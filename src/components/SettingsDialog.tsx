import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errText } from "../lib/api";
import type { HostInfo, OsFamily, VmConfig, VmSummary } from "../lib/types";
import { btn, Field, inputCls, Modal, Slider, Toggle } from "./ui";

const TABS = ["General", "System", "Display", "Storage", "Network", "USB & Shared folders"] as const;
type Tab = (typeof TABS)[number];
const RESOLUTIONS = ["1280x720", "1600x900", "1920x1080", "2560x1440"];
const basename = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;

interface Props { vm: VmSummary; host: HostInfo; onClose: () => void; onSaved: () => void; onDeleted: () => void }

export default function SettingsDialog({ vm, host, onClose, onSaved, onDeleted }: Props) {
  const [tab, setTab] = useState<Tab>("General");
  const [mode, setMode] = useState<"beginner" | "advanced">("beginner");
  const [d, setD] = useState<VmConfig>(vm.config);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const locked = vm.status !== "stopped";
  const set = (patch: Partial<VmConfig>) => setD((c) => ({ ...c, ...patch }));
  const setDisk = (patch: Partial<VmConfig["disk"]>) => setD((c) => ({ ...c, disk: { ...c.disk, ...patch } }));
  const win = d.osFamily === "windows";

  async function save() {
    try { await api.save(d); onSaved(); onClose(); } catch (e) { setError(errText(e)); }
  }
  async function pickIso(key: "isoPath" | "driversIso") {
    const p = await open({ multiple: false, filters: [{ name: "Disc image", extensions: ["iso", "img"] }] });
    if (typeof p === "string") set({ [key]: p } as Partial<VmConfig>);
  }
  async function addFolder() {
    const p = await open({ directory: true, multiple: false });
    if (typeof p === "string") set({ sharedFolders: [...d.sharedFolders, { name: basename(p).replace(/\W+/g, "_"), hostPath: p }] });
  }
  async function createDisk() {
    try { await api.createDisk(d.disk.path, d.disk.format, d.disk.sizeGb); setInfo("Disk created."); onSaved(); } catch (e) { setError(errText(e)); }
  }
  async function remove() {
    const files = window.confirm(`Delete "${d.name}" and its virtual disk? This can't be undone.\n\nOK = delete the machine and its files\nCancel = keep everything`);
    if (!files) return;
    try { await api.remove(d.id, true); onDeleted(); } catch (e) { setError(errText(e)); }
  }

  return (
    <Modal title={`${d.name} · Settings`} onClose={onClose} wide>
      <div className="flex min-h-0 flex-1">
        <nav className="w-52 shrink-0 space-y-0.5 border-r border-ink-700 p-3" aria-label="Settings sections">
          <div className="mb-3 flex gap-1 rounded-lg bg-ink-950 p-1">
            <button type="button" onClick={() => setMode("beginner")}
              className={`flex-1 rounded-md px-2 py-1.5 text-xs ${mode === "beginner" ? "bg-signal/20 text-signal-glow" : "text-fog-500"}`}>Beginner</button>
            <button type="button" onClick={() => setMode("advanced")}
              className={`flex-1 rounded-md px-2 py-1.5 text-xs ${mode === "advanced" ? "bg-signal/20 text-signal-glow" : "text-fog-500"}`}>Advanced</button>
          </div>
          {TABS.filter((t) => mode === "advanced" || t === "General" || t === "System" || t === "Storage").map((t) => (
            <button key={t} onClick={() => setTab(t)} aria-current={tab === t}
              className={`block w-full rounded-md px-3 py-2 text-left text-sm ${tab === t ? "bg-ink-600 text-fog-100" : "text-fog-300 hover:bg-ink-700"}`}>
              {t}
            </button>
          ))}
        </nav>

        <div className="min-h-[380px] flex-1 space-y-6 overflow-y-auto px-6 py-6">
          {locked && <p className="rounded-md border border-warn/40 bg-warn/10 px-3 py-2 text-sm text-warn">Shut the machine down to change its settings.</p>}

          {tab === "General" && (<>
            <Field label="Name"><input className={inputCls} disabled={locked} value={d.name} onChange={(e) => set({ name: e.target.value })} /></Field>
            <div className="grid grid-cols-2 gap-4">
              <Field label="Type">
                <select className={inputCls} disabled={locked} value={d.osFamily} onChange={(e) => set({ osFamily: e.target.value as OsFamily })}>
                  <option value="windows">Windows</option><option value="linux">Linux</option><option value="bsd">BSD</option><option value="other">Other</option>
                </select>
              </Field>
              <Field label="Architecture">
                <select className={inputCls} disabled={locked} value={d.arch} onChange={(e) => set({ arch: e.target.value as VmConfig["arch"] })}>
                  <option value="x86_64">x86-64 (Intel / AMD)</option><option value="aarch64">ARM64</option>
                </select>
              </Field>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <Field label="OS name"><input className={inputCls} disabled={locked} value={d.osName} onChange={(e) => set({ osName: e.target.value })} /></Field>
              <Field label="Version" hint="Use 11 for Windows 11, 7 for legacy BIOS guests.">
                <input className={inputCls} disabled={locked} value={d.osVersion} onChange={(e) => set({ osVersion: e.target.value })} />
              </Field>
            </div>
            <Field label="Base folder" hint="Disk and firmware files are stored in a subfolder of this location. Existing disks stay where they are.">
              <div className="flex gap-2">
                <input className={inputCls} disabled={locked} value={d.baseFolder} onChange={(e) => set({ baseFolder: e.target.value })} />
                <button className={btn.ghost} disabled={locked} onClick={async () => { const p = await open({ directory: true }); if (typeof p === "string") set({ baseFolder: p }); }}>Browse</button>
              </div>
            </Field>
            <div className="border-t border-ink-700 pt-5"><button className={btn.danger} onClick={remove}>Delete machine…</button></div>
          </>)}

          {tab === "System" && (<>
            <Slider label="Memory" value={d.ramMb} min={512} max={Math.max(1024, Math.floor(host.ramMb * 0.75 / 256) * 256)} step={256} unit="MB" disabled={locked} onChange={(v) => set({ ramMb: v })} hint={`Host has ${host.ramMb.toLocaleString()} MB.`} />
            <Slider label="CPU cores" value={d.cpuCores} min={1} max={Math.max(1, host.cores)} step={1} disabled={locked} onChange={(v) => set({ cpuCores: v })} hint={`Host has ${host.cores} logical cores.`} />
            <Field label="Acceleration" hint={`Detected on this host: ${host.accel}.`}>
              <select className={inputCls} disabled={locked} value={d.acceleration} onChange={(e) => set({ acceleration: e.target.value as VmConfig["acceleration"] })}>
                <option value="auto">Hardware (KVM / HVF / WHPX) when available</option><option value="software">Software emulation only</option>
              </select>
            </Field>
            <Field label="Firmware" hint="Auto picks UEFI for modern guests and legacy BIOS for very old ones. Windows 11 and ARM always use UEFI.">
              <select className={inputCls} disabled={locked} value={d.firmware} onChange={(e) => set({ firmware: e.target.value as VmConfig["firmware"] })}>
                <option value="auto">Automatic</option><option value="uefi">UEFI</option><option value="bios">Legacy BIOS</option>
              </select>
            </Field>
            <Toggle label="TPM 2.0" checked={d.tpm} disabled={locked} onChange={(v) => set({ tpm: v })}
              hint={host.swtpm ? "Emulated with swtpm. Required by Windows 11." : "swtpm wasn't found on this host, so the TPM can't be emulated."} />
          </>)}

          {tab === "Display" && (<>
            <Slider label="Video memory" value={d.vramMb} min={16} max={256} step={16} unit="MB" disabled={locked} onChange={(v) => set({ vramMb: v })} hint="Applies to the standard VGA adapter used by Windows guests." />
            <Field label="Resolution" hint="Applies to the virtio GPU used by Linux guests. Windows guests choose their own.">
              <select className={inputCls} disabled={locked} value={`${d.width}x${d.height}`} onChange={(e) => { const [w, h] = e.target.value.split("x").map(Number); set({ width: w, height: h }); }}>
                {RESOLUTIONS.map((r) => <option key={r}>{r}</option>)}
              </select>
            </Field>
            <Toggle label="3D acceleration" checked={d.gl3d} disabled={locked} onChange={(v) => set({ gl3d: v })} hint="Linux guests on Linux hosts only (virtio-gpu with OpenGL)." />
          </>)}

          {tab === "Storage" && (<>
            <Field label="Installer ISO">
              <div className="flex gap-2">
                <input className={inputCls} readOnly disabled={locked} value={d.isoPath ?? ""} placeholder="No disc inserted" />
                <button className={btn.ghost} disabled={locked} onClick={() => pickIso("isoPath")}>Browse</button>
                <button className={btn.ghost} disabled={locked || !d.isoPath} onClick={() => set({ isoPath: null })}>Eject</button>
              </div>
            </Field>
            {win && (
              <Field label="Windows drivers ISO (virtio-win)" hint={host.driversIso ? "Found in OmniBox's drivers folder and attached automatically." : `Optional. Place virtio-win.iso in ${host.dataDir}/drivers to attach it automatically.`}>
                <div className="flex gap-2">
                  <input className={inputCls} readOnly disabled={locked} value={d.driversIso ?? ""} placeholder="Not attached" />
                  <button className={btn.ghost} disabled={locked} onClick={() => pickIso("driversIso")}>Browse</button>
                </div>
              </Field>
            )}
            <div className="grid grid-cols-2 gap-4">
              <Field label="Disk format" hint={vm.diskCreated ? "Fixed once the disk exists." : undefined}>
                <select className={inputCls} disabled={locked || vm.diskCreated} value={d.disk.format} onChange={(e) => setDisk({ format: e.target.value as VmConfig["disk"]["format"] })}>
                  <option value="qcow2">QCOW2 (recommended)</option><option value="vdi">VDI (VirtualBox)</option><option value="raw">Raw</option>
                </select>
              </Field>
              <Field label="Disk size (GB)" hint={vm.diskCreated ? "Resize with qemu-img resize." : undefined}>
                <input type="number" min={4} max={4096} className={inputCls} disabled={locked || vm.diskCreated} value={d.disk.sizeGb} onChange={(e) => setDisk({ sizeGb: Math.max(4, Number(e.target.value)) })} />
              </Field>
            </div>
            <Field label="Disk controller" hint="Automatic uses SATA for Windows so Setup sees the disk without extra drivers, and virtio elsewhere.">
              <select className={inputCls} disabled={locked} value={d.storageBus} onChange={(e) => set({ storageBus: e.target.value as VmConfig["storageBus"] })}>
                <option value="auto">Automatic</option><option value="sata">SATA</option><option value="virtio">virtio (fastest, needs drivers on Windows)</option>
              </select>
            </Field>
            <div className="flex items-center gap-3">
              <button className={btn.ghost} disabled={locked || vm.diskCreated} onClick={createDisk}>{vm.diskCreated ? "Disk exists" : "Create disk now"}</button>
              <span className="text-xs text-fog-500">Otherwise the disk is created on first start.</span>
            </div>
          </>)}

          {tab === "Network" && (<>
            <Field label="Attachment">
              <select className={inputCls} disabled={locked} value={d.network} onChange={(e) => set({ network: e.target.value as VmConfig["network"] })}>
                <option value="nat">NAT (works everywhere, no setup)</option><option value="bridged">Bridged adapter</option>
              </select>
            </Field>
            {d.network === "bridged" && (
              <Field label="Host interface" hint="Linux: an existing bridge such as br0 (allow it in /etc/qemu/bridge.conf). macOS: an interface such as en0 (needs elevated rights). Windows: the name of a TAP adapter.">
                <input className={inputCls} disabled={locked} value={d.bridgeIface} onChange={(e) => set({ bridgeIface: e.target.value })} placeholder="br0" />
              </Field>
            )}
          </>)}

          {tab === "USB & Shared folders" && (<>
            <section className="space-y-3">
              <h3 className="text-sm font-medium">USB devices</h3>
              <p className="text-xs text-fog-500">Passes a host device through to the guest. Find ids with lsusb, System Information or Device Manager.</p>
              {d.usbDevices.map((u, i) => (
                <div key={i} className="flex items-center gap-2">
                  <input className={inputCls} disabled={locked} value={u.vendorId} placeholder="Vendor id (046d)" onChange={(e) => set({ usbDevices: d.usbDevices.map((x, j) => j === i ? { ...x, vendorId: e.target.value } : x) })} />
                  <input className={inputCls} disabled={locked} value={u.productId} placeholder="Product id (c52b)" onChange={(e) => set({ usbDevices: d.usbDevices.map((x, j) => j === i ? { ...x, productId: e.target.value } : x) })} />
                  <button className={btn.ghost} disabled={locked} onClick={() => set({ usbDevices: d.usbDevices.filter((_, j) => j !== i) })}>Remove</button>
                </div>
              ))}
              <button className={btn.ghost} disabled={locked} onClick={() => set({ usbDevices: [...d.usbDevices, { vendorId: "", productId: "" }] })}>Add USB device</button>
            </section>
            <section className="space-y-3 border-t border-ink-700 pt-6">
              <h3 className="text-sm font-medium">Shared folders</h3>
              <p className="text-xs text-fog-500">Linux hosts mount each folder in the guest with: mount -t 9p -o trans=virtio NAME /mnt/NAME. macOS and Windows hosts share the first folder over SMB.</p>
              {d.sharedFolders.map((f, i) => (
                <div key={i} className="flex items-center gap-2">
                  <input className={`${inputCls} max-w-[10rem]`} disabled={locked} value={f.name} onChange={(e) => set({ sharedFolders: d.sharedFolders.map((x, j) => j === i ? { ...x, name: e.target.value } : x) })} />
                  <input className={inputCls} readOnly value={f.hostPath} />
                  <button className={btn.ghost} disabled={locked} onClick={() => set({ sharedFolders: d.sharedFolders.filter((_, j) => j !== i) })}>Remove</button>
                </div>
              ))}
              <button className={btn.ghost} disabled={locked} onClick={addFolder}>Add folder…</button>
            </section>
          </>)}

          {info && <p className="text-sm text-ok">{info}</p>}
          {error && <p role="alert" className="rounded-md border border-bad/40 bg-bad/10 px-3 py-2 text-sm text-bad">{error}</p>}
        </div>
      </div>
      <div className="flex justify-end gap-3 border-t border-ink-700 px-6 py-4">
        <button className={btn.ghost} onClick={onClose}>Cancel</button>
        <button className={btn.primary} disabled={locked} onClick={save}>Save changes</button>
      </div>
    </Modal>
  );
}
