# Rust ↔ Python Engine Parity Tracker

**Purpose:** Catalog every known behavioral divergence between the Rust `digimon-engine` and the Python `digimon_gym/engine`, so that Phase 9 (PyO3 bindings) and any future cross-engine validation have a checklist to work against.

**Scope:** Semantic differences in game state evolution given identical inputs. Architectural differences (e.g. compile-time vs dynamic effect registration) are listed separately and are not bugs.

**Reading guide:**

- 🔴 **Parity-breaking** — given the same inputs, the two engines produce different game states. Must fix before claiming cross-engine correctness.
- 🟡 **Mask/tensor drift** — the model sees different observations or valid actions on at least one engine, but the game state itself can still evolve. Will degrade model transfer quality.
- 🟢 **Equivalent** — explicitly verified to match (recorded here so nobody re-investigates).
- ⚪ **By-design difference** — different implementations with the same observable outcome.

Each entry cites the canonical source lines so divergences can be rechecked after either engine evolves.

---

## 1. Core game flow

### 1.1 🔴 Play cost not deducted from memory

**Rust** — [game.rs:357-391](../digimon-engine/src/game.rs#L357) `play_from_hand` removes the card, creates a `Permanent`, fires `OnPlay`. It never calls `pay_memory`, so cards play for free.

**Python** — [player.py](../digimon_gym/engine/core/player.py) + game flow: `calculate_play_cost` resolves the effective cost (including reductions), the player's play path deducts it from memory before placement, and any OnPlay effects resolve after the card is on the field.

**Fix outline:** Before `player.hand.remove(hand_index)`, compute effective cost (base `play_cost` minus any applicable `BeforePayCost` reductions), call `pay_memory(cost)`. If `pay_memory` fails, return `None`. Move `fire_on_play` to run *after* memory is charged but *after* the permanent is on the field.

### 1.2 🔴 Memory reset on turn switch

**Rust** — [game.rs:236-246](../digimon-engine/src/game.rs#L236): both branches of the `if memory >= 0 / if memory <= 0` ladder clamp to 3. Overflow from over-cost plays is lost.

**Python** — [game/__init__.py:322](../digimon_gym/engine/game/__init__.py#L322) `self.memory = -self.memory`. No clamp. The seesaw is simply flipped from the next player's perspective.

**Consequence:** P0 ends their turn at −5 (spent beyond 0). Python gives P1 +5. Rust gives P1 +3. Tempo math off by 2 whenever a turn over-commits.

**Fix outline:** Replace the reset block with `self.memory = -self.memory;`. Remove the "at least 3" clamp entirely — that's not Digimon's rule.

### 1.3 🔴 `pass_turn` clobbers overflow

**Rust** — [game.rs:263-268](../digimon-engine/src/game.rs#L263): `self.memory = -3` unconditionally, then `end_turn()`.

**Python** — [game/__init__.py:329-334](../digimon_gym/engine/game/__init__.py#L329): `if self.memory >= 0: self.memory = -3`. Only forces the seesaw if the player still has memory to give. Over-cost plays that already put memory negative are preserved through the switch.

**Fix outline:** Gate the assignment: `if self.memory >= 0 { self.memory = -3; }`.

### 1.4 🔴 `pay_memory` auto-ends turn on crossing zero

**Rust** — [game.rs:279-282](../digimon-engine/src/game.rs#L279): if memory goes negative after paying, `end_turn()` fires inside `pay_memory`.

**Python** — never: turn end is checked in `check_turn_end` ([__init__.py:336](../digimon_gym/engine/game/__init__.py#L336)) after effect resolution, not synchronously with payment. This lets OnPlay, WhenDigivolving, etc. resolve on the same turn even if their cost already pushed memory negative.

**Fix outline:** Delete the `if self.memory < 0 { self.end_turn(); }` in `pay_memory`. Add a distinct `check_turn_end()` method that callers run after effect resolution completes.

### 1.5 🔴 Memory swing-back

**Python** — [game/__init__.py:276-280](../digimon_gym/engine/game/__init__.py#L276): if `OnEndTurn` effects restore memory from `< 0` back to `>= 0`, the turn continues and returns to Main. Real DCGO rule, used by some cards.

**Rust** — absent.

**Fix outline:** After firing `OnEndTurn` effects in `end_turn`, compare `memory_before` vs `memory` and short-circuit the turn switch if the sign flipped back. Gate behind `self.game_over == false`.

### 1.6 🔴 Mulligan phase

**Python** — [game/__init__.py:109-156](../digimon_gym/engine/game/__init__.py#L109): after randomizing turn player, each player chooses keep/mulligan once; security is laid *after* mulligan.

**Rust** — [game.rs:175-182](../digimon-engine/src/game.rs#L175) `start_game` jumps straight to turn 1. Mulligan is plumbed in the enum but never used.

**Fix outline:** Introduce `accept_mulligan(player, keep: bool)` action. Stay in `GamePhase::Mulligan` until every player has responded. Lay security after the mulligan resolves, mirroring `_finalize_opening_setup`.

### 1.7 🔴 First-turn draw semantics

**Python** — [game/__init__.py:228-230](../digimon_gym/engine/game/__init__.py#L228): hardcoded "no draw on turn 1 for anyone" — P0 and P1 both skip draw on their respective first turn.

**Rust** — [game.rs:196-211](../digimon-engine/src/game.rs#L196) with `SkipDraw::P1Only` skips draw only on turn 1 when `turn_player == 0`. P1's first draw (turn 2) is *not* skipped.

**Consequence:** Rust's P1 draws 1 extra card over the first two turns compared to Python's P1.

**Fix outline:** Change `SkipDraw::P1Only` to also skip on each player's first visit, not just absolute turn 1. Track a `has_drawn` flag per player, or compare `turn_count <= player_count`.

---

## 2. Combat & permanent state

### 2.1 🔴 Rush / Vortex summoning sickness exemption

**Python** — [permanent.py:405-407](../digimon_gym/engine/core/permanent.py#L405): a permanent with `_is_rush` or invoked with `is_vortex=True` can attack the turn it arrived.

**Rust** — [combat.rs:77-87](../digimon-engine/src/combat.rs#L77): if `turn_played == turn_count`, `can_attack` returns false. No keyword check.

**Fix outline:** After the summoning-sickness check, short-circuit with `|| self.modifiers.has_keyword(handle, Keyword::Rush)`. Accept a `vortex: bool` parameter on `can_attack` for vortex-triggered attacks.

### 2.2 🟡 `is_attacking` flag missing

**Python** — `permanent.is_attacking` is set to `True` at attack declare and cleared at attack end. Used by "Progress" (effect immunity while attacking) and observer effects.

**Rust** — no equivalent field on `Permanent`.

**Fix outline:** Add `pub is_attacking: bool` to `Permanent`. Set in `suspend_and_count_attack`, clear in an `end_attack` cleanup step alongside `modifiers.expire_end_of_attack()`.

### 2.3 🟡 Combat interrupt phases

**Python** — full interrupt state machine in [combat.py](../digimon_gym/engine/game/combat.py): Counter → Block → Alliance phases pause the attack flow, require defender input, and resume via `_continue_attack_*` helpers.

**Rust** — `combat.rs` is atomic. No counter digivolve, no blocker prompt, no alliance suspension.

**Status:** Known Phase 4 gap, [documented in RUST_ENGINE_API.md §9](RUST_ENGINE_API.md). Will require the selection/pending-action subsystem to fix.

### 2.4 🟡 Security Digimon tie rule

**Rust** — [combat.rs:234-247](../digimon-engine/src/combat.rs#L234): if attacker DP ≥ security Digimon DP, attacker survives (security is trashed).

**Python** — defers to `Player.security_attack` which returns an `AttackResolution`. Needs cross-check that ties favor the attacker identically.

**Verification needed:** write a test with equal-DP security and attacker, assert outcome matches Python's `AttackResolution::AttackerSurvives` / whatever it produces.

---

## 3. Tensor encoding (1375 floats)

### 3.1 🔴 Source DP contributions (slot offsets +8, +11, …)

**Python** — [permanent.py:755-774](../digimon_gym/engine/core/permanent.py#L755) `source_dp_contribution()` sums DP modifiers on each inherited source, gated by `can_use_condition`.

**Rust** — [tensor.rs:286](../digimon-engine/src/tensor.rs#L286) writes the raw card DP from `card_data`, no modifier lookup.

**Consequence:** Rust feeds different floats at these positions whenever a permanent has active DP-modifying sources (any archetype with stat boosts).

### 3.2 🔴 OPT state fields (slot offsets +3, +4)

**Python** — [tensor.py:158-159](../digimon_gym/engine/game/tensor.py#L158): `opt_total` and `opt_used` populate these slots.

**Rust** — [tensor.rs:270-271](../digimon-engine/src/tensor.rs#L270) hardcoded to `0.0`, comment: "deferred".

### 3.3 🟡 My face-down security visibility

**Python** — [tensor.py:178-183](../digimon_gym/engine/game/tensor.py#L178): only writes card IDs for positions in the `face_up_security` set; face-down stays 0.0.

**Rust** — [tensor.rs:118](../digimon-engine/src/tensor.rs#L118) writes every my-security card ID, ignoring face-down state.

### 3.4 🟡 Revealed cards section (slots 1360-1369)

**Python** — [tensor.py:104](../digimon_gym/engine/game/tensor.py#L104) populates from `game.revealed_cards`.

**Rust** — [tensor.rs:135-136](../digimon-engine/src/tensor.rs#L135) all zeros, marked "not yet implemented".

### 3.5 🟡 Selection context (slots 1371-1372)

**Python** — [tensor.py:108-120](../digimon_gym/engine/game/tensor.py#L108) writes phase value, valid_count, selecting_player if `pending_selection` is set.

**Rust** — [tensor.rs:139-141](../digimon-engine/src/tensor.rs#L139) writes only the phase value.

### 3.6 🟢 Verified equivalent

- Opponent face-down security ([tensor.rs:121-122](../digimon-engine/src/tensor.rs#L121)): both engines emit zeros.
- Global section [0-9]: turn_count/30, phase, memory/10.
- DP normalization constant: `DP_NORM = 30000.0` in both.
- Hand / trash / breeding / empty-slot encoding (0.0).
- `compute_positions()` card-vs-scalar split matches `tensor_layout.py`.

---

## 4. Action mask (2168 bits)

### 4.1 🟢 Verified equivalent

- All action range constants: PLAY_HAND (0-29), HAND_EFFECT (30-59), HATCH (60), MOVE_FROM_BREEDING (61), PASS (62), DNA_DIGIVOLVE (63-92), ATTACK (100-399), DIGIVOLVE (400-999), FIELD_EFFECT (1000-1149), TRASH_EFFECT (1150-1194), SOURCE_SELECT (2000-2167).
- `TARGETS_PER_ATTACKER = 15`, `FIELDS_PER_HAND = 15`, `SOURCES_PER_FIELD = 12`, `SECURITY_TARGET = 14`, `BREEDING_TARGET = 14`.
- Encode/decode formulas for attack, digivolve, field effect, source select.
- Total `ACTION_SPACE_SIZE = 2168`.

### 4.2 🔴 Option card color requirement

**Python** — [action_mask.py:77-99](../digimon_gym/engine/game/action_mask.py#L77): an Option card is only playable if the player has a Digimon of a matching color on the field (unless `IGNORE_COLOR_REQUIREMENT` is active).

**Rust** — [mask.rs:53](../digimon-engine/src/action/mask.rs#L53) doesn't check; comment marks this as "deferred to effect system".

**Consequence:** a Python-trained policy will sample Option plays that Rust's decoder will attempt. If the engine ever enforces the rule, the action silently fails; if not, games diverge from Python's.

### 4.3 🔴 Blitz attack exception

**Python** — [action_mask.py:107-114](../digimon_gym/engine/game/action_mask.py#L107): with memory < 0, a Blitz Digimon that digivolved this turn can still attack.

**Rust** — [mask.rs:60](../digimon-engine/src/action/mask.rs#L60) blocks all attacks when memory < 0.

### 4.4 🔴 Raid target rule

**Python** — [action_mask.py:142-148](../digimon_gym/engine/game/action_mask.py#L142): a Raid Digimon can attack the highest-DP unsuspended opponent.

**Rust** — [mask.rs:78](../digimon-engine/src/action/mask.rs#L78) filters to suspended targets only.

### 4.5 🔴 Entire action categories ungenerated

Rust's mask always emits 0.0 for:
- Hand effects (30-59) — [action_mask.py:176-185](../digimon_gym/engine/game/action_mask.py#L176) in Python
- DNA digivolve (63-92) — [action_mask.py:168-174](../digimon_gym/engine/game/action_mask.py#L168)
- Field effects (1000-1149) — [action_mask.py:201-214](../digimon_gym/engine/game/action_mask.py#L201)
- Trash effects (1150-1194) — [action_mask.py:216-225](../digimon_gym/engine/game/action_mask.py#L216)

These require the full effect-listing machinery (iterating permanent `effect_list()` by timing). Rust has the `Effect` struct but no equivalent "list all activatable effects" query yet.

### 4.6 🔴 Interrupt-phase mask coverage

**Rust** — [mask.rs:130](../digimon-engine/src/action/mask.rs#L130) default arm returns `mask[PASS] = 1.0` for every non-Main/Breeding/Mulligan phase.

**Python** has dedicated builders for:
- `BlockTiming` — which permanents can declare blocker
- `CounterTiming` — valid blast digivolve hand/field pairs
- `AllianceTiming` — which unsuspended allies can suspend for alliance
- `SelectTarget` / `SelectHand` / `SelectMaterial` / `SelectTrash` / `SelectSource` / `SelectReveal` / `SelectSecurity`
- `EffectChoice`
- `EndOfTurnAction` — Vortex, Overclock, MAY_ATTACK, FORCE_ATTACK

All are 0.0 in Rust except PASS. Model samples PASS universally in these phases.

### 4.7 🟡 Modifier-gated mask checks

Python checks these modifiers per-action; Rust does not:
- `CANNOT_ATTACK_TARGET` (per attacker-target pair)
- `CANNOT_DIGIVOLVE`
- `CANNOT_PLAY_FROM_HAND`
- `FORCE_ATTACK` (restricts mask to forced Digimon only)
- `DigiXros` cost-reduction optimistic calculation

---

## 5. Registry parity

### 5.1 🟢 CardRegistry

Fixed in [card_registry.rs](../digimon-engine/src/card_registry.rs). `CardData.index` from cards.json is the source of truth in both engines. Verified by [card_registry_parity.rs](../digimon-engine/tests/card_registry_parity.rs) against the real 4082-card cards.json.

### 5.2 ⚪ Effect registration strategy

- **Python:** `importlib.import_module` at `CardDatabase` load, keyed by `card_effect_class_name`.
- **Rust:** static `Arc<dyn CardEffect>` in `CardEffectRegistry`, wired at compile time.

Different by design. Both produce the same `card_id → effect` mapping observably; the trade-off is ergonomics (Python) vs. compile-time safety + performance (Rust). See [RUST_ENGINE_API.md §11](RUST_ENGINE_API.md).

---

## 6. Architectural differences (not bugs)

| Topic | Python | Rust | Notes |
|-------|--------|------|-------|
| Granted keywords / DP buffs storage | Fields on `Permanent` (`_granted_keywords`, `_dp_modifiers`) | External `ModifierRegistry` keyed by `PermanentHandle` | Observably equivalent once modifiers applied |
| Multiplayer deck-out | 2-player only — declares opponent winner | N-player — eliminates one player, continues | Rust richer |
| `EndOfOpponentsTurn` expiry | `modifiers.clear_opponent_turn_expiry(turn_player)` | `ModifierRegistry::expire_end_of_turn` filters by `source_player != ending_player` | Equivalent |
| Rules presets | Constants and conditionals | `Rules` struct with `standard/edh/titan_boss/titan_team()` | Rust exposes EDH/Titan cleanly |

---

## 7. Recommended fix order

Phase 9 (PyO3 bindings) readiness requires, in priority order:

1. **§1.1 — Deduct play cost** (single biggest correctness bug; every play is free today).
2. **§1.2 / §1.3 / §1.4 / §1.5 — Memory seesaw semantics** (tight cluster; probably ~30 lines of changes in `game.rs` plus tests).
3. **§2.1 — Rush exemption** (needed for any card with Rush; trivial once modifier lookup is in place).
4. **§1.7 — First-turn draw rule** (small, affects tempo analysis).
5. **§1.6 — Mulligan flow** (scoped change: new action, phase transition).
6. **§3.1 / §3.2 — Tensor source-DP + OPT slots** (blocks model transfer; requires Effect iteration to sum modifiers).
7. **§4.2 / §4.3 / §4.4 — Action mask main-phase parity** (Option color, Blitz, Raid).
8. **§4.5 / §4.6 — Mask phase coverage** (hand/field/trash effects + interrupt phases; depends on the effect-listing query).

The rest (combat interrupts §2.3, face-down security §3.3, etc.) can follow as cards that need them get implemented.

---

## 8. Test strategy

For each 🔴 item, we want a paired test:

1. **Snapshot test** — construct an identical game state in both Python and Rust, step the same action, diff `to_dict()` / `to_json()`. Should be runnable from the workspace root.
2. **Unit test** — a Rust-only behavioral test in `digimon-engine/tests/` that locks the correct semantics in place after the fix.

`digimon-engine/tests/card_registry_parity.rs` is the template — it loads Python's authoritative data and asserts the Rust implementation agrees.

For the memory tests specifically, a good integration test is:
```
P0 starts with 3 memory.
P0 plays two 3-cost cards (second goes into negative).
P0 passes.
P1's memory should be +3 (Python) — this will fail in Rust today until §1.2/§1.3 land.
```
