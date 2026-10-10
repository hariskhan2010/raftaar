import { useEffect, useState } from 'react'
import { api, type BoostConfig, type BoostState } from '../api'
import { errText, useT } from '../i18n'

export function Boost({ onError }: { onError: (msg: string) => void }) {
  const { t } = useT()
  const [config, setConfig] = useState<BoostConfig | null>(null)
  const [state, setState] = useState<BoostState | null>(null)
  const [running, setRunning] = useState<string[]>([])

  const load = () =>
    api.boostGet().then(
      (v) => {
        setConfig(v.config)
        setState(v.state)
      },
      (e) => onError(errText(e)),
    )
  const loadRunning = () => api.runningApps().then(setRunning, (e) => onError(errText(e)))

  useEffect(() => {
    load()
    loadRunning()
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
      onError(errText(e))
      load()
    }
  }

  return (
    <div className="stack">
      <p className="muted">{t.boostIntro}</p>
      <section className="card row">
        <strong>{config.enabled ? t.boostEnabled : t.boostDisabled}</strong>
        <button
          role="switch"
          aria-checked={config.enabled}
          aria-label={t.navBoost}
          className={`switch ${config.enabled ? 'on' : ''}`}
          onClick={() => save({ ...config, enabled: !config.enabled })}
        >
          <span className="knob" />
          <span className="switch-text">{config.enabled ? t.on : t.off}</span>
        </button>
      </section>
      {state?.game_name && <div className="banner ok">{t.boostActive(state.game_name)}</div>}
      <ExeList
        title={t.games}
        placeholder={t.exePlaceholder}
        items={config.games}
        running={running}
        onRefresh={loadRunning}
        onChange={(games) => save({ ...config, games })}
      />
      <ExeList
        title={t.closeApps}
        placeholder={t.closePlaceholder}
        items={config.close_apps}
        running={running}
        onRefresh={loadRunning}
        onChange={(close_apps) => save({ ...config, close_apps })}
      />
    </div>
  )
}

interface ExeListProps {
  title: string
  placeholder: string
  items: string[]
  running: string[]
  onRefresh: () => void
  onChange: (items: string[]) => void
}

function ExeList({ title, placeholder, items, running, onRefresh, onChange }: ExeListProps) {
  const { t } = useT()
  const [text, setText] = useState('')

  const add = (raw: string) => {
    let name = raw.trim().toLowerCase().split(/[\\/]/).pop() ?? ''
    if (!name) return
    if (!name.endsWith('.exe')) name += '.exe'
    if (!items.includes(name)) onChange([...items, name])
    setText('')
  }

  return (
    <section className="card">
      <h2>{title}</h2>
      {items.length === 0 ? (
        <p className="muted">{t.none}</p>
      ) : (
        <ul className="chips">
          {items.map((it) => (
            <li key={it} className="chip" dir="ltr">
              {it}
              <button aria-label={`${t.remove} ${it}`} onClick={() => onChange(items.filter((x) => x !== it))}>
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
      <form
        className="add-row"
        onSubmit={(e) => {
          e.preventDefault()
          add(text)
        }}
      >
        <input dir="ltr" value={text} placeholder={placeholder} onChange={(e) => setText(e.target.value)} />
        <button className="btn" type="submit">
          {t.addExe}
        </button>
      </form>
      <div className="add-row">
        <select dir="ltr" value="" onChange={(e) => add(e.target.value)} aria-label={t.pickRunning}>
          <option value="">{t.pickRunning}</option>
          {running
            .filter((r) => !items.includes(r))
            .map((r) => (
              <option key={r} value={r}>
                {r}
              </option>
            ))}
        </select>
        <button className="btn ghost" type="button" onClick={onRefresh}>
          {t.refresh}
        </button>
      </div>
    </section>
  )
}
