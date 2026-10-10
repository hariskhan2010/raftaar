import { check, type Update } from '@tauri-apps/plugin-updater'

export type UpdateCheck = { kind: 'update'; update: Update } | { kind: 'none' } | { kind: 'error' }

/** A failed check (offline, server down) is reported as an error, never as "up to date". */
export async function checkForUpdate(): Promise<UpdateCheck> {
  try {
    const update = await check()
    return update ? { kind: 'update', update } : { kind: 'none' }
  } catch {
    return { kind: 'error' }
  }
}

/** Downloads and runs the signed installer. On Windows the app closes and the installer restarts it. */
export async function installUpdate(update: Update, onProgress: (pct: number) => void): Promise<void> {
  let total = 0
  let got = 0
  await update.downloadAndInstall((ev) => {
    if (ev.event === 'Started') total = ev.data.contentLength ?? 0
    if (ev.event === 'Progress') {
      got += ev.data.chunkLength
      if (total > 0) onProgress(Math.min(100, Math.round((got / total) * 100)))
    }
  })
}
