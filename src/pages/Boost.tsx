import { useEffect, useState } from 'react'
import { Check, Plus, RefreshCw, X, Zap } from 'lucide-react'
import { api, type BoostConfig, type BoostState, type Game } from '../api'
import { errText, useT } from '../i18n'
import { failText, useApp } from '../store'
import { Banner, PageHead, PremiumNote, Toggle } from '../components/ui'

function normalizeExe(raw: string): string {
  let name = raw.trim().toLowerCase().split(/[\\/]/).pop() ?? ''
  if (name && !name.endsWith('.exe')) name += '.exe'
  return name
}

export function Boost() {
  const { t } = useT()
  const { toast, premium, go } = useApp()
  const [config, setConfig] = useState<BoostConfig | null>(null)
  const [state, setState] = useState<BoostState | null>(null)
  const [games, setGames] = useState<Game[] | null>(null)
  const [running, setRunning] = useState<string[]>([])

  const load = () =>
    api.boostGet().then(
      (v) => {
        setConfig(v.config)
        setState(v.state)
      },
      (e) => toast(errText(e)),
    )
  const loadRunning = () => api.runningApps().then(setRunning, (e) => toast(errText(e)))

  useEffect(() => {
    load()
    loadRunning()
    api.detectGames().then(setGames, () => setGames([]))
    const id = setInterval(load, 3000)
    return () => clearInterval(id)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  if (!config) return null

  const save = async (next: BoostConfig) => {
    setConfig(next)
    try {
      await api.boostSet(next)
    } catch (e) {
      toast(failText(errText(e), t))
      load()
    }
  }

  const toggleBoost = () => {
    if (!config.enabled && !premium) {
      go('premium')
      return
    }
    save({ ...config, enabled: !config.enabled })
  }

  const toggleGame = (exe: string) => {
    const has = config.games.includes(exe)
    const nextGames = has ? config.games.filter((g) => g !== exe) : [...config.games, exe]
    // Adding a first game is a clear sign the user wants Boost on (if they can have it).
    const turnOn = premium && !has && config.games.length === 0
    save({ ...config, games: nextGames, enabled: config.enabled || turnOn })
  }

  const manualGames = config.games.filter((g) => !games?.some((x) => x.exe === g))

  return (
    <>
      <PageHead title={t.boostTitle} sub={t.boostSub}>
        <span className="muted">{config.enabled ? t.boostEnabled : t.boostDisabled}</span>
        <Toggle on={config.enabled} label={t.nav.boost} onChange={toggleBoost} />
      </PageHead>
      <PremiumNote />

      {state?.game_name && (
        <Banner kind="ok">
          <span className="row">
            <Zap size={14} />
            {t.boostActive(state.game_name)}
          </span>
        </Banner>
      )}

      <section className="card">
        <div className="card-head">
          <h2>{t.detected}</h2>
          <span className="sub">{games ? games.length : ''}</span>
        </div>
        {games === null ? (
          <p className="muted">
            <RefreshCw size={14} className="spin" style={{ verticalAlign: -2 }} /> {t.detecting}
          </p>
        ) : games.length === 0 ? (
          <p className="muted">{t.noGames}</p>
        ) : (
          <div className="games-grid">
            {games.map((g) => {
              const on = config.games.includes(g.exe)
              return (
                <div key={g.exe} className={`game ${on ? 'on' : ''}`}>
                  <div className="row between">
                    <div className="game-icon">{g.name.slice(0, 1).toUpperCase()}</div>
                    <span className="badge">{g.launcher}</span>
                  </div>
                  <strong className="ellipsis" title={g.name}>
                    {g.name}
                  </strong>
                  <span className="muted small ltr ellipsis" title={g.exe}>
                    {g.exe}
                  </span>
                  <button className={`btn sm ${on ? 'primary' : ''}`} onClick={() => toggleGame(g.exe)}>
                    {on ? <Check size={14} /> : <Plus size={14} />}
                    {on ? t.boosted : t.addToBoost}
                  </button>
                </div>
              )
            })}
          </div>
        )}
      </section>

      <div className="grid cols-2">
        <ExeList
          title={t.games}
          hint={null}
          placeholder={t.exePlaceholder}
          items={manualGames}
          running={running}
          onRefresh={loadRunning}
          onAdd={(exe) => !config.games.includes(exe) && save({ ...config, games: [...config.games, exe] })}
          onRemove={(exe) => save({ ...config, games: config.games.filter((g) => g !== exe) })}
        />
        <ExeList
          title={t.closeApps}
          hint={t.closeHint}
          placeholder={t.closePlaceholder}
          items={config.close_apps}
          running={running}
          onRefresh={loadRunning}
          onAdd={(exe) => !config.close_apps.includes(exe) && save({ ...config, close_apps: [...config.close_apps, exe] })}
          onRemove={(exe) => save({ ...config, close_apps: config.close_apps.filter((g) => g !== exe) })}
        />
      </div>
    </>
  )
}

interface ExeListProps {
  title: string
  hint: string | null
  placeholder: string
  items: string[]
  running: string[]
  onRefresh: () => void
  onAdd: (exe: string) => void
  onRemove: (exe: string) => void
}

function ExeList({ title, hint, placeholder, items, running, onRefresh, onAdd, onRemove }: ExeListProps) {
  const { t } = useT()
  const [text, setText] = useState('')

  const add = (raw: string) => {
    const name = normalizeExe(raw)
    if (name) onAdd(name)
    setText('')
  }

  return (
    <section className="card stack">
      <div>
        <h2 style={{ fontSize: 15 }}>{title}</h2>
        {hint && <p className="muted small">{hint}</p>}
      </div>
      {items.length === 0 ? (
        <p className="muted small">{t.none}</p>
      ) : (
        <div className="chips">
          {items.map((it) => (
            <span key={it} className="tag ltr">
              {it}
              <button aria-label={`${t.remove} ${it}`} onClick={() => onRemove(it)}>
                <X size={13} />
              </button>
            </span>
          ))}
        </div>
      )}
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault()
          add(text)
        }}
      >
        <input className="input ltr" style={{ flex: 1 }} value={text} placeholder={placeholder} onChange={(e) => setText(e.target.value)} />
        <button className="btn" type="submit" disabled={!text.trim()}>
          <Plus size={15} />
          {t.add}
        </button>
      </form>
      <div className="row">
        <select className="input ltr" style={{ flex: 1 }} value="" onChange={(e) => add(e.target.value)} aria-label={t.pickRunning}>
          <option value="">{t.pickRunning}</option>
          {running
            .filter((r) => !items.includes(r))
            .map((r) => (
              <option key={r} value={r}>
                {r}
              </option>
            ))}
        </select>
        <button className="btn ghost icon-btn" type="button" onClick={onRefresh} aria-label={t.refresh}>
          <RefreshCw size={15} />
        </button>
      </div>
    </section>
  )
}
