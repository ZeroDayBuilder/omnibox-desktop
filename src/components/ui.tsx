import { ReactNode } from "react";

export const inputCls =
  "w-full rounded-xl bg-ink-950/80 border border-ink-600 px-3 py-2 text-sm text-fog-100 placeholder:text-fog-500 " +
  "disabled:opacity-50 focus:border-signal focus:shadow-glow";

export const btn = {
  primary:
    "rounded-xl bg-gradient-to-r from-signal to-signal-glow px-4 py-2 text-sm font-semibold text-white " +
    "shadow-glow hover:brightness-110 disabled:opacity-40 disabled:cursor-not-allowed transition",
  ghost:
    "rounded-xl border border-ink-600 bg-ink-800/50 px-3 py-1.5 text-sm text-fog-100 " +
    "hover:border-signal/40 hover:bg-ink-700 disabled:opacity-40 disabled:cursor-not-allowed transition",
  danger:
    "rounded-xl border border-bad/50 px-3 py-1.5 text-sm text-bad hover:bg-bad/10 disabled:opacity-40 transition",
  green:
    "rounded-xl bg-gradient-to-r from-ok to-cyan px-4 py-2 text-sm font-semibold text-ink-950 " +
    "shadow-glow-green hover:brightness-110 disabled:opacity-40 transition",
  purple:
    "rounded-xl bg-gradient-to-r from-purple to-purple-dim px-4 py-2 text-sm font-semibold text-white " +
    "shadow-glow-purple hover:brightness-110 disabled:opacity-40 transition",
};

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1.5 block text-sm text-fog-300">{label}</span>
      {children}
      {hint && <span className="mt-1.5 block text-xs text-fog-500">{hint}</span>}
    </label>
  );
}

export function Slider(p: {
  label: string; value: number; min: number; max: number; step: number;
  onChange: (v: number) => void; unit?: string; disabled?: boolean; hint?: string;
}) {
  return (
    <Field label={p.label} hint={p.hint}>
      <div className="flex items-center gap-4">
        <input
          type="range" min={p.min} max={p.max} step={p.step} value={p.value} disabled={p.disabled}
          onChange={(e) => p.onChange(Number(e.target.value))}
          className="h-1.5 flex-1 cursor-pointer accent-signal disabled:opacity-40"
        />
        <span className="w-24 text-right text-sm tabular-nums text-signal-glow">
          {p.value.toLocaleString()} {p.unit}
        </span>
      </div>
    </Field>
  );
}

export function Toggle(p: {
  label: string; hint?: string; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean;
}) {
  return (
    <div className="flex items-start justify-between gap-6">
      <div>
        <div className="text-sm text-fog-100">{p.label}</div>
        {p.hint && <div className="mt-1 text-xs text-fog-500">{p.hint}</div>}
      </div>
      <button
        type="button" role="switch" aria-checked={p.checked} aria-label={p.label} disabled={p.disabled}
        onClick={() => p.onChange(!p.checked)}
        className={`relative mt-0.5 h-6 w-11 shrink-0 rounded-full transition-colors disabled:opacity-40 ${
          p.checked ? "bg-signal" : "bg-ink-600"
        }`}
      >
        <span
          className={`absolute top-0.5 h-5 w-5 rounded-full bg-fog-100 transition-all ${
            p.checked ? "left-[22px]" : "left-0.5"
          }`}
        />
      </button>
    </div>
  );
}

export function Modal(p: { title: string; onClose: () => void; children: ReactNode; wide?: boolean }) {
  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-ink-950/80 p-6 backdrop-blur-sm"
      onMouseDown={(e) => e.target === e.currentTarget && p.onClose()}
    >
      <div
        role="dialog"
        aria-label={p.title}
        className={`flex max-h-full w-full flex-col rounded-2xl border border-ink-600 bg-ink-900 shadow-glow animate-fade-up ${
          p.wide ? "max-w-3xl" : "max-w-xl"
        }`}
      >
        <div className="flex items-center justify-between border-b border-ink-700 px-6 py-4">
          <h2 className="text-base font-semibold">{p.title}</h2>
          <button
            onClick={p.onClose}
            aria-label="Close"
            className="rounded-lg px-2 text-xl leading-none text-fog-500 hover:bg-ink-700 hover:text-fog-100"
          >
            ×
          </button>
        </div>
        {p.children}
      </div>
    </div>
  );
}
