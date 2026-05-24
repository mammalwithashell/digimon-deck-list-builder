# Gameplay QA Report — Medusamon vs Puppets (Rust engine via MCP)

## Test Setup
- **Date**: 2026-05-23
- **Engine**: Rust (digimon-engine) via `digimon-engine-mcp` (stdio JSON-RPC, `--pool implemented`)
- **Archetypes**: Medusamon (P0, 1st-place decklist), Puppets (P1, first decklist)
- **Seed**: 7
- **Game IDs**: `9qaltp5m`, `du0b1uvv`, `crknlu8m`, plus several reproduction games
- **Driver**: `.claude/tmp/mcp_client.py` (stdio JSON-RPC) + `play_full_game.py`
- **Focus areas**: MCP tool semantics, action validation, effect chains, event reporting

## Summary
- **Total Issues Found**: 9
- **Critical**: 3 | **High**: 5 | **Medium**: 1 | **Low**: 0

> **Update 2026-05-24**: After the initial 4 findings, a follow-up audit of sibling
> action methods uncovered 4 more spec violations in the same `LiveGame` surface
> (Issues 5–8). A subsequent multi-matchup scripted run (DNA Omnimon vs BG
> Imperial, Omnimon Ace, several seeds) surfaced Issue 9 — a mandatory-selection
> soft-lock. All nine are bundled into OpenSpec change
> `enforce-live-game-action-contracts`.

The card-level effect resolution we observed (Karakurumon → Hanimon chain, Kaguyamon
trigger on deletion, digivolution-cost handling, On-Play selections) all looked
behaviorally correct against the printed card text.  The bugs are in the MCP /
LiveGame harness layer — they affect how callers (RL agents, scripted QA, MCP
clients) interact with the engine, not the rules implementation itself.

## Detailed Findings

### Issue 1: `step(action_id)` silently no-ops invalid action IDs
- **Severity**: critical
- **Category**: engine_api / harness
- **Component**: `digimon-engine-mcp::tool_step` → `LiveGame::step` (`code/digimon-engine/src/live_game.rs:528`)
- **Expected**: An invalid `action_id` should either return `{ok: false, error: "..."}` like `play()` does, or escalate as a JSON-RPC error.
- **Actual**: `step` always returns `{ok: true, error: null, events_emitted: [], new_phase: <unchanged>}` for any non-negative `action_id` that isn't legal for `current_decision_player()`. State is unchanged.

  ```rust
  // live_game.rs:528
  pub fn step(&mut self, action_id: u16) -> ActionResult {
      if self.game.game_over { return self.make_result(false, ...); }
      let pid = self.current_decision_player();
      let before_seq = self.game.event_seq;
      self.game.decode_action(action_id, pid);   // ← decode silently drops invalid IDs
      self.make_result(true, None, before_seq)   // ← always ok: true
  }
  ```
- **Reproduction** (`.claude/tmp/test_invalid_step.py`):
  ```
  >>> step(60) during Mulligan   → {'ok': True, 'events_emitted': [], state unchanged}
  >>> step(100) during Mulligan  → {'ok': True, 'events_emitted': [], state unchanged}
  >>> step(9999) during Mulligan → {'ok': True, 'events_emitted': [], state unchanged}
  ```
  Only `action_id: -1` triggers an error (JSON-RPC arg-validation, not engine).
- **Impact**:
  - RL agents that submit an action masked-illegal can't see that nothing happened — they keep the same observation, conclude "this action does nothing," and never learn the constraint.
  - MCP scripting clients that walk a recorded action log will see no error if the action stream goes out of sync with engine state.
  - Combined with Issue 2 below, an automated game-playing loop can get stuck choosing actions from a phantom legal-actions list, producing a tight no-op loop that's invisible to the caller.
- **Suggested fix**: Make `step` return `{ok: false, error: "action <id> not legal for player <pid> in phase <P>"}` when the action decoder rejects the ID (i.e., when no state advance + no events).

### Issue 2: `legal_actions(player)` returns actions for the wrong player
- **Severity**: high
- **Category**: engine_api / harness
- **Component**: `LiveGame::legal_actions` → `legal_decoded_actions(&game, player)` (`code/digimon-engine/src/live_game.rs:361`)
- **Expected**: `legal_actions(player)` should return an empty list when `player != current_decision_player()`. Equivalently, it should reflect what `step` would actually accept right now.
- **Actual**: Returns the actions the given player *would* have if it were their turn / their selection, ignoring the live phase ownership.
- **Reproduction** (`.claude/tmp/verify_illegal_digivolve.py`):
  ```
  Game state: T3 P1 Main mem=2
  P0's BR stack: Egg→Elizamon(Lv3)→Cyclonemon(Lv4)→Lamiamon(Lv5)
  P0 hand: [Medusamon, Medusamon, Elizamon, Dimetromon, Dimetromon, Owen Dreadnought]

  legal_actions(player=0) returns 9 actions:
    aid=0 play Medusamon
    aid=1 play Medusamon
    ...
    aid=414 digivolve hand 0 (Medusamon) onto breeding   ← it's P1's turn
    aid=429 digivolve hand 1 (Medusamon) onto breeding   ← also illegal
  ```
  Each of those `step`s returns `ok: true` with no state change (silent no-op via Issue 1).
- **Impact**:
  - Naive callers iterate "legal" actions assuming each one will fire — they don't.
  - In MCP-driven automated play, the script picks an action from the union of both players' "legal actions," lands on one for the wrong player, and gets a silent no-op forever (caught while writing `play_full_game.py`).
- **Suggested fix**: In `LiveGame::legal_actions`, gate on `player == self.current_decision_player()`. Alternative: return all valid actions but tag each with `ownership: ActionableNow | NotActiveNow` so callers can filter.

### Issue 3: `play(player, hand_idx)` bypasses mulligan-phase check
- **Severity**: high
- **Category**: engine_api / phase_validation
- **Component**: `digimon-engine-mcp::tool_play` → `LiveGame::play` (`code/digimon-engine/src/live_game.rs:541`)
- **Expected**: Playing a card during the Mulligan phase is rule-illegal (no main phase has begun, no card-play actions exist). `play()` should return `{ok: false, error: "..."}`.
- **Actual**: `play(player=0, hand_idx=0)` during `phase: Mulligan` *succeeds* — the card lands, On-Play effects trigger, and the game enters `SelectReveal`:
  ```
  Test 1: play(player=0, hand_idx=0) during Mulligan
    {'error': None, 'events_emitted':
      ['MemoryChange { seq: 0, player: 1, delta: -3, total: -3 }',
       'Play { seq: 1, player: 0, card_id: "BT21-008", field_index: 0 }'],
     'new_phase': 'SelectReveal',
     'ok': True,
     'pending_selection_after': {... Elizamon's "Add a Reptile/Dragonkin card to hand" prompt ...}}
  ```
- **Reproduction**: `.claude/tmp/test_play_validation.py`
- **Impact**: `play()` is documented as bypassing the action decoder "for direct, structured rejection of illegal indices" — but it skips phase validation too. Anyone using `play` as a debug or batch-driver tool can corrupt game state by playing before mulligan resolves.
- **Note**: The companion checks (`play(P0, hand_idx=99)` → "hand index 99 out of range"; `play(P0, hand_idx=0)` during P1 breeding → "insufficient memory, field full, or play blocked") do return structured errors, so the validation surface exists — just doesn't include the mulligan / non-main-phase check.
- **Suggested fix**: Reject `play()` when `current_decision_player()` is not the requested `player`, or when `current_phase` is not Main.

### Issue 4: `events_emitted` and `events()` return Rust-Debug-stringified events, not structured JSON
- **Severity**: medium
- **Category**: engine_api / observability
- **Component**: `LiveGame::make_result` (`code/digimon-engine/src/live_game.rs:625`); `tool_events`
- **Expected**: Events are structured data (`kind`, `player`, `delta`, `card_id`, etc.) so clients can `event["delta"]` without regex.
- **Actual**: Each event is `{"event": "MemoryChange { seq: 0, player: 0, delta: -3, total: 0 }"}` — Rust's `Debug` formatting of the enum variant, dropped verbatim into the JSON response. The same applies to the `events` tool's response.

  ```rust
  // live_game.rs:631
  let events_emitted: Vec<String> = events_since(&self.game, before_seq)
      .map(|e| format!("{:?}", e))     // ← Debug format → string
      .collect();
  ```
- **Reproduction** (`.claude/tmp/test_events.py`):
  ```
  events_emitted: [
    'MemoryChange { seq: 0, player: 0, delta: -3, total: 0 }',
    'Play { seq: 1, player: 0, card_id: "BT21-008", field_index: 0 }'
  ]
  ```
- **Impact**: Any QA / replay / training-inspection tool that wants to count "how many deletions happened this turn" or "what was the memory swing" has to regex-parse the Debug output. Defeats the point of MCP returning JSON.
- **Suggested fix**: `#[derive(Serialize)]` on `GameEvent` (or a per-variant `to_json()`) and emit the serialized form. Field shape is already implied by the Debug output; this is a one-pass conversion in `make_result` and `tool_events`.

### Issue 5: `end_turn()` during Mulligan silently fast-forwards state
- **Severity**: critical
- **Category**: engine_api / phase_validation
- **Component**: `LiveGame::end_turn` (`code/digimon-engine/src/live_game.rs:588`) → `Game::end_turn` (`code/digimon-engine/src/game_phases.rs:194`)
- **Expected**: `end_turn()` during Mulligan is not a legal engine action and should return `ok: false` with state unchanged.
- **Actual**: `end_turn` silently advanced `T0 Mulligan → T1 Breeding`. `Game::end_turn` forcibly sets `current_phase = GamePhase::EndTurn` regardless of starting phase, then runs the normal end-of-turn machinery (tick turn, switch player). The mulligan choice is bypassed entirely.
- **Reproduction** (`.claude/tmp/probe_siblings.py` Test 1):
  ```
  before: (0, 'Mulligan', 0, 0)
  after:  (1, 'Breeding', 0, 0)
  result: {'error': None, 'events_emitted': [], 'new_phase': 'Breeding', 'ok': True, ...}
  ```
- **Impact**: state corruption; opening hand bypasses mulligan; opponent's mulligan choice never offered. Any debug/QA harness that calls `end_turn` defensively (e.g., to recover from a stuck state) can permanently corrupt the game.

### Issue 6: `pass_turn()` during Mulligan silently returns ok:true
- **Severity**: high
- **Category**: engine_api / phase_validation
- **Component**: `LiveGame::pass_turn` (`code/digimon-engine/src/live_game.rs:598`)
- **Expected**: `pass_turn` during Mulligan should return `ok: false`.
- **Actual**: returns `ok: true` with no state change and no events. Mirrors Issue 1 (`step` no-op) — `pass_turn` calls `decode_action(PASS, _)` which silently drops in non-pass-legal phases.
- **Reproduction** (`.claude/tmp/probe_siblings.py` Test 2): `{'ok': True, 'events_emitted': [], 'new_phase': 'Mulligan'}` with state unchanged.

### Issue 7: `play()` does not check `current_decision_player()` — allows OUT-OF-TURN plays
- **Severity**: critical
- **Category**: engine_api / turn_validation
- **Component**: `LiveGame::play` (`code/digimon-engine/src/live_game.rs:541`)
- **Expected**: `play(P0, hand_idx)` during P1's Main phase should reject — P0 cannot play during P1's turn.
- **Actual**: `play(P0, 0)` succeeded during P1's Main. P0's hand[0] (Elizamon) landed on P0's field, the `[On Play]` effect triggered, and the engine entered `SelectReveal` waiting for P0 to resolve Elizamon's draw effect.
- **Reproduction** (`.claude/tmp/probe_siblings.py` Test 6):
  ```
  state before: (1, 'Main', 0, 0)  (P1's main, P0 shouldn't act)
  state after:  (1, 'SelectReveal', -3, 2)
  events: [MemoryChange(player=1, delta=-3), Play(player=0, card_id='BT24-008', field_index=0)]
  ```
- **Impact**: Significant state corruption — out-of-turn plays during gameplay. The check Issue 3 covers (Mulligan-phase plays) is just one specific case of this larger missing-validation problem.

### Issue 8: `digivolve` and `attack` MCP tools are spec'd but missing
- **Severity**: high
- **Category**: engine_api / missing_capability
- **Spec reference**: `live-game-surface:96-97` and `engine-debug-mcp:94-95` both list `digivolve(host, source_hand_idx, paid_costs?) -> ActionResult` and `attack(attacker, target) -> ActionResult`.
- **Actual**: Neither tool exists. `tools/list` returns 24 tools but `digivolve` and `attack` are absent. Callers must reverse-engineer the right `step(action_id)` from `legal_actions` for any digivolution or attack action.
- **Reproduction** (`.claude/tmp/probe_siblings.py` Test 7):
  ```
  has 'digivolve': False
  has 'attack':    False
  ```
- **Note**: `docs/DEBUG_MCP.md` acknowledges this as a v1 limitation ("No dedicated `digivolve`/`attack` tools. Use `step <action_id>`...") but the spec does not.

### Issue 9: Mandatory pending selection with no fulfillable option soft-locks the game
- **Severity**: high
- **Category**: engine_api / selection_handling
- **Source**: discovered during a follow-up DNA Omnimon vs BG Imperial scripted run (seed=1) at iter 70 (see `.claude/tmp/probe_trigger_loop.py`)
- **Expected**: When a mandatory pending selection has only target options that cannot be fulfilled (e.g., "Select 1 of your [Omnimon]-named Digimon to attack" but the player has no Omnimon-named Digimon), the engine should either skip the effect (fizzle) or expose a way to advance past the unfulfillable state.
- **Actual**: `pending_selection` returns `is_optional: false` with one option (`action_id: 102`, `label: "Select 1 of your [Omnimon]-named Digimon to attack a player"`). `legal_actions` returns only that action. `step(102)` is a true no-op (no events, no state change). Repeated invocations stay stuck — no pass action, no other legal target, no error.
- **Reproduction state**:
  ```
  pending: {
    kind: "OwnField",
    is_optional: false,
    options: [{action_id: 102, label: "Select 1 of your [Omnimon]-named Digimon to attack a player"}],
    source_kind: "Tamer",
    source_permanent: {index: 1, player: 0}  # BT17-081 Tai Kamiya & Matt Ishida
  }
  legal_actions(0): [{aid: 102, kind: "selection", ...}]
  step(102) → {ok: True, events_emitted: [], state unchanged}
  ```
- **Root cause has two contributors**:
  - **Engine soft-lock**: this is a worse-shape variant of Issue 1 (`step` silently no-ops). In Issue 1 the caller can recover by picking a different action; here `legal_actions` returns ONLY the unfulfillable one, so even a corrected caller has no escape.
  - **Card-faithfulness**: BT17-081's `[End of Your Turn] [Once Per Turn] 1 of your Digimon with [Omnimon] in its name may attack a player` uses "may" (optional). Engine has flagged the resulting selection as `is_optional: false` — that part is a card-script implementation bug and falls under the `/batch-fix-cards` lane, not this MCP/LiveGame proposal.
- **Impact on proposal**: This case adds a requirement to `enforce-live-game-action-contracts` — the engine must not surface a pending selection whose only options are unfulfillable. Either fizzle the effect (skip and clear pending), or always include a pass/decline alternative when no real target exists.

## OpenSpec Change

All eight issues are bundled into change `enforce-live-game-action-contracts`:
- Proposal: [openspec/changes/enforce-live-game-action-contracts/proposal.md](openspec/changes/enforce-live-game-action-contracts/proposal.md)
- Design: [openspec/changes/enforce-live-game-action-contracts/design.md](openspec/changes/enforce-live-game-action-contracts/design.md)
- Tasks (42 items, 7 sections): [openspec/changes/enforce-live-game-action-contracts/tasks.md](openspec/changes/enforce-live-game-action-contracts/tasks.md)
- Spec deltas: `live-game-surface` + `engine-debug-mcp`

## Cards / Behavior Verified PASS

Despite the harness issues above, the actual rules implementation behaved
correctly across everything I observed. The bugs are entirely in the
LiveGame/MCP surface, not in the engine's rules. Confirmed:

- **Digivolution cost** — Cyclonemon onto Elizamon paid 2, Lamiamon onto Cyclonemon paid 3, Medusamon onto Lamiamon paid 3 (all match printed evo costs)
- **Breeding-area effect suppression (rule 14-3)** — Medusamon BT24-017's `[When Digivolving]` did NOT fire when digivolved in breeding, as expected
- **Kaguyamon EX9-033** — `[All Turns][Once Per Turn] When other Digimon are deleted, delete 1 of your opponent's lowest level Digimon` triggered correctly when Karakurumon's end-of-turn deletion killed the Hanimon it had played
- **Karakurumon EX11-022** — `[On Play] You may play 1 [Puppet] trait Digimon card with 4000 DP or less from your hand or trash without paying the cost. At turn end, delete the Digimon this effect played.` — selection prompt was optional (is_optional: true), the played Hanimon was tagged for end-of-turn deletion, and the deletion fired as the turn ended
- **Hanimon EX9-024** — `[On Play] By trashing 1 card in your hand, you may return 1 Digimon card with the [Puppet] trait from your trash to the hand.` — observed in isolation (`.claude/tmp/replay_to_hanimon.py`) the trash→return chain decremented trash by 1 and incremented hand by 1, BA unchanged. Correct.
- **TriggerOrder selection (BT21-029 chained trigger)** — When multiple triggers queued for the same player, the engine surfaced a `TriggerOrder` pending selection asking which to resolve first, with both options labeled `(mandatory)`. Correct.

## Cards / Effects Not Covered
- Medusamon BT24-017's `[When Digivolving]` in **battle area** (we only saw it digivolve in breeding where effects are suppressed)
- BT21-029 Medusamon's `[All Turns]` Petrification Token trigger from deletion (couldn't isolate cleanly)
- Combat / Alliance / Blocker timing (the script never got far enough due to Issues 1+2 cascading)
- Security checks
- Tamer effects (Mirai Kinosaki, Owen Dreadnought)

## Areas Not Covered
- Multi-turn full-game flow past T6 (driver got stuck due to no-op + wrong-player legal-actions interaction)
- Surrender / game-over states
- `load_recording` workflow

## Suggested Next Steps
- Fix Issues 1+2 first; they block reliable scripted MCP play. The combination
  creates an invisible failure mode (silent no-ops on phantom legal actions)
  that breaks any QA / RL loop that walks `legal_actions` for both players.
- Issue 3 (`play` during mulligan) is harder to hit organically but lethal for
  any tool that uses `play` as a setup primitive.
- Issue 4 is a quality-of-life fix that would unlock straightforward
  programmatic QA assertions (`assert any(ev.kind == "Delete" for ev in events)`
  vs the current regex parse).

## Artifacts (under `.claude/tmp/`)
- `mcp_client.py` — minimal stdio JSON-RPC client for `digimon-engine-mcp`
- `game_session.py` — interactive Session wrapper around the MCP client
- `play_game.py` — original full-game driver (got stuck via Issues 1+2)
- `play_full_game.py` — driver with state-fingerprint no-op detector
- `replay_to_hanimon.py` — isolated Hanimon-return-puppet repro (PASSED)
- `verify_illegal_digivolve.py` — Issue 2 reproduction
- `test_invalid_step.py` — Issue 1 reproduction
- `test_play_validation.py` — Issue 3 reproduction
- `test_events.py` — Issue 4 reproduction
- `game_log.txt`, `full_game.txt`, `repro_log.txt` — captured run logs
