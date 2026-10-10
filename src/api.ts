// Typed wrappers around the Tauri commands in src-tauri/src/commands.rs.
import { invoke } from '@tauri-apps/api/core'

export type PcClass = 'Low' | 'Mid' | 'High'
export type Risk = 'Low' | 'Medium'
export type Category = 'Fps' | 'Power' | 'Background' | 'Privacy' | 'Visuals' | 'Network' | 'Storage'
export type Preset = 'Lite' | 'Balanced' | 'Max'
export type CleanCategory = 'UserTemp' | 'WindowsTemp' | 'ShaderCache' | 'CrashDumps' | 'ErrorReports' | 'BrowserCache'
export type StartupSource = 'UserRun' | 'MachineRun' | 'MachineRun32' | 'UserFolder' | 'MachineFolder'

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
  category: Category
  name_en: string
  name_ur: string
  desc_en: string
  desc_ur: string
  risk: Risk
  needs_restart: boolean
  recommended_for: PcClass[]
  preset: Preset | null
  premium: boolean
  applied: boolean
  applied_at: number | null
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

export interface Game {
  name: string
  exe: string
  launcher: string
  install_dir: string
}

export interface CategoryReport {
  category: CleanCategory | null
  files: number
  bytes: number
  skipped: number
  blocked_by: string[]
}

export interface StartupItem {
  name: string
  command: string
  source: StartupSource
  enabled: boolean
}

export interface ProcUsage {
  name: string
  pid: number
  ram_mb: number
  cpu_pct: number
}

export interface Stats {
  cpu_pct: number
  ram_used_gb: number
  ram_total_gb: number
  gpu_pct: number | null
  top: ProcUsage[]
}

export interface FpsResult {
  avg_fps: number
  low1_fps: number
  low01_fps: number
  frames: number
  seconds: number
  frame_times: number[]
}

export interface FpsRecord {
  id: number
  game: string
  label: string
  at: number
  result: FpsResult
}

export interface RestorePoint {
  SequenceNumber: number
  Description: string
  Created: string
}

export interface License {
  name: string
  plan: 'Monthly' | 'Yearly' | 'Lifetime' | null
  expires: number | null
  source: 'Raftaar' | 'LemonSqueezy'
}

export type LicenseStatus =
  | { kind: 'Trial'; days_left: number }
  | { kind: 'Premium'; license: License }
  | { kind: 'Free'; expired: License | null }

export interface LicenseView {
  status: LicenseStatus
  card_keys: boolean
}

/** Error string the backend returns when a Premium feature is used without Premium. */
export const NEEDS_PREMIUM = 'PREMIUM'

export const api = {
  isAdmin: () => invoke<boolean>('is_admin'),
  scan: () => invoke<SystemInfo>('system_scan'),
  loadScore: () => invoke<Score>('load_score'),
  listTweaks: () => invoke<TweakView[]>('list_tweaks'),
  presetIds: (preset: Preset) => invoke<string[]>('preset_ids', { preset }),
  applyTweaks: (ids: string[], skipRestore: boolean) => invoke<ApplyResult>('apply_tweaks', { ids, skipRestore }),
  enableProtection: () => invoke<void>('enable_protection'),
  revertTweak: (id: string) => invoke<void>('revert_tweak', { id }),
  revertAll: () => invoke<void>('revert_all'),
  boostGet: () => invoke<{ config: BoostConfig; state: BoostState }>('boost_get'),
  boostSet: (config: BoostConfig) => invoke<void>('boost_set', { config }),
  runningApps: () => invoke<string[]>('running_apps'),
  detectGames: () => invoke<Game[]>('detect_games'),
  cleanupScan: (category: CleanCategory) => invoke<CategoryReport>('cleanup_scan', { category }),
  cleanupRun: (categories: CleanCategory[]) => invoke<CategoryReport[]>('cleanup_run', { categories }),
  startupList: () => invoke<StartupItem[]>('startup_list'),
  startupSet: (source: StartupSource, name: string, enabled: boolean) =>
    invoke<void>('startup_set', { source, name, enabled }),
  monitor: () => invoke<Stats>('monitor_sample'),
  fpsRecord: (game: string, seconds: number, label: string) =>
    invoke<FpsRecord>('fps_record', { game, seconds, label }),
  fpsHistory: () => invoke<FpsRecord[]>('fps_history'),
  fpsDelete: (id: number) => invoke<void>('fps_delete', { id }),
  restorePoints: () => invoke<RestorePoint[]>('restore_points'),
  createRestorePoint: () => invoke<RestoreResult>('create_restore_point'),
  openSystemRestore: () => invoke<void>('open_system_restore'),
  licenseStatus: () => invoke<LicenseView>('license_status'),
  licenseActivate: (key: string) => invoke<LicenseView>('license_activate', { key }),
  licenseRemove: () => invoke<LicenseView>('license_remove'),
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

/** Share codes are just the tweak ids, so they can be read and checked by hand. */
export function encodeShareCode(ids: string[]): string {
  return 'RFT1-' + btoa(ids.join(',')).replace(/=+$/, '')
}

export function decodeShareCode(code: string, known: string[]): string[] | null {
  const m = code.trim().match(/^RFT1-([A-Za-z0-9+/]+)$/)
  if (!m) return null
  try {
    const ids = atob(m[1]).split(',').filter(Boolean)
    return ids.every((id) => known.includes(id)) ? ids : null
  } catch {
    return null
  }
}
