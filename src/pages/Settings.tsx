import { useState } from 'react'
import { Download, Globe, Info, Lock, RefreshCw, Scale } from 'lucide-react'
import type { Update } from '@tauri-apps/plugin-updater'
import { LANGS, useT } from '../i18n'
import { useApp } from '../store'
import { checkForUpdate, installUpdate } from '../update'
import { PageHead } from '../components/ui'

const VERSION = __APP_VERSION__

export function Settings() {
  const { t, lang, setLang } = useT()
  const { admin } = useApp()
  const [state, setState] = useState<'idle' | 'checking' | 'none' | 'checkFailed' | 'failed'>('idle')
  const [update, setUpdate] = useState<Update | null>(null)
  const [progress, setProgress] = useState<number | null>(null)

  const check = async () => {
    setState('checking')
    const r = await checkForUpdate()
    setUpdate(r.kind === 'update' ? r.update : null)
    setState(r.kind === 'update' ? 'idle' : r.kind === 'none' ? 'none' : 'checkFailed')
  }

  const install = async () => {
    if (!update) return
    setProgress(0)
    try {
      await installUpdate(update, setProgress)
    } catch {
      setProgress(null)
      setState('failed')
    }
  }

  return (
    <>
      <PageHead title={t.settingsTitle} />
      <section className="card stack">
        <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
          <Globe size={16} color="var(--accent)" />
          {t.language}
        </h2>
        <div className="chips" role="radiogroup" aria-label={t.language}>
          {LANGS.map((l) => (
            <button
              key={l.code}
              role="radio"
              aria-checked={lang === l.code}
              lang={l.code}
              className={`chip-btn ${lang === l.code ? 'active' : ''}`}
              onClick={() => setLang(l.code)}
            >
              {l.native}
            </button>
          ))}
        </div>
        {lang !== 'en' && lang !== 'ur' && <p className="muted small">{t.translationNote}</p>}
      </section>

      <section className="card stack">
        <div className="row between wrap">
          <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
            <Download size={16} color="var(--accent)" />
            {t.updates}
          </h2>
          {update ? (
            <button className="btn primary" onClick={install} disabled={progress !== null}>
              {progress !== null ? t.updating(progress) : t.updateInstall}
            </button>
          ) : (
            <button className="btn" onClick={check} disabled={state === 'checking'}>
              <RefreshCw size={15} className={state === 'checking' ? 'spin' : ''} />
              {state === 'checking' ? t.checking : t.checkUpdates}
            </button>
          )}
        </div>
        {update && <p>{t.updateAvailable(update.version)}</p>}
        {state === 'none' && <p className="ok">{t.upToDate}</p>}
        {state === 'checkFailed' && <p className="bad">{t.updateCheckFailed}</p>}
        {state === 'failed' && <p className="bad">{t.updateFailed}</p>}
        <p className="muted small">{t.updateNote}</p>
      </section>

      <div className="grid cols-3">
        <section className="card stack">
          <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
            <Info size={16} color="var(--accent)" />
            {t.about}
          </h2>
          <p className="muted">
            {t.appName} · {t.version} <span className="ltr">{VERSION}</span>
          </p>
          <p className="muted small">{admin ? t.adminOk : t.adminNo}</p>
        </section>
        <section className="card stack">
          <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
            <Lock size={16} color="var(--accent)" />
            {t.privacy}
          </h2>
          <p className="muted">{t.privacyText}</p>
        </section>
        <section className="card stack">
          <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
            <Scale size={16} color="var(--accent)" />
            {t.licenses}
          </h2>
          <p className="muted">{t.licenseText}</p>
        </section>
      </div>
    </>
  )
}
