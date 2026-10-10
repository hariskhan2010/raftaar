import { useCallback, useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { api, type ApplyResult, type RestoreResult, type TweakView } from './api'
import { LangContext, errText, strings, type Lang } from './i18n'
import { RestoreModal, type RestoreChoice } from './RestoreModal'
import { Dashboard } from './screens/Dashboard'
import { Tweaks } from './screens/Tweaks'
import { Boost } from './screens/Boost'
import { Cleanup } from './screens/Cleanup'
import './App.css'

type Screen = 'dashboard' | 'tweaks' | 'boost' | 'cleanup'
type BoostEvent = { type: 'Started' | 'Ended'; game: string; closed?: number; reopened?: number }

function initialLang(): Lang {
  try {
    return localStorage.getItem('raftaar-lang') === 'ur' ? 'ur' : 'en'
  } catch {
    return 'en'
  }
}

export default function App() {
  const [lang, setLang] = useState<Lang>(initialLang)
  const t = strings[lang]
  const [screen, setScreen] = useState<Screen>('dashboard')
  const [tweaks, setTweaks] = useState<TweakView[]>([])
  const [admin, setAdmin] = useState(true)
  const [toast, setToast] = useState<string | null>(null)
  const [restore, setRestore] = useState<RestoreResult | null>(null)
  const restoreAnswer = useRef<(c: RestoreChoice) => void>(() => {})
  // Once the user picks "Continue without", don't ask again this session.
  const skipRestore = useRef(false)

  useEffect(() => {
    document.documentElement.lang = lang
    document.documentElement.dir = lang === 'ur' ? 'rtl' : 'ltr'
    try {
      localStorage.setItem('raftaar-lang', lang)
    } catch {
      /* storage unavailable */
    }
  }, [lang])

  const refreshTweaks = useCallback(async () => {
    try {
      setTweaks(await api.listTweaks())
    } catch (e) {
      setToast(errText(e))
    }
  }, [])

  useEffect(() => {
    refreshTweaks()
    api.isAdmin().then(setAdmin, () => {})
  }, [refreshTweaks])

  useEffect(() => {
    if (!toast) return
    const id = setTimeout(() => setToast(null), 6000)
    return () => clearTimeout(id)
  }, [toast])

  useEffect(() => {
    const un = listen<BoostEvent>('boost-event', ({ payload: p }) =>
      setToast(p.type === 'Started' ? t.boostStarted(p.game, p.closed ?? 0) : t.boostEnded(p.game, p.reopened ?? 0)),
    )
    return () => {
      un.then((f) => f(), () => {})
    }
  }, [t])

  const askRestore = (r: RestoreResult) =>
    new Promise<RestoreChoice>((resolve) => {
      restoreAnswer.current = (c) => {
        setRestore(null)
        resolve(c)
      }
      setRestore(r)
    })

  /** Applies tweaks; a restore point is made first, and if that fails the user decides. */
  const apply = useCallback(
    async (ids: string[]): Promise<ApplyResult | null> => {
      try {
        let res = await api.applyTweaks(ids, skipRestore.current)
        while (res.restore && res.restore.status !== 'Created') {
          const choice = await askRestore(res.restore)
          if (choice === 'cancel') return null
          if (choice === 'continue') skipRestore.current = true
          else await api.enableProtection()
          res = await api.applyTweaks(ids, skipRestore.current)
        }
        return res
      } catch (e) {
        setToast(errText(e))
        return null
      } finally {
        await refreshTweaks()
      }
    },
    [refreshTweaks],
  )

  const revertAll = async () => {
    if (!window.confirm(t.revertAllConfirm)) return
    try {
      await api.revertAll()
      setToast(t.reverted)
    } catch (e) {
      setToast(errText(e))
    }
    await refreshTweaks()
  }

  const nav: [Screen, string][] = [
    ['dashboard', t.navDashboard],
    ['tweaks', t.navTweaks],
    ['boost', t.navBoost],
    ['cleanup', t.navCleanup],
  ]

  return (
    <LangContext.Provider value={{ lang, t }}>
      <div className="app">
        <header className="header">
          <div className="brand">
            <span className="logo" aria-hidden>
              ⚡
            </span>
            <div>
              <div className="brand-name">{t.appName}</div>
              <div className="brand-tag">{t.tagline}</div>
            </div>
          </div>
          <div className="header-actions">
            <button className="btn ghost" onClick={() => setLang(lang === 'en' ? 'ur' : 'en')}>
              {t.langToggle}
            </button>
            <button className="btn danger" onClick={revertAll} disabled={!tweaks.some((tw) => tw.applied)}>
              {t.revertAll}
            </button>
          </div>
        </header>
        <nav className="nav" role="tablist">
          {nav.map(([id, label]) => (
            <button
              key={id}
              role="tab"
              aria-selected={screen === id}
              className={screen === id ? 'tab active' : 'tab'}
              onClick={() => setScreen(id)}
            >
              {label}
            </button>
          ))}
        </nav>
        {!admin && <div className="banner warn">{t.notAdmin}</div>}
        <main className="main">
          {screen === 'dashboard' && <Dashboard tweaks={tweaks} apply={apply} />}
          {screen === 'tweaks' && <Tweaks tweaks={tweaks} apply={apply} onChange={refreshTweaks} onError={setToast} />}
          {screen === 'boost' && <Boost onError={setToast} />}
          {screen === 'cleanup' && <Cleanup />}
        </main>
        {toast && (
          <div className="toast" role="status" onClick={() => setToast(null)}>
            {toast}
          </div>
        )}
        {restore && <RestoreModal result={restore} onChoose={(c) => restoreAnswer.current(c)} />}
      </div>
    </LangContext.Provider>
  )
}
