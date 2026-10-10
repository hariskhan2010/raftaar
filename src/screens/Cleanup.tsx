import { useState } from 'react'
import { api, formatBytes, type CleanupReport } from '../api'
import { errText, useT } from '../i18n'

export function Cleanup() {
  const { t } = useT()
  const [preview, setPreview] = useState<CleanupReport | null>(null)
  const [done, setDone] = useState<CleanupReport | null>(null)
  const [busy, setBusy] = useState<'check' | 'clean' | null>(null)
  const [error, setError] = useState<string | null>(null)

  const run = async (dryRun: boolean) => {
    setBusy(dryRun ? 'check' : 'clean')
    setError(null)
    try {
      const r = await api.cleanup(dryRun)
      if (dryRun) {
        setPreview(r)
        setDone(null)
      } else {
        setDone(r)
        setPreview(null)
      }
    } catch (e) {
      setError(errText(e))
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="stack">
      <p className="muted">{t.cleanupIntro}</p>
      <div className="banner warn" role="alert">
        {t.cleanupWarn}
      </div>
      {error && <div className="banner error">{error}</div>}
      <section className="card">
        <button className="btn" onClick={() => run(true)} disabled={busy !== null}>
          {busy === 'check' ? t.checking : t.checkSize}
        </button>
        {preview && (
          <>
            <p>{t.canFree(formatBytes(preview.bytes), preview.files)}</p>
            {preview.files > 0 && (
              <button className="btn danger" onClick={() => run(false)} disabled={busy !== null}>
                {busy === 'clean' ? t.cleaning : t.cleanNow}
              </button>
            )}
          </>
        )}
        {done && (
          <>
            <p className="ok">{t.freed(formatBytes(done.bytes), done.files)}</p>
            {done.skipped > 0 && <p className="muted">{t.skipped(done.skipped)}</p>}
          </>
        )}
      </section>
    </div>
  )
}
