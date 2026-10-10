import { useCallback, useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import {
  Activity,
  Gamepad2,
  Gauge as GaugeIcon,
  Home as HomeIcon,
  Power,
  RotateCcw,
  Settings as SettingsIcon,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Zap,
} from 'lucide-react'
import { api, type ApplyResult, type RestoreResult, type SystemInfo, type TweakView } from './api'
import { LangContext, errText, strings, type Lang } from './i18n'
import { AppContext, type Page } from './store'
import { RestoreModal, type RestoreChoice } from './RestoreModal'
import { TitleBar } from './components/TitleBar'
import { Home } from './pages/Home'
import { Optimize } from './pages/Optimize'
import { Boost } from './pages/Boost'
import { FpsTest } from './pages/FpsTest'
import { Cleanup } from './pages/Cleanup'
import { Startup } from './pages/Startup'
import { Safety } from './pages/Safety'
import { Settings } from './pages/Settings'

type BoostEvent = { type: 'Started' | 'Ended'; game: string; closed?: number; reopened?: number }

function initialLang(): Lang {
  try {
    return localStorage.getItem('raftaar-lang') === 'ur' ? 'ur' : 'en'
  } catch {
    return 'en'
  }
}

const NAV: { id: Page; icon: typeof HomeIcon }[] = [
  { id: 'home', icon: HomeIcon },
  { id: 'optimize', icon: SlidersHorizontal },
  { id: 'boost', icon: Gamepad2 },
  { id: 'fps', icon: GaugeIcon },
  { id: 'cleanup', icon: Sparkles },
  { id: 'startup', icon: Power },
  { id: 'safety', icon: ShieldCheck },
  { id: 'settings', icon: SettingsIcon },
]

export default function App() {
  const [lang, setLang] = useState<Lang>(initialLang)
  const t = strings[lang]
  const [page, setPage] = useState<Page>('home')
  const [tweaks, setTweaks] = useState<TweakView[]>([])
  const [info, setInfo] = useState<SystemInfo | null>(null)
  const [admin, setAdmin] = useState(true)
  const [boosting, setBoosting] = useState<string | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  const [restore, setRestore] = useState<RestoreResult | null>(null)
  const restoreAnswer = useRef<(c: RestoreChoice) => void>(() => {})
  // Once the user picks "Continue without", don't ask again this session.
  const skipRestore = useRef(false)
  const mainRef = useRef<HTMLElement>(null)

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
    api.scan().then(setInfo, (e) => setToast(errText(e)))
    api.boostGet().then((b) => setBoosting(b.state.game_name), () => {})
  }, [refreshTweaks])

  useEffect(() => {
    if (!toast) return
    const id = setTimeout(() => setToast(null), 6000)
    return () => clearTimeout(id)
  }, [toast])

  useEffect(() => {
    const un = listen<BoostEvent>('boost-event', ({ payload: p }) => {
      setBoosting(p.type === 'Started' ? p.game : null)
      setToast(p.type === 'Started' ? t.boostStarted(p.game, p.closed ?? 0) : t.boostEnded(p.game, p.reopened ?? 0))
    })
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

  const go = useCallback((p: Page) => {
    setPage(p)
    if (mainRef.current) mainRef.current.scrollTop = 0
  }, [])

  const appliedCount = tweaks.filter((tw) => tw.applied).length

  return (
    <LangContext.Provider value={{ lang, t, setLang }}>
      <AppContext.Provider
        value={{ info, tweaks, admin, boosting, refreshTweaks, apply, toast: setToast, go }}
      >
        <div className="shell">
          <TitleBar />
          <aside className="sidebar">
            <div className="brand">
              <div className="logo-mark">
                <Zap size={20} strokeWidth={2.6} />
              </div>
              <div>
                <div className="brand-name">{t.appName}</div>
                <div className="brand-tag">{t.tagline}</div>
              </div>
            </div>
            <nav className="stack" style={{ gap: 2 }} aria-label="Main">
              {NAV.map(({ id, icon: Icon }) => (
                <button
                  key={id}
                  className={`nav-item ${page === id ? 'active' : ''}`}
                  aria-current={page === id ? 'page' : undefined}
                  onClick={() => go(id)}
                >
                  <Icon size={18} />
                  <span>{t.nav[id]}</span>
                  {id === 'optimize' && appliedCount > 0 && <span className="nav-pill">{appliedCount}</span>}
                  {id === 'boost' && boosting && (
                    <span className="nav-pill live">
                      <Activity size={11} />
                    </span>
                  )}
                </button>
              ))}
            </nav>
            <div className="sidebar-foot">
              <div className="admin-chip">
                <span className={`dot ${admin ? '' : 'warn'}`} />
                {admin ? t.adminOk : t.adminNo}
              </div>
              <button className="btn danger" onClick={revertAll} disabled={appliedCount === 0}>
                <RotateCcw size={15} />
                {t.revertAll}
              </button>
            </div>
          </aside>
          <main className="main" ref={mainRef}>
            <div className="page" key={page}>
              {!admin && page !== 'settings' && (
                <div className="banner warn" role="note">
                  {t.notAdmin}
                </div>
              )}
              {page === 'home' && <Home />}
              {page === 'optimize' && <Optimize />}
              {page === 'boost' && <Boost />}
              {page === 'fps' && <FpsTest />}
              {page === 'cleanup' && <Cleanup />}
              {page === 'startup' && <Startup />}
              {page === 'safety' && <Safety />}
              {page === 'settings' && <Settings />}
            </div>
          </main>
        </div>
        {toast && (
          <div className="toast" role="status" onClick={() => setToast(null)}>
            {toast}
          </div>
        )}
        {restore && <RestoreModal result={restore} onChoose={(c) => restoreAnswer.current(c)} />}
      </AppContext.Provider>
    </LangContext.Provider>
  )
}
