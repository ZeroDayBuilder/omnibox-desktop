import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errText } from "../lib/api";
import type { HostInfo, IsoInfo } from "../lib/types";
import { btn, Field, inputCls, Modal, Slider } from "./ui";

interface Props { initialPath: string | null; host: HostInfo; onClose: () => void; onCreated: (id: string, start: boolean) => void }

export default function IsoWizard({ initialPath, host, onClose, onCreated }: Props) {
  const [info, setInfo] = useState<IsoInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [cpu, setCpu] = useState(2);
  const [ram, setRam] = useState(4096);
  const [disk, setDisk] = useState(40);
  const [busy, setBusy] = useState(false);

  async function inspect(path: string) {
    setError(null);
    try {
      const i = await api.inspectIso(path);
      setInfo(i);
      setName(`${i.osName}${i.osVersion && i.osVersion !== "10/11" ? " " + i.osVersion : ""}`.trim());
      setCpu(i.recommended.cpuCores);
      setRam(i.recommended.ramMb);
      setDisk(i.recommended.diskGb);
    } catch (e) {
      setInfo(null);
      setError(errText(e));
    }
  }

  useEffect(() => { if (initialPath) void inspect(initialPath); }, [initialPath]);

  async function browse() {
    const picked = await open({ multiple: false, filters: [{ name: "Disc image", extensions: ["iso", "img"] }] });
    if (typeof picked === "string") await inspect(picked);
  }

  async function create(start: boolean) {
    if (!info) return;
    setBusy(true);
    try {
      const vm = await api.createFromIso({ isoPath: info.path, name: name || info.osName, cpuCores: cpu, ramMb: ram, diskGb: disk });
      onCreated(vm.id, start);
    } catch (e) {
      setError(errText(e));
      setBusy(false);
    }
  }

  const maxRam = Math.max(1024, Math.floor(host.ramMb * 0.75 / 256) * 256);

  return (
    <Modal title="New machine from ISO" onClose={onClose}>
      <div className="space-y-6 overflow-y-auto px-6 py-6">
        {!info ? (
          <div className="rounded-lg border border-dashed border-ink-600 px-6 py-12 text-center">
            <p className="text-sm text-fog-300">Drop an ISO anywhere in this window, or choose one from disk.</p>
            <button className={`${btn.primary} mt-5`} onClick={browse}>Choose ISO…</button>
          </div>
        ) : (
          <>
            <div className="flex items-start justify-between gap-4 rounded-lg bg-ink-900 px-4 py-3">
              <div className="min-w-0">
                <div className="text-sm font-medium">Detected {info.osName} {info.osVersion}</div>
                <div className="mt-0.5 truncate text-xs text-fog-500">{info.fileName} · {info.sizeMb.toLocaleString()} MB · {info.arch}</div>
              </div>
              <button className="text-sm text-signal hover:underline" onClick={browse}>Change</button>
            </div>

            {info.warnings.map((w) => (
              <p key={w} className="rounded-md border border-warn/40 bg-warn/10 px-3 py-2 text-sm text-warn">{w}</p>
            ))}

            <Field label="Name"><input className={inputCls} value={name} onChange={(e) => setName(e.target.value)} /></Field>
            <Slider label="CPU cores" value={cpu} min={1} max={Math.max(1, host.cores)} step={1} onChange={setCpu} hint={`Recommended: ${info.recommended.cpuCores}`} />
            <Slider label="Memory" value={ram} min={512} max={maxRam} step={256} unit="MB" onChange={setRam} hint={`Recommended: ${info.recommended.ramMb.toLocaleString()} MB`} />
            <Slider label="Disk size" value={disk} min={8} max={512} step={4} unit="GB" onChange={setDisk} hint="The disk grows on demand, so it only uses the space the guest writes." />
            {info.needsTpm && <p className="text-xs text-fog-500">UEFI and a TPM 2.0 module will be added automatically for Windows 11.</p>}
          </>
        )}
        {error && <p role="alert" className="rounded-md border border-bad/40 bg-bad/10 px-3 py-2 text-sm text-bad">{error}</p>}
      </div>
      <div className="flex justify-end gap-3 border-t border-ink-700 px-6 py-4">
        <button className={btn.ghost} onClick={onClose}>Cancel</button>
        <button className={btn.ghost} disabled={!info?.supported || busy} onClick={() => create(false)}>Create</button>
        <button className={btn.primary} disabled={!info?.supported || busy} onClick={() => create(true)}>Create and start</button>
      </div>
    </Modal>
  );
}
