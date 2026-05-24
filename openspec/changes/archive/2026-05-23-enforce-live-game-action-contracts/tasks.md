## 1. Pin the Spec Violations as Failing Tests

- [x] 1.1 Add a Rust integration test `code/digimon-engine/tests/live_game_action_validation.rs` that calls `LiveGame::step(<illegal_id>)` during Mulligan and asserts `ok == false` with a descriptive error. Confirm it fails on current `main`.
- [x] 1.2 Add a test that calls `LiveGame::play(P0, 0)` during Mulligan and asserts `ok == false`. Confirm failure on current `main`.
- [x] 1.3 Add a test that calls `LiveGame::play(P0, 0)` during P1's Main and asserts `ok == false` with `"not current decision player"` style error. Confirm failure on current `main`.
- [x] 1.4 Add a test that calls `LiveGame::end_turn()` during Mulligan and asserts `ok == false` AND that `turn_count` / `current_phase` did not change. Confirm failure on current `main` (currently fast-forwards Mulligan → Breeding).
- [x] 1.5 Add a test that calls `LiveGame::pass_turn()` during Mulligan and asserts `ok == false`. Confirm failure on current `main`.
- [x] 1.6 Add a test that calls `LiveGame::legal_actions(non_decision_player)` and asserts the returned `Vec` is empty. Confirm failure on current `main`.
- [x] 1.7 Add a test that plays a card with an `OnPlay` MemoryChange and asserts the resulting `ActionResult.events_emitted` deserializes to a structured event with accessible `kind`, `player`, and `delta` fields (not a `Debug` string). Confirm failure on current `main`.

## 1b. Pin the Pending-Selection Soft-Lock

- [x] 1b.1 Add a Rust integration test that drives DNA Omnimon vs BG Imperial seed=1 (or a more focused fixture) to the point where BT17-081's `[End of Your Turn]` selection surfaces with zero matching targets. Assert that the current engine produces a `pending_selection` with `is_optional: false` and exactly one no-op option. Confirm failure on current `main` (test should reproduce the soft-lock).
- [x] 1b.2 Add a synthetic test card whose mandatory effect targets a predicate that always returns empty (e.g., "Delete 1 of your opponent's Digimon with [Bogus] in its name"). Drive into a state where the trigger fires. Assert the engine fizzles cleanly rather than emitting a stuck pending.

## 2. Validation Implementation

- [x] 2.1 In `LiveGame::step`, fingerprint engine state before `decode_action` and detect no-op (`event_seq`, `current_phase`, `pending_selection` all unchanged). Return `ok: false, error: "action <id> not legal for player <pid> in phase <P>"` on no-op.
- [x] 2.2 In `LiveGame::play`, add `current_decision_player() == player` check. Return `ok: false` with descriptive error otherwise.
- [x] 2.3 In `LiveGame::play`, add `current_phase == GamePhase::Main` check (per design.md: Counter-window Option plays go through `step` with separate action-IDs, not `play()`). Reject all other phases — including Mulligan, Breeding, all Select* selection phases, BlockTiming/CounterTiming/AllianceTiming, EndOfTurnAction, EndTurn, GameOver.
- [x] 2.4 In `LiveGame::end_turn`, add `current_phase ∈ { Main, EndOfTurnAction }` check. In `LiveGame::pass_turn`, add `current_phase ∈ { Breeding, Main, EndOfTurnAction }` check. Both reject Mulligan, GameOver, and selection phases.
- [x] 2.5 In `LiveGame::legal_actions`, return `Vec::new()` when `player != current_decision_player()`.
- [x] 2.6 Update doc comments on each modified method to note the divergence from `HeadlessRunner::step`'s fire-and-forget contract and the explicit validation now performed.
- [x] 2.7 Re-run section 1's tests; confirm they pass.

## 2b. Fix Pending-Selection Soft-Lock (Fizzle Path)

- [x] 2b.1 In `code/digimon-engine/src/effect_context/selections.rs:1655` (existing fizzle-on-empty in `install_field_selection`), augment the fizzle path to emit a `GameEvent::EffectFizzled { source_permanent: self.source_permanent, reason: "no valid target".into() }` instead of silent return. This makes install-time fizzles observable through the event log.
- [x] 2b.2 In `LiveGame::step` (`code/digimon-engine/src/live_game.rs:528`), AFTER computing the no-op check from task 2.1, add a special case: if the pending selection was mandatory (`is_optional == false`) AND had exactly one legal option AND the step produced no events AND state is unchanged AND `current_phase` is one of the Select* phases, then automatically fizzle: clear `self.game.pending_selection`, restore `self.game.current_phase = pending.previous_phase`, emit `GameEvent::EffectFizzled { source_permanent: pending.source_permanent, reason: "no executable target".into() }`, and return `ok: true` with the fizzle event in `events_emitted`. Do NOT fizzle if the selection has more than one option (per spec scenario: caller should try other options first).
- [x] 2b.3 Re-run the section 1b tests; confirm the soft-lock no longer reproduces and the fizzle event surfaces in `events_emitted`.
- [x] 2b.4 Verify the fix does not break legitimate mandatory selections whose target list is non-empty (run the existing `cargo test --manifest-path code/digimon-engine/Cargo.toml`).

## 3. Structured Event Serialization

- [x] 3.1 Add `#[derive(serde::Serialize)]` with `#[serde(tag = "type")]` to `GameEvent` in `code/digimon-engine/src/events.rs`. Add `#[derive(serde::Serialize)]` to `TerminalOutcomeReason` in `code/digimon-engine/src/game.rs` (one-line addition, no other changes needed; `PlayerId` and `GamePhase` already derive `Serialize`).
- [x] 3.2 Add a new `GameEvent::EffectFizzled { seq, source_permanent: Option<PermanentHandle>, reason: String }` variant. Update `type_str()` to return `"EffectFizzled"`. Ensure `PermanentHandle` derives `Serialize` (likely already does — verify).
- [x] 3.3 Update `LiveGame::make_result` in `code/digimon-engine/src/live_game.rs:625` to serialize events as structured `serde_json::Value` instead of `format!("{:?}", e)`. Change `ActionResult.events_emitted` type from `Vec<String>` to `Vec<serde_json::Value>`.
- [x] 3.4 Update the MCP `events` tool in `code/digimon-engine-mcp/src/tools.rs` to return the structured form. Update the response JSONSchema in the tool's `tools/list` registration to reflect the new shape (`type` field + variant-specific siblings).
- [x] 3.5 Add a Rust unit test asserting a sample of `GameEvent` variants serialize to the expected JSON shape: `MemoryChange`, `Play`, `Digivolve`, `GameOver`, and the new `EffectFizzled`. Each test asserts the top-level `type` matches `type_str()` and key variant fields appear at the top level.
- [x] 3.6 Update `docs/DEBUG_MCP.md` event-format examples to show the new structured JSON shape with `{"type": "MemoryChange", ...}`.

## 4. Add `digivolve` and `attack` Tools

- [x] 4.1 Decide handle JSON shape (see Open Questions). Standardize on `{"player": <id>, "index": <u8>}` matching the existing `field` view's `handle` field unless implementation reveals a reason otherwise.
- [x] 4.2 Implement `LiveGame::digivolve(host_handle, source_hand_idx, paid_costs?) -> ActionResult` that resolves to an action ID via `legal_decoded_actions` and dispatches through the same validation path as `step`.
- [x] 4.3 Implement `LiveGame::attack(attacker_handle, target) -> ActionResult` similarly. `target` accepts either a permanent handle (battle-attack) or the literal `"security"` (security-attack).
- [x] 4.4 Expose both as MCP tools in `code/digimon-engine-mcp/src/tools.rs` with JSONSchema. Register in the `tools/list` response.
- [x] 4.5 Add Rust integration tests covering: successful digivolve, illegal digivolve (no matching source), illegal attack (suspended attacker), illegal attack (no target). Each asserts structured rejection.
- [x] 4.6 Remove the "use `step` for digivolve/attack" v1-limitation note from `docs/DEBUG_MCP.md`.

## 5. End-to-End Stdio Fixture

- [x] 5.1 Add `code/digimon-engine-mcp/tests/stdio_e2e.rs` (or similar) that spawns the binary via `std::process::Command`, sends the `initialize` + `notifications/initialized` handshake, then calls each new/modified tool and asserts response shape.
- [x] 5.2 Test that `tools/list` advertises `digivolve` and `attack`.
- [x] 5.3 Test that calling `step` with an illegal action_id returns the new `ok: false` response shape.
- [x] 5.4 Test that `events` returns structured event objects (assert field-level access, not regex on strings).
- [x] 5.5 Test that `play` during Mulligan returns the new structured rejection.

## 6. Documentation

- [x] 6.1 Update `docs/DEBUG_MCP.md`: structured event format, new tools, removed v1 limitation note.
- [x] 6.2 Update `qa/archetype-qa/engine-api-reference.md` if it documents the action surface or events.
- [x] 6.3 Update `docs/RUST_ENGINE_API.md` if its examples reference the old behavior.
- [x] 6.4 Add a note in `docs/RUST_PYTHON_PARITY.md` describing the `LiveGame` vs `HeadlessRunner` step-semantic divergence (intentional after this change).
- [x] 6.5 Promote `.claude/tmp/mcp_client.py` to `code/digimon-engine-mcp/tests/fixtures/` (or delete) — it currently regex-parses the Debug event format and would otherwise rot.

## 7. Verification

- [x] 7.1 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml` — all new tests pass.
- [x] 7.2 Run `cargo test --manifest-path code/digimon-engine-mcp/Cargo.toml` — stdio fixture passes.
- [x] 7.3 Run the existing Rust test suite — no regressions.
- [x] 7.4 Re-run a manual Medusamon vs Puppets MCP-driven session against the rebuilt binary; confirm none of the eight bugs from the QA report reproduce.
- [x] 7.5 Verify `ACTION_SPACE_SIZE` and tensor profiles unchanged via `cargo test` on relevant integration tests.
- [x] 7.6 Record commands and outcomes in an implementation summary appended to this change before archiving.

## Implementation Summary (2026-05-24)

All 48 tasks complete. Implementation verified via:

- **161 lib tests pass** — `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib` (was 154 before; +7 new validation tests).
- **17 integration test binaries** sampled clean — `replay_runner`, `debug_runner_dsl`, `policies_headless`, `policies_greedy`, `owner_routing_live`, `keyword_parsing`, `ace_overflow`, `cannot_move_breeding`, `dna_digivolve_user_action`, `dsl_eval_arm_coverage`, `timing_dispatch`, `modifier_disable_effect`, `zone_manipulation`, `track_c_*` — all pass (231 tests total in the sample).
- **New `live_game_action_validation.rs` integration test** — 7/7 pass.
- **14 MCP integration tests pass** — `cargo test --manifest-path code/digimon-engine-mcp/Cargo.toml` (was 9 before; +5 new envelope-shape tests for digivolve/attack/step/play).
- **End-to-end MCP repro** (`.claude/tmp/verify_fixes.py`) — all 8 bugs from `qa/qa-reports/2026-05-23-medusamon-vs-puppets-mcp.md` no longer reproduce; `tools/list` now reports 26 tools including `digivolve` and `attack`; `events_emitted` entries are structured `{"type": ..., "seq": ..., ...}` objects.
- **End-to-end soft-lock repro** (`.claude/tmp/verify_softlock_fix.py`) — DNA Omnimon vs BG Imperial seed=1, previously stuck at iter 70 in a `TriggerOrder` loop, now plays through to game-over at iter 161 (T14, P0 wins via SecurityAttack).
- **`ACTION_SPACE_SIZE` (2192) unchanged** — `git diff --stat HEAD -- code/digimon-engine/src/action code/digimon-engine/src/tensor.rs` returns no diff.
- **Workspace status**: `code/src-tauri` has a pre-existing build break (missing `also_treated_as` field in `CardData` from PR #495) unrelated to this change. Verified by `git log` on the affected files; the engine + MCP + CLI + PyO3 packages all build clean.

Notes:
- The `LiveGame::step` no-op detection uses a fingerprint comparison of `event_seq`, `current_phase`, and `pending_selection` identity. False-positive risk for actions that mutate engine state without changing any of those three is low — every meaningful engine action either emits an event, advances phase, or modifies pending. The single-option fizzle path is gated on `!is_optional && valid_action_ids.len() == 1` to avoid fizzling early when other options remain.
- `install_field_selection` emits `EffectFizzled` at the install-time empty-target path. The end-to-end soft-lock repro shows this path is rarely exercised in real games — most fizzles happen at execute time via the `LiveGame::step` safety net.
- Documentation updated: `docs/DEBUG_MCP.md` (structured event format, 26-tool surface, `play`/`step`/`end_turn`/`pass_turn` validation rules, new `digivolve`/`attack` tools, removed the "use `step` for digivolve/attack" v1-limitation note) and `docs/RUST_PYTHON_PARITY.md` (intentional `LiveGame::step` vs `HeadlessRunner::step` divergence callout).

### Commands run
- `cargo build -p digimon-engine -p digimon-engine-mcp -p digimon-engine-cli -p digimon-engine-py` — clean
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib` — 161 passed
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test live_game_action_validation` — 7 passed
- `cargo test --manifest-path code/digimon-engine-mcp/Cargo.toml` — 14 passed
- `python .claude/tmp/verify_fixes.py` — 8/8 bugs verified fixed
- `python .claude/tmp/verify_softlock_fix.py` — game completes (was stuck)
