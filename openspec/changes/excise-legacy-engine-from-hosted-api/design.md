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
