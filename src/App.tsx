import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, errText } from "./lib/api";
import type { HostInfo, VmSummary } from "./lib/types";
import VmRow from "./components/VmRow";
import IsoWizard from "./components/IsoWizard";
import SettingsDialog from "./components/SettingsDialog";
import { btn } from "./components/ui";
import Sparkline from "./components/Sparkline";

const HISTORY = 40;
type History = Record<string, { cpu: number[]; mem: number[] }>;
type Nav = "overview" | "machines" | "isos" | "settings" | "logs" | "about";

const NAV_LABEL: Record<Nav, string> = {
  overview: "Overview",
  machines: "Virtual Machines",
  isos: "ISO Library",
  settings: "Settings",
  logs: "Logs",
  about: "About",
};

const navItems: { id: Nav; label: string; icon: string }[] = [
  { id: "overview", label: "Overview", icon: "⌂" },
  { id: "machines", label: "Virtual Machines", icon: "▣" },
  { id: "isos", label: "ISO Library", icon: "◎" },
  { id: "settings", label: "Settings", icon: "⚙" },
  { id: "logs", label: "Logs", icon: "☰" },
  { id: "about", label: "About", icon: "ℹ" },
];

export default function App() {
  const [host, setHost] = useState<HostInfo | null>(null);
  const [vms, setVms] = useState<VmSummary[]>([]);
  const [wizard, setWizard] = useState<{ path: string | null } | null>(null);
  const [settingsId, setSettingsId] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [notice, setNotice] = useState<{ kind: "error" | "info"; text: string } | null>(null);
  const [nav, setNav] = useState<Nav>("overview");
  const history = useRef<History>({});

  const refresh = useCallback(async () => {
    try {
      const list = await api.listVms();
      for (const v of list) {
        const h = (history.current[v.config.id] ??= { cpu: [], mem: [] });
        h.cpu = [...h.cpu, v.status === "stopped" ? 0 : v.cpuPercent].slice(-HISTORY);
        h.mem = [...h.mem, v.status === "stopped" ? 0 : v.memMb].slice(-HISTORY);
      }
      setVms(list);
    } catch (e) {
      setNotice({ kind: "error", text: errText(e) });
    }
  }, []);

  useEffect(() => {
    api.hostInfo().then(setHost).catch((e) => setNotice({ kind: "error", text: errText(e) }));
  }, []);
  useEffect(() => {
    void refresh();
    const t = setInterval(refresh, 2000);
    return () => clearInterval(t);
  }, [refresh]);

  useEffect(() => {
    let off: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === "enter" || p.type === "over") setDragging(true);
        else if (p.type === "leave") setDragging(false);
        else if (p.type === "drop") {
          setDragging(false);
          const iso = p.paths.find((x) => /\.(iso|img)$/i.test(x));
          if (iso) {
            setWizard({ path: iso });
            setNav("machines");
          } else setNotice({ kind: "error", text: "That file isn't an ISO image." });
        }
      })
      .then((u) => (off = u));
    return () => off?.();
  }, []);

  async function run(id: string, fn: () => Promise<unknown>) {
    setBusyId(id);
    setNotice(null);
    try {
      const result = await fn();
      if (Array.isArray(result) && result.length) setNotice({ kind: "info", text: result.join(" ") });
    } catch (e) {
      setNotice({ kind: "error", text: errText(e) });
    } finally {
      setBusyId(null);
      void refresh();
    }
  }

  const settingsVm = vms.find((v) => v.config.id === settingsId);
  const missing = host && !host.qemu;
  const running = vms.filter((v) => v.status !== "stopped");
  const selected = running[0] ?? vms[0] ?? null;

  function machineList() {
    if (vms.length === 0) {
      return (
        <div className="card flex flex-col items-center justify-center px-6 py-14 text-center animate-fade-up">
          <div className="mb-3 text-4xl opacity-80">🖥️</div>
          <p className="text-base font-medium">No virtual machines yet</p>
          <p className="mt-2 max-w-md text-sm text-fog-500">
            Open an ISO or create a new machine. After install, the next Start boots the disk automatically.
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <button className={btn.primary} onClick={() => setWizard({ path: null })}>
              New machine
            </button>
            <button className={btn.purple} onClick={() => setWizard({ path: null })}>
              Open ISO
            </button>
          </div>
        </div>
      );
    }
    return (
      <ul className="space-y-3">
        {vms.map((v) => (
          <VmRow
            key={v.config.id}
            vm={v}
            busy={busyId === v.config.id}
            onStart={() => run(v.config.id, () => api.start(v.config.id))}
            onStop={() => run(v.config.id, () => api.stop(v.config.id, false))}
            onForceStop={() => run(v.config.id, () => api.stop(v.config.id, true))}
            onPause={() => run(v.config.id, () => api.pause(v.config.id))}
            onResume={() => run(v.config.id, () => api.resume(v.config.id))}
            onSettings={() => setSettingsId(v.config.id)}
            onEjectIso={() =>
              run(v.config.id, async () => {
                await api.save({ ...v.config, isoPath: null });
                return ["Installer ISO ejected. Start boots the installed OS from disk."];
              })
            }
          />
        ))}
      </ul>
    );
  }

  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <aside className="flex w-56 shrink-0 flex-col border-r border-ink-700/80 bg-ink-900/60 backdrop-blur-md">
        <div className="flex items-center gap-2.5 border-b border-ink-700/80 px-4 py-4">
          <div
            className="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-[#3b82f6] via-[#6366f1] to-[#8b5cf6] text-sm font-bold text-white shadow-glow"
            title="OmniBox"
          >
            <svg viewBox="0 0 32 32" className="h-6 w-6" aria-hidden>
              <path fill="white" fillOpacity="0.95" d="M16 3l11 6.5v13L16 29 5 22.5v-13L16 3z"/>
              <path fill="#1e3a8a" fillOpacity="0.35" d="M16 3v26L5 22.5v-13L16 3z"/>
              <path fill="white" fillOpacity="0.2" d="M16 3l11 6.5L16 16 5 9.5 16 3z"/>
            </svg>
          </div>
          <div>
            <div className="text-sm font-semibold tracking-wide">OmniBox</div>
            <div className="text-[10px] uppercase tracking-wider text-fog-500">Desktop</div>
          </div>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          {navItems.map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => setNav(item.id)}
              className={`nav-item ${nav === item.id ? "nav-item-active" : "hover:bg-ink-700/60"}`}
            >
              <span className="w-5 text-center text-fog-500">{item.icon}</span>
              {item.label}
            </button>
          ))}
        </nav>
        <div className="border-t border-ink-700/80 p-3 text-[11px] text-fog-500">
          {host ? (
            <>
              <div className="flex items-center gap-1.5">
                <span className="h-1.5 w-1.5 rounded-full bg-ok animate-pulse-soft" />
                {host.accel}
              </div>
              <div className="mt-1">
                {host.cores} cores · {(host.ramMb / 1024).toFixed(0)} GB
              </div>
            </>
          ) : (
            "Checking host…"
          )}
        </div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="flex items-center justify-between border-b border-ink-700/80 px-6 py-3 backdrop-blur-sm">
          <h1 className="text-base font-semibold">{NAV_LABEL[nav]}</h1>
          <div className="flex gap-2">
            <button className={btn.ghost} onClick={() => setWizard({ path: null })}>
              Open ISO
            </button>
            <button className={btn.primary} onClick={() => setWizard({ path: null })}>
              + New machine
            </button>
          </div>
        </header>

        {missing && (
          <p role="alert" className="mx-6 mt-4 rounded-xl border border-warn/40 bg-warn/10 px-4 py-3 text-sm text-warn">
            QEMU wasn&apos;t found. Install it (see README), then restart OmniBox.
          </p>
        )}

        <main className="flex-1 overflow-y-auto px-6 py-5">
          {nav === "overview" && (
            <div className="grid gap-6 lg:grid-cols-[1fr_260px] animate-fade-up">
              <div className="space-y-6">
                {/* Welcome / hero when no focus */}
                {!selected && (
                  <div className="text-center sm:text-left">
                    <h2 className="text-xl font-semibold">
                      Welcome to <span className="bg-gradient-to-r from-signal-glow to-purple bg-clip-text text-transparent">OmniBox Desktop</span>
                    </h2>
                    <p className="mt-1 text-sm text-fog-500">Create and manage your virtual machines with ease.</p>
                  </div>
                )}

                {/* 3 action cards */}
                <div className="grid gap-4 sm:grid-cols-3">
                  <button
                    type="button"
                    onClick={() => setNav("machines")}
                    className="card card-hover group p-5 text-left"
                  >
                    <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-signal/20 text-2xl transition group-hover:scale-110">
                      🖥️
                    </div>
                    <div className="text-sm font-semibold">Open Machine</div>
                    <p className="mt-1 text-xs text-fog-500">View and control VMs ({vms.length})</p>
                    <span className="mt-3 inline-block text-xs font-medium text-signal">Open Machine →</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setWizard({ path: null })}
                    className="card card-hover group border-purple/30 p-5 text-left"
                  >
                    <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-purple/20 text-2xl transition group-hover:scale-110">
                      💿
                    </div>
                    <div className="text-sm font-semibold">Open ISO</div>
                    <p className="mt-1 text-xs text-fog-500">Mount an ISO and create a VM</p>
                    <span className="mt-3 inline-block text-xs font-medium text-purple">Open ISO →</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => setWizard({ path: null })}
                    className="card card-hover group border-ok/30 bg-gradient-to-br from-ok/10 to-cyan/5 p-5 text-left"
                  >
                    <div className="mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-ok/20 text-2xl transition group-hover:scale-110">
                      ＋
                    </div>
                    <div className="text-sm font-semibold text-ok">New Machine</div>
                    <p className="mt-1 text-xs text-fog-500">Wizard picks CPU, RAM and disk</p>
                    <span className="mt-3 inline-block text-xs font-medium text-ok">New Machine →</span>
                  </button>
                </div>

                {/* Live focus card when a VM is running */}
                {selected && selected.status !== "stopped" && (
                  <div className="card space-y-5 p-5">
                    <div className="flex flex-wrap items-start justify-between gap-4">
                      <div className="flex items-center gap-3">
                        <div className="flex h-14 w-14 items-center justify-center rounded-2xl bg-gradient-to-br from-signal/40 to-purple/30 text-2xl">
                          {selected.config.osFamily === "windows" ? "🪟" : "🐧"}
                        </div>
                        <div>
                          <div className="text-lg font-semibold">{selected.config.name}</div>
                          <div className="flex items-center gap-2 text-sm text-fog-500">
                            <span className="pill bg-ok/15 text-ok">
                              <span className="h-1.5 w-1.5 rounded-full bg-ok animate-pulse-soft" />
                              {selected.status}
                            </span>
                            {selected.config.osName} · {selected.config.cpuCores} vCPU ·{" "}
                            {(selected.config.ramMb / 1024).toFixed(1)} GB
                          </div>
                        </div>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        {selected.status === "running" && (
                          <button
                            className={btn.ghost}
                            disabled={busyId === selected.config.id}
                            onClick={() => run(selected.config.id, () => api.pause(selected.config.id))}
                          >
                            Pause
                          </button>
                        )}
                        {selected.status === "paused" && (
                          <button
                            className={btn.primary}
                            disabled={busyId === selected.config.id}
                            onClick={() => run(selected.config.id, () => api.resume(selected.config.id))}
                          >
                            Resume
                          </button>
                        )}
                        <button
                          className={btn.ghost}
                          disabled={busyId === selected.config.id}
                          onClick={() => run(selected.config.id, () => api.stop(selected.config.id, false))}
                        >
                          Shut down
                        </button>
                        <button
                          className={btn.ghost}
                          disabled={busyId === selected.config.id}
                          onClick={() => run(selected.config.id, () => api.stop(selected.config.id, true))}
                        >
                          Force off
                        </button>
                      </div>
                    </div>

                    {/* Mini meters */}
                    <div className="grid gap-4 sm:grid-cols-2">
                      <div className="rounded-xl border border-ink-600/60 bg-ink-950/40 p-3">
                        <div className="mb-1 flex justify-between text-xs">
                          <span className="text-fog-500">CPU Usage</span>
                          <span className="font-semibold tabular-nums text-signal-glow">
                            {selected.cpuPercent.toFixed(0)}%
                          </span>
                        </div>
                        <div className="h-2 overflow-hidden rounded-full bg-ink-700">
                          <div
                            className="h-full rounded-full bg-gradient-to-r from-signal to-signal-glow transition-all duration-500"
                            style={{ width: `${Math.min(100, selected.cpuPercent)}%` }}
                          />
                        </div>
                      </div>
                      <div className="rounded-xl border border-ink-600/60 bg-ink-950/40 p-3">
                        <div className="mb-1 flex justify-between text-xs">
                          <span className="text-fog-500">Memory</span>
                          <span className="font-semibold tabular-nums text-ok">
                            {selected.memMb.toLocaleString()} MB ·{" "}
                            {selected.config.ramMb > 0
                              ? ((selected.memMb / selected.config.ramMb) * 100).toFixed(0)
                              : 0}
                            %
                          </span>
                        </div>
                        <div className="h-2 overflow-hidden rounded-full bg-ink-700">
                          <div
                            className="h-full rounded-full bg-gradient-to-r from-ok to-cyan transition-all duration-500"
                            style={{
                              width: `${Math.min(
                                100,
                                (selected.memMb / Math.max(1, selected.config.ramMb)) * 100,
                              )}%`,
                            }}
                          />
                        </div>
                      </div>
                    </div>

                    {/* Charts */}
                    <div>
                      <div className="mb-3 text-xs font-medium uppercase tracking-wide text-fog-500">
                        Real-time CPU & Memory Usage
                      </div>
                      <div className="grid gap-4 sm:grid-cols-2">
                        <div className="rounded-xl border border-ink-600/50 bg-ink-950/50 p-4">
                          <div className="mb-2 flex items-baseline justify-between">
                            <span className="text-xs text-fog-500">CPU Usage</span>
                            <span className="text-2xl font-semibold tabular-nums text-signal-glow">
                              {selected.cpuPercent.toFixed(0)}
                              <span className="text-sm text-fog-500">%</span>
                            </span>
                          </div>
                          <Sparkline
                            data={history.current[selected.config.id]?.cpu ?? []}
                            max={100}
                            color="#4d8dff"
                            label="CPU"
                            height={110}
                          />
                        </div>
                        <div className="rounded-xl border border-ink-600/50 bg-ink-950/50 p-4">
                          <div className="mb-2 flex items-baseline justify-between">
                            <span className="text-xs text-fog-500">Memory Usage</span>
                            <span className="text-2xl font-semibold tabular-nums text-ok">
                              {selected.config.ramMb > 0
                                ? ((selected.memMb / selected.config.ramMb) * 100).toFixed(0)
                                : 0}
                              <span className="text-sm text-fog-500">%</span>
                            </span>
                          </div>
                          <Sparkline
                            data={history.current[selected.config.id]?.mem ?? []}
                            max={selected.config.ramMb}
                            color="#34d399"
                            label="Memory"
                            height={110}
                          />
                        </div>
                      </div>
                    </div>

                    {/* Config snapshot */}
                    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4 text-xs">
                      {[
                        ["CPU cores", String(selected.config.cpuCores)],
                        ["RAM", `${(selected.config.ramMb / 1024).toFixed(1)} GB`],
                        ["Disk", `${selected.config.disk.sizeGb} GB`],
                        ["Firmware", selected.config.firmware],
                        ["Network", selected.config.network],
                        ["Arch", selected.config.arch],
                        ["OS", selected.config.osName],
                        ["ISO", selected.config.isoPath ? "Mounted" : "None"],
                      ].map(([k, v]) => (
                        <div key={k} className="rounded-lg border border-ink-600/40 bg-ink-950/30 px-3 py-2">
                          <div className="text-fog-500">{k}</div>
                          <div className="mt-0.5 truncate font-medium text-fog-100">{v}</div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div>
                  <h2 className="mb-3 text-sm font-medium text-fog-300">Recent / installed machines</h2>
                  {machineList()}
                </div>
              </div>

              {/* Right system panel */}
              <aside className="space-y-4">
                <div className="card p-4 text-sm">
                  <h3 className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-fog-500">
                    <span className="text-signal">◉</span> System information
                  </h3>
                  {host ? (
                    <dl className="space-y-2.5 text-fog-300">
                      <div className="flex justify-between gap-2">
                        <dt className="text-fog-500">Accel</dt>
                        <dd className="font-medium text-fog-100">{host.accel}</dd>
                      </div>
                      <div className="flex justify-between gap-2">
                        <dt className="text-fog-500">OS</dt>
                        <dd>{host.os}</dd>
                      </div>
                      <div className="flex justify-between gap-2">
                        <dt className="text-fog-500">Cores</dt>
                        <dd>{host.cores}</dd>
                      </div>
                      <div className="flex justify-between gap-2">
                        <dt className="text-fog-500">RAM</dt>
                        <dd>{(host.ramMb / 1024).toFixed(0)} GB</dd>
                      </div>
                      <div className="flex justify-between gap-2">
                        <dt className="text-fog-500">QEMU</dt>
                        <dd className={host.qemu ? "text-ok" : "text-bad"}>{host.qemu ? "Ready" : "Missing"}</dd>
                      </div>
                      <div className="flex justify-between gap-2">
                        <dt className="text-fog-500">TPM</dt>
                        <dd>{host.swtpm ? "Yes" : "No"}</dd>
                      </div>
                      <div className="flex justify-between gap-2">
                        <dt className="text-fog-500">UEFI</dt>
                        <dd>{host.uefi ? "Yes" : "No"}</dd>
                      </div>
                      <div className="mt-2 flex items-center gap-1.5 text-ok">
                        <span className="h-2 w-2 rounded-full bg-ok animate-pulse-soft" />
                        Ready
                      </div>
                    </dl>
                  ) : (
                    <p className="text-fog-500">Loading…</p>
                  )}
                </div>
                <div className="card p-4 text-sm">
                  <h3 className="mb-3 text-xs font-semibold uppercase tracking-wide text-fog-500">Quick links</h3>
                  <ul className="space-y-2 text-fog-400">
                    <li className="text-fog-500">Documentation — coming soon</li>
                    <li className="text-fog-500">Support — coming soon</li>
                    <li className="text-fog-500">Check for Updates — coming soon</li>
                  </ul>
                </div>
              </aside>
            </div>
          )}

          {nav === "machines" && <div className="animate-fade-up">{machineList()}</div>}

          {nav === "isos" && (
            <div className="card max-w-2xl p-6 animate-fade-up">
              <h2 className="text-base font-semibold">ISO Library</h2>
              <p className="mt-2 text-sm text-fog-500">
                Drop an ISO anywhere, or use Open ISO. After install, ISO auto-ejects when the disk has an OS.
              </p>
              <button className={`${btn.primary} mt-4`} onClick={() => setWizard({ path: null })}>
                Open ISO…
              </button>
              <ul className="mt-6 space-y-2 text-sm">
                {vms
                  .filter((v) => v.config.isoPath)
                  .map((v) => (
                    <li
                      key={v.config.id}
                      className="flex items-center justify-between gap-4 rounded-xl border border-ink-600 px-3 py-2"
                    >
                      <span className="truncate">
                        {v.config.name}: {v.config.isoPath}
                      </span>
                      <button
                        className={btn.ghost}
                        onClick={() =>
                          run(v.config.id, async () => {
                            await api.save({ ...v.config, isoPath: null });
                            return ["ISO ejected"];
                          })
                        }
                      >
                        Eject
                      </button>
                    </li>
                  ))}
                {vms.every((v) => !v.config.isoPath) && (
                  <li className="text-fog-500">No ISOs currently attached.</li>
                )}
              </ul>
            </div>
          )}

          {nav === "settings" && (
            <div className="grid max-w-3xl gap-4 animate-fade-up">
              <div className="card p-6 text-sm text-fog-300">
                <h2 className="text-base font-semibold text-fog-100">App settings</h2>
                <p className="mt-2 text-fog-500">
                  Global options. Per-machine CPU/RAM/disk/ISO: open a machine → its Settings button.
                </p>
                <dl className="mt-5 space-y-3">
                  <div className="flex flex-wrap justify-between gap-2 rounded-xl border border-ink-600/50 bg-ink-950/40 px-4 py-3">
                    <dt className="text-fog-500">Data directory</dt>
                    <dd className="break-all text-xs text-signal-glow">{host?.dataDir ?? "…"}</dd>
                  </div>
                  <div className="flex flex-wrap justify-between gap-2 rounded-xl border border-ink-600/50 bg-ink-950/40 px-4 py-3">
                    <dt className="text-fog-500">QEMU path override</dt>
                    <dd className="text-xs">
                      env <code className="text-signal">OMNIBOX_QEMU_DIR</code>
                    </dd>
                  </div>
                  <div className="flex flex-wrap justify-between gap-2 rounded-xl border border-ink-600/50 bg-ink-950/40 px-4 py-3">
                    <dt className="text-fog-500">Disk images</dt>
                    <dd className="text-xs text-ok">qcow2 sparse (grows with guest use)</dd>
                  </div>
                  <div className="flex flex-wrap justify-between gap-2 rounded-xl border border-ink-600/50 bg-ink-950/40 px-4 py-3">
                    <dt className="text-fog-500">Acceleration</dt>
                    <dd>{host?.accel ?? "…"}</dd>
                  </div>
                </dl>
              </div>
              <div className="card p-6 text-sm">
                <h3 className="font-semibold text-fog-100">Machines — open settings</h3>
                <ul className="mt-3 space-y-2">
                  {vms.length === 0 && <li className="text-fog-500">No machines yet.</li>}
                  {vms.map((v) => (
                    <li key={v.config.id} className="flex items-center justify-between gap-3 rounded-xl border border-ink-600/50 px-3 py-2">
                      <span className="truncate text-fog-200">{v.config.name}</span>
                      <button type="button" className={btn.ghost} onClick={() => setSettingsId(v.config.id)}>
                        Settings
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          )}

          {nav === "logs" && (
            <div className="card max-w-3xl p-6 text-sm animate-fade-up">
              <h2 className="text-base font-semibold">Logs</h2>
              <p className="mt-2 text-fog-500">
                QEMU writes each machine&apos;s log here. Open the path in a terminal with{" "}
                <code className="text-signal">tail -f</code>.
              </p>
              <pre className="mt-3 overflow-x-auto rounded-xl bg-ink-950 p-3 text-xs text-signal-glow">
                {host?.dataDir ?? "~/.local/share/dev.omnibox.desktop"}/machines/&lt;id&gt;/qemu.log
              </pre>
              <ul className="mt-5 space-y-2">
                {vms.length === 0 && <li className="text-fog-500">No machines — no logs yet.</li>}
                {vms.map((v) => (
                  <li
                    key={v.config.id}
                    className="rounded-xl border border-ink-600/50 bg-ink-950/30 px-4 py-3"
                  >
                    <div className="font-medium text-fog-100">{v.config.name}</div>
                    <div className="mt-1 break-all text-xs text-fog-500">
                      {(host?.dataDir ?? "") + "/machines/" + v.config.id + "/qemu.log"}
                    </div>
                    <div className="mt-1 text-xs text-fog-500">id: {v.config.id}</div>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {nav === "about" && (
            <div className="card max-w-lg space-y-4 p-6 text-sm animate-fade-up">
              <h2 className="text-base font-semibold">About OmniBox</h2>
              <p className="text-fog-500">
                Desktop manager for QEMU virtual machines. Linux guests work well; Windows 11 needs Secure Boot
                firmware or LabConfig bypass during Setup.
              </p>
              {host && (
                <dl className="grid grid-cols-[7rem_1fr] gap-y-2 text-fog-300">
                  <dt className="text-fog-500">Host</dt>
                  <dd>
                    {host.os} / {host.arch}
                  </dd>
                  <dt className="text-fog-500">Accel</dt>
                  <dd>{host.accel}</dd>
                  <dt className="text-fog-500">Data</dt>
                  <dd className="break-all text-xs">{host.dataDir}</dd>
                </dl>
              )}
            </div>
          )}
        </main>

        {notice && (
          <div
            role={notice.kind === "error" ? "alert" : "status"}
            className={`flex items-start justify-between gap-4 border-t px-6 py-3 text-sm animate-fade-up ${
              notice.kind === "error"
                ? "border-bad/40 bg-bad/10 text-bad"
                : "border-ink-700 bg-ink-800/80 text-fog-300"
            }`}
          >
            <span className="whitespace-pre-wrap break-words">{notice.text}</span>
            <button className="shrink-0 text-fog-500 hover:text-fog-100" onClick={() => setNotice(null)}>
              Dismiss
            </button>
          </div>
        )}
      </div>

      {dragging && (
        <div className="pointer-events-none fixed inset-0 z-50 flex items-center justify-center bg-ink-950/85 backdrop-blur-sm">
          <div className="rounded-2xl border-2 border-dashed border-signal px-16 py-12 text-lg shadow-glow">
            Drop the ISO to create a machine
          </div>
        </div>
      )}

      {wizard && host && (
        <IsoWizard
          initialPath={wizard.path}
          host={host}
          onClose={() => setWizard(null)}
          onCreated={async (id, start) => {
            setWizard(null);
            setNav("machines");
            await refresh();
            if (start) void run(id, () => api.start(id));
          }}
        />
      )}
      {settingsVm && host && (
        <SettingsDialog
          key={settingsVm.config.id}
          vm={settingsVm}
          host={host}
          onClose={() => setSettingsId(null)}
          onSaved={refresh}
          onDeleted={() => {
            setSettingsId(null);
            void refresh();
          }}
        />
      )}
    </div>
  );
}
