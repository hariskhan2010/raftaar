import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { decodeShareCode, encodeShareCode, type FpsRecord, type SystemInfo, type TweakView } from './api'
import App from './App'
import { detectLang } from './i18n'
import { tweakText } from './i18n/tweaks'
import { check } from '@tauri-apps/plugin-updater'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

const tweak = (id: string, over: Partial<TweakView> = {}): TweakView => ({
  id,
  category: 'Fps',
  name_en: `${id} en`,
  name_ur: `${id} ur`,
  desc_en: 'desc',
  desc_ur: 'تفصیل',
  risk: 'Low',
  needs_restart: false,
  recommended_for: ['Low', 'Mid', 'High'],
  preset: 'Lite',
  applied: false,
  applied_at: null,
  ...over,
})

const info: SystemInfo = {
  cpu: 'Test CPU',
  cores: 4,
  threads: 8,
  ram_gb: 8,
  gpu_name: 'Intel UHD',
  vram_gb: 0,
  windows: 'Windows 10',
  startup_apps: 5,
  process_count: 120,
  class: 'Low',
}

const fps = (id: number, label: string, avg: number): FpsRecord => ({
  id,
  game: 'cs2.exe',
  label,
  at: 1_800_000_000 + id,
  result: { avg_fps: avg, low1_fps: avg / 2, low01_fps: avg / 3, frames: 1000, seconds: 30, frame_times: [16, 17, 16] },
})

let tweaks: TweakView[]
let handlers: Record<string, (args?: Record<string, unknown>) => unknown>

beforeEach(() => {
  tweaks = [tweak('game_mode'), tweak('visual_fx', { category: 'Visuals', preset: 'Balanced' }), tweak('hags_on', { preset: 'Max' })]
  handlers = {
    is_admin: () => true,
    system_scan: () => info,
    load_score: () => ({ score: 60, sample: { ram_used_pct: 50, process_count: 120, cpu_busy_pct: 5, startup_apps: 5 } }),
    list_tweaks: () => tweaks,
    preset_ids: (a) => (a!.preset === 'Lite' ? ['game_mode'] : a!.preset === 'Balanced' ? ['game_mode', 'visual_fx'] : ['game_mode', 'visual_fx', 'hags_on']),
    apply_tweaks: (a) => ({ restore: { status: 'Created' }, applied: a!.ids, failed: [] }),
    revert_tweak: () => undefined,
    boost_get: () => ({ config: { enabled: false, games: [], close_apps: [] }, state: { game_pid: null, game_name: null, closed: [] } }),
    running_apps: () => [],
    detect_games: () => [{ name: 'Counter-Strike 2', exe: 'cs2.exe', launcher: 'Steam', install_dir: 'C:\\g' }],
    boost_set: () => undefined,
    monitor_sample: () => ({ cpu_pct: 12, ram_used_gb: 4, ram_total_gb: 8, gpu_pct: null, top: [] }),
    fps_history: () => [],
    cleanup_scan: (a) =>
      ({
        UserTemp: { category: 'UserTemp', files: 3, bytes: 2048, skipped: 0, blocked_by: [] },
        ShaderCache: { category: 'ShaderCache', files: 9, bytes: 4096, skipped: 0, blocked_by: [] },
        BrowserCache: { category: 'BrowserCache', files: 0, bytes: 0, skipped: 0, blocked_by: ['chrome.exe'] },
      })[a!.category as string] ?? { category: a!.category, files: 0, bytes: 0, skipped: 0, blocked_by: [] },
    cleanup_run: () => [],
    startup_list: () => [{ name: 'Discord', command: 'discord.exe', source: 'UserRun', enabled: true }],
    startup_set: () => undefined,
    restore_points: () => [],
  }
  invoke.mockReset()
  invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    const h = handlers[cmd]
    if (!h) throw new Error(`unexpected command ${cmd}`)
    return h(args)
  })
})

const nav = (name: string) => userEvent.click(screen.getByRole('button', { name }))

async function clickOptimize() {
  const btn = await screen.findByRole('button', { name: /Optimize now/ })
  await waitFor(() => expect(btn).toBeEnabled())
  await userEvent.click(btn)
}

describe('Home', () => {
  it('one-click optimize applies the Balanced preset by default', async () => {
    render(<App />)
    await clickOptimize()
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('apply_tweaks', { ids: ['game_mode', 'visual_fx'], skipRestore: false }),
    )
    expect(await screen.findByText('2 tweaks applied.')).toBeInTheDocument()
  })

  it('choosing Lite applies only the Lite tweaks', async () => {
    render(<App />)
    await userEvent.click(await screen.findByRole('radio', { name: /Lite/ }))
    await waitFor(() => expect(screen.getByText('1 tweaks will be applied')).toBeInTheDocument())
    await clickOptimize()
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('apply_tweaks', { ids: ['game_mode'], skipRestore: false }))
  })

  it('asks when the restore point fails, and continue-without retries with skipRestore', async () => {
    let calls = 0
    handlers.apply_tweaks = (a) =>
      calls++ === 0 ? { restore: { status: 'ProtectionOff' }, applied: [], failed: [] } : { restore: null, applied: a!.ids, failed: [] }
    render(<App />)
    await clickOptimize()
    expect(await screen.findByRole('dialog')).toHaveTextContent('System Protection is turned off')
    await userEvent.click(screen.getByRole('button', { name: 'Continue without' }))
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('apply_tweaks', { ids: ['game_mode', 'visual_fx'], skipRestore: true }),
    )
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('cancel on the restore dialog applies nothing', async () => {
    handlers.apply_tweaks = () => ({ restore: { status: 'Failed', detail: 'boom' }, applied: [], failed: [] })
    render(<App />)
    await clickOptimize()
    expect(await screen.findByRole('dialog')).toHaveTextContent('boom')
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(invoke.mock.calls.filter(([c]) => c === 'apply_tweaks')).toHaveLength(1)
    expect(screen.queryByText(/tweaks applied/)).not.toBeInTheDocument()
  })
})

describe('Optimize', () => {
  it('turning an applied tweak off reverts it', async () => {
    tweaks[0] = tweak('game_mode', { applied: true })
    render(<App />)
    await nav('Optimize')
    await userEvent.click(await screen.findByRole('switch', { name: 'game_mode en' }))
    expect(invoke).toHaveBeenCalledWith('revert_tweak', { id: 'game_mode' })
  })

  it('category filter narrows the list', async () => {
    render(<App />)
    await nav('Optimize')
    await userEvent.click(await screen.findByRole('tab', { name: /Visuals/ }))
    expect(screen.getByRole('switch', { name: 'visual_fx en' })).toBeInTheDocument()
    expect(screen.queryByRole('switch', { name: 'game_mode en' })).not.toBeInTheDocument()
  })
})

describe('share codes', () => {
  it('round-trip and reject unknown ids', () => {
    const code = encodeShareCode(['game_mode', 'hags_on'])
    expect(code).toMatch(/^RFT1-/)
    expect(decodeShareCode(code, ['game_mode', 'hags_on', 'x'])).toEqual(['game_mode', 'hags_on'])
    expect(decodeShareCode(code, ['game_mode'])).toBeNull()
    expect(decodeShareCode('hello', ['game_mode'])).toBeNull()
  })
})

describe('Cleanup', () => {
  it('warns it cannot be undone, leaves shader caches unticked, and cleans only what is ticked', async () => {
    render(<App />)
    await nav('Cleanup')
    expect(screen.getByRole('alert')).toHaveTextContent('cannot be restored')
    // Opening the page runs a dry-run scan of every category by itself.
    expect(await screen.findByText('Close chrome.exe to clean this fully.')).toBeInTheDocument()
    expect(invoke).not.toHaveBeenCalledWith('cleanup_run', expect.anything())
    expect(screen.getByRole('checkbox', { name: /Shader caches/ })).not.toBeChecked()
    await userEvent.click(screen.getByRole('button', { name: 'Clean 2.0 KB' }))
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('cleanup_run', {
        categories: ['UserTemp', 'WindowsTemp', 'CrashDumps', 'ErrorReports', 'BrowserCache'],
      }),
    )
  })
})

describe('Game Boost', () => {
  it('detected games can be boosted in one click, which also turns Boost on', async () => {
    render(<App />)
    await nav('Game Boost')
    await userEvent.click(await screen.findByRole('button', { name: 'Boost this' }))
    expect(invoke).toHaveBeenCalledWith('boost_set', { config: { enabled: true, games: ['cs2.exe'], close_apps: [] } })
  })
})

describe('Startup', () => {
  it('toggling an item disables it', async () => {
    render(<App />)
    await nav('Startup')
    await userEvent.click(await screen.findByRole('switch', { name: 'Discord' }))
    expect(invoke).toHaveBeenCalledWith('startup_set', { source: 'UserRun', name: 'Discord', enabled: false })
  })
})

describe('FPS Test', () => {
  it('compares two picked results with percentage change', async () => {
    handlers.fps_history = () => [fps(2, 'after', 120), fps(1, 'before', 100)]
    render(<App />)
    await nav('FPS Test')
    const boxes = await screen.findAllByRole('checkbox', { name: 'Compare' })
    await userEvent.click(boxes[0])
    await userEvent.click(boxes[1])
    const avg = screen.getByText('Average FPS').closest('.fps-cell') as HTMLElement
    expect(within(avg).getByText('+20.0%')).toBeInTheDocument()
  })
})

describe('Settings', () => {
  it('urdu and arabic switch the whole app to right-to-left', async () => {
    render(<App />)
    await nav('Settings')
    await userEvent.click(screen.getByRole('radio', { name: 'اردو' }))
    expect(document.documentElement.dir).toBe('rtl')
    expect(document.documentElement.lang).toBe('ur')
    expect(screen.getByRole('button', { name: 'ہوم' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('radio', { name: 'العربية' }))
    expect(document.documentElement.dir).toBe('rtl')
    expect(screen.getByRole('button', { name: 'الرئيسية' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('radio', { name: 'English' }))
    expect(document.documentElement.dir).toBe('ltr')
  })

  it('spanish translates tweak names that come from the engine', async () => {
    tweaks = [tweak('game_mode')]
    render(<App />)
    await nav('Settings')
    await userEvent.click(screen.getByRole('radio', { name: 'Español' }))
    await userEvent.click(screen.getByRole('button', { name: 'Optimizar' }))
    expect(await screen.findByRole('switch', { name: 'Modo Juego de Windows activado' })).toBeInTheDocument()
  })

  it('checking for updates with no newer release says up to date', async () => {
    render(<App />)
    await nav('Settings')
    await userEvent.click(screen.getByRole('button', { name: 'Check for updates' }))
    expect(await screen.findByText('Raftaar is up to date.')).toBeInTheDocument()
  })

  it('a failed update check is reported as a failure, never as up to date', async () => {
    vi.mocked(check).mockRejectedValueOnce(new Error('404'))
    render(<App />)
    await nav('Settings')
    vi.mocked(check).mockRejectedValueOnce(new Error('404'))
    await userEvent.click(screen.getByRole('button', { name: 'Check for updates' }))
    expect(await screen.findByText(/Couldn't check for updates/)).toBeInTheDocument()
    expect(screen.queryByText('Raftaar is up to date.')).not.toBeInTheDocument()
  })
})

describe('languages', () => {
  it('first run follows the Windows language', () => {
    expect(detectLang(['tr-TR', 'en-US'])).toBe('tr')
    expect(detectLang(['pt-BR'])).toBe('pt')
    expect(detectLang(['fr-FR', 'de-DE'])).toBe('en')
  })

  it('every translated language covers the same tweaks', () => {
    const ids = Object.keys(tweakText.es!).sort()
    expect(ids).toHaveLength(26)
    for (const [lang, map] of Object.entries(tweakText)) {
      expect(Object.keys(map!).sort(), lang).toEqual(ids)
      for (const [id, [name, desc]] of Object.entries(map!)) {
        expect(name && desc, `${lang}/${id}`).toBeTruthy()
      }
    }
  })
})
