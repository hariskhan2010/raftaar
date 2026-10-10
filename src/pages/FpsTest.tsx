import { useEffect, useMemo, useRef, useState } from 'react'
import { Play, Trash2 } from 'lucide-react'
import { api, type FpsRecord } from '../api'
import { errText, formatDate, useT } from '../i18n'
import { useApp } from '../store'
import { Banner, FrameChart, PageHead } from '../components/ui'

const DURATIONS = [15, 30, 60]
const COUNTDOWN = 10
const COLORS = ['var(--muted)', 'var(--accent)']

function pct(a: number, b: number): string {
  const d = ((b - a) / a) * 100
  return `${d >= 0 ? '+' : ''}${d.toFixed(1)}%`
}

export function FpsTest() {
  const { t, lang } = useT()
  const { toast, admin } = useApp()
  const [options, setOptions] = useState<{ games: string[]; running: string[] }>({ games: [], running: [] })
  const [game, setGame] = useState('')
  const [seconds, setSeconds] = useState(30)
  const [label, setLabel] = useState<'before' | 'after'>('before')
  const [history, setHistory] = useState<FpsRecord[]>([])
  const [phase, setPhase] = useState<'idle' | 'countdown' | 'recording'>('idle')
  const [left, setLeft] = useState(0)
  const [error, setError] = useState<string | null>(null)
  const [picked, setPicked] = useState<number[]>([])
  const timer = useRef<ReturnType<typeof setInterval> | null>(null)

  useEffect(() => {
    api.fpsHistory().then(setHistory, () => {})
    Promise.all([api.boostGet().catch(() => null), api.detectGames().catch(() => []), api.runningApps().catch(() => [])]).then(
      ([boost, games, running]) => {
        const known = [...new Set<string>([...(boost?.config.games ?? []), ...games.map((g) => g.exe)])]
        setOptions({ games: known, running: running.filter((r) => !known.includes(r)) })
        // Only pre-select something we know is a game; never a random background app.
        setGame((g) => g || known[0] || '')
      },
    )
    return () => {
      if (timer.current) clearInterval(timer.current)
    }
  }, [])

  const start = () => {
    setError(null)
    setPhase('countdown')
    setLeft(COUNTDOWN)
    let n = COUNTDOWN
    timer.current = setInterval(() => {
      n -= 1
      setLeft(n)
      if (n <= 0) {
        clearInterval(timer.current!)
        record()
      }
    }, 1000)
  }

  const record = async () => {
    setPhase('recording')
    let n = seconds
    setLeft(n)
    timer.current = setInterval(() => {
      n = Math.max(0, n - 1)
      setLeft(n)
    }, 1000)
    try {
      const rec = await api.fpsRecord(game, seconds, label)
      setHistory((h) => [rec, ...h])
      setPicked([rec.id])
    } catch (e) {
      setError(errText(e))
    } finally {
      clearInterval(timer.current!)
      setPhase('idle')
    }
  }

  const remove = async (id: number) => {
    try {
      await api.fpsDelete(id)
      setHistory((h) => h.filter((r) => r.id !== id))
      setPicked((p) => p.filter((x) => x !== id))
    } catch (e) {
      toast(errText(e))
    }
  }

  const togglePick = (id: number) =>
    setPicked((p) => (p.includes(id) ? p.filter((x) => x !== id) : [...p, id].slice(-2)))

  const chosen = useMemo(
    () =>
      picked
        .map((id) => history.find((r) => r.id === id))
        .filter((r): r is FpsRecord => !!r)
        .sort((a, b) => a.at - b.at),
    [picked, history],
  )
  const shown = chosen.length ? chosen : history.slice(0, 1)
  const [a, b] = shown.length === 2 ? shown : [null, shown[0]]
  const labelText = (l: string) => (l === 'before' || l === 'after' ? t.fpsLabels[l] : l)
  const total = phase === 'countdown' ? COUNTDOWN : seconds

  return (
    <>
      <PageHead title={t.fpsTitle} sub={t.fpsSub} />
      {error && <Banner kind="error">{error}</Banner>}

      <section className="card stack">
        <div className="grid cols-3">
          <label className="stack" style={{ gap: 6 }}>
            <span className="muted small">{t.fpsGame}</span>
            <select className="input ltr" value={game} onChange={(e) => setGame(e.target.value)} disabled={phase !== 'idle'}>
              <option value="">{t.fpsPickGame}</option>
              {options.games.length > 0 && (
                <optgroup label={t.fpsYourGames}>
                  {options.games.map((o) => (
                    <option key={o} value={o}>
                      {o}
                    </option>
                  ))}
                </optgroup>
              )}
              {options.running.length > 0 && (
                <optgroup label={t.fpsRunning}>
                  {options.running.map((o) => (
                    <option key={o} value={o}>
                      {o}
                    </option>
                  ))}
                </optgroup>
              )}
            </select>
          </label>
          <div className="stack" style={{ gap: 6 }}>
            <span className="muted small">{t.fpsDuration}</span>
            <div className="seg">
              {DURATIONS.map((d) => (
                <button key={d} className={seconds === d ? 'active' : ''} onClick={() => setSeconds(d)} disabled={phase !== 'idle'}>
                  {d}s
                </button>
              ))}
            </div>
          </div>
          <div className="stack" style={{ gap: 6 }}>
            <span className="muted small">{t.fpsLabel}</span>
            <div className="seg">
              {(['before', 'after'] as const).map((l) => (
                <button
                  key={l}
                  className={label === l ? 'active accent' : ''}
                  onClick={() => setLabel(l)}
                  disabled={phase !== 'idle'}
                >
                  {t.fpsLabels[l]}
                </button>
              ))}
            </div>
          </div>
        </div>
        {phase === 'idle' ? (
          <div className="row between wrap">
            <p className="muted small" style={{ maxWidth: 560 }}>
              {t.fpsHowTo}
            </p>
            <button className="btn primary" onClick={start} disabled={!game || !admin}>
              <Play size={16} />
              {t.fpsStart}
            </button>
          </div>
        ) : (
          <div className="stack" style={{ gap: 8 }}>
            <strong>{phase === 'countdown' ? t.fpsCountdown(left) : t.fpsRecording(left)}</strong>
            <div className="progress">
              <span style={{ width: `${((total - left) / total) * 100}%` }} />
            </div>
          </div>
        )}
      </section>

      {b && (
        <section className="card stack">
          <div className="card-head" style={{ marginBottom: 0 }}>
            <h2 className="ltr">{b.game}</h2>
            <span className="sub">
              {a ? `${labelText(a.label)} → ${labelText(b.label)}` : `${labelText(b.label)} · ${formatDate(b.at, lang)}`}
            </span>
          </div>
          <div className="fps-big">
            {(
              [
                [t.fpsAvg, 'avg_fps'],
                [t.fps1, 'low1_fps'],
                [t.fps01, 'low01_fps'],
              ] as const
            ).map(([l, k]) => (
              <div className="fps-cell" key={k}>
                <div className="l">{l}</div>
                <div className="row" style={{ alignItems: 'baseline', gap: 10 }}>
                  <span className="v">{Math.round(b.result[k])}</span>
                  {a && (
                    <span className={`delta ${b.result[k] >= a.result[k] ? 'ok' : 'bad'}`}>
                      {pct(a.result[k], b.result[k])}
                    </span>
                  )}
                </div>
                {a && <div className="muted small">{Math.round(a.result[k])} → {Math.round(b.result[k])}</div>}
              </div>
            ))}
          </div>
          <div>
            <p className="muted small" style={{ marginBottom: 6 }}>
              {t.frameTimes}
            </p>
            <FrameChart
              series={(a ? [a, b] : [b]).map((r, i) => ({ values: r.result.frame_times, color: a ? COLORS[i] : COLORS[1] }))}
            />
          </div>
        </section>
      )}

      <section className="card">
        <div className="card-head">
          <h2>{t.fpsHistory}</h2>
          <span className="sub">{t.fpsCompareHint}</span>
        </div>
        {history.length === 0 ? (
          <p className="empty">{t.fpsEmpty}</p>
        ) : (
          <div className="list">
            {history.map((r) => (
              <div className="list-row" key={r.id}>
                <input
                  type="checkbox"
                  className="check"
                  checked={picked.includes(r.id)}
                  onChange={() => togglePick(r.id)}
                  aria-label={t.fpsCompare}
                />
                <div className="grow">
                  <div className="row" style={{ gap: 8 }}>
                    <strong className="ltr">{r.game}</strong>
                    <span className={`badge ${r.label === 'after' ? 'accent' : ''}`}>{labelText(r.label)}</span>
                  </div>
                  <span className="muted small">
                    {formatDate(r.at, lang)} · {r.result.frames} {t.frames}
                  </span>
                </div>
                <div className="ltr" style={{ textAlign: 'end', fontVariantNumeric: 'tabular-nums' }}>
                  <strong>{Math.round(r.result.avg_fps)} FPS</strong>
                  <div className="muted small">
                    1%: {Math.round(r.result.low1_fps)} · 0.1%: {Math.round(r.result.low01_fps)}
                  </div>
                </div>
                <button className="btn ghost icon-btn" onClick={() => remove(r.id)} aria-label={t.remove}>
                  <Trash2 size={15} />
                </button>
              </div>
            ))}
          </div>
        )}
      </section>
    </>
  )
}
