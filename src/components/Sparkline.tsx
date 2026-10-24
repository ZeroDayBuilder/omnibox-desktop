interface Props {
  data: number[];
  max: number;
  color: string;
  width?: number;
  height?: number;
  label: string;
  fill?: boolean;
}

/** Smooth-ish area chart with gradient fill for Overview live stats. */
export default function Sparkline({
  data,
  max,
  color,
  width = 400,
  height = 120,
  label,
  fill = true,
}: Props) {
  const n = 40;
  const pts = Array.from({ length: n }, (_, i) => data[data.length - n + i] ?? 0);
  const x = (i: number) => (i / Math.max(n - 1, 1)) * width;
  const y = (v: number) => height - 4 - (Math.min(v, max) / Math.max(max, 1)) * (height - 12);
  const line = pts.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  const guides = [0.25, 0.5, 0.75].map((f) => {
    const gy = height - 4 - f * (height - 12);
    return `M0,${gy.toFixed(1)} L${width},${gy.toFixed(1)}`;
  });
  const gid = `g-${label.replace(/\W/g, "")}-${color.replace("#", "")}`;
  return (
    <svg width="100%" viewBox={`0 0 ${width} ${height}`} role="img" aria-label={label} className="block">
      <defs>
        <linearGradient id={gid} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity={0.45} />
          <stop offset="100%" stopColor={color} stopOpacity={0.02} />
        </linearGradient>
      </defs>
      {guides.map((d, i) => (
        <path key={i} d={d} fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth={1} />
      ))}
      {fill && <path d={`${line} L${width},${height} L0,${height} Z`} fill={`url(#${gid})`} />}
      <path
        d={line}
        fill="none"
        stroke={color}
        strokeWidth={2.5}
        strokeLinejoin="round"
        strokeLinecap="round"
        style={{ filter: `drop-shadow(0 0 6px ${color})` }}
      />
    </svg>
  );
}
