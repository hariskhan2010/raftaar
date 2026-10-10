# Raftaar v0.2: world-class feature set + new UI

Goal (user, 2026-10-10): every useful feature of Hone and the other top optimizers, aimed at the
whole world, not only Pakistan, with a much better UI.
Tier: T3 (changes Windows). Builds on Phase 1 (.overpower/plan.md).

## What we copy, and what we deliberately don't

Researched: Hone 1.4 (hone.gg/blog/hone-1-4-update-boost-ups-and-merged-optimizations),
Hone Premium page, Razer Cortex, NVIDIA App, Process Lasso.

| Competitor feature | Raftaar v0.2 |
|---|---|
| Hone grouped optimizations (FPS/input lag, power, ads & tracking, background activity, storage, presentation latency, Explorer) | ~25 registry tweaks in categories, every one snapshotted + revertible |
| Hone presets + share codes | Lite / Balanced / Max presets + import/export share code |
| Hone Boost-Ups (junk, cache, shader cache cleaners) | Cleanup with categories: temp, Windows temp, shader caches, crash dumps, error reports, browser caches |
| Hone Game Mode / Cortex auto-boost | Game Boost + auto-detect installed Steam/Epic games + 1 ms timer while gaming |
| NVIDIA App / Cortex overlay metrics | Live CPU / RAM / GPU monitor + top processes |
| Task Manager / CCleaner startup manager | Startup manager (StartupApproved, same as Task Manager, reversible) |
| (nobody does this honestly) | **FPS Test**: PresentMon measures real avg FPS and 1% lows before vs after |
| Hone restore point | Safety center: restore points list, create one, open System Restore, change history, Revert All |

Not doing, on purpose (safety is the product):
- Defender, Windows Update, UAC, firewall, core services, "disable mitigations" (Spectre/Meltdown), BIOS, Win11 check bypass.
- NVIDIA/AMD driver profile edits (need vendor APIs; wrong values break drivers). Later, via NVAPI.
- Per-interface Nagle/TCP tweaks (disputed gains). Only NetworkThrottlingIndex.
- "RAM cleaner" (EmptyWorkingSet): placebo, makes games page-fault later.
- Payments / accounts / cafe mode: Phase 2.

## Tasks
1. Engine: `category` on Tweak, `RegValue::Binary`, `Registry::values()` (enumerate), catalog to ~25 tweaks, presets. Test: whole-catalog apply+revert_all leaves mock registry identical.
2. Engine: startup manager (Run keys + StartupApproved). Test: disable then enable restores bytes.
3. Engine: cleanup categories with per-category scan/clean, skip browser caches while browser runs. Test: tempdir fixtures.
4. Engine: game detection (Epic manifests, Steam libraryfolders.vdf + appmanifests). Test: fixtures.
5. Engine: live monitor (CPU, RAM, GPU via PDH, top processes).
6. Engine: FPS test via bundled PresentMon (sidecar), CSV parse → avg, 1% low, 0.1% low. Test: CSV fixture.
7. Engine: snapshot `applied_at`, restore point list.
8. Tauri: commands for 1-7, timer resolution in boost watcher, sidecar config.
9. UI: redesign, sidebar + custom title bar; pages Home, Optimize, Game Boost, FPS Test, Cleanup, Startup, Safety, Settings.
10. Verify: cargo test, npm test, tsc, eslint, release build, screenshots of every page, commit.

## Log
- 2026-10-10 plan written
- 2026-10-10 tasks 1-7 done: engine 61 tests pass (catalog 26 tweaks + presets, startup, cleanup categories, games, monitor with PDH GPU, fps/PresentMon 2.5.1 sidecar, restore point list, applied_at)
- 2026-10-10 task 8 done: Tauri commands compile; timer resolution held while a boosted game runs
- 2026-10-10 task 9 done: new UI (custom title bar, sidebar, 8 pages), 12 UI tests, tsc + eslint clean; screenshots of every page with real data on the dev PC
- Found by screenshots and fixed: window taller than 768 px screens, FPS Test pre-selected a random background app, Cleanup didn't auto-scan, slow scan on a 23k-file Temp (now per-category in parallel)
- 2026-10-11 v0.3: auto-update (tauri-plugin-updater 2.12.0, minisign key in %USERPROFILE%\.tauri, GitHub Releases endpoint, scripts/release.ps1) and Hindi, Arabic, Spanish, Portuguese, Turkish (UI + all 26 tweaks; tests enforce completeness). 16 UI tests, 62 engine tests.
- Not verifiable here: FPS Test end-to-end (needs admin + a running game), Steam/Epic detection on a PC with those launchers (this PC has none), Sandbox E2E
- 2026-10-11 published: public repo github.com/hariskhan2010/raftaar (author emails rewritten to the GitHub noreply address before the first push), releases v0.3.0 and v0.4.0, website at hariskhan2010.github.io/raftaar (HTTP 200)
- 2026-10-11 v0.4: Premium licensing (Ed25519 offline keys + Lemon Squeezy online keys, 7-day trial), enforced in the backend; 74 engine + 24 UI tests. Verified: CLI-issued key valid, tampered key invalid, keygen refuses overwrite; the v0.3.0 build detected v0.4.0 from GitHub and offered the update.
- Owner to do: fill src/config.ts (JazzCash/Easypaisa/WhatsApp), Lemon Squeezy store id + checkout link, back up both keys in %USERPROFILE%\.tauri, code signing (SignPath.io for OSS), FPS Test on a gaming PC as admin, Sandbox E2E.
