import { createContext, useContext } from 'react'
import type { ApplyResult, SystemInfo, TweakView } from './api'

export type Page = 'home' | 'optimize' | 'boost' | 'fps' | 'cleanup' | 'startup' | 'safety' | 'settings'

export interface AppCtx {
  info: SystemInfo | null
  tweaks: TweakView[]
  admin: boolean
  boosting: string | null
  refreshTweaks: () => Promise<void>
  /** Applies tweaks; makes a restore point first and asks the user if that fails. */
  apply: (ids: string[]) => Promise<ApplyResult | null>
  toast: (msg: string) => void
  go: (p: Page) => void
}

export const AppContext = createContext<AppCtx>(null as unknown as AppCtx)
export const useApp = () => useContext(AppContext)
