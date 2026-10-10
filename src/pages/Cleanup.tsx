import { useEffect, useState } from 'react'
import { Bug, FileWarning, Globe, Layers, RefreshCw, Sparkles, Trash2, Folder } from 'lucide-react'
import { api, formatBytes, type CategoryReport, type CleanCategory } from '../api'
import { errText, useT } from '../i18n'
import { Banner, PageHead } from '../components/ui'

const ICON: Record<CleanCategory, typeof Folder> = {
  UserTemp: Folder,
  WindowsTemp: Folder,
  ShaderCache: Layers,
  CrashDumps: Bug,
  ErrorReports: FileWarning,
  BrowserCache: Globe,
}
const CATS = Object.keys(ICON) as CleanCategory[]
// Shader caches rebuild with a stutter, so they are opt-in.
const DEFAULT_ON: CleanCategory[] = ['UserTemp', 'WindowsTemp', 'CrashDumps', 'ErrorReports', 'BrowserCache']

export function Cleanup() {
  const { t } = useT()
  const [scan, setScan] = useState<Partial<Record<CleanCategory, CategoryReport>>>({})
  const [selected, setSelected] = useState<CleanCategory[]>(DEFAULT_ON)
  const [busy, setBusy] = useState<'scan' | 'clean' | null>(null)
  const [done, setDone] = useState<{ bytes: number; files: number; skipped: number } | null>(null)
  const [error, setError] = useState<string | null>(null)

  /** Scans every category in parallel; each row fills in as soon as its own scan finishes. */
  const runScan = async () => {
    setBusy('scan')
    setError(null)
    setScan({})
    await Promise.all(
      CATS.map((c) =>
        api.cleanupScan(c).then(
          (r) => setScan((s) => ({ ...s, [c]: r })),
          (e) => setError(errText(e)),
        ),
      ),
    )
    setBusy(null)
  }

  // A scan is a dry run and deletes nothing, so do it as soon as the page opens.
  useEffect(() => {
    runScan()
  }, [])

  const clean = async () => {
    setBusy('clean')
    setError(null)
    try {
      const res = await api.cleanupRun(selected)
      setDone(
        res.reduce((acc, r) => ({ bytes: acc.bytes + r.bytes, files: acc.files + r.files, skipped: acc.skipped + r.skipped }), {
          bytes: 0,
          files: 0,
          skipped: 0,
        }),
      )
    } catch (e) {
      setError(errText(e))
    }
    setBusy(null)
    await runScan()
  }

  const scanned = CATS.every((c) => scan[c])
  const total = selected.reduce((s, c) => s + (scan[c]?.bytes ?? 0), 0)
  const toggle = (c: CleanCategory) => setSelected((s) => (s.includes(c) ? s.filter((x) => x !== c) : [...s, c]))

  return (
    <>
      <PageHead title={t.cleanTitle} sub={t.cleanSub}>
        <button className="btn" onClick={runScan} disabled={busy !== null}>
          <RefreshCw size={15} className={busy === 'scan' ? 'spin' : ''} />
          {busy === 'scan' ? t.cleanScanning : t.cleanScan}
        </button>
      </PageHead>
      <Banner kind="warn" role="alert">
        {t.cleanWarn}
      </Banner>
      {error && <Banner kind="error">{error}</Banner>}
      {done && (
        <Banner kind="ok">
          {t.freed(formatBytes(done.bytes), done.files)} {done.skipped > 0 && t.skipped(done.skipped)}
        </Banner>
      )}

      <section className="card">
        <div className="list">
          {CATS.map((c) => {
            const Icon = ICON[c]
            const r = scan[c]
            const [title, desc] = t.cleanCats[c]
            return (
              <label key={c} className="list-row" style={{ cursor: 'pointer' }}>
                <input type="checkbox" className="check" checked={selected.includes(c)} onChange={() => toggle(c)} />
                <Icon size={18} color="var(--accent)" />
                <div className="grow">
                  <strong>{title}</strong>
                  <p className="muted small">{desc}</p>
                  {r && r.blocked_by.length > 0 && <p className="small" style={{ color: 'var(--warn)' }}>{t.closeFirst(r.blocked_by.join(', '))}</p>}
                </div>
                <span className="ltr" style={{ fontVariantNumeric: 'tabular-nums', fontWeight: 650, minWidth: 80, textAlign: 'end' }}>
                  {r ? formatBytes(r.bytes) : <RefreshCw size={14} className="spin muted" />}
                </span>
              </label>
            )
          })}
        </div>
        <div className="row between" style={{ marginTop: 16 }}>
          <span className="muted small">
            <Sparkles size={13} style={{ verticalAlign: -2 }} /> {formatBytes(total)}
          </span>
          <button className="btn danger" onClick={clean} disabled={!scanned || busy !== null || total === 0}>
            <Trash2 size={15} />
            {busy === 'clean' ? t.cleaning : scanned ? t.cleanSelected(formatBytes(total)) : t.cleanScanning}
          </button>
        </div>
      </section>
    </>
  )
}
