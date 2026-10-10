import { useEffect, useState } from 'react'
import { api, type ApplyResult, type PcClass, type TweakView } from '../api'
import { errText, useT } from '../i18n'

interface Props {
  tweaks: TweakView[]
  apply: (ids: string[]) => Promise<ApplyResult | null>
  onChange: () => Promise<void>
  onError: (msg: string) => void
}

export function Tweaks({ tweaks, apply, onChange, onError }: Props) {
  const { lang, t } = useT()
  const [busy, setBusy] = useState<string | null>(null)
  const [pcClass, setPcClass] = useState<PcClass | null>(null)

  useEffect(() => {
    api.scan().then((i) => setPcClass(i.class), () => {})
  }, [])

  const toggle = async (tw: TweakView) => {
    setBusy(tw.id)
    try {
      if (tw.applied) {
        await api.revertTweak(tw.id)
        await onChange()
      } else {
        const res = await apply([tw.id])
        if (res?.failed.length) onError(res.failed[0][1])
      }
    } catch (e) {
      onError(errText(e))
      await onChange()
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="stack">
      <p className="muted">{t.tweaksIntro}</p>
      {tweaks.map((tw) => (
        <section key={tw.id} className={`card tweak ${tw.applied ? 'applied' : ''}`}>
          <div className="tweak-body">
            <h3>{lang === 'ur' ? tw.name_ur : tw.name_en}</h3>
            <p>{lang === 'ur' ? tw.desc_ur : tw.desc_en}</p>
            <div className="badges">
              <span className={`badge risk-${tw.risk.toLowerCase()}`}>
                {tw.risk === 'Low' ? t.riskLow : t.riskMedium}
              </span>
              {tw.needs_restart && <span className="badge restart">{t.needsRestart}</span>}
              {pcClass && tw.recommended_for.includes(pcClass) && <span className="badge rec">{t.recommended}</span>}
            </div>
          </div>
          <button
            role="switch"
            aria-checked={tw.applied}
            aria-label={lang === 'ur' ? tw.name_ur : tw.name_en}
            className={`switch ${tw.applied ? 'on' : ''}`}
            disabled={busy !== null}
            onClick={() => toggle(tw)}
          >
            <span className="knob" />
            <span className="switch-text">{tw.applied ? t.on : t.off}</span>
          </button>
        </section>
      ))}
    </div>
  )
}
