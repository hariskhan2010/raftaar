import { useEffect, useState } from 'react'
import { api, type ApplyResult, type PcClass, type Score, type SystemInfo, type TweakView } from '../api'
import { errText, useT } from '../i18n'

interface Props {
  tweaks: TweakView[]
  apply: (ids: string[]) => Promise<ApplyResult | null>
}

export function Dashboard({ tweaks, apply }: Props) {
  const { lang, t } = useT()
  const [info, setInfo] = useState<SystemInfo | null>(null)
  const [before, setBefore] = useState<Score | null>(null)
  const [after, setAfter] = useState<Score | null>(null)
  const [measuring, setMeasuring] = useState(false)
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<ApplyResult | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api.scan().then(setInfo, (e) => setError(errText(e)))
    setMeasuring(true)
    api
      .loadScore()
      .then(setBefore, (e) => setError(errText(e)))
      .finally(() => setMeasuring(false))
  }, [])

  const classLabel: Record<PcClass, string> = { Low: t.classLow, Mid: t.classMid, High: t.classHigh }
  const recommended = info ? tweaks.filter((tw) => tw.recommended_for.includes(info.class)) : []
  const pending = recommended.filter((tw) => !tw.applied).map((tw) => tw.id)

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
    return tw ? (lang === 'ur' ? tw.name_ur : tw.name_en) : id
  }
  const restartNeeded = result?.applied.some((id) => tweaks.find((x) => x.id === id)?.needs_restart)

  return (
    <div className="stack">
      {error && <div className="banner error">{error}</div>}
      <section className="card">
        <h2>{t.yourPc}</h2>
        {!info ? (
          <p className="muted">{t.scanning}</p>
        ) : (
          <>
            <div className={`class-badge class-${info.class.toLowerCase()}`}>
              {t.pcClass}: {classLabel[info.class]}
            </div>
            <dl className="specs">
              <dt>{t.cpu}</dt>
              <dd dir="ltr">
                {info.cpu} ({info.cores}C/{info.threads}T)
              </dd>
              <dt>{t.ram}</dt>
              <dd dir="ltr">{info.ram_gb.toFixed(1)} GB</dd>
              <dt>{t.gpu}</dt>
              <dd dir="ltr">
                {info.gpu_name}
                {info.vram_gb > 0 && ` (${info.vram_gb.toFixed(1)} GB)`}
              </dd>
              <dt>{t.windows}</dt>
              <dd dir="ltr">{info.windows}</dd>
              <dt>{t.startupApps}</dt>
              <dd>{info.startup_apps}</dd>
              <dt>{t.processes}</dt>
              <dd>{info.process_count}</dd>
            </dl>
          </>
        )}
      </section>

      <section className="card">
        <h2>{t.loadScore}</h2>
        <p className="muted small">{t.loadScoreNote}</p>
        <div className="scores">
          <ScoreBox label={t.before} score={before} />
          <ScoreBox label={t.after} score={after} />
        </div>
        {measuring ? (
          <p className="muted">{t.measuring}</p>
        ) : (
          before && (
            <button className="btn ghost" onClick={measure}>
              {t.measureAgain}
            </button>
          )
        )}
      </section>

      <section className="card optimize">
        {info && pending.length === 0 ? (
          <p>{t.allApplied}</p>
        ) : (
          <>
            <button className="btn primary big" onClick={optimize} disabled={!info || busy}>
              {busy ? t.optimizing : t.optimize}
            </button>
            <p className="muted small">{t.optimizeHint(pending.length)}</p>
          </>
        )}
        {result && (
          <div className="result">
            {result.applied.length > 0 && <p className="ok">{t.appliedCount(result.applied.length)}</p>}
            {result.failed.length > 0 && (
              <>
                <p className="bad">{t.failedCount(result.failed.length)}</p>
                <ul>
                  {result.failed.map(([id, msg]) => (
                    <li key={id}>
                      {name(id)}: <span dir="ltr">{msg}</span>
                    </li>
                  ))}
                </ul>
              </>
            )}
            {restartNeeded && <p className="muted">{t.restartNeeded}</p>}
          </div>
        )}
      </section>
    </div>
  )
}

function ScoreBox({ label, score }: { label: string; score: Score | null }) {
  return (
    <div className="score">
      <div className="score-label">{label}</div>
      <div className="score-value">{score ? score.score : '–'}</div>
      {score && (
        <div className="score-detail" dir="ltr">
          RAM {score.sample.ram_used_pct.toFixed(0)}% · CPU {score.sample.cpu_busy_pct.toFixed(0)}% ·{' '}
          {score.sample.process_count} proc
        </div>
      )}
    </div>
  )
}
