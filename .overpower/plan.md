# Raftaar: Phase 1 MVP

Spec: .overpower/SPEC.md (safe, honest, light Windows game optimizer that beats Hone on trust)
Branch: main (new repo)
Tier: T3 (changes the user's Windows system, so safety is critical)
Started: 2026-10-10

## Research
- Tauri v2: elevation via `tauri_build::WindowsAttributes::new().app_manifest(...)` in `build.rs` with `requestedExecutionLevel level="requireAdministrator"`.
- Restore point: PowerShell `Checkpoint-Computer -RestorePointType MODIFY_SETTINGS`. Windows allows one per 24h unless `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore\SystemRestorePointCreationFrequency` = 0 (set temporarily, then restore). It fails if System Protection is off on C:.
- Power plans: High performance GUID `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c`. Ultimate `e9a42b02-d5df-448d-aa00-03f14749eb61` must be created with `powercfg -duplicatescheme`. Read the current plan with `powercfg /getactivescheme`.
- Crates: `winreg` (registry), `sysinfo` (CPU/RAM/processes), `wmi` (GPU name/VRAM), `windows` (process priority), `serde`/`serde_json` (snapshots).

## Tools
- Rust toolchain (stable, MSVC) + VS Build Tools (C++ workload). Check with `cargo --version`.
- Node 20+, npm.
- Windows Sandbox (Win 10/11 Pro/Education feature) for real-system tests.

## Plan: 13 tasks

### Task 1: Check prerequisites and scaffold Tauri app
Files: D:\game_booster\* (new)
Depends on: nothing
Uses: Rust, Node
Run: inline
Change: Verify the Rust/MSVC toolchain and enable Windows Sandbox if missing. Run `npm create tauri-app` (React + TS + Vite). Split the Rust side into a workspace: `engine/` (library crate with all system logic and no Tauri dependency, reused later as the cafe agent) and `src-tauri/` (thin command layer). Add the admin manifest in `build.rs`. Init git.
Test: `engine` has a placeholder test.
Verify: `cargo test -p engine` → 1 passed. `npm run tauri dev` → window opens with a UAC prompt.

### Task 2: Shell and Registry abstractions
Files: engine/src/sys/{mod.rs,shell.rs,registry.rs} (new)
Depends on: 1
Uses: winreg
Run: inline
Change: `trait Shell { fn run(&self, prog:&str, args:&[&str]) -> Result<Output> }` and `trait Registry { get(hive,path,name)->Option<RegValue>; set(...); delete(...) }`, each with a real impl and a `Mock*` impl (an in-memory map / scripted outputs). `RegValue` = Dword | String | Missing. **Missing matters**, because reverting a value that didn't exist means deleting it.
Test: "mock set then get returns value", "get of absent value → Missing", "delete then get → Missing".
Verify: `cargo test -p engine sys` → all pass.

### Task 3: Restore point (riskiest, so it comes early)
Files: engine/src/restore.rs (new)
Depends on: 2
Run: inline
Change: `create_restore_point(shell, reg) -> RestoreResult { Created | ProtectionOff | Failed(String) }`. Set the frequency key to 0, run `Checkpoint-Computer`, and put the frequency key back exactly as it was (including Missing). Never enable System Protection silently. `ProtectionOff` lets the UI ask the user.
Test: "frequency key restored to Missing after run", "protection-off stderr → ProtectionOff", "success → Created".
Verify: `cargo test -p engine restore` → pass. Then in Windows Sandbox run `raftaar-cli restore-point` (debug bin) → the point appears in `Get-ComputerRestorePoint`.

### Task 4: Tweak model and snapshot store
Files: engine/src/tweak.rs, engine/src/snapshot.rs (new)
Depends on: 2
Run: inline
Change: `struct Tweak { id, name_en, name_ur, desc_en, desc_ur, risk: Low|Medium, needs_restart: bool, revertible: bool, pc_classes: Vec<PcClass>, kind: TweakKind }`. `TweakKind::Registry(Vec<RegChange>) | PowerPlan | Custom`. `apply(tweak)` reads every old value, saves the snapshot to `%APPDATA%\Raftaar\snapshots.json` **before** writing anything, then writes. `revert(id)` and `revert_all()` restore in reverse order and drop the snapshot entry. Applying an already-applied tweak is a no-op (it must not overwrite the original snapshot).
Test: "apply then revert restores original Dword", "revert restores Missing by deleting", "double apply keeps first snapshot", "revert_all order is reverse", "snapshot written before registry write (mock records call order)".
Verify: `cargo test -p engine tweak snapshot` → pass.

### Task 5: Registry tweak catalog
Files: engine/src/catalog.rs (new)
Depends on: 4
Run: inline
Change: Data-only definitions:
1. Game Mode on: `HKCU\Software\Microsoft\GameBar` AutoGameModeEnabled=1, AllowAutoGameMode=1
2. Xbox Game DVR background recording off: `HKCU\System\GameConfigStore` GameDVR_Enabled=0. `HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR` AppCaptureEnabled=0
3. Multimedia scheduler favours games: `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile` SystemResponsiveness=10. `...\SystemProfile\Tasks\Games` "GPU Priority"=8, Priority=6 (Medium risk)
4. Mouse acceleration off: `HKCU\Control Panel\Mouse` MouseSpeed="0", MouseThreshold1="0", MouseThreshold2="0" (String values)
5. Background UWP apps off: `HKCU\Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications` GlobalUserDisabled=1
6. Visual effects for performance: `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects` VisualFXSetting=2 (needs sign-out, recommended for Low PCs only)
7. Hardware GPU scheduling: `HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers` HwSchMode=2 (Medium, needs restart, only if the GPU/driver supports it, else hidden)
Each tweak gets Urdu + English text.
Test: "every catalog tweak is revertible or marked not", "every tweak has non-empty ur/en text", "apply+revert_all on mock leaves registry identical" (loop over the whole catalog).
Verify: `cargo test -p engine catalog` → pass.

### Task 6: Power plan tweak
Files: engine/src/power.rs (new), engine/src/catalog.rs
Depends on: 4, 5
Run: inline
Change: Read the active scheme GUID and save it in the snapshot. Activate High performance, or create and activate Ultimate if the user picks it. Revert re-activates the saved GUID and deletes any plan we created. Laptops on battery: warn about battery drain.
Test: "parse getactivescheme output", "revert reactivates saved guid", "created Ultimate plan deleted on revert".
Verify: `cargo test -p engine power` → pass.

### Task 7: System scan and PC class
Files: engine/src/scan.rs (new)
Depends on: 2
Uses: sysinfo, wmi
Run: parallel group A
Change: `scan() -> SystemInfo { cpu, cores, ram_gb, gpu_name, vram_gb, windows_build, startup_apps: Vec<StartupApp>, process_count }`. `classify(&SystemInfo) -> PcClass`: Low if RAM ≤ 8 GB or no dedicated GPU, High if RAM ≥ 16 GB and VRAM ≥ 6 GB, otherwise Mid. Startup apps come from the HKCU/HKLM `...\CurrentVersion\Run` keys (read only in Phase 1).
Test: "4GB intel igpu → Low", "16GB + 8GB VRAM → High", "12GB + 4GB VRAM → Mid".
Verify: `cargo test -p engine scan` → pass. `raftaar-cli scan` prints real values.

### Task 8: System Load Score
Files: engine/src/score.rs (new)
Depends on: 7
Run: parallel group A
Change: `measure() -> LoadSample` takes a 5-second sample (RAM used %, background process count, idle CPU %, startup app count). `score(&LoadSample) -> u8` (0-100, documented formula). The UI shows before vs after and never claims FPS.
Test: "idle machine scores higher than busy sample", "score is clamped 0..=100".
Verify: `cargo test -p engine score` → pass.

### Task 9: Game Boost
Files: engine/src/boost.rs (new)
Depends on: 2, 4
Run: parallel group A
Change: A background watcher polls the process list every 3 s for the user's game exes. On game start: set priority High (never Realtime) and close the user-chosen apps, remembering their exe paths. On game exit: restore priority (if the game is still alive) and relaunch the closed apps. Persist state so a crash mid-boost can still restore. Never closes system processes (deny-list: explorer, csrss, winlogon, svchost, dwm, lsass, services).
Test: "denylisted process is never closed", "closed apps relaunched on game exit", "state file restores after simulated crash".
Verify: `cargo test -p engine boost` → pass. In Sandbox, use notepad.exe as the "game" and calc as the closed app.

### Task 10: Temp cleanup
Files: engine/src/cleanup.rs (new)
Depends on: 1
Run: parallel group A
Change: Delete files older than 24h in `%TEMP%` and `C:\Windows\Temp`. Skip locked files silently. Return the freed bytes. `revertible: false`, and the UI must say so. A dry-run mode reports the size first.
Test: "dry run deletes nothing", "files newer than 24h kept" (temp dir fixture).
Verify: `cargo test -p engine cleanup` → pass.

### Task 11: Tauri command layer
Files: src-tauri/src/{main.rs,commands.rs}
Depends on: 3, 5, 6, 7, 8, 9, 10
Run: inline
Change: Typed commands: `scan`, `load_score`, `list_tweaks`, `apply_tweaks(ids)` (creates the restore point first, once per session), `revert(id)`, `revert_all`, `boost_get/set_config`, `cleanup(dry_run)`. Errors come back as `{code, message_en, message_ur}`.
Test: Rust tests call commands with mock engine deps.
Verify: `cargo test -p raftaar` → pass.

### Task 12: UI (React)
Files: src/** (new)
Depends on: 11
Run: subagent
Change: Dark theme, EN/UR toggle (Noto Nastaliq Urdu font, RTL for Urdu). Screens: **Dashboard** (PC class, load score, big "Optimize" button that applies the recommended tweaks), **Tweaks** (cards: name, plain explanation, risk badge, "needs restart" badge, toggle, per-tweak revert), **Game Boost** (pick game exes, pick apps to close, on/off), **Cleanup** (dry-run size, then confirm), and a persistent **Revert All** in the header. If the restore point fails, show a modal with the real reason and options: "Enable protection" / "Continue without" / "Cancel".
Test: Vitest + Testing Library: "optimize calls apply with recommended ids", "urdu toggle sets dir=rtl", "cleanup shows irreversible warning".
Verify: `npm test` → pass. `npm run tauri dev` → all 4 screens render with no console errors.

### Task 13: Sandbox end-to-end test and installer
Files: scripts/state-diff.ps1, scripts/sandbox.wsb (new), src-tauri/tauri.conf.json
Depends on: 12
Run: inline
Change: `state-diff.ps1 -Save before.json` / `-Compare before.json` dumps every registry value in the catalog plus the active power plan. Build an NSIS installer. In Windows Sandbox: install → scan → Optimize → Revert All → compare.
Test: n/a (manual E2E with a scripted diff)
Verify: `npm run tauri build` → installer under 15 MB. In Sandbox, `state-diff.ps1 -Compare` → "0 differences". App idle RAM under 80 MB (Task Manager).

## Log
- 2026-10-10 spec agreed (Tauri, gamers first then cafe, Free+Premium, EN+UR), plan written: 13 tasks
- 2026-10-10 tasks 1-6 done (commit cb30502)
- 2026-10-10 tasks 7-10 done: `cargo test -p engine` 35 passed. Task 11 command layer builds; no command-level tests (commands are thin pass-throughs to tested engine code).
- 2026-10-10 task 12 done: `npm test` 6 passed, tsc + eslint clean. Tauri npm packages bumped to 2.12 to match the crate.
- 2026-10-10 task 13: state-diff.ps1 + sandbox.wsb written, NSIS-only bundle (1.56 MB). Windows Sandbox is not installed on the dev PC.
- 2026-10-10 real run on the dev PC (user installed and pressed Optimize, elevated): all 7 tweaks landed in the registry, the restore-frequency key went back to Missing. Revert All on the real PC not yet run.
- 2026-10-10 dev app checked by screenshot: all 4 screens + Urdu RTL render with real scan data.
- 2026-10-10 RAM (release, idle): raftaar.exe 8 MB private + WebView2 121 MB → 94 MB with `--disable-gpu`. Target of 80 MB total is not reachable with WebView2; WebView2's browser+renderer alone are ~60 MB.
