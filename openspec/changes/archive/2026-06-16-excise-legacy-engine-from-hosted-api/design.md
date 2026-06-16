> **STATUS: DEFERRED.** Design captured for when this is prioritized. Not a commitment to the specifics below.

## Context

The hosted API is the only production surface still running the Python engine. The Rust engine already provides the primitives most of these clusters need — `RustHeadlessGame` (headless play), `runners/replay.rs` (replay core), `deck_tools.rs` (parse/validate/tested-cards, already consumed by the Tauri desktop layer), and the observation/state machinery. The gap is an **interactive** PyO3 surface (selection-driven, per-player views) and a network-side **state redaction** path over Rust state. The bindings boundary already encodes the Python 1/2 ↔ Rust 0/1 player-ID convention (rule 20), which any new interactive surface must preserve.

## Goals / Non-Goals

**Goals**
- `code/server/` imports zero `engine_py_legacy`; all gameplay/replay/deck/redaction logic runs on Rust.
- Preserve network contracts: per-player redaction (rules 9 & 14), deck legality + restricted list, recording/replay compatibility.
- Make `docs/RUST_PYTHON_PARITY.md` retirable.

**Non-Goals**
- Training build (separate change) and desktop (already Python-free).
- Changing the wire/state JSON schema beyond what redaction parity requires.

## Phasing sketch (each phase independently shippable)

1. **Deck rules first (lowest risk).** Route `parse_deck`/`validate_deck`/`summarize_deck`/restricted-list through the Rust deck tools via PyO3. Pure functions, easy differential testing against the Python implementation. Re-home `PlayerType` and small enums.
2. **Replay + recordings.** Move `ReplayRunner`/`HeadlessGame` server usage onto the Rust replay core via PyO3; verify recording-format compatibility with a corpus of existing recordings (the `dcgo-replay`/recording oracles are precedent).
3. **State redaction over Rust state.** Provide a redaction filter for Rust game state that satisfies the `state_filter` contract; differential-test redaction output against the Python filter.
4. **Live PvP runtime (highest risk).** Add an interactive Rust runner PyO3 surface (selection prompts, per-player observation, the play-order/turn machinery) and migrate `ws_*`/`matchmaking`/`lobby`. Shadow-run against the Python engine before cutover.
5. **Admin AI `script_promotion`.** Decide retire vs migrate (it is Python-card-script machinery; likely retired as card authoring is Rust DSL-first). Sequence last.

## Decisions (provisional)

- **Differential testing is the safety net.** Each phase keeps the Python path available behind a flag until the Rust path is proven byte-for-byte equivalent on the relevant contract (deck legality, redacted state, replay output).
- **Restricted-list source of truth.** Consolidate onto the Rust side (or a shared data file the Rust tools read) so server and desktop agree; verify no divergence during phase 1.
- **`script_promotion` likely retires**, not migrates — confirm with the admin AI pipeline owner before phase 5.

## Risks / Trade-offs

- **Live PvP cutover** is the dominant risk; mitigated by shadow-running and per-route flags.
- **Recording compatibility** — existing persisted recordings must replay; mitigated by a corpus regression gate before cutover.
- **Interactive PyO3 surface is net-new** and the largest build item; it is the reason this change is large and deferred.

## Open Questions

- Does the interactive Rust runner need a new crate-level API, or can it be assembled from existing `selection.rs` + `runners/`?
- Is `script_promotion` retired outright, and if so what replaces the admin AI pipeline's promotion step under Rust DSL authoring?
- Can redaction be done Rust-native, or is a thin Python filter over PyO3-exported state simpler and equally safe?

## Findings update (2026-06-14 — explore-legacy-removal investigation)

A deep read-only investigation (11 agents + adversarial verification) revised several assumptions in the sketch above. Corrections, in priority order:

**1. The Python engine cannot be the migration oracle.** "Shadow-run against the Python engine" (Decisions, above) is unsafe. The Python engine still runs, but: (a) **14 scripts across ST14/BT18/BT19/EX10/EX11** (incl. the EX10 Link archetype's `OnLinkCardDiscarded` cards) throw *uncaught* `AttributeError` mid-game from removed `EffectTiming` members; (b) `card_database.py` **silently swallows** script load errors → broken scripts become effect-less vanilla cards → a diff can show **false parity** where Python silently did nothing; (c) the Python action encoding has drifted from the 2192 space. Use the Python path only as a *scoped* reference on bit-rot-free decks, never as the cutover gate.

**2. The real engine oracle is strong but headless.** DCGO-recording replay (`tools/dcgo-replay`) + per-card DebugRunner behavioral tests + the 30-scenario judge-quiz suite + 26 archetype combo suites certify game-*rules* faithfulness — but none cover the **interactive wire**: redacted per-player views, selection-prompt payloads, or event emission. The PvP migration must build **net-new tests** for these (redaction over Rust `to_ui_json`; selection-payload parity; event-stream parity). The existing `test_rust_backend_parity.py` is shape/key-set only and excluded from the default run; it is not a migration gate.

**3. New prerequisite — Rust GameEvents are largely unwired (engine work).** Rust emits only `MemoryChange`/`Play`/`GameOver`; `TurnStart`/`PhaseChange`/`Digivolve`/`Attack`/`Trash`/`Mill`/`SecurityReveal` are defined-but-unwired (`RUST_PYTHON_PARITY.md:808-810`). The frontend animations subscribe to that stream, so **wiring these events is a prerequisite for the PvP cutover** — engine work, not just binding work. Add to phase 4.

**4. Phase 4 is smaller than "build an interactive runner."** The bot/human turn loop is **pure Python orchestration** over the existing `RustHeadlessGame` PyO3 surface — everything funnels through the unified `Game::decode_action` dispatcher, and `RustHeadlessGame` already exposes `step`/`get_action_mask`/`current_player_id`/`greedy_action`/`get_pending_selection`/`accept_mulligan`/`concede`/`get_events_since_last_step`/`to_ui_json`. **No new pyclass is required.** The residual gap is a thin **method-surface adapter** (the WS path expects `surrender`→`concede`, `get_last_events`→`get_events_since_last_step`, a real `get_last_log` (today a stub), and `describe_actions` (absent)) plus the test infra in (2). The long pole is the event wiring + redaction/selection test infra, *not* the runner. (This resolves the first Open Question: assemble from `RustHeadlessGame` + a Python loop, not a new crate API.)

**5. `to_ui_json` is drop-in for the server.** It is already in production on the Rust-backed `/games` + `/debug` routes; `state_filter.py` consumes it unchanged (it only touches `handIds`/`handCards`/`securityIds` under `player1`/`player2`). This settles the redaction Open Question: **keep the Python dict filter over PyO3-exported state** — it works today, no Rust-native filter needed.

**6. The deletion gate is larger than a `rm -rf`.** `engine_py_legacy` ships **live production modules** (`state_filter.py`, `script_promotion.py`, and `engine/data/*` loaders) imported by ~9-11 server routers — these must be **relocated** before the directory can be deleted. No engine-*rules* test coverage is lost (timing/digixros/dna-digivolve/security are replicated in `code/digimon-engine/tests`), but `test_rust_backend_parity.py` lives in the legacy tree yet tests *Rust* via PyO3 — **migrate it to `code/tests`, don't drop it.** Config: drop two `pyproject.toml` lines (`--ignore=code/engine_py_legacy`, `norecursedirs`).

**7. Scope reduction — most of this change moves to `shrink-legacy-engine-surface`.** That new change delivers phases 1 (deck rules), 2 (replay/recordings), the module-relocation half of 3 (state redaction), and 5 (`script_promotion` retire) independently and at low risk. **After it lands, this change's remaining scope is phase 4 only**: the live PvP runtime (`InteractiveGame` in `ws_*`/`lobby`) + the GameEvent wiring (3 above) + the interactive-wire test infra (2 above) + the final `code/engine_py_legacy/` deletion (6 above). Update `proposal.md`/`tasks.md` scope accordingly when this is prioritized.
