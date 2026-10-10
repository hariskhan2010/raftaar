import { createContext, useContext } from 'react'
import { NEEDS_PREMIUM, type ApplyResult, type LicenseView, type SystemInfo, type TweakView } from './api'
import type { Strings } from './i18n'

export type Page = 'home' | 'optimize' | 'boost' | 'fps' | 'cleanup' | 'startup' | 'safety' | 'premium' | 'settings'

export interface AppCtx {
  info: SystemInfo | null
  tweaks: TweakView[]
  admin: boolean
  boosting: string | null
  license: LicenseView | null
  setLicense: (l: LicenseView) => void
  /** Premium or trial. */
  premium: boolean
  refreshTweaks: () => Promise<void>
  /** Applies tweaks; makes a restore point first and asks the user if that fails. */
  apply: (ids: string[]) => Promise<ApplyResult | null>
  toast: (msg: string) => void
  go: (p: Page) => void
}

export const AppContext = createContext<AppCtx>(null as unknown as AppCtx)
export const useApp = () => useContext(AppContext)

/** Backend error text for the user; the Premium marker becomes a readable sentence. */
export function failText(msg: string, t: Strings): string {
  return msg === NEEDS_PREMIUM ? t.premiumNeeded : msg
}
