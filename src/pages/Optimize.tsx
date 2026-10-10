import { useMemo, useState } from 'react'
import { Copy, Crown, Download, Search } from 'lucide-react'
import { api, decodeShareCode, encodeShareCode, type Category, type Preset, type TweakView } from '../api'
import { errText, tweakDesc, tweakName, useT } from '../i18n'
import { failText, useApp } from '../store'
import { CATEGORY_ICON } from '../components/icons'
import { PageHead, Toggle } from '../components/ui'

const ORDER: Category[] = ['Fps', 'Power', 'Background', 'Privacy', 'Visuals', 'Network', 'Storage']

export function Optimize() {
  const { t, lang } = useT()
  const { tweaks, info, apply, refreshTweaks, toast, premium, go } = useApp()
  const [cat, setCat] = useState<Category | 'all'>('all')
  const [q, setQ] = useState('')
  const [busy, setBusy] = useState<string | null>(null)
  const [code, setCode] = useState('')

  const name = (tw: TweakView) => tweakName(tw, lang)
  const desc = (tw: TweakView) => tweakDesc(tw, lang)

  const visible = useMemo(() => {
    const needle = q.trim().toLowerCase()
    return tweaks.filter(
      (tw) =>
        (cat === 'all' || tw.category === cat) &&
        (!needle || `${tw.name_en} ${tweakName(tw, lang)} ${tweakDesc(tw, lang)}`.toLowerCase().includes(needle)),
    )
  }, [tweaks, cat, q, lang])

  const toggle = async (tw: TweakView) => {
    // Reverting is always free; only switching a Premium tweak on is locked.
    if (!tw.applied && tw.premium && !premium) {
      toast(t.premiumNeeded)
      go('premium')
      return
    }
    setBusy(tw.id)
    try {
      if (tw.applied) {
        await api.revertTweak(tw.id)
        await refreshTweaks()
      } else {
        const res = await apply([tw.id])
        if (res?.failed.length) toast(failText(res.failed[0][1], t))
      }
    } catch (e) {
      toast(errText(e))
      await refreshTweaks()
    } finally {
      setBusy(null)
    }
  }

  const applyPreset = async (p: Preset) => {
    setBusy('preset')
    try {
      const ids = await api.presetIds(p)
      const pending = ids.filter((id) => !tweaks.find((tw) => tw.id === id)?.applied)
      const res = await apply(pending)
      if (res) toast(t.appliedCount(res.applied.length))
    } catch (e) {
      toast(errText(e))
    } finally {
      setBusy(null)
    }
  }

  const copyCode = async () => {
    const c = encodeShareCode(tweaks.filter((tw) => tw.applied).map((tw) => tw.id))
    try {
      await navigator.clipboard.writeText(c)
      toast(t.copied)
    } catch {
      setCode(c)
    }
  }

  const importCode = async () => {
    const ids = decodeShareCode(code, tweaks.map((tw) => tw.id))
    if (!ids) {
      toast(t.badCode)
      return
    }
    setBusy('preset')
    const res = await apply(ids.filter((id) => !tweaks.find((tw) => tw.id === id)?.applied))
    setBusy(null)
    if (res) {
      toast(t.appliedCount(res.applied.length))
      setCode('')
    }
  }

  const count = (c: Category | 'all') => tweaks.filter((tw) => c === 'all' || tw.category === c).length
  const groups = ORDER.map((c) => [c, visible.filter((tw) => tw.category === c)] as const).filter(([, l]) => l.length)

  return (
    <>
      <PageHead title={t.optTitle} sub={t.optSub}>
        <span className="badge accent">{t.activeCount(tweaks.filter((x) => x.applied).length, tweaks.length)}</span>
      </PageHead>

      <section className="card stack">
        <div className="row between wrap">
          <div className="row wrap">
            <span className="muted small">{t.applyPreset}:</span>
            {(['Lite', 'Balanced', 'Max'] as Preset[]).map((p) => (
              <button key={p} className="btn sm" disabled={busy !== null || !info} onClick={() => applyPreset(p)}>
                {t.presets[p]}
              </button>
            ))}
          </div>
          <div className="row wrap">
            <button className="btn sm ghost" onClick={copyCode} disabled={!tweaks.some((x) => x.applied)}>
              <Copy size={14} />
              {t.copyCode}
            </button>
            <input
              className="input ltr"
              style={{ width: 200, padding: '5px 10px', fontSize: 12 }}
              placeholder={t.importPlaceholder}
              value={code}
              onChange={(e) => setCode(e.target.value)}
              aria-label={t.shareCode}
            />
            <button className="btn sm" onClick={importCode} disabled={!code.trim() || busy !== null}>
              <Download size={14} />
              {t.importCode}
            </button>
          </div>
        </div>
      </section>

      <div className="row between wrap">
        <div className="chips" role="tablist">
          {(['all', ...ORDER] as const).map((c) => (
            <button
              key={c}
              role="tab"
              aria-selected={cat === c}
              className={`chip-btn ${cat === c ? 'active' : ''}`}
              onClick={() => setCat(c)}
            >
              {c === 'all' ? t.all : t.categories[c]}
              <span className="count">{count(c)}</span>
            </button>
          ))}
        </div>
        <label className="input-icon" style={{ width: 240 }}>
          <Search size={15} />
          <input className="input" placeholder={t.search} value={q} onChange={(e) => setQ(e.target.value)} />
        </label>
      </div>

      {groups.map(([c, list]) => {
        const Icon = CATEGORY_ICON[c]
        return (
          <section key={c} className="stack" style={{ gap: 8 }}>
            <div className="section-title">
              <Icon size={15} />
              {t.categories[c]}
            </div>
            {list.map((tw) => (
              <article key={tw.id} className={`tweak ${tw.applied ? 'applied' : ''}`}>
                <div className="grow">
                  <h3>{name(tw)}</h3>
                  <p>{desc(tw)}</p>
                  <div className="badges">
                    <span className={`badge ${tw.risk === 'Low' ? 'ok' : 'warn'}`}>
                      {tw.risk === 'Low' ? t.riskLow : t.riskMedium}
                    </span>
                    {tw.needs_restart && <span className="badge info">{t.needsRestart}</span>}
                    {tw.preset === null ? (
                      <span className="badge">{t.fixOnly}</span>
                    ) : (
                      info && tw.recommended_for.includes(info.class) && <span className="badge accent">{t.recommended}</span>
                    )}
                    {tw.preset && <span className="badge">{t.presets[tw.preset]}</span>}
                    {tw.premium && (
                      <span className="badge accent">
                        <Crown size={11} />
                        {t.navPremium}
                      </span>
                    )}
                  </div>
                </div>
                <Toggle on={tw.applied} label={name(tw)} disabled={busy !== null} onChange={() => toggle(tw)} />
              </article>
            ))}
          </section>
        )
      })}
    </>
  )
}
