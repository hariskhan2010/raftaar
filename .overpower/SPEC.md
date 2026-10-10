# Raftaar (working name), Spec

A Windows PC/game optimizer that aims to beat Hone (hone.gg) on **safety, honesty,
lightness and low-end PCs**, priced for Pakistan. "Raftaar" (رفتار) means speed.
Before release, check that no existing product uses the name.

## Why users would pick us over Hone
Hone's public reviews report boot failures, driver conflicts, FPS that didn't improve or
dropped, ads in the free tier and dollar-only pricing.

| Hone weakness | Our answer |
|---|---|
| Changes can break Windows | Restore point before the first optimization, every change snapshotted, one-click Revert All |
| "No FPS difference" | Before/after measurement shown to the user, with no invented numbers |
| Black box | Every tweak shows what it does, its risk level and whether it needs a restart (English + Urdu) |
| Heavy app on weak PCs | Tauri: ~10 MB installer, ~30-60 MB RAM |
| Ads and USD pricing | No ads. Premium in PKR (JazzCash/Easypaisa) |
| One profile for every PC | Scan RAM/CPU/GPU and recommend tweaks per PC class (low / mid / high) |

## Decisions (agreed 2026-10-10)
- **Stack:** Tauri v2 (Rust backend) + React + TypeScript + Vite.
- **Audience:** home gamers first (Phase 1). Gaming cafes in Phase 2, reusing the same engine as a per-PC agent, so the Rust engine is a library crate with no UI coupling.
- **Money:** Free + Premium (300-500 PKR/month). Cafe mode is priced per PC.
- **Language:** English + Urdu, user-selectable.

| Free | Premium |
|---|---|
| System scan, PC class | Game Boost (auto while a game runs) |
| ~10 safe tweaks + Revert All | FPS before/after benchmark (Phase 2) |
| Load score before/after | Advanced per-PC-class tweaks |
| Temp cleanup | Cafe mode (Phase 2, per-PC price) |

Phase 1 builds every feature unlocked. Gating lives behind one `is_premium()` check, wired to payments later.

## Must-do (Phase 1 MVP)
1. Runs elevated (requireAdministrator manifest) on Windows 10/11 x64.
2. Creates a System Restore Point before the first optimization of a session, and tells the user honestly if it can't (protection off, or the 24h limit).
3. Every tweak records the value it replaces. Revert one tweak, or Revert All, restores the exact prior state. Snapshots survive app restarts.
4. ~10 well-known low/medium-risk tweaks. **Never** touches Defender, Windows Update, UAC, or disables core services.
5. System scan: CPU, RAM, GPU, Windows build, startup apps, background process count. Classifies the PC as low/mid/high and pre-selects recommended tweaks.
6. "System Load Score" before/after (RAM in use, background processes, startup items, idle CPU %). Labelled as a load score, **not** FPS.
7. Game Boost: the user picks game .exe files. While one runs, raise its priority and close user-chosen background apps, then restore both on exit.
8. Temp cleanup, clearly marked "cannot be undone".
9. UI in English + Urdu, dark theme.
10. NSIS installer, elevated.

## Not in Phase 1
FPS benchmark (PresentMon), cafe mode, accounts/payments, network tweaks (Nagle/TCP: disputed benefit).

## Testing rule
Rust logic is unit-tested against mocked `Registry` / `Shell` traits (`cargo test`).
Real tweaks run first in **Windows Sandbox or a VM**, never first on the dev PC.

## Done when
In Windows Sandbox: scan → optimize → load score shown → Revert All → `scripts/state-diff.ps1` reports zero differences from the pre-optimize state.
