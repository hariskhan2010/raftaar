import { useEffect, useState } from 'react'
import { Activity, Cpu, Gamepad2, Rocket, Zap } from 'lucide-react'
import { api, type ApplyResult, type FpsRecord, type Preset, type Score, type Stats } from '../api'
import { errText, tweakName, useT } from '../i18n'
import { useApp } from '../store'
import { Banner, Gauge, Meter, PageHead } from '../components/ui'

const PRESETS: Preset[] = ['Lite', 'Balanced', 'Max']

export function Home() {
  const { t, lang } = useT()
  const { info, tweaks, apply, boosting, go } = useApp()
  const [before, setBefore] = useState<Score | null>(null)
  const [after, setAfter] = useState<Score | null>(null)
  const [measuring, setMeasuring] = useState(false)
  const [preset, setPreset] = useState<Preset>('Balanced')
  // null until loaded, so we never flash "everything is applied".
  const [presetIds, setPresetIds] = useState<string[] | null>(null)
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<ApplyResult | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [lastFps, setLastFps] = useState<FpsRecord | null>(null)

  useEffect(() => {
    setMeasuring(true)
    api
      .loadScore()
      .then(setBefore, (e) => setError(errText(e)))
      .finally(() => setMeasuring(false))
    api.fpsHistory().then((h) => setLastFps(h[0] ?? null), () => {})
  }, [])

  useEffect(() => {
    if (!info) return
    setPresetIds(null)
    api.presetIds(preset).then(setPresetIds, (e) => setError(errText(e)))
  }, [preset, info])

  const pending = (presetIds ?? []).filter((id) => !tweaks.find((tw) => tw.id === id)?.applied)
  const ready = info !== null && presetIds !== null

  const measure = async () => {
    setMeasuring(true)
    try {
      setAfter(await api.loadScore())
    } catch (e) {
      setError(errText(e))
    } finally {
      setMeasuring(false)
    }
  }

  const optimize = async () => {
    setBusy(true)
    setResult(null)
    const res = await apply(pending)
    setBusy(false)
    if (!res) return
    setResult(res)
    await measure()
  }

  const name = (id: string) => {
    const tw = tweaks.find((x) => x.id === id)
    return tw ? tweakName(tw, lang) : id
  }
  const restartNeeded = result?.applied.some((id) => tweaks.find((x) => x.id === id)?.needs_restart)
  const shown = after ?? before

  return (
    <>
      <PageHead title={t.homeTitle} sub={t.homeSub} />
      {error && <Banner kind="error">{error}</Banner>}

      <div className="grid hero">
        <section className="card">
          <div className="row" style={{ gap: 24, alignItems: 'center', flexWrap: 'wrap' }}>
            <Gauge value={shown?.score ?? null} label={t.loadScore} />
            <div className="stack" style={{ flex: 1, minWidth: 220 }}>
              {info ? (
                <>
                  <span className={`class-badge class-${info.class.toLowerCase()}`}>
                    <Cpu size={13} />
                    {t.pcClass}: {t.classes[info.class]}
                  </span>
                  <dl className="specs">
                    <dt>{t.cpu}</dt>
                    <dd className="ltr">
                      {info.cpu} ({info.cores}C/{info.threads}T)
                    </dd>
                    <dt>{t.gpu}</dt>
                    <dd className="ltr">
                      {info.gpu_name}
                      {info.vram_gb > 0 && ` · ${info.vram_gb.toFixed(1)} GB`}
                    </dd>
                    <dt>{t.ram}</dt>
                    <dd className="ltr">{info.ram_gb.toFixed(1)} GB</dd>
                    <dt>{t.windows}</dt>
                    <dd className="ltr">{info.windows}</dd>
                  </dl>
                </>
              ) : (
                <p className="muted">{t.scanning}</p>
              )}
            </div>
          </div>
          <div className="row between" style={{ marginTop: 16 }}>
            <div className="row" style={{ gap: 18 }}>
              <div className="stat">
                <span className="l">{t.before}</span>
                <span className="v">{before ? before.score : '–'}</span>
              </div>
              <div className="stat">
                <span className="l">{t.after}</span>
                <span className="v" style={{ color: after && before && after.score > before.score ? 'var(--ok)' : undefined }}>
                  {after ? after.score : '–'}
                </span>
              </div>
            </div>
            <button className="btn ghost sm" onClick={measure} disabled={measuring}>
              <Activity size={14} className={measuring ? 'spin' : ''} />
              {measuring ? t.measuring : t.measureAgain}
            </button>
          </div>
          <p className="muted small" style={{ marginTop: 8 }}>
            {t.loadScoreNote}
          </p>
        </section>

        <section className="card stack">
          <div className="card-head" style={{ marginBottom: 0 }}>
            <h2>{t.quickOptimize}</h2>
            <Rocket size={18} color="var(--accent)" />
          </div>
          <div className="presets" role="radiogroup">
            {PRESETS.map((p) => (
              <button
                key={p}
                role="radio"
                aria-checked={preset === p}
                className={`preset ${preset === p ? 'active' : ''}`}
                onClick={() => setPreset(p)}
              >
                <strong>{t.presets[p]}</strong>
                <span>{t.presetDesc[p]}</span>
              </button>
            ))}
          </div>
          {ready && pending.length === 0 && !busy ? (
            <Banner kind="ok">{t.allApplied}</Banner>
          ) : (
            <button className="btn primary big" onClick={optimize} disabled={!ready || busy}>
              <Zap size={18} />
              {busy ? t.optimizing : t.optimize}
            </button>
          )}
          {ready && pending.length > 0 && <p className="muted small">{t.willApply(pending.length)}</p>}
          {result && (
            <div className="stack" style={{ gap: 6 }}>
              {result.applied.length > 0 && <Banner kind="ok">{t.appliedCount(result.applied.length)}</Banner>}
              {result.failed.length > 0 && (
                <Banner kind="error">
                  {t.failedCount(result.failed.length)}
                  <ul style={{ margin: '4px 0 0', paddingInlineStart: 18 }}>
                    {result.failed.map(([id, msg]) => (
                      <li key={id}>
                        {name(id)}: <span className="ltr">{msg}</span>
                      </li>
                    ))}
                  </ul>
                </Banner>
              )}
              {restartNeeded && <Banner kind="info">{t.restartNeeded}</Banner>}
            </div>
          )}
        </section>
      </div>

      <div className="grid cols-3">
        <button className="card stat" style={{ textAlign: 'start', cursor: 'pointer' }} onClick={() => go('optimize')}>
          <span className="l">{t.statApplied}</span>
          <span className="v">
            {tweaks.filter((x) => x.applied).length}
            <span className="muted small"> / {tweaks.length}</span>
          </span>
        </button>
        <button className="card stat" style={{ textAlign: 'start', cursor: 'pointer' }} onClick={() => go('boost')}>
          <span className="l">{t.nav.boost}</span>
          <span className="v row" style={{ fontSize: 16, gap: 8 }}>
            <Gamepad2 size={18} color={boosting ? 'var(--accent)' : 'var(--muted)'} />
            <span className="ellipsis">{boosting ? t.boostActive(boosting) : t.boostDisabled}</span>
          </span>
        </button>
        <button className="card stat" style={{ textAlign: 'start', cursor: 'pointer' }} onClick={() => go('fps')}>
          <span className="l">{t.statLastFps}</span>
          <span className="v">
            {lastFps ? Math.round(lastFps.result.avg_fps) : '–'}
            {lastFps && <span className="muted small ltr"> FPS · {lastFps.game}</span>}
          </span>
        </button>
      </div>

      <LiveMonitor />
    </>
  )
}

function LiveMonitor() {
  const { t } = useT()
  const [stats, setStats] = useState<Stats | null>(null)
  const [hist, setHist] = useState<{ cpu: number[]; ram: number[]; gpu: number[] }>({ cpu: [], ram: [], gpu: [] })

  useEffect(() => {
    let alive = true
    const tick = async () => {
      try {
        const s = await api.monitor()
        if (!alive) return
        setStats(s)
        const push = (a: number[], v: number) => [...a.slice(-39), v]
        setHist((h) => ({
          cpu: push(h.cpu, s.cpu_pct),
          ram: push(h.ram, (s.ram_used_gb / s.ram_total_gb) * 100),
          gpu: push(h.gpu, s.gpu_pct ?? 0),
        }))
      } catch {
        /* monitor unavailable */
      }
    }
    tick()
    const id = setInterval(tick, 1500)
    return () => {
      alive = false
      clearInterval(id)
    }
  }, [])

  return (
    <section className="card">
      <div className="card-head">
        <h2>
          <span className="row" style={{ gap: 8 }}>
            <span className="dot" />
            {t.live}
          </span>
        </h2>
      </div>
      <div className="grid cols-3">
        <Meter
          label={t.cpu}
          value={stats?.cpu_pct ?? 0}
          display={stats ? `${stats.cpu_pct.toFixed(0)}%` : '–'}
          history={hist.cpu}
        />
        <Meter
          label={t.ram}
          value={stats ? (stats.ram_used_gb / stats.ram_total_gb) * 100 : 0}
          display={stats ? `${stats.ram_used_gb.toFixed(1)} / ${stats.ram_total_gb.toFixed(1)} GB` : '–'}
          history={hist.ram}
        />
        <Meter
          label={t.gpu}
          value={stats?.gpu_pct ?? 0}
          display={stats ? (stats.gpu_pct == null ? t.gpuNa : `${stats.gpu_pct.toFixed(0)}%`) : '–'}
          history={hist.gpu}
        />
      </div>
      {stats && stats.top.length > 0 && (
        <>
          <p className="muted small" style={{ margin: '18px 0 6px' }}>
            {t.topApps}
          </p>
          <div className="list">
            {stats.top.slice(0, 6).map((p) => (
              <div className="list-row" key={p.name} style={{ padding: '7px 4px' }}>
                <span className="grow ltr ellipsis">{p.name}</span>
                <span className="muted small ltr" style={{ width: 70, textAlign: 'end' }}>
                  {p.cpu_pct.toFixed(1)}%
                </span>
                <span className="ltr" style={{ width: 90, textAlign: 'end', fontVariantNumeric: 'tabular-nums' }}>
                  {p.ram_mb >= 1024 ? `${(p.ram_mb / 1024).toFixed(1)} GB` : `${p.ram_mb.toFixed(0)} MB`}
                </span>
              </div>
            ))}
          </div>
        </>
      )}
    </section>
  )
}
