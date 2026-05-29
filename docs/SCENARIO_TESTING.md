# Scenario testing substrate

A two-layer harness for gameplay-rules conformance, driven by a shared
fixture corpus. One `qa/scenarios/*.json` fixture stages an arbitrary
mid-game board and declares expected outcomes; the **same file** runs in
both layers:

- **Engine layer** (fast, headless): `cargo test -p digimon-engine --test
  scenario_corpus` — stages a `DebugRunner` from the fixture using the real
  `data/cards.json` pool and evaluates the `engine` assertions.
- **UI/server layer**: `npx playwright test scenario-conformance` (with
  FastAPI up) — stages via the Rust-backed `/debug/games` router and
  evaluates the same `engine` assertions server-side, plus DOM-level `ui`
  assertions in navigating game-flow specs.

Engine-correctness ("is the rule right?") has one source of truth and is
evaluated against engine state; UI-wiring ("can a human reach it?") is
DOM-level and lives only in the Playwright layer.

## Components

| Layer | Where | Role |
|---|---|---|
| Staging API (Rust) | `Game::stage_*` in `code/digimon-engine/src/game.rs`; `DebugRunner` setters | place field/breeding stacks, inject zone cards, set memory/phase/turn/first-player, `validate()` |
| PyO3 surface | `RustDebugGame` in `code/digimon-engine-py/src/lib.rs` | wraps `HeadlessRunner` (real cards + RL surface) + staging mutators + `internal_state()` |
| HTTP router | `code/server/routers/debug_games.py` (`/debug/...`) | stage / inspect / evaluate over HTTP; staged games play through the live `/games` routes |
| Fixture format | `qa/scenarios/README.md` | declarative JSON schema + assertion vocabulary |
| Corpus audit | `qa/scenarios/CORPUS.md` | 30 quiz scenarios → category + readiness |
| Rust runner | `code/digimon-engine/tests/scenario_corpus.rs` | headless conformance |
| Playwright | `code/frontend/e2e/scenario-conformance.spec.ts` + `fixtures/debug-game.ts` | UI-layer conformance |

## Authoring a fixture

See `qa/scenarios/README.md` for the full schema and assertion vocabulary.
Minimal flow:

1. Write `qa/scenarios/<id>.json` (decks, `state`, `zones`, `assertions`).
2. Tag `readiness`: `expected_pass` or `blocked_on_card_impl` (+ reason).
3. Add the `id` to the `CORPUS` list in `scenario-conformance.spec.ts`
   (the Rust runner auto-discovers all `*.json`).

`stack` arrays are bottom-to-top (last id = top card). The loader clears a
zone before populating it, and `validate()` rejects rule-illegal boards.

## Scope notes

- **Not the real Tauri binary.** Playwright cannot drive WebView2/WKWebView;
  the browser-mode bundle (same React + same Rust engine over HTTP) is the
  tested proxy.
- **Substrate ≠ all-pass.** Most quiz scenarios assert card-effect
  resolution and are `blocked_on_card_impl` until those cards exist; they
  flip to `expected_pass` as the cards land.
- **`RustDebugGame` is test/browser-only.** It lives in the PyO3 binding
  crate, which the Python-free desktop bundle does not link; no debug
  staging command is registered in the Tauri `invoke` handlers.
- The live `/games` router also carries `seed`, `action_script`, and
  `/undo` — a lightweight "reach a state by replaying decisions" entry
  point complementary to `/debug` direct board injection (design D8).
