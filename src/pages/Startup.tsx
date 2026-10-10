import { useEffect, useState } from 'react'
import { RefreshCw, Search } from 'lucide-react'
import { api, type StartupItem } from '../api'
import { errText, useT } from '../i18n'
import { useApp } from '../store'
import { Banner, PageHead, Toggle } from '../components/ui'

const ADMIN_SOURCES = ['MachineRun', 'MachineRun32', 'MachineFolder']

export function Startup() {
  const { t } = useT()
  const { admin, toast } = useApp()
  const [items, setItems] = useState<StartupItem[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [q, setQ] = useState('')

  const load = () => api.startupList().then(setItems, (e) => setError(errText(e)))
  useEffect(() => {
    load()
  }, [])

  const toggle = async (it: StartupItem) => {
    const key = it.source + it.name
    setBusy(key)
    try {
      await api.startupSet(it.source, it.name, !it.enabled)
      setItems((list) => list?.map((x) => (x.source + x.name === key ? { ...x, enabled: !x.enabled } : x)) ?? null)
    } catch (e) {
      toast(errText(e))
    } finally {
      setBusy(null)
    }
  }

  const visible = (items ?? []).filter((i) => `${i.name} ${i.command}`.toLowerCase().includes(q.trim().toLowerCase()))

  return (
    <>
      <PageHead title={t.startTitle} sub={t.startSub}>
        {items && <span className="badge accent">{t.enabledCount(items.filter((i) => i.enabled).length, items.length)}</span>}
        <button className="btn ghost icon-btn" onClick={load} aria-label={t.refresh}>
          <RefreshCw size={15} />
        </button>
      </PageHead>
      {error && <Banner kind="error">{error}</Banner>}

      <section className="card">
        <label className="input-icon" style={{ marginBottom: 8 }}>
          <Search size={15} />
          <input className="input" placeholder={t.searchApps} value={q} onChange={(e) => setQ(e.target.value)} />
        </label>
        {items === null ? (
          <p className="empty">{t.loading}</p>
        ) : visible.length === 0 ? (
          <p className="empty">{t.startEmpty}</p>
        ) : (
          <div className="list">
            {visible.map((it) => {
              const locked = ADMIN_SOURCES.includes(it.source) && !admin
              return (
                <div className="list-row" key={it.source + it.name} style={{ opacity: it.enabled ? 1 : 0.6 }}>
                  <div className="grow">
                    <div className="row" style={{ gap: 8 }}>
                      <strong className="ltr ellipsis">{it.name.replace(/\.lnk$/i, '')}</strong>
                      <span className="badge">{t.sources[it.source]}</span>
                      {locked && <span className="badge warn">{t.needsAdmin}</span>}
                    </div>
                    <div className="muted small mono ltr ellipsis selectable" title={it.command}>
                      {it.command}
                    </div>
                  </div>
                  <Toggle
                    on={it.enabled}
                    label={it.name}
                    disabled={busy !== null || locked}
                    onChange={() => toggle(it)}
                  />
                </div>
              )
            })}
          </div>
        )}
      </section>
    </>
  )
}
