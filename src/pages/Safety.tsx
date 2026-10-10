import { useEffect, useState } from 'react'
import { History, LifeBuoy, ShieldCheck, ShieldOff, Undo2 } from 'lucide-react'
import { api, type RestorePoint } from '../api'
import { errText, formatDate, useT } from '../i18n'
import { useApp } from '../store'
import { PageHead } from '../components/ui'

export function Safety() {
  const { t, lang } = useT()
  const { tweaks, refreshTweaks, toast, admin } = useApp()
  const [points, setPoints] = useState<RestorePoint[] | null>(null)
  const [pointsError, setPointsError] = useState<string | null>(null)
  const [creating, setCreating] = useState(false)
  const [busy, setBusy] = useState<string | null>(null)

  const loadPoints = () =>
    api.restorePoints().then(
      (p) => {
        setPoints(p)
        setPointsError(null)
      },
      (e) => setPointsError(errText(e)),
    )

  useEffect(() => {
    if (admin) loadPoints()
  }, [admin])

  const applied = tweaks.filter((tw) => tw.applied).sort((a, b) => (b.applied_at ?? 0) - (a.applied_at ?? 0))

  const revert = async (id: string) => {
    setBusy(id)
    try {
      await api.revertTweak(id)
    } catch (e) {
      toast(errText(e))
    }
    await refreshTweaks()
    setBusy(null)
  }

  const create = async () => {
    setCreating(true)
    try {
      const r = await api.createRestorePoint()
      if (r.status === 'Created') {
        toast(t.pointCreated)
        loadPoints()
      } else {
        toast(r.status === 'Failed' ? r.detail : t.restoreOff)
      }
    } catch (e) {
      toast(errText(e))
    } finally {
      setCreating(false)
    }
  }

  return (
    <>
      <PageHead title={t.safetyTitle} sub={t.safetySub} />

      <section className="card">
        <div className="card-head">
          <h2 className="row" style={{ gap: 8 }}>
            <History size={16} color="var(--accent)" />
            {t.changes}
          </h2>
          <span className="sub">{applied.length}</span>
        </div>
        {applied.length === 0 ? (
          <p className="empty">{t.noChanges}</p>
        ) : (
          <div className="list">
            {applied.map((tw) => (
              <div className="list-row" key={tw.id}>
                <div className="grow">
                  <strong>{lang === 'ur' ? tw.name_ur : tw.name_en}</strong>
                  <div className="muted small">
                    {t.categories[tw.category]}
                    {tw.applied_at && ` · ${t.appliedOn(formatDate(tw.applied_at, lang))}`}
                  </div>
                </div>
                <button className="btn sm" onClick={() => revert(tw.id)} disabled={busy !== null}>
                  <Undo2 size={14} />
                  {t.revert}
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      <div className="grid cols-2">
        <section className="card stack">
          <div className="card-head" style={{ marginBottom: 0 }}>
            <h2 className="row" style={{ gap: 8 }}>
              <LifeBuoy size={16} color="var(--accent)" />
              {t.restorePoints}
            </h2>
          </div>
          {!admin ? (
            <p className="muted small">{t.pointsAdmin}</p>
          ) : pointsError ? (
            <p className="bad small ltr">{pointsError}</p>
          ) : points === null ? (
            <p className="muted small">{t.loading}</p>
          ) : points.length === 0 ? (
            <p className="muted small">{t.noPoints}</p>
          ) : (
            <div className="list">
              {points.slice(0, 6).map((p) => (
                <div className="list-row" key={p.SequenceNumber} style={{ padding: '8px 2px' }}>
                  <span className="grow ellipsis">{p.Description}</span>
                  <span className="muted small ltr">{p.Created.replace('T', ' ')}</span>
                </div>
              ))}
            </div>
          )}
          <div className="row wrap">
            <button className="btn primary" onClick={create} disabled={!admin || creating}>
              <ShieldCheck size={15} />
              {creating ? t.creatingPoint : t.createPoint}
            </button>
            <button className="btn" onClick={() => api.openSystemRestore().catch((e) => toast(errText(e)))}>
              {t.openRestore}
            </button>
          </div>
        </section>

        <section className="card stack">
          <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
            <ShieldOff size={16} color="var(--ok)" />
            {t.promises}
          </h2>
          <ul className="stack" style={{ gap: 8, margin: 0, paddingInlineStart: 18 }}>
            {t.promiseList.map((p) => (
              <li key={p} className="muted">
                {p}
              </li>
            ))}
          </ul>
        </section>
      </div>
    </>
  )
}
