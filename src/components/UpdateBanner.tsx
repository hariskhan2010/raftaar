import { useEffect, useState } from 'react'
import type { Update } from '@tauri-apps/plugin-updater'
import { Download } from 'lucide-react'
import { useT } from '../i18n'
import { checkForUpdate, installUpdate, type UpdateCheck } from '../update'

// One check per app run, shared by every page so switching pages doesn't re-check.
// A failed startup check stays silent; Settings shows the error when the user asks.
let checked: Promise<UpdateCheck> | null = null

export function UpdateBanner() {
  const { t } = useT()
  const [update, setUpdate] = useState<Update | null>(null)
  const [progress, setProgress] = useState<number | null>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    checked ??= checkForUpdate()
    checked.then((r) => setUpdate(r.kind === 'update' ? r.update : null))
  }, [])

  if (!update) return null

  const install = async () => {
    setFailed(false)
    setProgress(0)
    try {
      await installUpdate(update, setProgress)
    } catch {
      setFailed(true)
      setProgress(null)
    }
  }

  return (
    <div className="banner info" role="status">
      <Download size={16} />
      <div className="row between" style={{ flex: 1 }}>
        <span>
          {failed ? t.updateFailed : progress !== null ? t.updating(progress) : t.updateAvailable(update.version)}
        </span>
        {progress === null && (
          <button className="btn sm primary" onClick={install}>
            {t.updateInstall}
          </button>
        )}
      </div>
    </div>
  )
}
