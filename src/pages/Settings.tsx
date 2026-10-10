import { Globe, Info, Lock, Scale } from 'lucide-react'
import { useT } from '../i18n'
import { useApp } from '../store'
import { PageHead } from '../components/ui'

export const VERSION = '0.2.0'

export function Settings() {
  const { t, lang, setLang } = useT()
  const { admin } = useApp()
  return (
    <>
      <PageHead title={t.settingsTitle} />
      <section className="card row between wrap">
        <h2 className="row" style={{ gap: 8, fontSize: 15 }}>
          <Globe size={16} color="var(--accent)" />
          {t.language}
        </h2>
        <div className="seg">
          <button className={lang === 'en' ? 'active accent' : ''} onClick={() => setLang('en')}>
            English
          </button>
          <button className={lang === 'ur' ? 'active accent' : ''} onClick={() => setLang('ur')}>
            اردو
          </button>
        </div>
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
