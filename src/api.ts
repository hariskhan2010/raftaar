// Typed wrappers around the Tauri commands in src-tauri/src/commands.rs.
import { invoke } from '@tauri-apps/api/core'

export type PcClass = 'Low' | 'Mid' | 'High'
export type Risk = 'Low' | 'Medium'

export interface SystemInfo {
  cpu: string
  cores: number
  threads: number
  ram_gb: number
  gpu_name: string
  vram_gb: number
  windows: string
  startup_apps: number
  process_count: number
  class: PcClass
}

export interface LoadSample {
  ram_used_pct: number
  process_count: number
  cpu_busy_pct: number
  startup_apps: number
}

export interface Score {
  sample: LoadSample
  score: number
}

export interface TweakView {
  id: string
  name_en: string
  name_ur: string
  desc_en: string
  desc_ur: string
  risk: Risk
  needs_restart: boolean
  recommended_for: PcClass[]
  applied: boolean
}

export type RestoreResult =
  | { status: 'Created' }
  | { status: 'ProtectionOff' }
  | { status: 'Failed'; detail: string }

export interface ApplyResult {
  restore: RestoreResult | null
  applied: string[]
  failed: [string, string][]
}

export interface BoostConfig {
  enabled: boolean
  games: string[]
  close_apps: string[]
}

export interface BoostState {
  game_pid: number | null
  game_name: string | null
  closed: string[]
}

export interface CleanupReport {
  files: number
  bytes: number
  skipped: number
}

export const api = {
  isAdmin: () => invoke<boolean>('is_admin'),
  scan: () => invoke<SystemInfo>('system_scan'),
  loadScore: () => invoke<Score>('load_score'),
  listTweaks: () => invoke<TweakView[]>('list_tweaks'),
  applyTweaks: (ids: string[], skipRestore: boolean) => invoke<ApplyResult>('apply_tweaks', { ids, skipRestore }),
  enableProtection: () => invoke<void>('enable_protection'),
  revertTweak: (id: string) => invoke<void>('revert_tweak', { id }),
  revertAll: () => invoke<void>('revert_all'),
  boostGet: () => invoke<{ config: BoostConfig; state: BoostState }>('boost_get'),
  boostSet: (config: BoostConfig) => invoke<void>('boost_set', { config }),
  runningApps: () => invoke<string[]>('running_apps'),
  cleanup: (dryRun: boolean) => invoke<CleanupReport>('cleanup', { dryRun }),
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  const units = ['KB', 'MB', 'GB']
  let v = n / 1024
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(1)} ${units[i]}`
}
