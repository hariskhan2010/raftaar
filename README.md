# Raftaar

A safe, honest Windows game optimizer. Every change is snapshotted and can be reverted,
a System Restore Point is made before the first optimization, and the app shows a
load score (not invented FPS numbers). English + Urdu.

Spec and plan: `.overpower/SPEC.md`, `.overpower/plan.md`.

## Layout

| Path | What |
|---|---|
| `engine/` | Rust library with all system logic (registry, power plans, restore point, scan, score, Game Boost, cleanup). No UI dependency; reused later as the cafe agent. |
| `engine/src/bin/raftaar-cli.rs` | Debug CLI: `scan`, `score`, `list`, `apply ID`, `revert ID`, `revert-all`, `restore-point` |
| `src-tauri/` | Thin Tauri command layer (`src/commands.rs`) |
| `src/` | React UI: Dashboard, Tweaks, Game Boost, Cleanup |
| `scripts/` | `state-diff.ps1` and `sandbox.wsb` for the end-to-end test |

Rust build output goes to `E:/raftaar-target` (see `.cargo/config.toml`).

## Commands

```
cargo test -p engine      # engine unit tests (mocked registry/shell)
npm test                  # UI tests (Vitest)
npm run tauri dev         # dev app (not elevated; HKLM tweaks fail, a banner says so)
npm run tauri build       # NSIS installer -> E:/raftaar-target/release/bundle/nsis
```

## Testing real tweaks

Never test writes on your own PC first. Use Windows Sandbox:

1. `npm run tauri build`
2. Open `scripts/sandbox.wsb`
3. In the sandbox: `state-diff.ps1 -Save before.json` → install → Optimize → Revert All →
   `state-diff.ps1 -Compare before.json` must print `0 differences`.
