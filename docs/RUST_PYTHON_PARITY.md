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

### 1.6 🟢 Mulligan phase — implemented

**Python** — [game/__init__.py:109-156](../digimon_gym/engine/game/__init__.py#L109): after randomizing turn player, each player chooses keep/mulligan once; security is laid *after* mulligan.

**Rust** — [game.rs](../digimon-engine/src/game.rs) `Game::new` now shuffles `turn_order` via the seeded rng (first-player coin flip), draws opening hands, and initializes `mulligan_pending`/`mulligan_used`. `accept_mulligan(player, keep)` drives the state machine; the last decision triggers `finalize_mulligan`, which lays security and begins turn 1. `start_game` auto-keeps for every pending player, preserving backward compatibility with callers that don't care about mulligan.

**Action mask** — [mask.rs](../digimon-engine/src/action/mask.rs): during `GamePhase::Mulligan`, only the current decider sees any non-zero bits. Bit 0 (keep) is always set; bit 1 (mulligan) is suppressed after `mulligan_used[decider]` is true.

**Tauri surface** — `rust_mulligan_decide(keep)` command + `mulligan_current_player` / `mulligan_used` fields on `GameStateDto`. TypeScript adapter at [frontend/src/api/rustEngine.ts](../frontend/src/api/rustEngine.ts).

**Coverage:** [tests/mulligan.rs](../digimon-engine/tests/mulligan.rs) (new) and first-player draw skip regression in [tests/first_turn_draw.rs](../digimon-engine/tests/first_turn_draw.rs).

### 1.7 🟢 First-turn draw semantics — verified equivalent

**Python** — [game/__init__.py:228-230](../digimon_gym/engine/game/__init__.py#L228) `phase_draw`: `if self.turn_count == 1: pass`. Since `switch_turn` increments `turn_count`, P0's first turn is turn 1 and P1's first turn is turn 2. **Only P0 skips** — matches the standard Digimon TCG rule.

**Rust** — [game.rs](../digimon-engine/src/game.rs) with `SkipDraw::FirstPlayerOnly` (renamed from the misleading `P1Only`): skips draw when `turn_count == 1 && turn_player == 0`. Same behavior.

**Previous audit was wrong** — an earlier pass over this file reported a divergence that doesn't actually exist. Keeping the entry here to prevent a future auditor from reproducing the same mistake.

**Coverage:** [tests/first_turn_draw.rs](../digimon-engine/tests/first_turn_draw.rs) locks in the P0-skips / P1-draws-on-turn-2 / P0-draws-on-turn-3 rule.

---

## 2. Combat & permanent state

### 2.1 🟢 Rush / Vortex summoning-sickness exemption — implemented

**Python** — [permanent.py:404-407](../digimon_gym/engine/core/permanent.py#L404): a permanent with `_is_rush` or invoked with `is_vortex=True` can attack the turn it arrived.

**Rust** — [combat.rs](../digimon-engine/src/combat.rs) `can_attack(handle, vortex)`, `attack_digimon(…, vortex)`, `attack_player(…, vortex)` all carry the `vortex: bool` flag. Summoning sickness short-circuits when `vortex` is true *or* `modifiers.has_keyword(handle, Keyword::Rush)` is true. The mask helper `can_basic_attack` in [mask.rs](../digimon-engine/src/action/mask.rs) checks modifier-granted Rush so the Main-phase mask agrees with the engine.

**Coverage:** [tests/rush_exemption.rs](../digimon-engine/tests/rush_exemption.rs) — `freshly_played_without_rush_cannot_attack`, `freshly_played_with_rush_can_attack`, `rush_does_not_override_suspended_state`, `freshly_played_with_vortex_can_attack`, `vortex_does_not_override_suspended_state`, `mask_allows_rush_granted_attack_on_turn_played`.

### 2.1b 🟡 Native (card-text) Rush not parsed

Rust's [CardData](../digimon-engine/src/card_data.rs#L18) has no `keywords: Vec<Keyword>` field — static card keywords live inside `effect_text: String`. Cards that print Rush on their face don't trigger the exemption in `can_attack` because there's nothing to inspect. Fix requires either a keyword-parsing pass over `effect_text` or an explicit `keywords` field on `CardData` + a migration of cards.json. Track with §4.5 effect-listing work.

### 2.2 🟢 `is_attacking` flag — implemented

**Python** — `permanent.is_attacking` is set to `True` at attack declare and cleared at attack end. Used by "Progress" (effect immunity while attacking) and observer effects.

**Rust** — `pub is_attacking: bool` field on [Permanent](../digimon-engine/src/permanent.rs). Set by `begin_attack` right after `PendingAttack` is installed; cleared by `cleanup_attack` alongside `modifiers.expire_end_of_attack()`.

**Coverage:** [tests/block_interrupt.rs](../digimon-engine/tests/block_interrupt.rs) `is_attacking_flag_lifecycle` — verifies the flag is live while the attack is parked on BlockTiming and cleared after resolution.

### 2.3 🟢 Combat interrupt phases — implemented

**Python** — full interrupt state machine in [combat.py](../digimon_gym/engine/game/combat.py): Counter → Block → Alliance phases pause the attack flow, require defender input, and resume via `_continue_attack_*` helpers.

**Rust** — [combat.rs](../digimon-engine/src/combat.rs) is a state machine: `attack_digimon`/`attack_player` are wrappers over `begin_attack(attacker, AttackTarget, vortex)`, which installs `PendingAttack` and calls `advance_pending_attack`. The state progression is `Declared → AllianceOpen → CounterOpen → BlockOpen → Battle → Cleanup`, pausing on a `PendingSelection` at each open-window state that has candidates.

- **Alliance** ✅ implemented. `try_enter_alliance` scans the attacker's side for unsuspended allies with modifier-granted `Keyword::Alliance`. Declaration grants attacker +ally_dp and +1 security attack (both EndOfAttack expiry) and suspends the ally. Trait-matching refinement (Alliance only fires when ally shares a trait with attacker) is blocked on the trait-parsing infrastructure noted in §2.1b.
- **Counter** ✅ implemented. `try_enter_counter` scans the defender's hand for cards whose effects set `blast_digivolve = true` and pairs each against valid field-digivolve targets via `Game::can_digivolve`. Declaration stacks the card onto the target (zero memory), fires `WhenDigivolving` via the effect queue, then advances to BlockOpen — unless the attacker was deleted mid-counter, in which case the state machine skips to Cleanup (matches DCGO `AttackProcess.cs:301`). **Digimon-target attacks only** — matches Python `combat.py:139`, which scopes Counter to Digimon targets. `OnCounterTiming` (Python's pre-WhenDigivolving counter-specific trigger) is intentionally deferred — no pilot card uses it yet.
- **Block** ✅ implemented. `try_enter_block` scans defender's battle area for unsuspended Blocker-keyword Digimon. Declaration rewrites `effective_target` to the blocker; `resolve_pending_battle` reads `effective_target`, so the redirect works for both Digimon and Player attacks (a blocker on a player attack cancels the security loop and runs a Digimon battle against the blocker instead). **Collision** (attacker-side keyword) expands the candidate pool — when the attacker has `Keyword::Collision`, every unsuspended opponent Digimon is treated as having Blocker for this attack, matching Python `permanent.py::can_be_blocker:502`.
- **Vortex** short-circuits directly to Battle after OnAttack (skips every interrupt window — matches DCGO).

New return variant `AttackResult::InProgress` signals that an attack is parked on a `PendingSelection`; the terminal outcome arrives once the resolution chain completes.

**Coverage:** [tests/block_interrupt.rs](../digimon-engine/tests/block_interrupt.rs) (10 cases); [tests/alliance_interrupt.rs](../digimon-engine/tests/alliance_interrupt.rs) (7 cases); [tests/counter_interrupt.rs](../digimon-engine/tests/counter_interrupt.rs) (12 cases: no-candidates baseline, invalid-pairing skip, prompt install, mask rendering, decline, declaration + stack growth + `WhenDigivolving` firing, attacker-delete cascade to Cleanup, Vortex bypass, Counter → Block sequence, player-target attack skips Counter, wrong-player rejection, encode/decode round-trip).

### 2.3-residual 🟡 `OnCounterTiming` distinct timing

Python's `_decode_counter` ([action_decoder.py:268-269](../digimon_gym/engine/game/action_decoder.py#L268)) fires `OnCounterTiming` *before* `WhenDigivolving`. Rust only fires `WhenDigivolving` today. Adding the distinct timing is a small surface change (new `EffectTiming::OnCounterTiming` variant + one `enqueue_triggered` call in `execute_blast_digivolve`) — deferred until the first card script actually uses it.

### 2.4 🟡 Security Digimon tie rule

**Rust** — [combat.rs:234-247](../digimon-engine/src/combat.rs#L234): if attacker DP ≥ security Digimon DP, attacker survives (security is trashed).

**Python** — defers to `Player.security_attack` which returns an `AttackResolution`. Needs cross-check that ties favor the attacker identically.

**Verification needed:** write a test with equal-DP security and attacker, assert outcome matches Python's `AttackResolution::AttackerSurvives` / whatever it produces.

---

## 3. Tensor encoding (1375 floats)

### 3.1 🟢 Source DP contributions — implemented

**Python** — [permanent.py:755-774](../digimon_gym/engine/core/permanent.py#L755) `source_dp_contribution()` sums DP modifiers on each inherited source, gated by `can_use_condition`.

**Rust** — `Game::source_dp_contribution(perm, source_index)` ([game.rs](../digimon-engine/src/game.rs)) mirrors the Python impl: iterates the single source's effects via `CardEffectRegistry`, applies the inherited-vs-top filter (`is_under == effect.inherited`), and evaluates each effect's condition via a read-only `EffectReadContext`. The tensor writes `source_dp_contribution / DP_NORM` at per-source offset +2 ([tensor.rs `write_slot`](../digimon-engine/src/tensor.rs)).

**Coverage:** [tests/tensor_helpers.rs](../digimon-engine/tests/tensor_helpers.rs) unit-tests the helper; [tests/tensor_source_contributions.rs](../digimon-engine/tests/tensor_source_contributions.rs) drives through `build_tensor` end-to-end including the digivolution-stack and memory-gated cases.

**Residual gap §3.1b:** linked-card effects are still not iterated — if a card's `dp_modifier` lives on a linked Option, it won't contribute. No current archetype needs this; will flag if it arises.

### 3.2 🟢 OPT state fields — implemented

**Python** — [tensor.py:158-159](../digimon_gym/engine/game/tensor.py#L158): `opt_total` and `opt_used` populate slot offsets +3/+4; `source_opt_state(src)` at each source's +1.

**Rust** — `Game::opt_total / opt_used / source_opt_state` ([game.rs](../digimon-engine/src/game.rs)) count effects with `max_per_turn > 0` across the permanent's stack with the same inherited/top filter, consulting `Permanent::effect_activations` to determine which have reached their cap this turn. Counters reset in `Permanent::new_turn` (via `Player::new_turn` during `begin_turn`). Tensor offsets +3/+4 write the raw counts, and per-source +1 writes the availability fraction — matching Python's `build_board_state_tensor`.

**Coverage:** Same tests as §3.1.

### 3.3 🟢 Face-down security visibility — implemented

**Python** — [tensor.py:178-183](../digimon_gym/engine/game/tensor.py#L178): only writes card IDs for positions in the `face_up_security` set; face-down stays 0.0.

**Rust** — [player.rs](../digimon-engine/src/player.rs) now carries `face_up_security: HashSet<u16>` (keyed by `CardSource.card_index`). The new `write_security_ids` helper in [tensor.rs](../digimon-engine/src/tensor.rs) mirrors Python's writer — face-down slots stay 0.0; only `card_index`es present in the set emit their registry index. Applied to both `OFF_MY_SECURITY` and `OFF_OPP_SECURITY` so cross-player reveal effects have a symmetric slot.

**Previous behavior was a hidden-info leak** — Rust wrote every my-security card ID, so an RL agent trained on the Rust tensor could "peek" at its own face-down security stack and play perfectly around its own security effects.

**Coverage:** [tests/tensor_hidden_info.rs](../digimon-engine/tests/tensor_hidden_info.rs) — `my_security_is_zero_by_default`, `opp_security_is_zero_by_default`, `my_security_visible_when_face_up`.

### 3.4 🟢 Revealed cards section (slots 1360-1369) — implemented

**Python** — [tensor.py:104](../digimon_gym/engine/game/tensor.py#L104) populates from `game.revealed_cards`.

**Rust** — `Game::revealed_cards: Vec<CardSource>` field ([game.rs](../digimon-engine/src/game.rs)) feeds the `OFF_REVEALED` slot via the existing `write_card_ids` helper. Cleared in `rotate_turn_player` so reveals don't leak across turns. No card effects populate the vec yet, but the scaffold is in place for reveal-from-deck / search effects.

**Coverage:** [tests/tensor_hidden_info.rs](../digimon-engine/tests/tensor_hidden_info.rs) — `revealed_cards_populates_offset`, `revealed_cards_cleared_on_turn_rotation`.

### 3.5 🟢 Selection context (slots 1371-1372) — implemented

**Python** — [tensor.py:108-120](../digimon_gym/engine/game/tensor.py#L108) writes phase value, valid_count, selecting_player if `pending_selection` is set.

**Rust** — [tensor.rs](../digimon-engine/src/tensor.rs) writes phase value at slot 1370 whenever the engine is in a selection / combat-interrupt phase; writes `valid_action_ids.len() / ACTION_SPACE_SIZE` at slot 1371 and `selecting_player` at slot 1372 whenever `pending_selection.is_some()` (covers both selection-phase parks and `TriggerOrder` prompts parked under `EffectChoice`).

**Coverage:** [tests/select_opponent_permanent.rs](../digimon-engine/tests/select_opponent_permanent.rs) `tensor_reports_valid_count_and_selecting_player`.

### 3.6 🟢 Verified equivalent

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

### 4.2 🟢 Option card color requirement — implemented

**Python** — [action_mask.py:77-99](../digimon_gym/engine/game/action_mask.py#L77): an Option card is only playable if the player has a matching-color Digimon or Tamer on field or a matching-color Digimon in breeding.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Play-cards loop calls `option_color_match_available(card, me, &card_data)`, which iterates the player's battle_area + breeding_area for a color-set intersection with the Option's colors.

**Coverage:** [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs) — `mask_option_requires_matching_color_on_field` (walks empty → wrong-color → matching-color), `mask_option_color_check_accepts_tamer`.

### 4.2b 🟡 Script-based color bypasses not yet honored

Python cards can set `card._match_color_requirement = False` or register a `_match_color_requirement_fn` callback (~10 cards, e.g. `ex1_071`, `lm_050`, `st20_15`). Rust's `CardData` has no such field and no scripting infra — these Options will be *over*-masked (Rust refuses to play them when Python would allow). Similarly, Python's `IGNORE_COLOR_REQUIREMENT` aura modifier has no `ModifierType` variant in Rust yet. Both await §4.5 effect-listing / card-scripting infra.

### 4.3 🟢 Blitz attack exception under negative memory — implemented

**Python** — [action_mask.py:107-114](../digimon_gym/engine/game/action_mask.py#L107): with `memory < 0`, a Blitz Digimon that digivolved this turn can still attack.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs): the memory gate is per-attacker. `memory_ok = memory >= 0 || (turn_digivolved == turn_count && modifiers.has_keyword(handle, Keyword::Blitz))`.

**Coverage:** [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs) — `mask_blitz_can_attack_under_negative_memory`, `mask_blitz_without_digivolving_does_not_attack_under_negative_memory`.

### 4.3b 🟡 Native / static Blitz not parsed

Same pattern as §2.1b — only modifier-granted Blitz is honored because `CardData` has no `keywords` field. Native Blitz printed on a card's face awaits §4.5 effect-listing / keyword-parsing infra.

### 4.4 🟢 Raid target rule — implemented

**Python** — [action_mask.py:121-140](../digimon_gym/engine/game/action_mask.py#L121): unsuspended enemies are targetable if attacker has `CAN_ATTACK_UNSUSPENDED` (any unsuspended) or Raid (tied-for-highest-DP unsuspended).

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs): target loop precomputes the max effective DP across unsuspended enemy Digimon and emits attack bits for each target whose `effective_dp` equals the max under Raid; emits for every unsuspended target under `ModifierType::CanAttackUnsuspended`. DP tiebreak uses `Game::effective_dp` so `ChangeDp` modifiers are honored (slight improvement over Python's raw `.dp`).

**Coverage:** [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs) — `mask_raid_targets_highest_dp_unsuspended`, `mask_raid_allows_all_tied_for_highest`, `mask_can_attack_unsuspended_modifier_allows_all_unsuspended`.

### 4.5 🟡 Entire action categories ungenerated — partial

DNA digivolve plumbing has landed; Hand/Field/Trash `[Main]` effect masks remain blocked on effect-listing infrastructure.

### 4.5a 🟢 DNA digivolve mask — implemented

**Python** — [action_mask.py:161-166](../digimon_gym/engine/game/action_mask.py#L161): `if card.is_digimon and has_valid_dna_targets(card, me.battle_area): mask[63 + h] = 1.0`.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) `GamePhase::Main` arm emits `DNA_DIGIVOLVE_START + hand_index` when the hand card's `CardData.dna_costs` is non-empty and [`dna_digivolve::has_valid_dna_targets`](../digimon-engine/src/dna_digivolve.rs) finds some pair of battle-area permanents satisfying any `DnaCost` entry in either ordering. Memory cost is NOT gated at mask-generation time — Python's `action_mask.py:161-166` emits the bit regardless of memory and defers the cost check to action execution. `text_contains` searches the concatenation of `effect_text + inherited_text + security_text` to match Python's `_perm_matches_dna_req`.

**Coverage:** [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs) — `mask_dna_digivolve_emits_when_valid_pair_exists`, `mask_dna_digivolve_accepts_either_ordering`, `mask_dna_digivolve_rejects_when_no_pair`, `mask_dna_digivolve_respects_memory_cost`, `mask_dna_digivolve_skips_cards_without_dna_costs`.

### 4.5b 🟡 `dna_costs` data-population pipeline

`CardData.dna_costs` is present and deserialized, but cards.json's ingest pipeline doesn't emit the field today. Every card loaded from production data has `dna_costs = []`, so the mask branch above never fires in actual games. Python populates DNA costs from per-card scripts; Rust needs the cross-language export pipeline to emit DNA costs (or an auxiliary `dna_costs.json` sidecar) before this work is meaningful at runtime.

### 4.5c 🟢 Hand / Field / Trash `[Main]` effect masks — implemented

**Python** — [action_mask.py:176-225](../digimon_gym/engine/game/action_mask.py#L176): iterates a card's effects and filters by `_is_hand_main` / `_is_field_main` / `_is_trash_main` bool flags.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Main-phase arm emits bits for three new zone-scoped timing variants:
- `EffectTiming::MainFromHand` → bits `HAND_EFFECT_START + h` (30-59)
- `EffectTiming::MainOnField` → bits `FIELD_EFFECT_START + i * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN` (1000-1149, sub-slot +2)
- `EffectTiming::MainFromTrash` → bits `TRASH_EFFECT_START + t` (1150-1194)

The effect-listing primitive is [`Game::effects_for_card(card_id, handle)`](../digimon-engine/src/game.rs) — analogous to Python's `CardSource.effect_list(timing)` but expressed Rust-idiomatically with the registry owned by `Game`. Callers filter on `effect.timing`. This primitive also unblocks §2.1b / §4.2b / §4.3b (native static keyword parsing) and §4.7e (DigiXros cost-reduction).

**Field [Main]** additionally enforces OPT via the existing per-permanent `activation_count((source_handle, slot))` map and applies the same `inherited == is_under` filter used by `source_dp_contribution` / `source_opt_state`, so the mask agrees with the tensor helpers.

**Coverage:** [tests/mask_main_effects_parity.rs](../digimon-engine/tests/mask_main_effects_parity.rs) — 12 cases across the three zones: emit/suppress by condition, first-match-wins per slot, inherited-only-when-under, OPT exhaustion via `record_activation`, and phase gating (all three bits stay 0 outside Main).

### 4.5c-residual 🟡 Hand / Trash per-turn activation counters

Rust (like Python's mask) does NOT currently track `_turn_activate_count` for Hand / Trash [Main] effects at mask-generation time — the effect's `can_use_condition` closure is the sole gate. When execution-side support lands (firing these activated actions and recording activation), we'll revisit whether to add a parallel activation map on `Player` keyed by `(CardHandle, slot)`. Field [Main] already uses `Permanent::effect_activations`.

### 4.5c-residual 🟢 Action execution for [Main] bits — implemented

**Rust** — [game.rs](../digimon-engine/src/game.rs) `Game::activate_hand_main(player, hand_index)`, `Game::activate_field_main(player, field_index)`, `Game::activate_trash_main(player, trash_index)` each walk the card's / permanent's effects in the same order the mask emits, apply the same condition / inherited / OPT filters, and fire the first match. Memory cost, card movement, and all other side effects are handled inside the effect's `process` closure — mirroring Python's `_execute_*_main_effect` (no upfront `pay_memory` call, matching Python's inline `player.add_memory(-cost)` model).

**Field activation recording:** `activate_field_main` calls `perm.record_activation(source_handle, slot as u8)` before invoking the process closure, using the same `(CardHandle, slot)` key the mask inspects via `perm.activation_count`. Mask ↔ decoder agreement is verified by a regression test (`mask_and_field_decoder_agree_on_opt_exhaustion`).

**Hand/Trash activation counters:** intentionally omitted. See §4.5c-residual 🟡 below — Python's mask doesn't gate on `_turn_activate_count` either, and the execution-side counter is a separate architectural item worth its own plan.

**Coverage:** [tests/action_main_effects_parity.rs](../digimon-engine/tests/action_main_effects_parity.rs) — 14 cases: fires / suppressions per zone (condition gate, OOB index, wrong timing), Field OPT exhaustion, Field inherited-filter, mask ↔ decoder consistency for both Field (OPT-aware) and Hand (no OPT).

### 4.6 🟡 Interrupt-phase mask coverage — partial

End-of-turn surface is complete for mask parity — Vortex (§4.6a) + Overclock/MayAttack/ForceAttack (§4.6c) emission, plus phase transition (§4.6b) and `pass_end_of_turn_action` resumption. Overclock sacrifice *execution* landed with §4.6c-residual. Combat interrupts (§4.6d) support Alliance, Counter, and Block; selection helpers now cover `SelectTarget` / `SelectHand` / `SelectTrash` / `SelectMaterial` / `EffectChoice` / `TriggerOrder`. Remaining per-effect selection kinds (`SelectReveal` / `SelectSecurity` / `SelectSource`) track as §4.6d-residual follow-up work.

### 4.6a 🟢 Vortex mask emission — implemented

**Python** — [action_mask.py:321-335](../digimon_gym/engine/game/action_mask.py#L321): during `GamePhase.EndOfTurnAction`, permanents with `_is_vortex` and a passing `can_attack(is_vortex=True)` emit attack bits against any enemy Digimon.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) `GamePhase::EndOfTurnAction` arm: mirrors Python via `modifiers.has_keyword(handle, Keyword::Vortex)` + `can_attack(handle, /* vortex = */ true)`. Any enemy Digimon (suspended or not) is a valid target.

**Coverage:** [tests/mask_end_of_turn_parity.rs](../digimon-engine/tests/mask_end_of_turn_parity.rs) — `mask_vortex_emits_attacks_in_end_of_turn_phase`, `mask_vortex_without_keyword_only_emits_pass`, `mask_vortex_bypasses_summoning_sickness`, `mask_vortex_targets_unsuspended_digimon_too`.

### 4.6b 🟢 Phase transition into `EndOfTurnAction` — implemented

**Python** — [game/__init__.py:294-324](../digimon_gym/engine/game/__init__.py#L294) `_complete_end_phase` parks in `GamePhase.EndOfTurnAction` when `_has_end_of_turn_keywords` returns true (Vortex / Overclock w/ sacrifice / MayAttack). Turn rotation defers until the player calls `next_phase` via PASS action 62.

**Rust** — [game.rs](../digimon-engine/src/game.rs) `Game::end_turn` mirrors the Python flow: fire OnEndTurn effects → swing-back check → `has_end_of_turn_keywords` → park in `EndOfTurnAction` or fall through to `rotate_turn_player`. `Game::pass_end_of_turn_action` resumes rotation. The turn-rotation tail of the old `end_turn` is extracted into a private `rotate_turn_player(ending_player)` helper so the resume path doesn't re-evaluate the EOT keyword check. `ModifierType::ForceAttack` is intentionally excluded from the EOT-park check (matches Python) — it's enforced Main-phase by §4.7d.

**Coverage:** [tests/end_turn_phase_transition.rs](../digimon-engine/tests/end_turn_phase_transition.rs) — 9 cases covering Vortex/Overclock/MayAttack parking, sacrifice-availability gating, suspended-MayAttack no-park, swing-back short-circuit, rotation resumption, and EOT-modifier expiry on resume.

### 4.6b-residual 🟡 Token detection

Rust's `CardKind` has no `Token` variant (tokens are registered as `Digimon`-kind via `token_registry`). Python's Overclock-sacrifice check is `p.is_token or p.is_digimon`; Rust collapses it to `is_digimon` alone. No observable gap today because token registrations produce `CardKind::Digimon` anyway, but promoting Token to a first-class kind will be needed if a card ever introduces Token-specific sacrifice restrictions.

### 4.6c 🟢 Overclock / MAY_ATTACK / FORCE_ATTACK mask bits — implemented

**Python** — [action_mask.py:354-389](../digimon_gym/engine/game/action_mask.py#L354): EndOfTurnAction branch emits Overclock at `1000 + i * EFFECTS_PER_PERM + 0`, MAY_ATTACK and FORCE_ATTACK attacks at `100 + i * TARGETS_PER_ATTACKER + j` (shared with normal attack range).

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) `GamePhase::EndOfTurnAction` arm folds Vortex/MayAttack/ForceAttack into a single target-loop shared with the §4.7a `CannotAttackTarget` filter; Vortex uses `can_attack(vortex=true)` and the other two use `can_attack(vortex=false)`. Overclock emits at sub-slot `FIELD_EFFECT_SLOT_FOR_OVERCLOCK` (=0) via the new `Game::has_overclock_sacrifice` helper.

**Coverage:** [tests/mask_end_of_turn_parity.rs](../digimon-engine/tests/mask_end_of_turn_parity.rs) extends with `mask_overclock_emits_sub_slot_0_bit_with_sacrifice_available`, `mask_overclock_suppressed_when_no_sacrifice`, `mask_may_attack_emits_attack_bits_against_digimon_and_security`, `mask_may_attack_respects_cannot_attack_target`, `mask_force_attack_emits_attack_bits_in_eot`.

### 4.6c-residual 🟢 Overclock sacrifice execution — implemented

**Python** — [action_decoder.py:501-522](../digimon_gym/engine/game/action_decoder.py#L501) `_initiate_overclock`: installs a `SelectTarget` prompt over Token-or-Digimon sacrifices on the turn player's field; the callback deletes the sacrifice and calls `resolve_attack(overclock_perm, opponent_player, without_suspend=True, return_phase=EndOfTurnAction)`. The attacker is not suspended.

**Rust** — [game.rs](../digimon-engine/src/game.rs) `Game::activate_overclock(overclock_index)` validates phase + keyword + sacrifice-availability, then installs an `OwnField` selection via direct `pending_selection = Some(...)` install (no `EffectContext` because this is an engine-level action, not an effect). The callback calls `Game::delete_permanent_with_effects(sacrifice)` then `Game::begin_attack_overclock(overclock_handle, AttackTarget::Player(opponent))`. Interrupts (Alliance / Counter / Block) still fire normally — only Vortex is uninterruptible per DCGO.

The suspend-skip flows through a new `is_overclock: bool` field on `PendingAttack` + a `begin_attack_overclock` constructor that delegates to a shared `begin_attack_impl(vortex, is_overclock)` private helper. When `is_overclock`, the declaration-time `suspend_and_count_attack` call is skipped; everything else (OnAttack triggers, state machine, interrupts, cleanup) matches the normal path.

`OverclockError::{WrongPhase, Busy, NotOverclock, NoSacrifice, InvalidIndex}` exposes the validation failures so callers (Tauri, tests, future Python bindings) can distinguish between them.

**Coverage:** [tests/overclock_execution.rs](../digimon-engine/tests/overclock_execution.rs) — 10 cases: prompt install, reject-without-keyword, reject-without-sacrifice, reject-wrong-phase, decline-leaves-state-untouched, full-flow sacrifice + security hit, full-flow wins game on empty security, low-level `begin_attack_overclock` skips suspend, regression guard on normal attack still suspending, higher-index sacrifice action-ID round-trip.

### 4.6d 🟡 Full interrupt / selection-phase mask builders — partial

Unified by PR3-PR5 into a single generic branch in [action/mask.rs](../digimon-engine/src/action/mask.rs) that reads `pending_selection.valid_action_ids` directly — mask correctness is now driven by the selection install site rather than a dedicated per-kind builder.

- ✅ `BlockTiming` — `combat.rs::try_enter_block` installs a selection with every unsuspended Blocker-keyword Digimon's action ID (`encode_attack(0, field_idx)`) + PASS (Block is always a may-trigger). Attacker's `Keyword::Collision` widens the pool to every unsuspended opponent Digimon.
- ✅ `AllianceTiming` — `combat.rs::try_enter_alliance` installs with every unsuspended Alliance-keyword ally.
- ✅ `CounterTiming` — `combat.rs::try_enter_counter` installs one action ID per valid `(hand, field)` blast pairing via `encode_digivolve(h, f)` + PASS. Blast candidates detected by `Effect.blast_digivolve` flag; field targets validated via `Game::can_digivolve` (color + level). Scoped to Digimon-target attacks (Python parity).
- ✅ `SelectTarget` (OppField / OwnField kinds) — [effect_context.rs](../digimon-engine/src/effect_context.rs) `select_opponent_permanent` / `select_own_permanent`; reuses the ATTACK target-half range. Pilot: [TEST-010](../digimon-engine/src/cards/test_cards.rs) (delete opp Digimon).
- ✅ `SelectHand` — `effect_context.rs::select_hand`, reuses PLAY_HAND 0-29. Pilot: [TEST-011](../digimon-engine/src/cards/test_cards.rs) (trash from hand, draw 2).
- ✅ `SelectTrash` — `effect_context.rs::select_trash`, reuses TRASH_EFFECT 1150-1194. No pilot card yet — infra validated by shared test scaffolding.
- ✅ `EffectChoice` — `effect_context.rs::select_effect_choice`, reuses HAND_EFFECT 30-59 with effect_choices labels. Pilot: [TEST-012](../digimon-engine/src/cards/test_cards.rs) (choose memory / draw).
- ✅ `TriggerOrder` (drainer-installed, parks under EffectChoice phase) — [effect_queue.rs](../digimon-engine/src/effect_queue.rs) `install_trigger_order_selection`; reuses HAND_EFFECT 30-59. Handles player-chosen ordering of simultaneous triggers, plus PASS=decline-all on all-optional bundles.
- ✅ `SelectMaterial` — `effect_context.rs::select_material`, reuses SOURCE_SELECT 2000-2168. Prompts the controller to pick a source (digivolution-stack card) from a target permanent. Covered by [tests/select_material.rs](../digimon-engine/tests/select_material.rs) (7 cases).
- 🔴 `SelectReveal` / `SelectSecurity` / `SelectSource` — helpers not yet authored. Infrastructure is uniform with the landed kinds; add when a card needs them.

**Coverage:** [tests/effect_queue_drainer.rs](../digimon-engine/tests/effect_queue_drainer.rs) (9 cases), [tests/select_opponent_permanent.rs](../digimon-engine/tests/select_opponent_permanent.rs) (10), [tests/selection_kinds.rs](../digimon-engine/tests/selection_kinds.rs) (7), [tests/select_material.rs](../digimon-engine/tests/select_material.rs) (7), [tests/block_interrupt.rs](../digimon-engine/tests/block_interrupt.rs) (10), [tests/alliance_interrupt.rs](../digimon-engine/tests/alliance_interrupt.rs) (7), [tests/counter_interrupt.rs](../digimon-engine/tests/counter_interrupt.rs) (12).

### 4.6d-residual 🔴 Remaining selection kinds

`SelectReveal` / `SelectSecurity` / `SelectSource` helpers not yet authored. Infrastructure is uniform with the landed kinds (share `install_field_selection` or an analogous encoder); add per pilot card need.

### 4.7 🟡 Modifier-gated mask checks — partial

Four of the five checks have landed; §4.7e (DigiXros cost-reduction) and per-action context discriminants (§4.7x) remain future work.

### 4.7a 🟢 CannotAttackTarget — implemented

**Python** — [action_mask.py:129-136](../digimon_gym/engine/game/action_mask.py#L129): `has_modifier(target, CANNOT_ATTACK_TARGET, {'attacker': attacker})` gates each Digimon-attack bit; same check repeats in Vortex / MAY_ATTACK / FORCE_ATTACK arms.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Main-phase Digimon-attack inner loop + `GamePhase::EndOfTurnAction` arm call `modifiers.has(t_handle, ModifierType::CannotAttackTarget)` and skip the target. Per-attacker discriminant is dropped — see §4.7x.

**Coverage:** `mask_cannot_attack_target_suppresses_digimon_attack_bit` in [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs); `mask_vortex_respects_cannot_attack_target` in [tests/mask_end_of_turn_parity.rs](../digimon-engine/tests/mask_end_of_turn_parity.rs).

### 4.7b 🟢 CannotDigivolve — implemented

**Python** — [action_mask.py:151-153](../digimon_gym/engine/game/action_mask.py#L151): `has_modifier(base_perm, CANNOT_DIGIVOLVE, {'digivolving_card': card})`.

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Main-phase digivolve loop checks `modifiers.has(base_handle, ModifierType::CannotDigivolve)` before `can_basic_digivolve`. `digivolving_card` discriminant dropped (§4.7x).

**Coverage:** `mask_cannot_digivolve_suppresses_digivolve_bits_on_base` in [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs).

### 4.7c 🟢 CannotPlayFromHand — implemented

**Python** — [action_mask.py:58](../digimon_gym/engine/game/action_mask.py#L58) → `_is_play_blocked_by_modifier(card)` (effects.py:303-311).

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) Main-phase play-cards loop short-circuits when `modifiers.any_with_type(ModifierType::CannotPlayFromHand)` is true.

**Coverage:** `mask_cannot_play_from_hand_suppresses_all_hand_bits` in [tests/mask_main_parity.rs](../digimon-engine/tests/mask_main_parity.rs).

### 4.7d 🟢 FORCE_ATTACK — implemented

**Python** — [action_mask.py:227-280](../digimon_gym/engine/game/action_mask.py#L227): if any friendly Digimon has `ModifierType::FORCE_ATTACK`, every non-attack bit is zeroed and only those Digimons' attack bits remain. Falls through to the normal mask when no forced Digimon can legally act (all suspended, etc.).

**Rust** — [mask.rs](../digimon-engine/src/action/mask.rs) `apply_force_attack_mask_replacement` runs at the tail of the `GamePhase::Main` arm. Builds a fresh replacement mask, walks forced attackers through `can_basic_attack` + the same Raid / CanAttackUnsuspended / CannotAttackTarget filters the normal Main-phase attack loop uses, and `mask.copy_from_slice(&replacement)` when at least one attack bit was emitted. No memory gate on forced attackers (matches Python).

**Coverage:** [tests/mask_force_attack.rs](../digimon-engine/tests/mask_force_attack.rs) — 5 cases: non-attack bits zeroed when active, multiple forced Digimon all retain attacks, fall-through when forced attacker is suspended, CannotAttackTarget filtering, Raid-target tiebreak against unsuspended enemies.

### 4.7e 🔴 DigiXros cost-reduction — outstanding

Python's play-cost check (`action_mask.py:66-72`) computes `effective_cost = max(0, play_cost - max_reduction)` for cards with `digixros_cost`. Blocked on `CardData.digixros_cost` schema + `has_any_digixros_material` validator + ingest-pipeline data (same data-population shape as §4.5b). Own plan.

### 4.7x 🟡 Context-aware modifier queries — outstanding

Python's `has_modifier(target, type, context)` can refine the match via the modifier's `condition` closure — e.g. `CannotAttackTarget` that applies only to Red attackers, or `CannotDigivolve` that applies only when digivolving into a specific card. Rust's `ModifierEntry` ([modifiers.rs:13-19](../digimon-engine/src/modifiers.rs#L13)) has no condition closure, so §4.7a and §4.7b are unconditional (any active modifier blocks regardless of the attacker/digivolving_card discriminant). Adding condition closures is an architectural change worthy of its own plan.

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

1. **§1.1 — Deduct play cost** (single biggest correctness bug; every play is free today). ✅ done
2. **§1.2 / §1.3 / §1.4 / §1.5 — Memory seesaw semantics** (tight cluster; probably ~30 lines of changes in `game.rs` plus tests). ✅ done
3. ~~**§2.1 — Rush / Vortex exemption**~~ ✅ done — `vortex: bool` threaded through combat, modifier-granted Rush honored in both combat and Main-phase mask. Residual §2.1b (native static keyword) deferred until effect-listing lands.
4. ~~**§1.7 — First-turn draw rule**~~ — audit was wrong; behavior already matches Python. Tested as of this cycle.
5. ~~**§1.6 — Mulligan flow**~~ ✅ done — accept_mulligan state machine + first-player coin flip + tests/mulligan.rs.
6. ~~**§3.1 / §3.2 — Tensor source-DP + OPT slots**~~ ✅ done — `EffectReadContext` + `Permanent::effect_activations` + Game helpers + tensor wiring. Residual §3.1b (linked-card effects) deferred.
7. ~~**§4.2 / §4.3 / §4.4 — Action mask main-phase parity**~~ ✅ done — Option color check, Blitz memory exception, Raid / CAN_ATTACK_UNSUSPENDED targeting. Residual: §4.2b (script-based bypass) and §4.3b (native/static Blitz) await §4.5 effect-listing.
8. **§4.5 / §4.6 — Mask phase coverage** — partial. ✅ §4.5a DNA digivolve mask + data types; ✅ §4.5c Hand/Field/Trash `[Main]` masks + `Game::effects_for_card` effect-listing primitive; ✅ §4.5c-residual decoder execution via `Game::activate_hand_main` / `activate_field_main` / `activate_trash_main`; ✅ §4.6a Vortex mask emission; ✅ §4.6b `end_turn` phase transition + `pass_end_of_turn_action`; ✅ §4.6c Overclock/MayAttack/ForceAttack mask bits; ✅ §4.6d (partial) Block + Alliance interrupt builders + generic selection-phase mask branch. Blocked: §4.5b `dna_costs` data-population pipeline (cards.json ingest); §4.6c-residual Overclock sacrifice execution; §4.6d-residual Counter (blast-digivolve infra) + remaining per-effect selection kinds.
9. **§4.7 — Modifier-gated mask checks** — partial. ✅ §4.7a CannotAttackTarget, §4.7b CannotDigivolve, §4.7c CannotPlayFromHand (unconditional semantics); ✅ §4.7d FORCE_ATTACK Main-phase mask replacement. Outstanding: §4.7e DigiXros cost-reduction (own plan; also blocked on data-population like §4.5b), §4.7x context-aware modifier queries (architectural).
10. ~~**§2.2 / §2.3 — Combat state machine + `is_attacking`**~~ ✅ done — `PendingAttack` state machine with Alliance + Counter + Block windows; `is_attacking` flag lifecycle; `AttackResult::InProgress` signals paused attacks. Residual: `OnCounterTiming` distinct timing (§2.3-residual) awaits first card script.
11. ~~**§3.5 — Selection tensor slots**~~ ✅ done — `valid_count / ACTION_SPACE_SIZE` and `selecting_player` written at slots 1371/1372 whenever `pending_selection.is_some()`.
12. ~~**§2.3 Counter + blast digivolve**~~ ✅ done — `Effect::blast_digivolve` flag, `Game::can_digivolve` validator, `combat::try_enter_counter` + `execute_blast_digivolve`. Defender-only, Digimon-target only (Python parity). Attacker-deletion cascade routes to Cleanup without re-running Block/Battle.

The rest (face-down security §3.3, remaining selection kinds §4.6d-residual, etc.) can follow as cards that need them get implemented.

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
