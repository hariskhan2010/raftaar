# Raftaar

A safe, honest Windows game optimizer. Every change is snapshotted and can be reverted,
a System Restore Point is made before the first optimization, and FPS gains are measured
with Intel PresentMon instead of being promised. English + Urdu.

Plans: `.overpower/SPEC.md`, `.overpower/plan.md` (v0.1), `.overpower/plan-v2.md` (v0.2).

## Features (v0.2)

| Page | What it does |
|---|---|
| Home | PC scan and class, load score gauge, one-click optimize with Lite / Balanced / Max presets, live CPU / RAM / GPU monitor |
| Optimize | 26 tweaks in 7 categories (FPS & input lag, power, background, ads & tracking, visuals, network, storage); search, presets, share codes |
| Game Boost | Auto-detects Steam, Epic, Riot and Roblox games; while one runs: high priority, 1 ms timer, chosen apps closed and reopened after |
| FPS Test | Records a game with PresentMon: average FPS, 1% and 0.1% lows, frame-time chart, before/after comparison |
| Cleanup | Temp, Windows temp, shader caches, crash dumps, error reports, browser caches (skipped while the browser runs) |
| Startup | Enable/disable startup apps the same way Task Manager does |
| Safety | Every active change with time and one-click revert, restore points, System Restore |

Never touched: Defender, Windows Update, UAC, firewall, security mitigations, services, drivers.
A unit test (`catalog::never_touches_forbidden_areas`) enforces this.

## Layout

| Path | What |
|---|---|
| `engine/` | Rust library with all system logic. No UI dependency; reused later as the cafe agent. |
| `engine/src/bin/raftaar-cli.rs` | Debug CLI: `scan`, `score`, `list`, `apply ID`, `revert ID`, `revert-all`, `restore-point` |
| `src-tauri/` | Thin Tauri command layer (`src/commands.rs`); PresentMon sidecar in `binaries/` |
| `src/` | React UI: `pages/`, `components/`, `i18n.tsx` (EN + UR) |
| `scripts/` | `state-diff.ps1` and `sandbox.wsb` for the end-to-end test |

Rust build output goes to `E:/raftaar-target` (see `.cargo/config.toml`).

## Commands

```
cargo test -p engine      # engine unit tests (mocked registry/shell)
npm test                  # UI tests (Vitest)
npm run tauri dev         # dev app (not elevated; system tweaks, FPS Test and restore points need admin)
npm run tauri build       # NSIS installer -> E:/raftaar-target/release/bundle/nsis
```

## Languages

English, Urdu, Hindi, Arabic, Spanish, Portuguese (BR) and Turkish. UI strings live in
`src/i18n/<lang>.ts` (TypeScript makes every language define every string). Tweak names:
English and Urdu in `engine/src/catalog.rs`, the rest in `src/i18n/tweaks.ts`; a Rust test
fails if a tweak is missing from any language. First run picks the Windows display language.

## Free and Premium

Free: every safe tweak (Lite and Balanced presets), Revert All, restore points, Safety center,
Cleanup, Startup manager, live monitor. Premium (7-day trial on first run): Game Boost, FPS Test,
and the 4 Max-preset tweaks. Reverting is never locked. The backend enforces this
(`src-tauri/src/commands.rs`), not just the UI.

Selling keys (owner):

| Customer pays with | What happens |
|---|---|
| JazzCash / Easypaisa | They message you on WhatsApp; run `cargo run -p engine --bin raftaar-cli -- license-new "<name or phone>" 30` (or `365`, or `lifetime`) and send them the `RFTP-…` key. Keys are Ed25519-signed and checked offline. |
| Card (worldwide) | Lemon Squeezy emails the key. Set `LEMON_SQUEEZY_STORE_ID` in `src-tauri/src/lib.rs` and `checkoutUrl` in `src/config.ts`, then release. Keys are checked online, with 14 days offline grace. |

Payment numbers, WhatsApp, prices and the checkout link live in `src/config.ts`; empty values hide that
option. The license signing key is `%USERPROFILE%\.tauri\raftaar-license.key`: back it up, never share it.

## Website

`docs/` is the landing page, served by GitHub Pages at https://hariskhan2010.github.io/raftaar/ and by Vercel at https://raftaar-eight.vercel.app (redeploy: `vercel deploy --prod` inside `docs/`).

## Releasing an update

Installed copies check `https://github.com/hariskhan2010/raftaar/releases/latest/download/latest.json`
on start and offer the update; installs are verified against the public key in `tauri.conf.json`.

1. Bump `version` in `package.json` and `src-tauri/tauri.conf.json`.
2. `.\scripts\release.ps1 -Notes "What changed"` builds a signed installer and `latest.json`.
3. Create a GitHub release `v<version>` and upload both files.

The private signing key is `%USERPROFILE%\.tauri\raftaar-updater.key`. Back it up somewhere safe
and never commit it: without it, existing installs cannot receive updates.

## Testing real tweaks

Never test writes on your own PC first. Use Windows Sandbox:

1. `npm run tauri build`
2. Open `scripts/sandbox.wsb`
3. In the sandbox: `state-diff.ps1 -Save before.json` → install → Optimize → Revert All →
   `state-diff.ps1 -Compare before.json` must print `0 differences`.

## Third-party

FPS Test bundles Intel PresentMon 2.5.1 (MIT License, signed by Intel Corporation).
