# Rust ↔ Python Engine Parity Tracker

**Role:** Rust is the target engine; Python is retained only until card-script migration completes. This tracker exists to catalog divergences during the transition and will be retired when the Python engine is. Always consult this file before editing engine code in either language — it is the authoritative source for known behavioral differences and per-phase progress.

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

### 1.1 🟢 Play cost deducted from memory — implemented

**Python** — [player.py](../digimon_gym/engine/core/player.py) + game flow: `calculate_play_cost` resolves the effective cost (including reductions), the player's play path deducts it from memory before placement, and any OnPlay effects resolve after the card is on the field.

**Rust** — [game_actions.rs:63-91](../digimon-engine/src/game_actions.rs#L63) `play_from_hand_with_cost` computes the effective cost via `cost_delta.resolve(printed_cost)`, then calls `self.pay_memory(effective_cost)` at line 88 before removing the card from hand. If `pay_memory` returns `false`, the function aborts with `None`. The old `play_from_hand` is now a thin wrapper that delegates to `play_from_hand_with_cost` with `CostDelta::Reduce(0)`. `OnPlay` fires through the standard effect queue after the permanent is placed.

**Coverage:** confirmed by Phase 2 engine tests and `test_cards_behavioral.rs` pilots.

### 1.2 🟢 Memory pure-negation on turn switch — implemented

**Python** — [game/__init__.py:322](../digimon_gym/engine/game/__init__.py#L322) `self.memory = -self.memory`. No clamp. The seesaw is simply flipped from the next player's perspective.

**Rust** — [game_phases.rs:129](../digimon-engine/src/game_phases.rs#L129) `rotate_turn_player` executes `self.memory = -self.memory;` with an explicit comment that no clamping is applied. Over-cost plays that push memory deep negative carry their full magnitude across the switch as positive memory for the next player — the intended tempo consequence.

**Coverage:** confirmed by the first-turn draw and turn-rotation tests; no regression introduced by Phase 2.

### 1.3 🟢 `pass_turn` preserves overflow — implemented

**Python** — [game/__init__.py:329-334](../digimon_gym/engine/game/__init__.py#L329): `if self.memory >= 0: self.memory = -3`. Only forces the seesaw if the player still has memory to give. Over-cost plays that already put memory negative are preserved through the switch.

**Rust** — [game_phases.rs:333-335](../digimon-engine/src/game_phases.rs#L333) `pass_turn` gates the assignment: `if self.memory >= 0 { self.memory = -3; }`. If memory is already negative because an over-cost play pushed it there, the overflow is preserved and carried through via the subsequent `end_turn()` call. Matches Python exactly.

### 1.4 🟢 `pay_memory` is a pure memory mutator — implemented

**Python** — turn end is checked in `check_turn_end` ([__init__.py:336](../digimon_gym/engine/game/__init__.py#L336)) after effect resolution, not synchronously with payment. This lets OnPlay, WhenDigivolving, etc. resolve on the same turn even if their cost already pushed memory negative.

**Rust** — [game.rs:445](../digimon-engine/src/game.rs#L445) `pay_memory` is a pure mutator: it updates `self.memory`, emits a `MemoryChange` event, and returns `true`/`false` — it never calls `end_turn()`. A separate [game.rs:466](../digimon-engine/src/game.rs#L466) `check_turn_end` method is provided for callers to invoke at the natural resolution boundary after all effects of a play/action have resolved. This matches the Python contract exactly.

### 1.5 🟢 Memory swing-back on OnEndTurn — implemented

**Python** — [game/__init__.py:276-280](../digimon_gym/engine/game/__init__.py#L276): if `OnEndTurn` effects restore memory from `< 0` back to `>= 0`, the turn continues and returns to Main. Real DCGO rule, used by some cards.

**Rust** — [game_phases.rs:78-85](../digimon-engine/src/game_phases.rs#L78) `end_turn` captures `memory_before = self.memory` before firing `fire_end_of_your_turn(ending_player)`. After the drain, it checks `if memory_before < 0 && self.memory >= 0 && !self.game_over`: if the sign flipped back, the function sets `self.current_phase = GamePhase::Main` and returns immediately, short-circuiting the turn switch. This matches the Python swing-back rule exactly.

**Coverage:** [tests/end_turn_phase_transition.rs](../digimon-engine/tests/end_turn_phase_transition.rs) `swing_back_short_circuit` case.

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

### 2.1b 🟢 Native (card-text) Rush parsed — implemented

Phase 3 added `CardData::keywords` field populated at load time by `parse_printed_keywords` (card_data.rs). The unified `Game::has_keyword` query (game.rs) checks both modifier-granted AND native printed keywords. All 14 call sites migrated; cards printing ＜Rush＞ now exempt the permanent from summoning sickness in `can_attack` without needing a granting modifier. See docs/RUST_ENGINE_API.md §Phase 3.

**Coverage:** `tests/keyword_parsing.rs` — `native_printed_rush_allows_same_turn_attack`.

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

### 2.5 🟡 Security effect execution — partial

The security-effect pipeline landed end-to-end for straight-line effects (trigger → process → trash, or trigger → `play_from_security` → stay-on-field). Three synthetic pilots (TEST-020/021/022) exercise the pipeline in both engines and are pinned by paired tests. Real-card parity is blocked on the sub-gaps §2.5b–§2.5m below.

**Python** — [player.py:556-699](../digimon_gym/engine/core/player.py#L556) `Player.security_attack` + [combat.py:188-221](../digimon_gym/engine/game/combat.py#L188) `_execute_security_checks`: pops the top security card, fires `EffectTiming.SecuritySkill` effects with `is_security_effect=True` against a context dict carrying `{game, player, card, attacker, security_digimon, turn_player, opponent_player}`, fires `OnSecurityCheck` globally after effects, runs the Digimon-vs-security DP battle (with `DONT_BATTLE_SECURITY_DIGIMON` skip, `_applies_to_opponent_security_digimon` DP adjustments, and native `_is_jamming` attacker protection), and routes the revealed card to trash unless an effect flipped `card._security_played = True` via `Game.effect_play_from_security`. `OnLoseSecurity` fires unconditionally at the end.

**Rust** — [combat.rs:882](../digimon-engine/src/combat.rs#L882) `resolve_security_card` parks the popped card in the new `Game.pending_security: Option<PendingSecurity>` slot ([selection.rs](../digimon-engine/src/selection.rs)), enqueues `EffectTiming::SecuritySkill` via a new `TriggerSource::SecurityRevealed { defender, card }` variant, drains the queue, runs the Digimon DP battle (with `Keyword::Jamming` attacker-survival gate), fires `EffectTiming::OnLoseSecurity`, and trashes the card unless `pending_security.played == true`. `EffectContext::play_from_security` is the sole way to raise the `played` bit — mirrors Python's `effect_play_from_security`.

Three `EffectTiming` variants replace the former single `SecurityEffect` entry, matching Python:
- `SecuritySkill` — per-card trigger on reveal (Python timing 38).
- `OnSecurityCheck` — observer timing fired after the revealed card's effects (Python timing 35). Infrastructure present; **not yet wired in `resolve_security_card` — see §2.5b.**
- `OnLoseSecurity` — fires when the card leaves the security stack (Python timing 19). Wired.

New `EffectContext` helpers landed alongside: `play_from_security`, `select_security`, `select_reveal`, `mark_security_face_up` — also closing §4.6d-residual's `SelectSecurity` / `SelectReveal` gap for arbitrary non-security effects. Action-ID ranges mirror Python's `SEL_REVEALED_START=30`, `SEL_MY_SECURITY_START=40`, `SEL_OPP_SECURITY_START=50` (phase-disambiguated sub-ranges of the shared 30-59 HAND_EFFECT space).

**Coverage:** Rust side — [tests/security_effects.rs](../digimon-engine/tests/security_effects.rs) (5 cases: TEST-020 draw, TEST-021 play-from-security, TEST-022 memory gain, no-effect trash regression, `pending_security` cleared post-check). Python side — [tests/behavioral/synthetic/test_security_pilots.py](../tests/behavioral/synthetic/test_security_pilots.py) (8 cases across the same three pilots + script registration).

**Pilot cards** — synthetic TEST-020 / TEST-021 / TEST-022 live in both engines:
- Rust: [digimon-engine/src/cards/test_cards.rs](../digimon-engine/src/cards/test_cards.rs) (structs Test020/Test021/Test022).
- Python: [scripts/test/test_020.py](../digimon_gym/engine/data/scripts/test/test_020.py), [test_021.py](../digimon_gym/engine/data/scripts/test/test_021.py), [test_022.py](../digimon_gym/engine/data/scripts/test/test_022.py) + entries 4083/4084/4085 in [cards.json](../digimon_gym/engine/data/cards.json).

### 2.5a 🟢 Basic `SecuritySkill` dispatch — implemented

Enqueue + drain of the revealed card's `SecuritySkill` effects; `play_from_security` + `pending_security.played` flag; `OnLoseSecurity` fires unconditionally. Covered by the three pilots above. Any security effect that (a) is unconditional, (b) needs no context beyond `self.player`/`game`, and (c) installs no pending selection, now behaves identically in both engines.

### 2.5b 🟢 `OnSecurityCheck` observer timing fired — implemented

**Python** — [combat.py:206-214](../digimon_gym/engine/game/combat.py#L206) `_execute_security_checks` calls `execute_effects(EffectTiming.OnSecurityCheck, sec_check_ctx)` after each reveal-and-resolve pass. Field effects that watch for security checks globally (e.g. "When your security is checked, gain 1 memory") rely on this timing.

**Rust** — [combat.rs:949-955](../digimon-engine/src/combat.rs#L949) the `OnSecurityCheckDrain` security phase builds a `TriggerSource::OnSecurityCheck { attacker, defender, revealed_card, was_face_up }` and calls `self.enqueue_triggered(EffectTiming::OnSecurityCheck, trigger)`. [effect_queue.rs:65](../digimon-engine/src/effect_queue.rs#L65) dispatches `TriggerSource::OnSecurityCheck` by iterating the defender's entire `battle_area` and calling `enqueue_from_permanent` for each permanent — matching the Python fan-out. After draining, the state machine advances past `OnSecurityCheckDrain`.

**Note:** Non-combat security removal (effect-driven security trashing) does not yet emit this timing — that path is tracked in the engine-gaps doc under "Global `OnOpponentSecurityRemoved` observer timing". The attack-path dispatch (§2.5b) is confirmed equivalent.

**Coverage:** [tests/security_effects.rs](../digimon-engine/tests/security_effects.rs) security-observer pilot cases.

### 2.5c 🔴 Progress / immunity-to-opponent-effects gate not wired

**Python** — [player.py:614-616](../digimon_gym/engine/core/player.py#L614) `if attacker.is_immune_to_opponent_effects: ... else: <fire SecuritySkill effects>`. An attacker with Progress entirely skips the defender's security effects.

**Rust** — [combat.rs:912-922](../digimon-engine/src/combat.rs#L912) unconditionally fires `SecuritySkill`. No `ModifierType::ImmunityToOpponentEffects` exists in the enum; no `Keyword::Progress` variant either.

**Fix outline:** Add `ModifierType::ImmunityToOpponentEffects` (or `Keyword::Progress`) + a `modifiers.any_with_type(attacker, ...)` gate before the `enqueue_triggered(SecuritySkill, ...)` call. Deferred until the first Progress card is ported.

### 2.5d 🔴 `DONT_BATTLE_SECURITY_DIGIMON` modifier not checked

**Python** — [player.py:644-650](../digimon_gym/engine/core/player.py#L644) checks `modifiers.has_modifier(attacker, ModifierType.DONT_BATTLE_SECURITY_DIGIMON)` and skips the Digimon-vs-security battle entirely when set.

**Rust** — [combat.rs:928-942](../digimon-engine/src/combat.rs#L928) always runs the DP comparison for `CardKind::Digimon` security. No matching `ModifierType` variant.

**Fix outline:** Add `ModifierType::DontBattleSecurityDigimon` + gate the DP branch on `!modifiers.any_with_type(attacker, ...)`.

### 2.5e 🔴 Inherited-effect DP adjustments to security Digimon not applied

**Python** — [player.py:654-666](../digimon_gym/engine/core/player.py#L654) iterates the attacker's inherited effects for `_applies_to_opponent_security_digimon + dp_modifier != 0`, adjusts `s_dp` before comparing. Powers cards like "This Digimon gains +3000 DP when attacking security".

**Rust** — [combat.rs:931](../digimon-engine/src/combat.rs#L931) uses `sec_card.dp(&self.card_data).unwrap_or(0)` raw — no inherited-effect pass over the attacker's stack.

**Fix outline:** Introduce an `Effect` flag `applies_to_opponent_security_dp` and an `attacker_security_dp_adjustment(attacker)` helper on `Game` that iterates the attacker's `card_sources[..last]` inherited effects.

### 2.5f 🟢 Native Jamming honored — implemented

Phase 3 landed unified keyword lookup (see §2.1b). The security DP battle in `combat.rs` now checks `self.has_keyword(attacker, Keyword::Jamming)` which includes native printed Jamming. Cards with ＜Jamming＞ printed on their face survive losing security battles without needing a granting modifier.

**Coverage:** `tests/keyword_parsing.rs` — `native_printed_jamming_survives_losing_security_battle`.

### 2.5g 🔴 EffectContext missing security-specific context

**Python** — security-effect context dict passed to each callback ([player.py:622-632](../digimon_gym/engine/core/player.py#L622)):
```
{ game, player, permanent=None, card, security_digimon, attacker, turn_player, opponent_player }
```

**Rust** — `EffectContext` exposes only `{ game, source_card, source_permanent, player }`. A script that needs to inspect the attacker (e.g. "if the attacker is Red, gain 2 memory") or the security Digimon (e.g. "if this Digimon's DP is less than 5000, Jamming") has no API. `ctx.opponent_id()` returns `next_clockwise(self.player)` which happens to equal the attacker's side in 2-player games — but breaks under EDH/Titan seating.

**Fix outline:** Enrich `EffectContext` with optional handles set by the security resolver:
```rust
pub attacker: Option<PermanentHandle>,      // set during SecuritySkill
pub security_digimon: Option<CardHandle>,   // set if the revealed card is a Digimon
pub turn_player: PlayerId,                   // always valid
```
The existing `TriggerSource::SecurityRevealed` carries enough info to populate these at drain time.

### 2.5h 🟡 Condition-check divergence on `SecuritySkill`

**Python** — [player.py:619-622](../digimon_gym/engine/core/player.py#L619) iterates `effect_list(SecuritySkill)`, checks `if effect.is_security_effect`, calls the callback directly. **No `effect.can_use_condition` check.**

**Rust** — [effect_queue.rs:218-228](../digimon-engine/src/effect_queue.rs#L218) `run_queued_effect` evaluates `effect.condition` and returns without firing if it's false. A conditional `[Security]` effect (`[Security] If your opponent controls a Digimon, delete it.`) fires unconditionally in Python and only when the condition passes in Rust.

**Fix outline:** Either (a) drop the condition check for queued effects whose `security` flag is set — simplest, matches Python exactly — or (b) audit Python to decide whether the condition-skip is intentional or a latent bug, then align both engines.

### 2.5i 🟡 `TriggerOrder` prompt on multi-effect security cards

**Python** — fires multiple SecuritySkill effects in `effect_list` order with no prompt.

**Rust** — [effect_queue.rs:116-124](../digimon-engine/src/effect_queue.rs#L116) the drainer installs a `TriggerOrder` selection whenever a single controller has ≥ 2 queued effects. For a card with two `[Security]` effects (rare but real — see BT1-087 for the adjacent "two security effects on one card" pattern), Rust would prompt the defender and Python would not.

**Fix outline:** Short-circuit `TriggerOrder` installation when every queued effect in the bundle comes from the same `TriggerSource::SecurityRevealed` — fire in collection order like Python. Alternatively, only install the prompt when ≥ 2 effects are both `is_optional` (currently the prompt fires for any multi-bundle).

### 2.5j 🔴 Selections inside security effects are not re-entrant with combat

**Python** — `security_attack` is re-entrant: an effect that installs a pending_selection (via `effect_play_from_zone`, `effect_select_opponent_permanent`, etc.) parks the selection; later resolution re-enters the combat flow via the attack state machine's selection-resume hooks (e.g. `_maybe_resume_combat_after_wa_selection`, [combat.py:85-90](../digimon_gym/engine/game/combat.py)).

**Rust** — `resolve_security_card` is synchronous. If a `SecuritySkill` process calls `ctx.select_hand` / `select_security` / etc., `drain_effect_queue` returns with `pending_selection = Some(...)`, but the surrounding `resolve_player_security_loop` doesn't know combat is mid-resolution — it treats the check as complete and returns to the caller. The `OnLoseSecurity` fire, Digimon battle, and trash disposition all never happen for the paused check.

**Impact:** Blocks porting every real-card security effect that includes a selection — BT1-087 (T.K., select a security card to reveal), BT10-094 (Breaclaw, play from hand/trash), and most tamers in the security-search family.

**Fix outline:** Extend `PendingAttack` with `security_state: Option<SecurityResolutionState { defender: PlayerId, remaining_checks: u8, mid_resolve_card: Option<CardSource> }>`. When `drain_effect_queue` pauses mid-security, stash the state and return. `resolve_generic_selection` resumes the remainder of `resolve_security_card` (Digimon battle + `OnLoseSecurity` + trash + loop-continue) after the selection's callback fires.

This is the **biggest load-bearing gap in §2.5** — it's what blocks real cards, not just synthetic pilots.

### 2.5k 🟡 `face_up_security` stale entries on reveal

**Python** — [player.py:575](../digimon_gym/engine/core/player.py#L575) `self.face_up_security.discard(security_card)` — removes the revealed card from the face-up set the moment it's popped.

**Rust** — [combat.rs:771](../digimon-engine/src/combat.rs#L771) + [resolve_player_security_loop](../digimon-engine/src/combat.rs#L752) pops the card from `player.security` but never touches `player.face_up_security`. The stale `card_index` remains in the set forever.

**Observable impact today:** none (the set is only consulted for cards still in security, and a popped card is no longer there). But if an effect ever returns a revealed card to security (possible via future scripts), the stale entry would make it appear face-up by accident.

**Fix outline:** Add `player.face_up_security.remove(&card.card_index)` in `resolve_player_security_loop` immediately after the pop.

### 2.5l 🟡 `_last_security_card` / `_last_security_was_face_up` not stored

**Python** — [player.py:577-578](../digimon_gym/engine/core/player.py#L577) stashes the just-revealed card on the defender so the subsequent `OnSecurityCheck` context ([combat.py:211-212](../digimon_gym/engine/game/combat.py#L211)) can hand it to observer effects.

**Rust** — no equivalent storage. Coupled to §2.5b — once `OnSecurityCheck` is fired, the observer context needs access to the revealed card. `Game.pending_security` covers this *during* the SecuritySkill drain but is cleared before the (currently unfired) `OnSecurityCheck` hook would run.

**Fix outline:** Widen `Game.pending_security`'s lifetime so it persists through the `OnSecurityCheck` fire, or add a distinct `last_security_reveal: Option<SecurityRevealSnapshot>` field cleared on turn rotation.

### 2.5m 🟡 `security_reveal` event not emitted

**Python** — [player.py:596-608](../digimon_gym/engine/core/player.py#L596) emits a rich `security_reveal` event (card id, name, remaining count, card type, DP, effect text) consumed by the UI logger and RL replay recorder.

**Rust** — no equivalent. The engine's event surface is thinner overall; this is one instance of a cross-cutting event-parity gap, not a security-specific one. Tracked here for visibility.

### 2.5-harness 🔴 Cross-engine YAML parity harness not built

Planned in [plan-out-the-security-sorted-storm.md](../.claude/plans/plan-out-the-security-sorted-storm.md) as a `scenario_runner` Rust binary + `tests/behavioral/test_parity_scenarios.py` pytest driver + `tests/scenarios/parity/security/*.yaml` fixture set, diffing JSON snapshots after each step. Deferred — scope is substantial (new Rust binary crate target, serde_yaml integration, JSON snapshot schema design, subprocess bridge, pytest parameterization). Parity is proved manually today via the per-engine pilot tests listed above.

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

### 4.6b-residual 🟢 Token detection — implemented

Rust's `CardKind` now includes `Token` (Phase 10). Tokens are
registered via `token_registry.rs` with synthetic `CardData` rows
absorbed into `game.card_data` at `Game::new`. Python's `is_token:
bool` flag and Rust's `CardKind::Token` are kept in sync at the PyO3
binding boundary (any helper that returns a token permanent
translates the flag appropriately).

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
- 🟢 `SelectReveal` — `effect_context.rs::select_reveal`, reuses `SEL_REVEAL_START` 30-39. Landed alongside §2.5 (security pilot infrastructure).
- 🟢 `SelectSecurity` — `effect_context.rs::select_security`, reuses `SEL_MY_SECURITY_START` 40-49 (own) / `SEL_OPP_SECURITY_START` 50-59 (opponent). Landed alongside §2.5.
- 🔴 `SelectSource` — helper not yet authored. Infrastructure is uniform with the landed kinds; add when a card needs it.

**Coverage:** [tests/effect_queue_drainer.rs](../digimon-engine/tests/effect_queue_drainer.rs) (9 cases), [tests/select_opponent_permanent.rs](../digimon-engine/tests/select_opponent_permanent.rs) (10), [tests/selection_kinds.rs](../digimon-engine/tests/selection_kinds.rs) (7), [tests/select_material.rs](../digimon-engine/tests/select_material.rs) (7), [tests/block_interrupt.rs](../digimon-engine/tests/block_interrupt.rs) (10), [tests/alliance_interrupt.rs](../digimon-engine/tests/alliance_interrupt.rs) (7), [tests/counter_interrupt.rs](../digimon-engine/tests/counter_interrupt.rs) (12).

### 4.6d-residual 🟡 Remaining selection kinds

✅ `SelectReveal` and `SelectSecurity` helpers landed with §2.5 (see above). 🔴 `SelectSource` helper not yet authored — infrastructure is uniform with the landed kinds (share `install_field_selection` or an analogous encoder); add when a card needs it.

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

### §5.1 Cost-reduction closures + pay_cost_fn hook — Rust-only (Phase 5)

**Status (2026-04-21):** Rust exclusively supports closure-valued cost reduction at `EffectTiming::BeforePayCost` and a synchronous `pay_cost_fn` hook on triggered effects (and at BeforePayCost dispatch). Python uses a `_temp_play_cost_reduction` instance variable that leaks across effects (Issue 24 per project memory). Rust **intentionally does not replicate** this pattern; scripts requiring dynamic reduction must use `.cost_reduction_fn`.

No Python parity — this is a strict improvement in Rust. Python will not catch up; migration targets Rust as the source of truth for these mechanics.

Cards unblocked (per audits): ~50 across Rocks (primary), some Dark Masters and TS Olympos cost-gating effects. See `.claude/plans/rust-engine-gaps-rocks.md` for the Rocks-specific list.

Rust implementation: `Game::scan_before_pay_cost_reduction` in `digimon-engine/src/game_actions.rs` + `pay_cost_fn` hook in `digimon-engine/src/effect_queue.rs::run_queued_effect`.

---

### §6.1 Player-scoped flood gates — Rust (Phase 6)

Rust adds a parallel `player_modifiers` tier to `ModifierRegistry` (`HashMap<PlayerId, Vec<PlayerModifierEntry>>`) plus 13 new `ModifierType` variants for action-category flood gates (`CannotPlayDigimonByEffect`, `CannotGainMemoryByEffect`, `CannotGainMemoryExceptFromTamers`, `CannotReducePlayCost`, `CannotActivateMainEffects`, `CannotActivateWhenDigivolvingEffects`, `CannotActivateSecurityEffects`, `CannotAddSecurityByEffect`, `CannotTrashOpponentSecurity`, `CannotReduceOpponentSecurity`, `CannotDrawByEffect`, `CannotDigivolveDigimonByEffect`, `IgnoreColorRequirement`). Gates are enforced at BOTH the action-mask layer (RL-visible suppression) and the resolver layer (defense-in-depth).

Python stores modifiers as a flat `HashMap<ModifierType, Vec<Entry>>` with closure-valued per-entry conditions. Rust v1 uses flag-based entries + card-script `.condition` closures at install-time, following DCGO's separate-class-per-restriction pattern (see `DCGO/Assets/Scripts/CardEffect/BT3/Green/BT3_046.cs` for Tamer-source-discriminated `CannotAddMemoryClass`). Phase 7 may add closure conditions to `ModifierEntry` for the would-replacement framework.

Python's `ctx.get('played_by_effect', False)` context is matched by Rust's typed `PlaySource` enum (`ByHand` / `ByEffect` / `ByDigivolve`), threaded through play/digivolve helpers — strictly cleaner than Python's dict-based context.

The `source_is_tamer` helper matches DCGO's `ICardEffect.IsTamerEffect` property; Rust uses a fast path via `source_permanent` + slow-path `card_kind` lookup. Used by `CannotGainMemoryExceptFromTamers` to pass memory gains originating from Tamer effects through the restriction gate.

Cards unblocked (per audits): ~55 across all 5 audited archetypes (Dark Masters lockout shell, Medusamon Petrification, TS Olympos Tamer-anchoring, Rocks Plug-In lockouts).

---

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

13. **§2.5 — Security effect execution** — partial. ✅ §2.5a basic `SecuritySkill` dispatch (trigger → process → trash + `play_from_security` bypass + `OnLoseSecurity`); ✅ §2.5b `OnSecurityCheck` observer timing fired (attack path — `OnSecurityCheckDrain` phase + `effect_queue.rs:65` defender-battle-area fan-out); ✅ `face_up_security` tensor parity (§3.3); ✅ `SelectSecurity` / `SelectReveal` helpers (§4.6d). Outstanding, in rough order of load-bearing-ness: **§2.5j re-entrant selections mid-security-resolve** (blocks every real card with a selection-using `[Security]` effect), §2.5g EffectContext extras (attacker / security_digimon / turn_player), §2.5c Progress immunity gate, §2.5d DontBattleSecurityDigimon modifier, §2.5e inherited-effect DP adjustments to security Digimon, §2.5f native Jamming (blocked on §2.1b), §2.5h condition-check divergence, §2.5i TriggerOrder-for-multi-security-effect, §2.5k `face_up_security` cleanup on reveal, §2.5l `_last_security_card` snapshot, §2.5m `security_reveal` event emission, §2.5-harness cross-engine YAML harness.

The rest (face-down security §3.3, remaining selection kinds §4.6d-residual `SelectSource`, etc.) can follow as cards that need them get implemented.

---

## 9. PyO3 PvP bindings (2026-04-18)

The following Python-parity behaviors are known to diverge and will be
addressed as card-migration and engine completeness work proceeds.

### Stubbed per-permanent fields (card-script-dependent)

`serialization::to_ui_json` emits neutral/empty defaults for these per-permanent fields until the corresponding card scripts are migrated to Rust:

- `mainEffectText` — empty string
- `inheritedEffectText` — empty string
- `inheritedEffects` — empty array
- `keywords` — empty array
- `keywordBreakdown` — `{innate: [], gained: []}`
- `securityAttackModifier` — 0
- `dpBreakdown.sources` — empty array
- `dpBreakdown.aura` — 0
- `dpBreakdown.temporary` — 0.0
- Per-source `optState`, `dpContribution` — 0.0 / 0

Rule 17 (no-approximations) applies to card effects; these are UI-rendering
artifacts that follow naturally once card scripts land in Rust.

### GameEvent emission coverage

Rust emits `MemoryChange`, `Play`, and `GameOver` today. `TurnStart`,
`PhaseChange`, `Digivolve`, `Attack`, `Trash`, `Mill`, and `SecurityReveal`
variants are defined but unwired; the PyO3 `event_to_pydict` handles all
variants so emission can be added without schema churn.

### Recording initial_state timing

Python's `GameRecorder.capture_initial_state` is called by `base_runner.py`
after `start_game()` completes. Rust captures lazily in `HeadlessRunner::step`
the first time mulligan is complete. Net effect for recorded games:

- `initial_hand` reflects the POST-mulligan hand in Rust; PRE-mulligan in Python
- `security_order` is populated on both sides

This is a deliberate Rust choice ("correct over strict parity") because
Python's timing captures empty security.

### Recording timestamp precision

Python's `datetime.now(timezone.utc).isoformat()` produces microsecond
precision (e.g. `2026-04-18T12:34:56.123456+00:00`). Rust produces
second precision via a hand-written `civil_from_days` algorithm. Replay
tooling that parses timestamps should tolerate both.

### pendingSelection.kind

Rust emits an extra `kind` string (e.g. `"OppField"`) not present in
Python. It's a deliberate affordance for typed WebSocket consumers.
Python's `pendingSelection` dict does not contain this key, but Task 6
parity tests compare only top-level `to_ui_json` keys, so the divergence
is tolerated.

### pendingSelection.keywordPrompt

Python's `PendingSelection` has an optional `keyword_prompt: dict` field.
Rust's `PendingSelection` struct does not (no equivalent field). When this
field would be non-null in Python, Rust simply omits it — net result on
the wire is the absence of the key.

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

---

## 10. Policies

Parity tracking for opponent policies. ONNX inference lives in
[`digimon-engine/src/inference/`](../digimon-engine/src/inference/) with
its own parity test suite (`tests/onnx_parity/`). This section covers
the heuristic side — the greedy bot — plus shape assumptions shared
across both engines.

### 10.1 🟡 Greedy heuristic re-implemented in Rust

**Python** — [`digimon_gym/digimon_gym.py::greedy_policy`](../digimon_gym/digimon_gym.py) inspects full game state (phases, hand/field, level/DP/cost) and prioritizes Digivolve > Attack > Play > Pass in Main and Hatch > Move > Pass in Breeding. Tie-breaks are deterministic by (level, DP, -cost, -hand_idx, -field_idx).

**Rust** — [`digimon-engine/src/policies/greedy.rs`](../digimon-engine/src/policies/greedy.rs) hand-ports the same heuristic. `greedy_action(game, mask) -> u16` is wired into `PlayerKind::Greedy` in `src-tauri/src/engine_commands.rs::run_agent_steps`, replacing the pre-port `first_valid_action` placeholder.

**Parity hazard:** if Python's `greedy_policy()` changes (new tie-break rule, phase handling, archetype-specific logic), the Rust port will silently diverge. Any edit to the Python greedy must be mirrored in Rust and covered by a deterministic-seed behavioral test under [`digimon-engine/tests/policies/greedy.rs`](../digimon-engine/tests/policies/greedy.rs). The `self_play.rs` tripwire (20 seeds of greedy-vs-greedy to conclusion) catches gross breakage but not nuanced decision divergence.

### 10.2 🟡 ONNX inference shape contract

**Python** — [`digimon_gym/engine/onnx_policy.py`](../digimon_gym/engine/onnx_policy.py) binds input `"obs"` (shape `(1, TENSOR_SIZE)`) and output `"logits"` (shape `(1, ACTION_SPACE_SIZE)`). LSTM variant adds `h_in`/`c_in`/`h_out`/`c_out` at `(1, 1, 256)`.

**Rust** — [`digimon-engine/src/inference/`](../digimon-engine/src/inference/) binds the same names and asserts the same shapes at session-load time; the compatibility gate in [`src-tauri/src/models.rs`](../src-tauri/src/models.rs) rejects drifted models before the download starts.

**Historical drift (resolved):** pre-2026-04-18, `tools/export_onnx.py` hardcoded `obs=981 / logits=2120` — the pre-rewrite layout. Any `.onnx` on disk dated before the fix is unusable by either engine. Re-export from the original `.zip` checkpoint is mandatory; if that checkpoint was trained against the old layout, it must be retrained from scratch. The exporter now imports `TENSOR_SIZE` / `ACTION_SPACE_SIZE` from `digimon_gym.engine.game.constants` and raises before writing on any shape mismatch.

**Ongoing hazard:** any future change to [`digimon-engine/src/tensor.rs`](../digimon-engine/src/tensor.rs) (`TENSOR_SIZE`) or [`digimon-engine/src/action/space.rs`](../digimon-engine/src/action/space.rs) (`ACTION_SPACE_SIZE`) invalidates every bundled or cached `.onnx`. The compatibility gate in `models.rs` and the exporter's shape assertion together make this a loud error, not a silent regression — but re-exports of all live checkpoints are required whenever either constant changes.

## 11. Phase 10 — Tokens + De-Digivolve

### 11.1 🟢 Token creation + CardKind::Token — implemented

**Python** — `digimon_gym/engine/data/token_registry.py`: `TOKENS`
dict mapping token names to metadata; `create_token_card_source`
factory; `CardSource.is_token: bool` flag; `Permanent.is_token`
property; Game.effect_play_token for spawning.
`Player.delete_permanent` branches on `is_token` to skip trash
(`player.py:506`).

**Rust** — `digimon-engine/src/token_registry.rs` defines
`TokenDef` + `TokenRegistry` + `build_registry`. `CardKind::Token`
variant in `enums.rs`. `EffectContext::play_token(controller,
token_name)` materializes a `CardSource::new_token(...)` +
`Permanent::new(...)` on the target's battle area.
`Player::delete_permanent` branches on `top_card().is_token` to
skip trash append.

**Divergences:** None observable. Rust's `CardKind::Token` is a
first-class enum variant where Python uses `is_token: bool` on a
Digimon-kind CardSource — the Rust shape is cleaner and supports
exhaustive match-based Token-specific branching (e.g. future Overclock
sacrifice filter). The PyO3 binding layer (`digimon-engine-py`)
must translate `CardKind::Token` → `is_token=True, card_kind=Digimon`
when synthesizing Python mirrors (not yet wired — no PvP code
surfaces tokens today).

**Coverage:** `digimon-engine/tests/cards_behavioral/tokens.rs` +
`digimon-engine/src/token_registry.rs` unit tests.

### 11.2 🟢 De-Digivolve N — implemented (superset of Python)

**Python** — per-archetype scripts call `card.lose_digivolution(N)` or
similar helpers.

**Rust** — `EffectContext::de_digivolve(target, stop_at_level: Option<u8>, amount: Option<u8>) -> u8`.
Pops up to `amount` sources, stops at `stop_at_level`, routes trash
to owner side. `None` for either arg expresses unbounded. Returns
actual count popped so callers can gate follow-up effects ("if at
least one was popped, gain 1 memory").

**Divergences:** Rust's API is a strict superset of the Python
surface — every Python call site reduces to
`de_digivolve(target, Some(3), Some(N))` in Rust.

**Coverage:** `digimon-engine/tests/cards_behavioral/de_digivolve.rs`.

---

## 12. Replacement framework (Phase 7)

### 12.1 🟢 Would* timings + `try_replace` dispatcher — Rust-only

**Python** — no equivalent. Python approximates all "would"
semantics via post-hoc observer timings (`OnDeletion`, `OnReturn`,
`OnLeaveField`) which fire *after* the state change commits. For
mechanics that need to intercept-and-substitute (Barrier, Evade,
Decode, Partition, ArmorPurge, Fragment, "cannot be returned to
deck", "cannot be de-digivolved"), Python scripts either
approximate the effect as a post-hoc reaction (breaking
faithfulness — e.g. Barrier can't prevent the OnDeletion queue
from being enqueued) or the card is marked BLOCKED in
`qa/archetype-qa/engine-gaps.md`. This is a known faithfulness
gap per CLAUDE.md rule 17 (no-approximations policy) and is
tracked at cross-archetype scope (~60 cards).

**Rust** — full Phase 7 replacement framework:

- `EffectTiming::Would*` family (9 dispatching variants + 2
  reserved for Phase 9). Each fires before the corresponding
  state-change helper commits.
- `Game::try_replace(timing, subject, cause, original_destination)
  -> ReplacementOutcome` — canonical fire-site entry point. Walks
  registered candidates (card effects + passive modifiers),
  layers by controller, installs `PendingSelection::Replacement`
  for optional candidates, and composes outcomes (last-non-None
  wins in v1).
- `ReplacementContext` — curated mutation API for effect
  processes: `cancel()`, `redirect_to(zone)`, `substitute(subject)`,
  `handled()`.
- `ReplacementCause` — Battle / OwnEffect / OpponentEffect /
  SecurityCheck / Cost. Derived at fire-site; scripts filter on
  it but never compute it.
- Passive-modifier auto-install: `CannotBeReturnedToDeck`,
  `CannotBeReturnedToHand`, `CannotBeTrashedByEffect`,
  `CannotBeDeDigivolved`, `CannotBeDestroyed*` all wire as
  mandatory cancels via the modifier registry's replacement path.
- Native-keyword auto-install: `<Barrier>`, `<Evade>`, `<Decode>`
  parsed from `CardData::keywords` produce the right
  replacement at `effects_for_card` time.
- Spec §7.5 once-per-event guard: `(timing, subject)` pairs that
  already fired in the current call chain are skipped;
  strengthened during callback-commit continuations to "any prior
  fire for this subject blocks" so redirect routes don't
  cascade into additional prompts for what is logically a single
  event.

**Divergences:** Rust has this entire layer; Python does not.
Every replacement-semantics card in the catalog is a parity gap
that resolves only by migrating the card to Rust (per CLAUDE.md
rule 21 — cards are not dual-implemented).

**Phase 7 v1 constraints** (documented in
`docs/RUST_ENGINE_API.md` § Phase 7):

1. `<Partition>`, `<ArmorPurge>`, `<Fragment(N)>` parse into
   `Keyword` variants but don't auto-install — each needs a
   nested `PendingSelection::Source` inside the replacement
   window, which is uncharted. Hand-authored scripts can install
   them via `Effect::when_would_be_deleted(card).optional()`.
2. Optional replacements for `Card` / `Player` subjects silently
   no-op on commit — `commit_deferred_outcome` is Permanent-only
   in v1 (debug_assert-guarded; unreachable today).
3. Multi-replacement `TriggerOrder` prompts not emitted when both
   sides have >1 candidates — runs in collection order, last
   non-None outcome wins.
4. `ACTION_SPACE_SIZE` unchanged at 2168 — `REPLACEMENT_ACCEPT`
   reuses the existing `EffectChoice` range; `PASS` (62) is
   decline. No tensor/mask regression.

**Coverage:** `digimon-engine/tests/replacements/` (55 tests across
`dispatcher_core`, `dispatcher_guard`, `deletion_replacements`,
`route_replacements`, `native_keywords`, `passive_modifier_migration`,
`enum_and_context`, `behavioral_end_to_end`).

**When Python retires:** all replacement-semantics cards (~60 from
the cross-archetype audit) become Rust-only from their first
implementation; there is no Python port and no dual-engine
parity to maintain.

---

## 13. Phase 8 Training sideways inheritance — scope looseness

### 13.1 🟡 Training `.inherited()` sideways scan — broader scope than spec

Rust Task 5 (2026-04-21) implemented Training `.inherited()` sideways scan
with broader scope than spec: fires on any same-owner permanent's timing
dispatch, not just breeding permanent's. This is due to the engine
currently not exposing a `TriggerSource::BreedingArea`. No printed Training
card ships in the v1 card pool today, so the deviation is latent.

Refinement required: once breeding-area timing dispatch is added, tighten
the scan at `digimon-engine/src/effect_queue.rs` (Phase 8 Task 5 sideways
scan) to gate on source-is-breeding-perm. Python side: Python implements
Training with targeted inheritance (breeding-specific); Rust is wider.
