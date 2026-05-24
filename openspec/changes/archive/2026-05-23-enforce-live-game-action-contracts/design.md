## Context

`LiveGame` was introduced as the unified surface for `digimon-engine-cli`, `digimon-engine-mcp`, recordings, and debug forensics — the human/agent-facing analogue of `HeadlessRunner`, which serves the RL training loop. The `live-game-surface` and `engine-debug-mcp` specs explicitly promise structured errors on illegal actions, structured JSON events, and a tool surface that includes dedicated `digivolve` and `attack` methods.

A scripted Medusamon vs Puppets QA session driven through `digimon-engine-mcp` (full report: [qa/qa-reports/2026-05-23-medusamon-vs-puppets-mcp.md](qa/qa-reports/2026-05-23-medusamon-vs-puppets-mcp.md)) uncovered that the action surface diverges from the spec in eight ways:

| # | Severity | Symptom | Spec violation |
|---|----------|---------|----------------|
| 1 | critical | `step(<illegal id>)` returns `ok: true`, no state change | `live-game-surface:111` — "SHALL NOT panic; SHALL return `ok: false`" |
| 2 | high | `legal_actions(P0)` lists actions during P1's turn; `step` then silently no-ops them | violates the spirit of `legal_actions` scenarios (all assume current decision player) |
| 3 | high | `play()` during Mulligan phase succeeds — Elizamon lands, On-Play fires, phase → SelectReveal | `live-game-surface:111` (any illegal play SHALL return ok:false) |
| 4 | critical | `play(P0, hand_idx=0)` during P1's Main turn — P0 plays a card out of turn | `live-game-surface:111` |
| 5 | critical | `end_turn()` during Mulligan silently advances T0 Mulligan → T1 Breeding | unspec'd; `Game::end_turn` forcibly sets `current_phase = EndTurn` regardless of starting phase |
| 6 | high | `pass_turn()` during Mulligan returns `ok: true` with no state change | `live-game-surface:111` |
| 7 | high | `digivolve` and `attack` MCP tools do not exist | `engine-debug-mcp:94-95` and `live-game-surface:96-97` both list them |
| 8 | medium | `events_emitted` contains `'MemoryChange { seq: 0, player: 1, delta: -3, total: -3 }'` (Rust `Debug` strings) | `live-game-surface:107` types `events_emitted: Vec<GameEvent>` |

All eight cluster in two files (`live_game.rs`, `events.rs`) and share two root causes:

1. **Validation gap** — `LiveGame::step` was modelled after `HeadlessRunner::step`, which is intentionally fire-and-forget because the RL pipeline always sends action IDs from a precomputed mask. That contract is wrong for a debugging surface whose callers ask the engine "what's legal?" and need to know whether their action took effect. `play`, `end_turn`, and `pass_turn` inherit similar gaps.
2. **Serialization gap** — `GameEvent` only derives `Debug, Clone`. `make_result` does `format!("{:?}", e)` and the result is dropped verbatim into the JSON response. The spec's `Vec<GameEvent>` type was never wired to `serde`.

## Goals / Non-Goals

**Goals**:
- Every `LiveGame` action method enforces phase and decision-player preconditions and returns `ActionResult { ok: false, error: "..." }` when violated.
- `legal_actions(player)` returns `[]` when `player` is not the current decision player.
- `GameEvent` and the types it contains are JSON-serializable via `serde`; both `events_emitted` and the MCP `events` tool return structured objects with field-level access.
- The MCP server exposes `digivolve` and `attack` tools that wrap structured `LiveGame` methods of the same name.
- Integration tests pin every scenario from the spec deltas; an end-to-end test against `digimon-engine-mcp` over stdio confirms the JSON shape.

**Non-Goals**:
- Change `HeadlessRunner::step` or any RL training surface — it stays fire-and-forget.
- Change `ACTION_SPACE_SIZE`, tensor profiles, or observation contracts.
- Implement card-level fixes (these are spec-layer fixes; card-effect QA is the `/batch-fix-cards` lane).
- Unstub `seek` (its own deferred change, per `engine-debug-mcp:128`).
- Add `snapshot` / `restore` / branching tools (also deferred per spec).
- Re-engineer `Game::decode_action`. Validation goes in the `LiveGame` wrapper so we don't touch the engine core or `HeadlessRunner`.

## Decisions

### Decision: Validate in `LiveGame`, not in `Game::decode_action`

The decoder is shared by `HeadlessRunner` (RL) and `LiveGame` (debug/MCP). Adding rejection there would break HeadlessRunner's fire-and-forget contract and could complicate the training loop. We add the validation in the `LiveGame` wrapper methods so RL is untouched.

Alternative considered: validate in `decode_action`. Rejected — too much blast radius, and the RL pipeline genuinely doesn't want it.

### Decision: Detect `step` no-op via `before_seq == after_seq && before_phase == after_phase && pending unchanged`

For arbitrary `action_id`, the cleanest way to detect rejection after `decode_action` is to fingerprint engine state before and after. If nothing changed and no events were emitted, the action was rejected — return `ok: false` with `"action <id> not legal for player <pid> in phase <P>"`. This is a small `make_result` change.

Alternative considered: reach into `build_action_mask` and reject pre-decode. Rejected — duplicates the legality logic. The post-decode fingerprint piggy-backs on facts the engine already maintains.

### Decision: Gate `play`, `end_turn`, `pass_turn` on `current_decision_player()` and `current_phase`

Each method adds an upfront check returning `ok: false` with a descriptive error. Concrete phase sets, confirmed by reading the action decoder and play helpers:

- **`play(player, hand_idx)`** — accept ONLY when `current_phase == GamePhase::Main` AND `current_decision_player() == player`. Reading [code/digimon-engine/src/action/decode.rs:96-117](code/digimon-engine/src/action/decode.rs) confirms hand-plays (action_id 0..29) are dispatched only through the Main-phase decoder branch; Counter-window plays use a separate action-ID range restricted to Standard Counter Options (decode.rs:1229 region) and go through `step()`, never through `play()`. `Game::play_from_hand` itself ([code/digimon-engine/src/game_actions.rs:370](code/digimon-engine/src/game_actions.rs:370)) validates hand index, field capacity, and memory cost but does NOT validate phase — the wrapper layer is genuinely the only enforcement point.

- **`end_turn()`** — accept when `current_phase ∈ { Main, EndOfTurnAction }`. Reading [code/digimon-engine/src/game_phases.rs:194](code/digimon-engine/src/game_phases.rs:194) (`Game::end_turn`) shows it forcibly sets `current_phase = GamePhase::EndTurn` regardless of starting phase — this is why calling `end_turn()` during Mulligan corrupts state.

- **`pass_turn()`** — accept when `current_phase ∈ { Breeding, Main, EndOfTurnAction }`. `pass_turn` is the engine's "advance phase" gate during the active player's turn and during the optional end-of-turn-action window.

For all three, the wrapper SHALL also reject during `GamePhase::GameOver` and any selection phase (`SelectTarget`, `SelectMaterial`, ...) — those phases require `resolve_selection`, not these methods.

### Decision: Add structured `GameEvent` serialization via direct serde derive with `#[serde(tag = "type")]`

`#[derive(serde::Serialize)]` directly on `GameEvent` with `#[serde(tag = "type")]`. Confirmed by reading [code/digimon-engine/src/events.rs](code/digimon-engine/src/events.rs):

- `GameEvent` has 10 variants (`MemoryChange`, `TurnStart`, `PhaseChange`, `Play`, `Digivolve`, `Attack`, `Trash`, `Mill`, `SecurityReveal`, `GameOver`).
- All variant fields are serde-friendly primitives: `u64`, `u16`, `i16`, `u8`, `String`, `Option<…>`.
- Nested types: `PlayerId` (u8-backed enum in [enums.rs:108](code/digimon-engine/src/enums.rs:108) — already derives `Serialize`), `GamePhase` (already derives `Serialize`), `TerminalOutcomeReason` ([game.rs:59](code/digimon-engine/src/game.rs:59) — needs `Serialize` added, trivial one-line change).
- A `type_str()` method already exists returning stable variant names (`"MemoryChange"`, `"Play"`, ...) — `#[serde(tag = "type")]` will emit exactly these.
- No `Box`, `Rc`, function pointers, or other non-Serialize gunk in any variant.

Wire shape per variant (target):

```json
{ "type": "MemoryChange", "seq": 0, "player": 0, "delta": -3, "total": -3 }
{ "type": "Play",         "seq": 1, "player": 0, "card_id": "BT24-008", "field_index": 0 }
{ "type": "GameOver",     "seq": 9, "winner": 0, "reason": "SecurityAttack" }
```

Rejected `GameEventView` mirror: would duplicate every variant for no real benefit because the enum is clean. If cross-engine Python-wire parity ever becomes necessary, a `GameEventPyWire` mirror can be added later as a separate change without affecting this one.

Python parity note: [code/engine_py_legacy/engine/events.py](code/engine_py_legacy/engine/events.py) uses a flatter `{type, seq, player, source_card_id, source_slot, target_card_id, target_slot, meta: {...}}` shape with variant-specific fields in `meta`. We are NOT matching that wire shape — Python is being sunset and there are no current Python consumers of `events_emitted`. The Rust-native shape is cleaner and more type-safe.

Alternative considered: keep Debug strings + add an `event_data: Value` field alongside. Rejected — doubles the payload and confuses consumers.

### Decision: Implement `digivolve` and `attack` tools as thin wrappers over `step` + lookup

The decoder already understands digivolve and attack action IDs. The new tools take typed arguments (`host_handle`, `source_hand_idx`, `paid_costs?`; `attacker_handle`, `target`), translate to the matching action ID by consulting `legal_decoded_actions`, and dispatch via `step`. Same validation/serialization treatment.

### Decision: Soft-lock fix is FIZZLE (not always-decline)

Confirmed by reading [code/digimon-engine/src/effect_context/selections.rs:1626-1657](code/digimon-engine/src/effect_context/selections.rs:1626): the engine's existing convention for selection installation IS fizzle-on-empty:

```rust
// install_field_selection
let mut valid_action_ids: Vec<u16> = Vec::new();
for i in 0..target_count {
    if filter(self.game, h) { valid_action_ids.push(encode_attack(0, i as u16)); }
}
// Empty valid set → silently no-op. The RL policy never sees a
// "mandatory prompt with no legal answer" state, matching Python.
if valid_action_ids.is_empty() { return; }
```

So fizzle-on-empty is already engine policy at install time. **The BT17-081 soft-lock proves this isn't enough**: the install-time filter passed exactly one target (`action_id: 102`), so the selection was installed. But when `step(102)` executes, it's a no-op — the engine accepted the target as "passes filter" but couldn't actually do anything with it (likely the cost-payment or sub-action under the selection has its own legality check that fails).

The fix has two layers:

1. **Primary (in scope)**: In `LiveGame::step`, when the action_id is the only option in a mandatory pending selection AND the step produces no events and no state change, fizzle the selection automatically: clear `pending_selection`, emit `GameEvent::EffectFizzled { source, reason }`, and return `ok: true` with the fizzle event in `events_emitted`. This treats the wrapper as the safety net for any install-time filter that admits an unfulfillable target.

2. **Secondary (out of scope, separate change)**: Audit individual selection-installation sites in `effect_context/selections.rs` for cases where the filter passes but the resulting callback is itself unfulfillable. These are per-helper bugs that belong with the affected card scripts, not in this MCP/LiveGame proposal.

The fizzle path matches existing Python parity precedent ("matches Python — a 'delete target X' effect with no valid targets silently does nothing") and matches the existing convention at install time. Always-decline was rejected because it pollutes the action mask with extra options for RL training and contradicts the rules' fizzle-on-no-target semantics.

Rejected alternative: always include a pass / decline action in `legal_actions` for mandatory pending selections. This would force callers (especially RL training via `HeadlessRunner`) to learn to pass on mandatory selections, contradicting both the rules and the existing engine convention.

### Decision: Add a stdio JSON-RPC fixture under `code/digimon-engine-mcp/tests/`

In-process Rust integration tests cover `LiveGame`. The MCP server's behavior (wire format, tool dispatch) needs an end-to-end fixture that spawns the binary, talks JSON-RPC, and asserts the response shape. Pattern: a Rust integration test that does `Command::new("target/debug/digimon-engine-mcp").stdin(piped).stdout(piped).spawn()` and exchanges JSON-RPC messages.

## Risks / Trade-offs

- **Wire-format change for `events_emitted` and `events`** is breaking for any consumer that regex-parses Debug strings today. Mitigation: grep the repo for consumers (the QA scripts under `.claude/tmp/` are the only known ones); update docs in `docs/DEBUG_MCP.md` to advertise the structured format; note this in the change log.
- **`HeadlessRunner` and `LiveGame::step` semantic divergence** — they were intentionally aligned. After this change, `LiveGame::step` rejects illegal IDs while `HeadlessRunner::step` silently drops them. Document the divergence in `live_game.rs` doc comments and in `RUST_PYTHON_PARITY.md`.
- **`play()` phase check might be over-tight** if there are legitimate non-Main contexts (Tamer plays during Counter timing, security-effect plays). Mitigation: enumerate the legal-play phases by reading `Game::play_from_hand` and the spec's combat/counter requirements before locking in the check; the integration tests should cover both the legal and illegal phase sets.
- **`digivolve` / `attack` typed argument schema** isn't fully pinned in the spec (params listed but no JSONSchema). We'll need to design a `handle` JSON shape that round-trips through `field` views (`{"player": 0, "index": 1}` matches what existing views return).
- **Spec scenarios proliferate** — we're adding ~10-15 scenarios across two specs. Mitigation: keep scenario bodies short and group by validation theme.

## Migration Plan

1. **Add failing Rust integration tests** in `code/digimon-engine/tests/` for each spec scenario. Run; confirm failures match the bug table above.
2. **Implement validation** in `LiveGame::step`, `play`, `end_turn`, `pass_turn`, `legal_actions`. Re-run tests; confirm they pass.
3. **Add `Serialize`** to `GameEvent` and contained types; introduce `GameEventView` if name stability requires it. Update `LiveGame::make_result` to emit structured events. Update MCP `events` tool similarly. Tests for wire shape.
4. **Implement `digivolve` and `attack`** in `LiveGame` (resolving typed args to action IDs via `legal_decoded_actions`) and expose as MCP tools.
5. **End-to-end stdio fixture** under `code/digimon-engine-mcp/tests/` — spawn binary, exchange JSON-RPC, assert shape.
6. **Documentation** — update `docs/DEBUG_MCP.md` to advertise structured events and `digivolve`/`attack` tools; remove the "use `step` for digivolve/attack" v1 limitation note. Update `docs/RUST_ENGINE_API.md` if it mentions the action surface. Update `qa/archetype-qa/engine-api-reference.md` if it advertises the old behavior.
7. **Cleanup** of QA temp scripts that regex-parse Debug events (move to `code/digimon-engine-mcp/tests/` as proper test fixtures or delete).

Rollback is a normal code revert; no persisted data, tensor, action-space, or model metadata migration is required.

## Open Questions

- **`digivolve` typed arg shape** — should `host_handle` be `{player, index}` or a `PermanentHandle`? Decision: use `{"player": <0|1>, "index": <u8>}` matching the existing `field` view's `handle` field shape (already what `field` returns; round-trips cleanly without translation).
- **Should `legal_actions(non-decision-player)` return `[]` or a structured `{actions: [], reason: "not current decision player", current_decision_player: 1}`?** Default to `[]` to match the spec's enumeration shape. If callers want metadata about who CAN act, they should call `state()` and read `turn_player` + `pending_selection.selecting_player`. Revisit only if a real caller surfaces.
- **Should `EffectFizzled` event include the failing `action_id`?** Tentatively yes — including `{ "type": "EffectFizzled", "seq": N, "source_permanent": {…}, "reason": "no valid target", "attempted_action_id": 102 }` is more useful for debugging than a bare fizzle. Confirm during implementation whether the source-permanent handle is reliably available at fizzle time.

(The three previously-listed questions on `play()` accepted phases, `GameEventView` vs direct derive, and soft-lock approach are now resolved — see Decisions above.)
