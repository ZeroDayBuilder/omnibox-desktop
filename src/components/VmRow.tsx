import { btn } from "./ui";
import type { VmStatus, VmSummary } from "../lib/types";

const statusStyle: Record<VmStatus, { dot: string; label: string; pill: string }> = {
  running: { dot: "bg-ok animate-pulse-soft", label: "Running", pill: "bg-ok/15 text-ok" },
  paused: { dot: "bg-warn", label: "Paused", pill: "bg-warn/15 text-warn" },
  stopped: { dot: "bg-fog-500", label: "Stopped", pill: "bg-ink-600 text-fog-300" },
};

interface Props {
  vm: VmSummary;
  busy: boolean;
  onStart: () => void;
  onStop: () => void;
  onForceStop: () => void;
  onPause: () => void;
  onResume: () => void;
  onSettings: () => void;
  onEjectIso?: () => void;
}

export default function VmRow({ vm, busy, ...a }: Props) {
  const c = vm.config;
  const s = statusStyle[vm.status];
  const live = vm.status !== "stopped";
  const hasIso = !!c.isoPath;

  return (
    <li className="card card-hover flex flex-wrap items-center justify-between gap-4 px-5 py-4 animate-fade-up">
      <div className="flex min-w-0 flex-1 items-center gap-4">
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-signal/30 to-purple/20 text-xl">
          {c.osFamily === "windows" ? "🪟" : c.osFamily === "linux" ? "🐧" : "💻"}
        </div>
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="truncate text-[15px] font-semibold">{c.name}</span>
            <span className={`pill ${s.pill}`}>
              <span className={`h-1.5 w-1.5 rounded-full ${s.dot}`} />
              {s.label}
            </span>
          </div>
          <div className="mt-0.5 truncate text-xs text-fog-500">
            {c.osName} · {c.cpuCores} vCPU · {(c.ramMb / 1024).toFixed(c.ramMb % 1024 ? 1 : 0)} GB
            {hasIso && <span className="text-warn"> · ISO attached</span>}
          </div>
        </div>
      </div>

      <div className="flex flex-wrap justify-end gap-2">
        {vm.status === "stopped" && (
          <button className={btn.primary} disabled={busy} onClick={a.onStart}>
            Start
          </button>
        )}
        {vm.status === "running" && (
          <button className={btn.ghost} disabled={busy} onClick={a.onPause}>
            Pause
          </button>
        )}
        {vm.status === "paused" && (
          <button className={btn.primary} disabled={busy} onClick={a.onResume}>
            Resume
          </button>
        )}
        {live && (
          <button className={btn.ghost} disabled={busy} onClick={a.onStop}>
            Shut down
          </button>
        )}
        {live && (
          <button className={btn.ghost} disabled={busy} onClick={a.onForceStop}>
            Force off
          </button>
        )}
        {hasIso && !live && a.onEjectIso && (
          <button className={btn.ghost} disabled={busy} onClick={a.onEjectIso}>
            Eject ISO
          </button>
        )}
        <button className={btn.ghost} onClick={a.onSettings}>
          Settings
        </button>
      </div>
    </li>
  );
}
