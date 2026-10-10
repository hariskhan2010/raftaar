import type { ReactNode } from 'react'
import { AlertTriangle, CheckCircle2, Info, XCircle } from 'lucide-react'

export function Toggle({
  on,
  onChange,
  label,
  disabled,
}: {
  on: boolean
  onChange: () => void
  label: string
  disabled?: boolean
}) {
  return (
    <button
      role="switch"
      aria-checked={on}
      aria-label={label}
      className={`switch ${on ? 'on' : ''}`}
      disabled={disabled}
      onClick={onChange}
    >
      <span className="track" />
    </button>
  )
}

export function PageHead({ title, sub, children }: { title: string; sub?: string; children?: ReactNode }) {
  return (
    <header className="page-head">
      <div>
        <h1>{title}</h1>
        {sub && <p>{sub}</p>}
      </div>
      {children && <div className="row wrap">{children}</div>}
    </header>
  )
}

const bannerIcon = { warn: AlertTriangle, error: XCircle, ok: CheckCircle2, info: Info }

export function Banner({
  kind,
  children,
  role,
}: {
  kind: keyof typeof bannerIcon
  children: ReactNode
  role?: string
}) {
  const Icon = bannerIcon[kind]
  return (
    <div className={`banner ${kind}`} role={role}>
      <Icon size={16} />
      <div>{children}</div>
    </div>
  )
}

/** Circular gauge, 0-100. */
export function Gauge({ value, label, size = 168 }: { value: number | null; label: string; size?: number }) {
  const stroke = 12
  const r = (size - stroke) / 2
  const c = 2 * Math.PI * r
  const v = value == null ? 0 : Math.max(0, Math.min(100, value))
  const color = v >= 70 ? 'var(--ok)' : v >= 40 ? 'var(--accent-2)' : 'var(--accent)'
  return (
    <div className="gauge" style={{ width: size, height: size }}>
      <svg width={size} height={size} aria-hidden>
        <defs>
          <linearGradient id="gauge-grad" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="var(--accent)" />
            <stop offset="100%" stopColor={color} />
          </linearGradient>
        </defs>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--surface-3)" strokeWidth={stroke} />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="url(#gauge-grad)"
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - v / 100)}
          style={{ transition: 'stroke-dashoffset 0.8s ease' }}
        />
      </svg>
      <div className="gauge-center">
        <div className="gauge-value">{value == null ? '–' : Math.round(value)}</div>
        <div className="gauge-label">{label}</div>
      </div>
    </div>
  )
}

/** Small line chart of recent values (0-max). */
export function Sparkline({ values, max = 100 }: { values: number[]; max?: number }) {
  const w = 200
  const h = 36
  if (values.length < 2) return <svg className="spark" viewBox={`0 0 ${w} ${h}`} aria-hidden />
  const step = w / (values.length - 1)
  const pts = values.map((v, i) => `${(i * step).toFixed(1)},${(h - (Math.min(v, max) / max) * (h - 2) - 1).toFixed(1)}`)
  return (
    <svg className="spark" viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" aria-hidden>
      <polygon points={`0,${h} ${pts.join(' ')} ${w},${h}`} fill="var(--accent-soft)" />
      <polyline points={pts.join(' ')} fill="none" stroke="var(--accent)" strokeWidth="1.6" vectorEffect="non-scaling-stroke" />
    </svg>
  )
}

export function Meter({ label, value, display, history }: { label: string; value: number; display: string; history?: number[] }) {
  return (
    <div className="meter">
      <div className="meter-top">
        <span className="muted">{label}</span>
        <span className="v ltr">{display}</span>
      </div>
      <div className="meter-bar">
        <span style={{ width: `${Math.max(0, Math.min(100, value))}%` }} />
      </div>
      {history && <Sparkline values={history} />}
    </div>
  )
}

/** Frame-time line chart. */
export function FrameChart({ series }: { series: { values: number[]; color: string }[] }) {
  const w = 600
  const h = 140
  const all = series.flatMap((s) => s.values)
  if (all.length === 0) return null
  const max = Math.max(...all) * 1.1
  return (
    <svg className="chart" viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" aria-hidden>
      {[0.25, 0.5, 0.75].map((f) => (
        <line key={f} x1="0" x2={w} y1={h * f} y2={h * f} stroke="var(--border)" strokeDasharray="3 5" />
      ))}
      {series.map((s, si) => {
        const step = w / Math.max(1, s.values.length - 1)
        const pts = s.values.map((v, i) => `${(i * step).toFixed(1)},${(h - (v / max) * h).toFixed(1)}`).join(' ')
        return <polyline key={si} points={pts} fill="none" stroke={s.color} strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
      })}
    </svg>
  )
}
