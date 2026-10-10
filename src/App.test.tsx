import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { SystemInfo, TweakView } from './api'
import App from './App'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

const tweak = (id: string, recommended_for: TweakView['recommended_for'], applied = false): TweakView => ({
  id,
  name_en: `${id} en`,
  name_ur: `${id} ur`,
  desc_en: 'desc',
  desc_ur: 'تفصیل',
  risk: 'Low',
  needs_restart: false,
  recommended_for,
  applied,
})

const info: SystemInfo = {
  cpu: 'Test CPU',
  cores: 4,
  threads: 8,
  ram_gb: 8,
  gpu_name: 'Intel UHD',
  vram_gb: 0,
  windows: 'Windows 10 (19045)',
  startup_apps: 5,
  process_count: 120,
  class: 'Low',
}

let tweaks: TweakView[]
let handlers: Record<string, (args?: Record<string, unknown>) => unknown>

beforeEach(() => {
  tweaks = [tweak('game_mode', ['Low', 'Mid', 'High']), tweak('visual_fx', ['Low']), tweak('hw_sched', ['High'])]
  handlers = {
    is_admin: () => true,
    system_scan: () => info,
    load_score: () => ({ score: 60, sample: { ram_used_pct: 50, process_count: 120, cpu_busy_pct: 5, startup_apps: 5 } }),
    list_tweaks: () => tweaks,
    apply_tweaks: (a) => ({ restore: { status: 'Created' }, applied: a!.ids, failed: [] }),
    cleanup: () => ({ files: 3, bytes: 2048, skipped: 0 }),
    boost_get: () => ({ config: { enabled: false, games: [], close_apps: [] }, state: { game_pid: null, game_name: null, closed: [] } }),
    running_apps: () => [],
  }
  invoke.mockReset()
  invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
    const h = handlers[cmd]
    if (!h) throw new Error(`unexpected command ${cmd}`)
    return h(args)
  })
})

describe('App', () => {
  it('optimize applies only the tweaks recommended for this PC class', async () => {
    render(<App />)
    const btn = await screen.findByRole('button', { name: 'Optimize' })
    await waitFor(() => expect(btn).toBeEnabled())
    await userEvent.click(btn)
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('apply_tweaks', { ids: ['game_mode', 'visual_fx'], skipRestore: false }),
    )
    expect(await screen.findByText('2 tweaks applied.')).toBeInTheDocument()
  })

  it('urdu toggle sets dir=rtl and shows urdu text', async () => {
    render(<App />)
    await userEvent.click(screen.getByRole('button', { name: 'اردو' }))
    expect(document.documentElement.dir).toBe('rtl')
    expect(screen.getByRole('tab', { name: 'ڈیش بورڈ' })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'English' }))
    expect(document.documentElement.dir).toBe('ltr')
  })

  it('cleanup screen warns that it cannot be undone and dry-runs first', async () => {
    render(<App />)
    await userEvent.click(screen.getByRole('tab', { name: 'Cleanup' }))
    expect(screen.getByRole('alert')).toHaveTextContent('cannot be undone')
    await userEvent.click(screen.getByRole('button', { name: 'Check size' }))
    expect(invoke).toHaveBeenCalledWith('cleanup', { dryRun: true })
    expect(await screen.findByText('2.0 KB can be freed (3 files).')).toBeInTheDocument()
    expect(invoke).not.toHaveBeenCalledWith('cleanup', { dryRun: false })
  })

  it('asks the user when the restore point fails, and continue-without retries with skipRestore', async () => {
    let calls = 0
    handlers.apply_tweaks = (a) =>
      calls++ === 0
        ? { restore: { status: 'ProtectionOff' }, applied: [], failed: [] }
        : { restore: null, applied: a!.ids, failed: [] }
    render(<App />)
    const btn = await screen.findByRole('button', { name: 'Optimize' })
    await waitFor(() => expect(btn).toBeEnabled())
    await userEvent.click(btn)
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
    const btn = await screen.findByRole('button', { name: 'Optimize' })
    await waitFor(() => expect(btn).toBeEnabled())
    await userEvent.click(btn)
    expect(await screen.findByRole('dialog')).toHaveTextContent('boom')
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(invoke.mock.calls.filter(([c]) => c === 'apply_tweaks')).toHaveLength(1)
    expect(screen.queryByText(/tweaks applied/)).not.toBeInTheDocument()
  })

  it('turning an applied tweak off reverts it', async () => {
    tweaks[0] = tweak('game_mode', ['Low'], true)
    handlers.revert_tweak = () => undefined
    render(<App />)
    await userEvent.click(screen.getByRole('tab', { name: 'Tweaks' }))
    await userEvent.click(await screen.findByRole('switch', { name: 'game_mode en' }))
    expect(invoke).toHaveBeenCalledWith('revert_tweak', { id: 'game_mode' })
  })
})
