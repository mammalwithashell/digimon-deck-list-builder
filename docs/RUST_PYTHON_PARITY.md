# Rust ↔ Python Engine Parity Tracker

**Role:** Rust is the target engine; Python is retained only until card-script migration completes. This tracker exists to catalog divergences during the transition and will be retired when the Python engine is. Always consult this file before editing engine code in either language — it is the authoritative source for known behavioral differences and per-phase progress.

**Purpose:** Catalog every known behavioral divergence between the Rust `code/digimon-engine/` and the sunset Python `code/engine_py_legacy/engine/`, so that Phase 9 (PyO3 bindings) and any future cross-engine validation have a checklist to work against.

**Scope:** Semantic differences in game state evolution given identical inputs. Architectural differences (e.g. compile-time vs dynamic effect registration) are listed separately and are not bugs.

**Reading guide:**

- 🔴 **Parity-breaking** — given the same inputs, the two engines produce different game states. Must fix before claiming cross-engine correctness.
- 🟡 **Mask/tensor drift** — the model sees different observations or valid actions on at least one engine, but the game state itself can still evolve. Will degrade model transfer quality.
- 🟢 **Equivalent** — explicitly verified to match (recorded here so nobody re-investigates).
- ⚪ **By-design difference** — different implementations with the same observable outcome.

Each entry cites the canonical source lines so divergences can be rechecked after either engine evolves.

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. The Rust engine added Track A `ProvenanceToken` (PR #451),
> Track B replacement framework (PR #449), Track C modifier taxonomy +
> typed `ModifierPayload` (PRs #452, #455), Track D centralized attack
> flow (PR #450), Track E zone-movement helpers + owner-routing fix
> (PR #453), Track E DSL verbs (PR #454), Track G keyword library
> close (PR #457), and the `Expiry::UntilCondition` continuous
> controller (PR #458). The PyO3 binding surface
> (`code/digimon-engine-py`) preserves the Python 1/2 ↔ Rust 0/1
> player-ID convention; no parity entries were affected by this batch
> (no Python-side card-script changes; the Python engine is sunset
> reference). The pre-scaling cleanup batch (§2) added owner-routing
> live coverage in `tests/owner_routing_live.rs`; that test exercises
> Rust-only zone helpers and does not interact with the Python engine.

---

## 1. Core game flow

### 1.1 🟢 Play cost deducted from memory — implemented

**Python** — [player.py](../code/engine_py_legacy/engine/core/player.py) + game flow: `calculate_play_cost` resolves the effective cost (including reductions), the player's play path deducts it from memory before placement, and any OnPlay effects resolve after the card is on the field.

**Rust** — [game_actions.rs:63-91](../code/digimon-engine/src/game_actions.rs#L63) `play_from_hand_with_cost` computes the effective cost via `cost_delta.resolve(printed_cost)`, then calls `self.pay_memory(effective_cost)` at line 88 before removing the card from hand. If `pay_memory` returns `false`, the function aborts with `None`. The old `play_from_hand` is now a thin wrapper that delegates to `play_from_hand_with_cost` with `CostDelta::Reduce(0)`. `OnPlay` fires through the standard effect queue after the permanent is placed.

**Coverage:** confirmed by Phase 2 engine tests and `test_cards_behavioral.rs` pilots.

### 1.2 🟢 Memory pure-negation on turn switch — implemented

**Python** — [game/__init__.py:322](../code/engine_py_legacy/engine/game/__init__.py#L322) `self.memory = -self.memory`. No clamp. The seesaw is simply flipped from the next player's perspective.

**Rust** — [game_phases.rs:129](../code/digimon-engine/src/game_phases.rs#L129) `rotate_turn_player` executes `self.memory = -self.memory;` with an explicit comment that no clamping is applied. Over-cost plays that push memory deep negative carry their full magnitude across the switch as positive memory for the next player — the intended tempo consequence.

**Coverage:** confirmed by the first-turn draw and turn-rotation tests; no regression introduced by Phase 2.

### 1.3 🟢 `pass_turn` preserves overflow — implemented

**Python** — [game/__init__.py:329-334](../code/engine_py_legacy/engine/game/__init__.py#L329): `if self.memory >= 0: self.memory = -3`. Only forces the seesaw if the player still has memory to give. Over-cost plays that already put memory negative are preserved through the switch.

**Rust** — [game_phases.rs:333-335](../code/digimon-engine/src/game_phases.rs#L333) `pass_turn` gates the assignment: `if self.memory >= 0 { self.memory = -3; }`. If memory is already negative because an over-cost play pushed it there, the overflow is preserved and carried through via the subsequent `end_turn()` call. Matches Python exactly.

### 1.4 🟢 `pay_memory` is a pure memory mutator — implemented

**Python** — turn end is checked in `check_turn_end` ([__init__.py:336](../code/engine_py_legacy/engine/game/__init__.py#L336)) after effect resolution, not synchronously with payment. This lets OnPlay, WhenDigivolving, etc. resolve on the same turn even if their cost already pushed memory negative.

**Rust** — [game.rs](../code/digimon-engine/src/game.rs) `pay_memory` is a pure mutator: it updates `self.memory`, emits a `MemoryChange` event, and returns `true`/`false` — it never calls `end_turn()`. A separate `check_turn_end` method is provided for callers to invoke at the natural resolution boundary after all effects of a play/action have resolved. `check_turn_end` also defers while `pending_selection` is installed; `resolve_generic_selection` re-runs the turn-end check after a Main-phase selection chain finishes. This keeps mandatory OnPlay choices, such as BT12-047's reveal buckets, visible in the action mask instead of parking the game in `EndTurn`. DCGO's `AutoProcessing.EndTurnCheck` / `EndTurnProcess` similarly waits for automatic processing and pending effect flow before the final turn handoff. This matches the Python contract and the intended DCGO phase flow.

**Coverage:** [tests/phase_flow/pending_selection_turn_end.rs](../code/digimon-engine/tests/phase_flow/pending_selection_turn_end.rs) covers memory crossing into a mandatory reveal-bucket selection, exposes `pending_selection.valid_action_ids` in the headless mask, verifies optional reveal prompts keep PASS legal, and confirms turn rotation after the selection resolves.

### 1.5 🟢 Memory swing-back on OnEndTurn — implemented

**Python** — [game/__init__.py:276-280](../code/engine_py_legacy/engine/game/__init__.py#L276): if `OnEndTurn` effects restore memory from `< 0` back to `>= 0`, the turn continues and returns to Main. Real DCGO rule, used by some cards.

**Rust** — [game_phases.rs:78-85](../code/digimon-engine/src/game_phases.rs#L78) `end_turn` captures `memory_before = self.memory` before firing `fire_end_of_your_turn(ending_player)`. After the drain, it checks `if memory_before < 0 && self.memory >= 0 && !self.game_over`: if the sign flipped back, the function sets `self.current_phase = GamePhase::Main` and returns immediately, short-circuiting the turn switch. This matches the Python swing-back rule exactly.

**Coverage:** [tests/end_turn_phase_transition.rs](../code/digimon-engine/tests/end_turn_phase_transition.rs) `swing_back_short_circuit` case.

### 1.6 🟢 Mulligan phase — implemented

**Python** — [game/__init__.py:109-156](../code/engine_py_legacy/engine/game/__init__.py#L109): after randomizing turn player, each player chooses keep/mulligan once; security is laid *after* mulligan.

**Rust** — [game.rs](../code/digimon-engine/src/game.rs) `Game::new` now shuffles `turn_order` via the seeded rng (first-player coin flip), draws opening hands, and initializes `mulligan_pending`/`mulligan_used`. `accept_mulligan(player, keep)` drives the state machine; the last decision triggers `finalize_mulligan`, which lays security and begins turn 1. `start_game` auto-keeps for every pending player, preserving backward compatibility with callers that don't care about mulligan.

**Action mask** — [mask.rs](../code/digimon-engine/src/action/mask.rs): during `GamePhase::Mulligan`, only the current decider sees any non-zero bits. Bit 0 (keep) is always set; bit 1 (mulligan) is suppressed after `mulligan_used[decider]` is true.

**Tauri surface** — `rust_mulligan_decide(keep)` command + `mulligan_current_player` / `mulligan_used` fields on `GameStateDto`. TypeScript adapter at [code/frontend/src/api/rustEngine.ts](../code/frontend/src/api/rustEngine.ts).

**Coverage:** [tests/mulligan.rs](../code/digimon-engine/tests/mulligan.rs) (new) and first-player draw skip regression in [tests/first_turn_draw.rs](../code/digimon-engine/tests/first_turn_draw.rs).

### 1.7 🟢 First-turn draw semantics — verified equivalent

**Python** — [game/__init__.py:228-230](../code/engine_py_legacy/engine/game/__init__.py#L228) `phase_draw`: `if self.turn_count == 1: pass`. Since `switch_turn` increments `turn_count`, P0's first turn is turn 1 and P1's first turn is turn 2. **Only P0 skips** — matches the standard Digimon TCG rule.

**Rust** — [game.rs](../code/digimon-engine/src/game.rs) with `SkipDraw::FirstPlayerOnly` (renamed from the misleading `P1Only`): skips draw when `turn_count == 1 && turn_player == 0`. Same behavior.

**Previous audit was wrong** — an earlier pass over this file reported a divergence that doesn't actually exist. Keeping the entry here to prevent a future auditor from reproducing the same mistake.

**Coverage:** [tests/first_turn_draw.rs](../code/digimon-engine/tests/first_turn_draw.rs) locks in the P0-skips / P1-draws-on-turn-2 / P0-draws-on-turn-3 rule.

### 1.8 🔴 RL cross-engine certification failures — observed 2026-04-28

The Python-side certification tests added in [test_rust_python_parity.py](../code/tests/rl/test_rust_python_parity.py) and [test_player_id_translation.py](../code/tests/rl/test_player_id_translation.py) currently expose several PyO3/Rust-backend blockers:

- Initial observation tensor mismatch at index `1`: Python reports `17.0`, Rust reports `3.0` for the same ST1 mirror deck and seed `12345`.
- Initial action-mask mismatch at indices `[0, 1, 60, 62]`: Python marks `[0, 1]` legal and `[60, 62]` illegal; Rust marks `[60, 62]` legal and `[0, 1]` illegal.
- Multi-step parity and player-ID translation tests cannot advance through `DigimonEnv.step()` on the Rust backend because `digimon_engine.RustHeadlessGame` does not expose the `.game` attribute that `DigimonEnv._compute_reward()` expects.
- `RustHeadlessGame.get_board_tensor(0)` and `get_board_tensor(3)` do not reject invalid Python player IDs, so the Python 1/2 ↔ Rust 0/1 boundary is not yet guarded.

**Disposition:** do not compensate in `digimon_gym`; fix the Rust/PyO3 backend surface and player-ID validation, then re-run `python -m pytest code/tests/rl/test_rust_python_parity.py code/tests/rl/test_player_id_translation.py -v`.

---

## 2. Combat & permanent state

### 2.1 🟢 Rush / Vortex summoning-sickness exemption — implemented

**Python** — [permanent.py:404-407](../code/engine_py_legacy/engine/core/permanent.py#L404): a permanent with `_is_rush` or invoked with `is_vortex=True` can attack the turn it arrived.

**Rust** — [combat.rs](../code/digimon-engine/src/combat.rs) `can_attack(handle, vortex)`, `attack_digimon(…, vortex)`, `attack_player(…, vortex)` all carry the `vortex: bool` flag. Summoning sickness short-circuits when `vortex` is true *or* `modifiers.has_keyword(handle, Keyword::Rush)` is true. The mask helper `can_basic_attack` in [mask.rs](../code/digimon-engine/src/action/mask.rs) checks modifier-granted Rush so the Main-phase mask agrees with the engine.

**Coverage:** [tests/rush_exemption.rs](../code/digimon-engine/tests/rush_exemption.rs) — `freshly_played_without_rush_cannot_attack`, `freshly_played_with_rush_can_attack`, `rush_does_not_override_suspended_state`, `freshly_played_with_vortex_can_attack`, `vortex_does_not_override_suspended_state`, `mask_allows_rush_granted_attack_on_turn_played`.

### 2.1b 🟢 Native (card-text) Rush parsed — implemented

Phase 3 added `CardData::keywords` field populated at load time by `parse_printed_keywords` (card_data.rs). The unified `Game::has_keyword` query (game.rs) checks both modifier-granted AND native printed keywords. All 14 call sites migrated; cards printing ＜Rush＞ now exempt the permanent from summoning sickness in `can_attack` without needing a granting modifier. See docs/RUST_ENGINE_API.md §Phase 3.

**Coverage:** `tests/keyword_parsing.rs` — `native_printed_rush_allows_same_turn_attack`.

### 2.2 🟢 `is_attacking` flag — implemented

**Python** — `permanent.is_attacking` is set to `True` at attack declare and cleared at attack end. Used by "Progress" (effect immunity while attacking) and observer effects.

**Rust** — `pub is_attacking: bool` field on [Permanent](../code/digimon-engine/src/permanent.rs). Set by `begin_attack` right after `PendingAttack` is installed; cleared by `cleanup_attack` alongside `modifiers.expire_end_of_attack()`.

**Coverage:** [tests/block_interrupt.rs](../code/digimon-engine/tests/block_interrupt.rs) `is_attacking_flag_lifecycle` — verifies the flag is live while the attack is parked on BlockTiming and cleared after resolution.

### 2.3 🟢 Combat interrupt phases — implemented

**Python** — full interrupt state machine in [combat.py](../code/engine_py_legacy/engine/game/combat.py): Counter → Block → Alliance phases pause the attack flow, require defender input, and resume via `_continue_attack_*` helpers.

**Rust** — [combat.rs](../code/digimon-engine/src/combat.rs) is a state machine: `attack_digimon`/`attack_player` are wrappers over `begin_attack(attacker, AttackTarget, vortex)`, which installs `PendingAttack` and calls `advance_pending_attack`. The state progression is `Declared → AllianceOpen → CounterOpen → BlockOpen → Battle → Cleanup`, pausing on a `PendingSelection` at each open-window state that has candidates.

- **Alliance** ✅ implemented. `try_enter_alliance` scans the attacker's side for unsuspended allies with modifier-granted `Keyword::Alliance`. Declaration grants attacker +ally_dp and +1 security attack (both EndOfAttack expiry) and suspends the ally. Trait-matching refinement (Alliance only fires when ally shares a trait with attacker) is blocked on the trait-parsing infrastructure noted in §2.1b.
- **Counter** ✅ implemented. `try_enter_counter` scans the defender's hand for cards whose effects set `blast_digivolve = true` and pairs each against valid field-digivolve targets via `Game::can_digivolve`. Declaration stacks the card onto the target (zero memory), fires `WhenDigivolving` via the effect queue, then advances to BlockOpen — unless the attacker was deleted mid-counter, in which case the state machine skips to Cleanup (matches DCGO `AttackProcess.cs:301`). **Digimon-target attacks only** — matches Python `combat.py:139`, which scopes Counter to Digimon targets. `OnCounterTiming` (Python's pre-WhenDigivolving counter-specific trigger) is intentionally deferred — no pilot card uses it yet.
- **Block** ✅ implemented. `try_enter_block` scans defender's battle area for unsuspended Blocker-keyword Digimon. Declaration rewrites `effective_target` to the blocker; `resolve_pending_battle` reads `effective_target`, so the redirect works for both Digimon and Player attacks (a blocker on a player attack cancels the security loop and runs a Digimon battle against the blocker instead). **Collision** (attacker-side keyword) expands the candidate pool — when the attacker has `Keyword::Collision`, every unsuspended opponent Digimon is treated as having Blocker for this attack, matching Python `permanent.py::can_be_blocker:502`.
- **Vortex** short-circuits directly to Battle after OnAttack (skips every interrupt window — matches DCGO).

New return variant `AttackResult::InProgress` signals that an attack is parked on a `PendingSelection`; the terminal outcome arrives once the resolution chain completes.

**Coverage:** [tests/block_interrupt.rs](../code/digimon-engine/tests/block_interrupt.rs) (10 cases); [tests/alliance_interrupt.rs](../code/digimon-engine/tests/alliance_interrupt.rs) (7 cases); [tests/counter_interrupt.rs](../code/digimon-engine/tests/counter_interrupt.rs) (12 cases: no-candidates baseline, invalid-pairing skip, prompt install, mask rendering, decline, declaration + stack growth + `WhenDigivolving` firing, attacker-delete cascade to Cleanup, Vortex bypass, Counter → Block sequence, player-target attack skips Counter, wrong-player rejection, encode/decode round-trip).

### 2.3-residual 🟡 `OnCounterTiming` distinct timing

Python's `_decode_counter` ([action_decoder.py:268-269](../code/engine_py_legacy/engine/game/action_decoder.py#L268)) fires `OnCounterTiming` *before* `WhenDigivolving`. Rust only fires `WhenDigivolving` today. Adding the distinct timing is a small surface change (new `EffectTiming::OnCounterTiming` variant + one `enqueue_triggered` call in `execute_blast_digivolve`) — deferred until the first card script actually uses it.

### 2.4 🟡 Security Digimon tie rule

**Rust** — [combat.rs:234-247](../code/digimon-engine/src/combat.rs#L234): if attacker DP ≥ security Digimon DP, attacker survives (security is trashed).

**Python** — defers to `Player.security_attack` which returns an `AttackResolution`. Needs cross-check that ties favor the attacker identically.

**Verification needed:** write a test with equal-DP security and attacker, assert outcome matches Python's `AttackResolution::AttackerSurvives` / whatever it produces.

### 2.5 🟡 Security effect execution — mostly implemented

The security-effect pipeline is closed end-to-end for the core flow: trigger → process → Digimon-vs-security DP battle (with DontBattleSecurityDigimon skip + inherited-stack DP adjustments) → OnSecurityCheck observer drain → OnLoseSecurity drain → trash-or-play disposition → OnOpponentSecurityRemoved observer drain (with selection-pause support). Selections installed at any drain site park the phase machine via `SecurityResolutionState` and resume through `advance_security_resolution`, so real-card security effects with prompts resolve correctly.

**Known correctness residuals:**
- §2.5c Progress / ImmunityToOpponentEffects — Rust correct (Phase A + B); selection-filter exclusion + every opponent-sourced `EffectContext` mutation entry point gated. Python sunsetted with the SecuritySkill-skip bug intact. Tracked long-term in [DCGO_KEYWORD_PARITY.md](DCGO_KEYWORD_PARITY.md).
- `§2.5-harness` cross-engine YAML parity harness — deferred tooling, not a correctness blocker.
- §2.5m `security_reveal` event emission — UI/replay surface, not a gameplay concern.

**Python** — [player.py:556-699](../code/engine_py_legacy/engine/core/player.py#L556) `Player.security_attack` + [combat.py:188-221](../code/engine_py_legacy/engine/game/combat.py#L188) `_execute_security_checks`: pops the top security card, fires `EffectTiming.SecuritySkill` effects with `is_security_effect=True` against a context dict carrying `{game, player, card, attacker, security_digimon, turn_player, opponent_player}`, fires `OnSecurityCheck` globally after effects, runs the Digimon-vs-security DP battle (with `DONT_BATTLE_SECURITY_DIGIMON` skip, `_applies_to_opponent_security_digimon` DP adjustments, and native `_is_jamming` attacker protection), and routes the revealed card to trash unless an effect flipped `card._security_played = True` via `Game.effect_play_from_security`. `OnLoseSecurity` fires unconditionally at the end.

**Rust** — [combat.rs:882](../code/digimon-engine/src/combat.rs#L882) `resolve_security_card` parks the popped card in the new `Game.pending_security: Option<PendingSecurity>` slot ([selection.rs](../code/digimon-engine/src/selection.rs)), enqueues `EffectTiming::SecuritySkill` via a new `TriggerSource::SecurityRevealed { defender, card }` variant, drains the queue, runs the Digimon DP battle (with `Keyword::Jamming` attacker-survival gate), fires `EffectTiming::OnLoseSecurity`, and trashes the card unless `pending_security.played == true`. `EffectContext::play_from_security` is the sole way to raise the `played` bit — mirrors Python's `effect_play_from_security`.

Three `EffectTiming` variants replace the former single `SecurityEffect` entry, matching Python:
- `SecuritySkill` — per-card trigger on reveal (Python timing 38).
- `OnSecurityCheck` — observer timing fired after the revealed card's effects (Python timing 35). Infrastructure present; **not yet wired in `resolve_security_card` — see §2.5b.**
- `OnLoseSecurity` — fires when the card leaves the security stack (Python timing 19). Wired.

New `EffectContext` helpers landed alongside: `play_from_security`, `select_security`, `select_reveal`, `mark_security_face_up` — also closing §4.6d-residual's `SelectSecurity` / `SelectReveal` gap for arbitrary non-security effects. Action-ID ranges mirror Python's `SEL_REVEALED_START=30`, `SEL_MY_SECURITY_START=40`, `SEL_OPP_SECURITY_START=50` (phase-disambiguated sub-ranges of the shared 30-59 HAND_EFFECT space).

**Coverage:** Rust side — [tests/security_effects.rs](../code/digimon-engine/tests/security_effects.rs) (5 cases: TEST-020 draw, TEST-021 play-from-security, TEST-022 memory gain, no-effect trash regression, `pending_security` cleared post-check). Python side — [tests/behavioral/synthetic/test_security_pilots.py](../tests/behavioral/synthetic/test_security_pilots.py) (8 cases across the same three pilots + script registration).

**Pilot cards** — synthetic TEST-020 / TEST-021 / TEST-022 live in both engines:
- Rust: [code/digimon-engine/src/cards/test_cards.rs](../code/digimon-engine/src/cards/test_cards.rs) (structs Test020/Test021/Test022).
- Python: [scripts/test/test_020.py](../code/engine_py_legacy/engine/data/scripts/test/test_020.py), [test_021.py](../code/engine_py_legacy/engine/data/scripts/test/test_021.py), [test_022.py](../code/engine_py_legacy/engine/data/scripts/test/test_022.py) + entries 4083/4084/4085 in [cards.json](../data/cards.json).

### 2.5a 🟢 Basic `SecuritySkill` dispatch — implemented

Enqueue + drain of the revealed card's `SecuritySkill` effects; `play_from_security` + `pending_security.played` flag; `OnLoseSecurity` fires unconditionally. Covered by the three pilots above. Any security effect that (a) is unconditional, (b) needs no context beyond `self.player`/`game`, and (c) installs no pending selection, now behaves identically in both engines.

### 2.5b 🟢 `OnSecurityCheck` observer timing fired — implemented

**Python** — [combat.py:206-214](../code/engine_py_legacy/engine/game/combat.py#L206) `_execute_security_checks` calls `execute_effects(EffectTiming.OnSecurityCheck, sec_check_ctx)` after each reveal-and-resolve pass. Field effects that watch for security checks globally (e.g. "When your security is checked, gain 1 memory") rely on this timing.

**Rust** — [combat.rs:949-955](../code/digimon-engine/src/combat.rs#L949) the `OnSecurityCheckDrain` security phase builds a `TriggerSource::OnSecurityCheck { attacker, defender, revealed_card, was_face_up }` and calls `self.enqueue_triggered(EffectTiming::OnSecurityCheck, trigger)`. [effect_queue.rs:65](../code/digimon-engine/src/effect_queue.rs#L65) dispatches `TriggerSource::OnSecurityCheck` by iterating the defender's entire `battle_area` and calling `enqueue_from_permanent` for each permanent — matching the Python fan-out. After draining, the state machine advances past `OnSecurityCheckDrain`.

**Note:** Non-combat security removal (effect-driven security trashing) does not yet emit this timing — that path is tracked in the engine-gaps doc under "Global `OnOpponentSecurityRemoved` observer timing". The attack-path dispatch (§2.5b) is confirmed equivalent.

**Coverage:** [tests/security_effects.rs](../code/digimon-engine/tests/security_effects.rs) security-observer pilot cases.

### 2.5c 🟢 Progress / immunity-to-opponent-effects — Rust correct (Phase A + B), Python incorrect (sunsetted)

**Python** — [player.py:614-617](../code/engine_py_legacy/engine/core/player.py#L614) `if attacker.is_immune_to_opponent_effects: ... else: <fire SecuritySkill effects>`. Python entirely skips the defender's `SecuritySkill` phase when the attacker has Progress. This is incorrect per DCGO [`Progress.cs`](../DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Progress.cs) and RULES_CONTEXT 16-38. Not back-ported; Python is the transitional engine.

**Rust** — Phase A landed the selection-filter half: the wrong `SecuritySkillDrain` gate was never re-introduced after revert, and `Game::progress_excludes` gates `select_opponent_permanent` to exclude a `Progress + is_attacking` target from opponent-sourced selections. Phase B (2026-04-24) closed the mutation-site half: every opponent-sourced `EffectContext` mutation entry point — `ctx.delete_permanent`, `ctx.return_to_hand`, `ctx.return_to_deck`, `ctx.de_digivolve` (covering both the all-pops and `amount=Some(N)` N-pop forms), `ctx.suspend`, and `ctx.add_dp_modifier` / `ctx.add_modifier` — now hard-gates on `progress_excludes` (the modifier path was subsequently broadened to every `ModifierType` and every value — see §2.5c-E). Rule-driven mutations (own-sourced deletes, security-check redirects, cost trash) flow through unchanged because the gate keys on `self.player` vs the target's controller. `Keyword::Progress` and `ModifierType::ImmunityToOpponentEffects` remain the correct primitives.

**Status:** Rust correct (Phase A + B); Python sunsetted.

Example: Digital Gate Open's `[Security]` ("Play 1 Digimon with cost ≤3 from hand or trash free; add this card to the hand") has no attacker-targeting clause, so its effect must still fire even when the attacker has Progress. Mega Death's `[Security]` ("delete 1 opp Digimon with cost ≤5") does target, so its selection pool would exclude the Progress attacker — but the prompt still installs and the defender may pick a different target.

Tracked long-term in [DCGO_KEYWORD_PARITY.md](DCGO_KEYWORD_PARITY.md) under "Progress".

### 2.5c-E 🟢 Progress gate scope (Rust broader than Python) — Phase E prep

**Rust** ([`code/digimon-engine/src/effect_context/mod.rs::add_modifier`](../code/digimon-engine/src/effect_context/mod.rs)) — unconditional `progress_excludes` short-circuit at the top of `add_modifier`. Every opponent-sourced modifier against the Progress attacker is suppressed regardless of `ModifierType` variant or sign — including positive `ChangeDp` / `ChangeBaseDp` buffs, lockdown variants (`CannotAttack`, `CannotUnsuspend`, `DontHaveDp`, `SecurityAttackChange`), and even notionally-protective modifiers (e.g. `CannotBeDestroyedByEffect`) sourced by the opponent. `add_dp_modifier` is a thin pass-through. Matches DCGO's [`Progress.cs:99`](../DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Progress.cs#L99) `IsOpponentEffect` `SkillCondition` literal — purely a source-controller check, hostility-blind and sign-blind. Mirrors the `targetPermanent.TopCard.CanNotBeAffected(activateClass)` check that every [`GiveEffectToPermanent/*.cs`](../DCGO/Assets/Scripts/Script/CardEffectCommons/GiveEffect/GiveEffectToPermanent) helper performs.

**Python** (`code/engine_py_legacy/engine/...`) — no Progress gate at the modifier sites. Opponent-sourced positive-DP buffs and protective modifiers land on the Progress attacker. (Same Phase-B-style hostility-blind gap that Rust just closed; additionally Python's broader Progress handling is incorrect per §2.5c.)

**Disposition:** deliberate divergence — Rust matches DCGO literally; Python does not. Reverts the Phase B "positive DP buffs still apply" sanity carve-out (the `opponent_effect_positive_dp_still_applies_to_progress_attacker` test was flipped to its DCGO-faithful inverse). Hostility classification was rejected as the alternative because every new `ModifierType` variant would need re-classifying and protective-modifier riders sourced by opponents would silently leak. This row retires when the Python engine is retired.

**Coverage:** [tests/combat/progress_mutation_gates.rs](../code/digimon-engine/tests/combat/progress_mutation_gates.rs) — 19 tests covering positive/negative DP, `ChangeBaseDp` (both signs), `SecurityAttackChange`, `CannotAttack`, `CannotUnsuspend`, `DontHaveDp`, opponent-granted `CannotBeDestroyedByEffect`, plus own-sourced negative controls (own buffs and own lockdowns still install on the carrier).

### 2.5d 🟢 `DontBattleSecurityDigimon` modifier — implemented

**Python** — [player.py:644-650](../code/engine_py_legacy/engine/core/player.py#L644) checks `modifiers.has_modifier(attacker, ModifierType.DONT_BATTLE_SECURITY_DIGIMON)` and skips the Digimon-vs-security battle entirely when set.

**Rust** — [combat.rs BattleResolved arm](../code/digimon-engine/src/combat.rs) gates the `CardKind::Digimon` DP-compare block on `!self.modifiers.has(attacker, ModifierType::DontBattleSecurityDigimon)`. Security card still leaves the stack and trashes; only the DP battle is skipped.

**Coverage:** [tests/combat/security_effects.rs](../code/digimon-engine/tests/combat/security_effects.rs) — `dont_battle_security_digimon_skips_dp_compare`, `without_dont_battle_modifier_attacker_dies_to_higher_dp_security`.

### 2.5e 🟢 Inherited-stack DP adjustments to security Digimon — implemented

**Python** — [player.py:654-666](../code/engine_py_legacy/engine/core/player.py#L654) iterates the attacker's inherited effects for `_applies_to_opponent_security_digimon + dp_modifier != 0`, adjusts `s_dp` before comparing.

**Rust** — `Effect::applies_to_opponent_security_dp` flag ([effect.rs](../code/digimon-engine/src/effect.rs)) + `Game::attacker_security_dp_adjustment` helper ([combat.rs](../code/digimon-engine/src/combat.rs)) walks the attacker's full `card_sources` stack, sums `dp_modifier` on any effect carrying the flag, and the `BattleResolved` arm adds the total to the raw security DP via `saturating_add` before the compare. Matches Python's sign convention (positive `dp_modifier` raises security DP; negative favors the attacker).

**Coverage:** [tests/combat/security_effects.rs](../code/digimon-engine/tests/combat/security_effects.rs) — `inherited_applies_to_opponent_security_dp_adjusts_battle` via TEST-027 pilot.

### 2.5f 🟢 Native Jamming honored — implemented

Phase 3 landed unified keyword lookup (see §2.1b). The security DP battle in `combat.rs` now checks `self.has_keyword(attacker, Keyword::Jamming)` which includes native printed Jamming. Cards with ＜Jamming＞ printed on their face survive losing security battles without needing a granting modifier.

**Coverage:** `tests/keyword_parsing.rs` — `native_printed_jamming_survives_losing_security_battle`.

### 2.5g 🟢 EffectContext security sugar — implemented

**Python** — security-effect context dict passed to each callback ([player.py:622-632](../code/engine_py_legacy/engine/core/player.py#L622)):
```
{ game, player, permanent=None, card, security_digimon, attacker, turn_player, opponent_player }
```

**Rust** — `EffectContext` and `EffectReadContext` now expose `attacker() -> Option<PermanentHandle>`, `security_digimon() -> Option<CardHandle>`, and `turn_player_at_check() -> Option<PlayerId>` ([effect_context/mod.rs](../code/digimon-engine/src/effect_context/mod.rs)). All three read from `Game::security_resolution`, which is already populated by the security phase machine — no resolver changes required. Scripts now gate on attacker traits / security DP without reaching into `ctx.game`.

### 2.5h 🟢 Condition-check skip for `SecuritySkill` — implemented

**Python** — [player.py:619-622](../code/engine_py_legacy/engine/core/player.py#L619) iterates `effect_list(SecuritySkill)` without evaluating `effect.can_use_condition`.

**Rust** — [effect_queue.rs `run_queued_effect_inner`](../code/digimon-engine/src/effect_queue.rs) sets `let skip_condition = qe.timing == EffectTiming::SecuritySkill;` and bypasses the `effect.condition` check when set. Matches Python exactly. (Was tracked as open in an earlier revision of this doc; verified against code as part of §2.5 closure in 2026-04-24.)

### 2.5i 🟢 `TriggerOrder` suppression for single-source security — implemented

**Python** — fires multiple SecuritySkill effects in `effect_list` order with no prompt.

**Rust** — [effect_queue.rs drainer](../code/digimon-engine/src/effect_queue.rs) short-circuits `install_trigger_order_selection` when every queued effect in the bundle is timing `SecuritySkill` and shares the same `source_card`. In that case the bundle is drained in queue order (which mirrors effect-list order) with no prompt installed. Mixed-source bundles (e.g. a SecuritySkill plus an unrelated OnPlay observer firing into the same controller's queue) still install the prompt.

**Coverage:** [tests/combat/security_effects.rs](../code/digimon-engine/tests/combat/security_effects.rs) — `two_security_effects_same_source_auto_fire_in_order` via TEST-028 pilot.

### 2.5j 🟢 Selections inside security effects re-entrant with combat — implemented

**Python** — `security_attack` is re-entrant: an effect that installs a pending_selection (via `effect_play_from_zone`, `effect_select_opponent_permanent`, etc.) parks the selection; later resolution re-enters the combat flow via the attack state machine's selection-resume hooks.

**Rust** — the security phase machine (`SecurityPhase` in [selection.rs](../code/digimon-engine/src/selection.rs) + `drive_security_resolution` in [combat.rs](../code/digimon-engine/src/combat.rs)) parks on `pending_selection` at each drain site and resumes via `advance_security_resolution`, called unconditionally from [effect_queue.rs `resolve_generic_selection`](../code/digimon-engine/src/effect_queue.rs) after any selection's callback fires. `SecurityResolutionState` holds the defender, attacker, turn player, current phase, remaining checks, and running outcome across the pause. The original `Dispose` phase has been split into `Dispose` (trash + OnOpponentSecurityRemoved drain) and `DisposeFinalize` (post-observer terminal decision) so a selection installed by the OnOpponentSecurityRemoved observer resumes without re-enqueueing the observer.

**Coverage:** regression covered implicitly by the existing pilots (TEST-020/021/022) and the new pilots introduced with §2.5c/d/e/i closures.

### 2.5k 🟢 `face_up_security` cleared on reveal — implemented

**Python** — [player.py:575](../code/engine_py_legacy/engine/core/player.py#L575) `self.face_up_security.discard(security_card)`.

**Rust** — [combat.rs `pop_and_start_security_check`](../code/digimon-engine/src/combat.rs) calls `self.player_mut(defender).face_up_security.remove(&sec_card.card_index)` immediately after popping the revealed card, capturing the prior state into `was_face_up` for `OnSecurityCheck` observers.

### 2.5l 🟢 `last_security_reveal` snapshot stored — implemented

**Python** — [player.py:577-578](../code/engine_py_legacy/engine/core/player.py#L577) stashes the just-revealed card so subsequent observer contexts can inspect it.

**Rust** — `SecurityResolutionState` ([selection.rs](../code/digimon-engine/src/selection.rs)) carries `revealed_card`, `card_kind`, and `was_face_up` for the full duration of the security check. `Game::last_security_reveal` exposes the same shape to observer scripts. Satisfies the observer-context requirement without widening `pending_security`'s lifetime.

### 2.5m 🟡 `security_reveal` event not emitted

**Python** — [player.py:596-608](../code/engine_py_legacy/engine/core/player.py#L596) emits a rich `security_reveal` event (card id, name, remaining count, card type, DP, effect text) consumed by the UI logger and RL replay recorder.

**Rust** — no equivalent. The engine's event surface is thinner overall; this is one instance of a cross-cutting event-parity gap, not a security-specific one. Tracked here for visibility.

### 2.5-harness 🔴 Cross-engine YAML parity harness not built

Planned in [plan-out-the-security-sorted-storm.md](../.claude/plans/plan-out-the-security-sorted-storm.md) as a `scenario_runner` Rust binary + `tests/behavioral/test_parity_scenarios.py` pytest driver + `tests/scenarios/parity/security/*.yaml` fixture set, diffing JSON snapshots after each step. Deferred — scope is substantial (new Rust binary crate target, serde_yaml integration, JSON snapshot schema design, subprocess bridge, pytest parameterization). Parity is proved manually today via the per-engine pilot tests listed above.

---

## 3. Tensor encoding (1375 floats)

### 3.1 🟢 Source DP contributions — implemented

**Python** — [permanent.py:755-774](../code/engine_py_legacy/engine/core/permanent.py#L755) `source_dp_contribution()` sums DP modifiers on each inherited source, gated by `can_use_condition`.

**Rust** — `Game::source_dp_contribution(perm, source_index)` ([game.rs](../code/digimon-engine/src/game.rs)) mirrors the Python impl: iterates the single source's effects via `CardEffectRegistry`, applies the inherited-vs-top filter (`is_under == effect.inherited`), and evaluates each effect's condition via a read-only `EffectReadContext`. The tensor writes `source_dp_contribution / DP_NORM` at per-source offset +2 ([tensor.rs `write_slot`](../code/digimon-engine/src/tensor.rs)).

**Coverage:** [tests/tensor_helpers.rs](../code/digimon-engine/tests/tensor_helpers.rs) unit-tests the helper; [tests/tensor_source_contributions.rs](../code/digimon-engine/tests/tensor_source_contributions.rs) drives through `build_tensor` end-to-end including the digivolution-stack and memory-gated cases.

**Residual gap §3.1b:** linked-card effects are still not iterated — if a card's `dp_modifier` lives on a linked Option, it won't contribute. No current archetype needs this; will flag if it arises.

### 3.2 🟢 OPT state fields — implemented

**Python** — [tensor.py:158-159](../code/engine_py_legacy/engine/game/tensor.py#L158): `opt_total` and `opt_used` populate slot offsets +3/+4; `source_opt_state(src)` at each source's +1.

**Rust** — `Game::opt_total / opt_used / source_opt_state` ([game.rs](../code/digimon-engine/src/game.rs)) count effects with `max_per_turn > 0` across the permanent's stack with the same inherited/top filter, consulting `Permanent::effect_activations` to determine which have reached their cap this turn. Counters reset in `Permanent::new_turn` (via `Player::new_turn` during `begin_turn`). Tensor offsets +3/+4 write the raw counts, and per-source +1 writes the availability fraction — matching Python's `build_board_state_tensor`.

**Coverage:** Same tests as §3.1.

### 3.3 🟢 Face-down security visibility — implemented

**Python** — [tensor.py:178-183](../code/engine_py_legacy/engine/game/tensor.py#L178): only writes card IDs for positions in the `face_up_security` set; face-down stays 0.0.

**Rust** — [player.rs](../code/digimon-engine/src/player.rs) now carries `face_up_security: HashSet<u16>` (keyed by `CardSource.card_index`). The new `write_security_ids` helper in [tensor.rs](../code/digimon-engine/src/tensor.rs) mirrors Python's writer — face-down slots stay 0.0; only `card_index`es present in the set emit their registry index. Applied to both `OFF_MY_SECURITY` and `OFF_OPP_SECURITY` so cross-player reveal effects have a symmetric slot.

**Previous behavior was a hidden-info leak** — Rust wrote every my-security card ID, so an RL agent trained on the Rust tensor could "peek" at its own face-down security stack and play perfectly around its own security effects.

**Coverage:** [tests/tensor_hidden_info.rs](../code/digimon-engine/tests/tensor_hidden_info.rs) — `my_security_is_zero_by_default`, `opp_security_is_zero_by_default`, `my_security_visible_when_face_up`.

### 3.4 🟢 Revealed cards section (slots 1360-1369) — implemented

**Python** — [tensor.py:104](../code/engine_py_legacy/engine/game/tensor.py#L104) populates from `game.revealed_cards`.

**Rust** — `Game::revealed_cards: Vec<CardSource>` field ([game.rs](../code/digimon-engine/src/game.rs)) feeds the `OFF_REVEALED` slot via the existing `write_card_ids` helper. Cleared in `rotate_turn_player` so reveals don't leak across turns. No card effects populate the vec yet, but the scaffold is in place for reveal-from-deck / search effects.

**Coverage:** [tests/tensor_hidden_info.rs](../code/digimon-engine/tests/tensor_hidden_info.rs) — `revealed_cards_populates_offset`, `revealed_cards_cleared_on_turn_rotation`.

### 3.5 🟢 Selection context (slots 1371-1372) — implemented

**Python** — [tensor.py:108-120](../code/engine_py_legacy/engine/game/tensor.py#L108) writes phase value, valid_count, selecting_player if `pending_selection` is set.

**Rust** — [tensor.rs](../code/digimon-engine/src/tensor.rs) writes phase value at slot 1370 whenever the engine is in a selection / combat-interrupt phase; writes `valid_action_ids.len() / ACTION_SPACE_SIZE` at slot 1371 and `selecting_player` at slot 1372 whenever `pending_selection.is_some()` (covers both selection-phase parks and `TriggerOrder` prompts parked under `EffectChoice`).

**Coverage:** [tests/select_opponent_permanent.rs](../code/digimon-engine/tests/select_opponent_permanent.rs) `tensor_reports_valid_count_and_selecting_player`.

### 3.6 🟢 Verified equivalent

- Global section [0-9]: turn_count/30, phase, memory/10.
- DP normalization constant: `DP_NORM = 30000.0` in both.
- Hand / trash / breeding / empty-slot encoding (0.0).
- `compute_positions()` card-vs-scalar split matches `tensor_layout.py`.

---

## 4. Action mask (2192 bits)

### 4.1 🟢 Verified equivalent

- All action range constants: PLAY_HAND (0-29), HAND_EFFECT (30-59), HATCH (60), MOVE_FROM_BREEDING (61), PASS (62), DNA_DIGIVOLVE (63-92), ATTACK (100-399), DIGIVOLVE (400-999), FIELD_EFFECT (1000-1149), TRASH_EFFECT (1150-1194), SOURCE_SELECT (2000-2167), BREEDING_SOURCE_SELECT (2168-2191).
- `TARGETS_PER_ATTACKER = 15`, `FIELDS_PER_HAND = 15`, `SOURCES_PER_FIELD = 12`, `SECURITY_TARGET = 14`, `BREEDING_TARGET = 14`, `BREEDING_SOURCE_CARRIERS = 2`.
- Encode/decode formulas for attack, digivolve, field effect, source select, breeding-carrier source select.
- Total `ACTION_SPACE_SIZE = 2192`.

> **Task S1.3 (Rust-led, 2026-05-20):** the Rust engine appended a
> breeding-carrier source-selection sub-range (`2168..2192`), raising
> `ACTION_SPACE_SIZE` 2168 → 2192. The breeding-source *behavior* (selecting
> sources from a King Drasil breeding-area carrier) exists only in the Rust
> engine — the sunset Python engine has no such effect. The Python
> `ACTION_SPACE_SIZE` constant (`code/engine_py_legacy/engine/game/constants.py`)
> was bumped to 2192 purely to keep the mask **shape** in sync, so the
> transitional cross-engine parity harness (`tests/rl/test_rust_python_parity.py`)
> still compares same-length masks; the trailing 24 Python mask slots are
> always zero.

### 4.2 🟢 Option card color requirement — implemented

**Python** — [action_mask.py:77-99](../code/engine_py_legacy/engine/game/action_mask.py#L77): an Option card is only playable if the player has a matching-color Digimon or Tamer on field or a matching-color Digimon in breeding.

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) Play-cards loop calls `option_color_match_available(card, me, &card_data)`, which iterates the player's battle_area + breeding_area for a color-set intersection with the Option's colors.

**Coverage:** [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs) — `mask_option_requires_matching_color_on_field` (walks empty → wrong-color → matching-color), `mask_option_color_check_accepts_tamer`.

### 4.2b 🟡 Script-based color bypasses not yet honored

Python cards can set `card._match_color_requirement = False` or register a `_match_color_requirement_fn` callback (~10 cards, e.g. `ex1_071`, `lm_050`, `st20_15`). Rust's `CardData` has no such field and no scripting infra — these Options will be *over*-masked (Rust refuses to play them when Python would allow). Similarly, Python's `IGNORE_COLOR_REQUIREMENT` aura modifier has no `ModifierType` variant in Rust yet. Both await §4.5 effect-listing / card-scripting infra.

### 4.3 🟢 Blitz attack exception under negative memory — implemented

**Python** — [action_mask.py:107-114](../code/engine_py_legacy/engine/game/action_mask.py#L107): with `memory < 0`, a Blitz Digimon that digivolved this turn can still attack.

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs): the memory gate is per-attacker. `memory_ok = memory >= 0 || (turn_digivolved == turn_count && modifiers.has_keyword(handle, Keyword::Blitz))`.

**Coverage:** [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs) — `mask_blitz_can_attack_under_negative_memory`, `mask_blitz_without_digivolving_does_not_attack_under_negative_memory`.

### 4.3b 🟡 Native / static Blitz not parsed

Same pattern as §2.1b — only modifier-granted Blitz is honored because `CardData` has no `keywords` field. Native Blitz printed on a card's face awaits §4.5 effect-listing / keyword-parsing infra.

### 4.4 🟢 Raid target rule — implemented

**Python** — [action_mask.py:121-140](../code/engine_py_legacy/engine/game/action_mask.py#L121): unsuspended enemies are targetable if attacker has `CAN_ATTACK_UNSUSPENDED` (any unsuspended) or Raid (tied-for-highest-DP unsuspended).

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs): target loop precomputes the max effective DP across unsuspended enemy Digimon and emits attack bits for each target whose `effective_dp` equals the max under Raid; emits for every unsuspended target under `ModifierType::CanAttackUnsuspended`. DP tiebreak uses `Game::effective_dp` so `ChangeDp` modifiers are honored (slight improvement over Python's raw `.dp`).

**Coverage:** [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs) — `mask_raid_targets_highest_dp_unsuspended`, `mask_raid_allows_all_tied_for_highest`, `mask_can_attack_unsuspended_modifier_allows_all_unsuspended`.

### 4.5 🟡 Entire action categories ungenerated — partial

DNA digivolve plumbing has landed; Hand/Field/Trash `[Main]` effect masks remain blocked on effect-listing infrastructure.

### 4.5a 🟢 DNA digivolve mask — implemented

**Python** — [action_mask.py:161-166](../code/engine_py_legacy/engine/game/action_mask.py#L161): `if card.is_digimon and has_valid_dna_targets(card, me.battle_area): mask[63 + h] = 1.0`.

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) `GamePhase::Main` arm emits `DNA_DIGIVOLVE_START + hand_index` when the hand card's `CardData.dna_costs` is non-empty and [`dna_digivolve::has_valid_dna_targets`](../code/digimon-engine/src/dna_digivolve.rs) finds some pair of battle-area permanents satisfying any `DnaCost` entry in either ordering. Memory cost is NOT gated at mask-generation time — Python's `action_mask.py:161-166` emits the bit regardless of memory and defers the cost check to action execution. `text_contains` searches the concatenation of `effect_text + inherited_text + security_text` to match Python's `_perm_matches_dna_req`.

**Coverage:** [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs) — `mask_dna_digivolve_emits_when_valid_pair_exists`, `mask_dna_digivolve_accepts_either_ordering`, `mask_dna_digivolve_rejects_when_no_pair`, `mask_dna_digivolve_respects_memory_cost`, `mask_dna_digivolve_skips_cards_without_dna_costs`.

### 4.5b 🟡 `dna_costs` data-population pipeline

`CardData.dna_costs` is present and deserialized, but cards.json's ingest pipeline doesn't emit the field today. Every card loaded from production data has `dna_costs = []`, so the mask branch above never fires in actual games. Python populates DNA costs from per-card scripts; Rust needs the cross-language export pipeline to emit DNA costs (or an auxiliary `dna_costs.json` sidecar) before this work is meaningful at runtime.

### 4.5c 🟢 Hand / Field / Trash `[Main]` effect masks — implemented

**Python** — [action_mask.py:176-225](../code/engine_py_legacy/engine/game/action_mask.py#L176): iterates a card's effects and filters by `_is_hand_main` / `_is_field_main` / `_is_trash_main` bool flags.

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) Main-phase arm emits bits for three new zone-scoped timing variants:
- `EffectTiming::MainFromHand` → bits `HAND_EFFECT_START + h` (30-59)
- `EffectTiming::MainOnField` → bits `FIELD_EFFECT_START + i * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN` (1000-1149, sub-slot +2)
- `EffectTiming::MainFromTrash` → bits `TRASH_EFFECT_START + t` (1150-1194)

The effect-listing primitive is [`Game::effects_for_card(card_id, handle)`](../code/digimon-engine/src/game.rs) — analogous to Python's `CardSource.effect_list(timing)` but expressed Rust-idiomatically with the registry owned by `Game`. Callers filter on `effect.timing`. This primitive also unblocks §2.1b / §4.2b / §4.3b (native static keyword parsing) and §4.7e (DigiXros cost-reduction).

**Field [Main]** additionally enforces OPT via the existing per-permanent `activation_count((source_handle, slot))` map and applies the same `inherited == is_under` filter used by `source_dp_contribution` / `source_opt_state`, so the mask agrees with the tensor helpers.

**Coverage:** [tests/mask_main_effects_parity.rs](../code/digimon-engine/tests/mask_main_effects_parity.rs) — 12 cases across the three zones: emit/suppress by condition, first-match-wins per slot, inherited-only-when-under, OPT exhaustion via `record_activation`, and phase gating (all three bits stay 0 outside Main).

### 4.5c-residual 🟡 Hand / Trash per-turn activation counters

Rust (like Python's mask) does NOT currently track `_turn_activate_count` for Hand / Trash [Main] effects at mask-generation time — the effect's `can_use_condition` closure is the sole gate. When execution-side support lands (firing these activated actions and recording activation), we'll revisit whether to add a parallel activation map on `Player` keyed by `(CardHandle, slot)`. Field [Main] already uses `Permanent::effect_activations`.

### 4.5c-residual 🟢 Action execution for [Main] bits — implemented

**Rust** — [game.rs](../code/digimon-engine/src/game.rs) `Game::activate_hand_main(player, hand_index)`, `Game::activate_field_main(player, field_index)`, `Game::activate_trash_main(player, trash_index)` each walk the card's / permanent's effects in the same order the mask emits, apply the same condition / inherited / OPT filters, and fire the first match. Memory cost, card movement, and all other side effects are handled inside the effect's `process` closure — mirroring Python's `_execute_*_main_effect` (no upfront `pay_memory` call, matching Python's inline `player.add_memory(-cost)` model).

**Field activation recording:** `activate_field_main` calls `perm.record_activation(source_handle, slot as u8)` before invoking the process closure, using the same `(CardHandle, slot)` key the mask inspects via `perm.activation_count`. Mask ↔ decoder agreement is verified by a regression test (`mask_and_field_decoder_agree_on_opt_exhaustion`).

**Hand/Trash activation counters:** intentionally omitted. See §4.5c-residual 🟡 below — Python's mask doesn't gate on `_turn_activate_count` either, and the execution-side counter is a separate architectural item worth its own plan.

**Coverage:** [tests/action_main_effects_parity.rs](../code/digimon-engine/tests/action_main_effects_parity.rs) — 14 cases: fires / suppressions per zone (condition gate, OOB index, wrong timing), Field OPT exhaustion, Field inherited-filter, mask ↔ decoder consistency for both Field (OPT-aware) and Hand (no OPT).

### 4.5e 🟢 DNA digivolve execution — implemented

**User-action path** — `Game::initiate_dna_digivolve` ([game_actions.rs:2111](../code/digimon-engine/src/game_actions.rs#L2111)) installs a two-stage `SelectionKind::Material` chain. Stage 1 picks the first material, stage 2 picks the second. Stage 2 resolution computes the matching `DnaCost` via `get_dna_stacking_order`, applies `BeforePayCost` reductions, and calls `Game::dna_digivolve_inner`.

**Engine-effect path** — `EffectContext::effect_initiated_dna_digivolve` ([effect_context/mod.rs](../code/digimon-engine/src/effect_context/mod.rs)) delegates to the same `Game::dna_digivolve_inner`. The wrapper handles the IR's `cost: i32` + `ignore_requirements: bool` shape and the pay-memory-bypass branch.

**Shared core** — `Game::dna_digivolve_inner` performs material consumption, hand-card consumption, stack merging, optional memory payment, optional digivolution-bonus draw, and trigger firing.

**Trigger surface** (both paths): `WhenDigivolving` (merged perm) → drain → `OnDigivolve` (global) → drain → `OnDnaDigivolve` (merged perm) → drain.

**Stack ordering** (canonical, both paths): `target_a.card_sources ++ target_b.card_sources ++ [from_hand]`. `target_a` corresponds to `DnaCost::requirement1`. Diverges from Python's `bottom + top + [evo]` order; Python is sunset and the printed rules don't specify intra-stack ordering between materials.

**Coverage:**
- `tests/effect_context/effect_initiated_dna_digivolve.rs` — engine-effect path (4 tests including `OnDnaDigivolve` firing)
- `tests/dna_digivolve_user_action.rs` — user-action path (4 tests covering two-stage flow, memory cost, draw bonus, phase rejection)
- `tests/dsl/phase2g_on_dna_digivolve.rs` — DSL `<OnDnaDigivolve>` clause from both paths

**Known DSL schema gap:** `CardSpec` does not yet support authoring `dna_costs` in YAML — DNA-digivolve cards must supply cost data via the `CardData` ingest path. The OnDnaDigivolve *clause* is fully expressible in DSL via `when: on_dna_digivolve`; only the cost-data side is missing. Tracked separately.

### 4.6 🟡 Interrupt-phase mask coverage — partial

End-of-turn surface is complete for mask parity — Vortex (§4.6a) + Overclock/MayAttack/ForceAttack (§4.6c) emission, plus phase transition (§4.6b) and `pass_end_of_turn_action` resumption. Overclock sacrifice *execution* landed with §4.6c-residual. Combat interrupts (§4.6d) support Alliance, Counter, and Block; selection helpers now cover `SelectTarget` / `SelectHand` / `SelectTrash` / `SelectMaterial` / `EffectChoice` / `TriggerOrder`. Remaining per-effect selection kinds (`SelectReveal` / `SelectSecurity` / `SelectSource`) track as §4.6d-residual follow-up work.

### 4.6a 🟢 Vortex mask emission — implemented

**Python** — [action_mask.py:321-335](../code/engine_py_legacy/engine/game/action_mask.py#L321): during `GamePhase.EndOfTurnAction`, permanents with `_is_vortex` and a passing `can_attack(is_vortex=True)` emit attack bits against any enemy Digimon.

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) `GamePhase::EndOfTurnAction` arm: mirrors Python via `modifiers.has_keyword(handle, Keyword::Vortex)` + `can_attack(handle, /* vortex = */ true)`. Any enemy Digimon (suspended or not) is a valid target.

**Coverage:** [tests/mask_end_of_turn_parity.rs](../code/digimon-engine/tests/mask_end_of_turn_parity.rs) — `mask_vortex_emits_attacks_in_end_of_turn_phase`, `mask_vortex_without_keyword_only_emits_pass`, `mask_vortex_bypasses_summoning_sickness`, `mask_vortex_targets_unsuspended_digimon_too`.

### 4.6b 🟢 Phase transition into `EndOfTurnAction` — implemented

**Python** — [game/__init__.py:294-324](../code/engine_py_legacy/engine/game/__init__.py#L294) `_complete_end_phase` parks in `GamePhase.EndOfTurnAction` when `_has_end_of_turn_keywords` returns true (Vortex / Overclock w/ sacrifice / MayAttack). Turn rotation defers until the player calls `next_phase` via PASS action 62.

**Rust** — [game.rs](../code/digimon-engine/src/game.rs) `Game::end_turn` mirrors the Python flow: fire OnEndTurn effects → swing-back check → `has_end_of_turn_keywords` → park in `EndOfTurnAction` or fall through to `rotate_turn_player`. `Game::pass_end_of_turn_action` resumes rotation. The turn-rotation tail of the old `end_turn` is extracted into a private `rotate_turn_player(ending_player)` helper so the resume path doesn't re-evaluate the EOT keyword check. `ModifierType::ForceAttack` is intentionally excluded from the EOT-park check (matches Python) — it's enforced Main-phase by §4.7d.

**Coverage:** [tests/end_turn_phase_transition.rs](../code/digimon-engine/tests/end_turn_phase_transition.rs) — 9 cases covering Vortex/Overclock/MayAttack parking, sacrifice-availability gating, suspended-MayAttack no-park, swing-back short-circuit, rotation resumption, and EOT-modifier expiry on resume.

### 4.6b-residual 🟢 Token detection — implemented

Rust's `CardKind` now includes `Token` (Phase 10). Tokens are
registered via `token_registry.rs` with synthetic `CardData` rows
absorbed into `game.card_data` at `Game::new`. Python's `is_token:
bool` flag and Rust's `CardKind::Token` are kept in sync at the PyO3
binding boundary (any helper that returns a token permanent
translates the flag appropriately).

### 4.6c 🟢 Overclock / MAY_ATTACK / FORCE_ATTACK mask bits — implemented

**Python** — [action_mask.py:354-389](../code/engine_py_legacy/engine/game/action_mask.py#L354): EndOfTurnAction branch emits Overclock at `1000 + i * EFFECTS_PER_PERM + 0`, MAY_ATTACK and FORCE_ATTACK attacks at `100 + i * TARGETS_PER_ATTACKER + j` (shared with normal attack range).

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) `GamePhase::EndOfTurnAction` arm folds Vortex/MayAttack/ForceAttack into a single target-loop shared with the §4.7a `CannotAttackTarget` filter; Vortex uses `can_attack(vortex=true)` and the other two use `can_attack(vortex=false)`. Overclock emits at sub-slot `FIELD_EFFECT_SLOT_FOR_OVERCLOCK` (=0) via the new `Game::has_overclock_sacrifice` helper.

**Coverage:** [tests/mask_end_of_turn_parity.rs](../code/digimon-engine/tests/mask_end_of_turn_parity.rs) extends with `mask_overclock_emits_sub_slot_0_bit_with_sacrifice_available`, `mask_overclock_suppressed_when_no_sacrifice`, `mask_may_attack_emits_attack_bits_against_digimon_and_security`, `mask_may_attack_respects_cannot_attack_target`, `mask_force_attack_emits_attack_bits_in_eot`.

### 4.6c-residual 🟢 Overclock sacrifice execution — implemented

**Python** — [action_decoder.py:501-522](../code/engine_py_legacy/engine/game/action_decoder.py#L501) `_initiate_overclock`: installs a `SelectTarget` prompt over Token-or-Digimon sacrifices on the turn player's field; the callback deletes the sacrifice and calls `resolve_attack(overclock_perm, opponent_player, without_suspend=True, return_phase=EndOfTurnAction)`. The attacker is not suspended.

**Rust** — [game.rs](../code/digimon-engine/src/game.rs) `Game::activate_overclock(overclock_index)` validates phase + keyword + sacrifice-availability, then installs an `OwnField` selection via direct `pending_selection = Some(...)` install (no `EffectContext` because this is an engine-level action, not an effect). The callback calls `Game::delete_permanent_with_effects(sacrifice)` then `Game::begin_attack_overclock(overclock_handle, AttackTarget::Player(opponent))`. Interrupts (Alliance / Counter / Block) still fire normally — only Vortex is uninterruptible per DCGO.

The suspend-skip flows through a new `is_overclock: bool` field on `PendingAttack` + a `begin_attack_overclock` constructor that delegates to a shared `begin_attack_impl(vortex, is_overclock)` private helper. When `is_overclock`, the declaration-time `suspend_and_count_attack` call is skipped; everything else (OnAttack triggers, state machine, interrupts, cleanup) matches the normal path.

`OverclockError::{WrongPhase, Busy, NotOverclock, NoSacrifice, InvalidIndex}` exposes the validation failures so callers (Tauri, tests, future Python bindings) can distinguish between them.

**Coverage:** [tests/overclock_execution.rs](../code/digimon-engine/tests/overclock_execution.rs) — 10 cases: prompt install, reject-without-keyword, reject-without-sacrifice, reject-wrong-phase, decline-leaves-state-untouched, full-flow sacrifice + security hit, full-flow wins game on empty security, low-level `begin_attack_overclock` skips suspend, regression guard on normal attack still suspending, higher-index sacrifice action-ID round-trip.

### 4.6d 🟡 Full interrupt / selection-phase mask builders — partial

Unified by PR3-PR5 into a single generic branch in [action/mask.rs](../code/digimon-engine/src/action/mask.rs) that reads `pending_selection.valid_action_ids` directly — mask correctness is now driven by the selection install site rather than a dedicated per-kind builder.

- ✅ `BlockTiming` — `combat.rs::try_enter_block` installs a selection with every unsuspended Blocker-keyword Digimon's action ID (`encode_attack(0, field_idx)`) + PASS (Block is always a may-trigger). Attacker's `Keyword::Collision` widens the pool to every unsuspended opponent Digimon.
- ✅ `AllianceTiming` — `combat.rs::try_enter_alliance` installs with every unsuspended Alliance-keyword ally.
- ✅ `CounterTiming` — `combat.rs::try_enter_counter` installs one action ID per valid `(hand, field)` blast pairing via `encode_digivolve(h, f)` + PASS. Blast candidates detected by `Effect.blast_digivolve` flag; field targets validated via `Game::can_digivolve` (color + level). Scoped to Digimon-target attacks (Python parity).
- ✅ `SelectTarget` (OppField / OwnField kinds) — [effect_context.rs](../code/digimon-engine/src/effect_context.rs) `select_opponent_permanent` / `select_own_permanent`; reuses the ATTACK target-half range. Pilot: [TEST-010](../code/digimon-engine/src/cards/test_cards.rs) (delete opp Digimon).
- ✅ `SelectHand` — `effect_context.rs::select_hand`, reuses PLAY_HAND 0-29. Pilot: [TEST-011](../code/digimon-engine/src/cards/test_cards.rs) (trash from hand, draw 2).
- ✅ `SelectTrash` — `effect_context.rs::select_trash`, reuses TRASH_EFFECT 1150-1194. No pilot card yet — infra validated by shared test scaffolding.
- ✅ `EffectChoice` — `effect_context.rs::select_effect_choice`, reuses HAND_EFFECT 30-59 with effect_choices labels. Pilot: [TEST-012](../code/digimon-engine/src/cards/test_cards.rs) (choose memory / draw).
- ✅ `TriggerOrder` (drainer-installed, parks under EffectChoice phase) — [effect_queue.rs](../code/digimon-engine/src/effect_queue.rs) `install_trigger_order_selection`; reuses HAND_EFFECT 30-59. Handles player-chosen ordering of simultaneous triggers, plus PASS=decline-all on all-optional bundles.
- ✅ `SelectMaterial` — `effect_context.rs::select_material`, uses SOURCE_SELECT (2000-2167) for a battle-area carrier or BREEDING_SOURCE_SELECT (2168-2191) for a breeding-area carrier (Task S1.3). Prompts the controller to pick a source (digivolution-stack card) from a target permanent. Covered by [tests/selection/material.rs](../code/digimon-engine/tests/selection/material.rs).
- 🟢 `SelectReveal` — `effect_context.rs::select_reveal`, reuses `SEL_REVEAL_START` 30-39. Landed alongside §2.5 (security pilot infrastructure).
- 🟢 `SelectSecurity` — `effect_context.rs::select_security`, reuses `SEL_MY_SECURITY_START` 40-49 (own) / `SEL_OPP_SECURITY_START` 50-59 (opponent). Landed alongside §2.5.
- 🔴 `SelectSource` — helper not yet authored. Infrastructure is uniform with the landed kinds; add when a card needs it.

**Coverage:** [tests/effect_queue_drainer.rs](../code/digimon-engine/tests/effect_queue_drainer.rs) (9 cases), [tests/select_opponent_permanent.rs](../code/digimon-engine/tests/select_opponent_permanent.rs) (10), [tests/selection_kinds.rs](../code/digimon-engine/tests/selection_kinds.rs) (7), [tests/select_material.rs](../code/digimon-engine/tests/select_material.rs) (7), [tests/block_interrupt.rs](../code/digimon-engine/tests/block_interrupt.rs) (10), [tests/alliance_interrupt.rs](../code/digimon-engine/tests/alliance_interrupt.rs) (7), [tests/counter_interrupt.rs](../code/digimon-engine/tests/counter_interrupt.rs) (12).

### 4.6d-residual 🟡 Remaining selection kinds

✅ `SelectReveal` and `SelectSecurity` helpers landed with §2.5 (see above). 🔴 `SelectSource` helper not yet authored — infrastructure is uniform with the landed kinds (share `install_field_selection` or an analogous encoder); add when a card needs it.

### 4.7 🟡 Modifier-gated mask checks — partial

Four of the five checks have landed; §4.7e (DigiXros cost-reduction) and per-action context discriminants (§4.7x) remain future work.

### 4.7a 🟢 CannotAttackTarget — implemented

**Python** — [action_mask.py:129-136](../code/engine_py_legacy/engine/game/action_mask.py#L129): `has_modifier(target, CANNOT_ATTACK_TARGET, {'attacker': attacker})` gates each Digimon-attack bit; same check repeats in Vortex / MAY_ATTACK / FORCE_ATTACK arms.

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) Main-phase Digimon-attack inner loop + `GamePhase::EndOfTurnAction` arm call `modifiers.has(t_handle, ModifierType::CannotAttackTarget)` and skip the target. Per-attacker discriminant is dropped — see §4.7x.

**Coverage:** `mask_cannot_attack_target_suppresses_digimon_attack_bit` in [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs); `mask_vortex_respects_cannot_attack_target` in [tests/mask_end_of_turn_parity.rs](../code/digimon-engine/tests/mask_end_of_turn_parity.rs).

### 4.7b 🟢 CannotDigivolve — implemented

**Python** — [action_mask.py:151-153](../code/engine_py_legacy/engine/game/action_mask.py#L151): `has_modifier(base_perm, CANNOT_DIGIVOLVE, {'digivolving_card': card})`.

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) Main-phase digivolve loop checks `modifiers.has(base_handle, ModifierType::CannotDigivolve)` before `can_basic_digivolve`. `digivolving_card` discriminant dropped (§4.7x).

**Coverage:** `mask_cannot_digivolve_suppresses_digivolve_bits_on_base` in [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs).

### 4.7c 🟢 CannotPlayFromHand — implemented

**Python** — [action_mask.py:58](../code/engine_py_legacy/engine/game/action_mask.py#L58) → `_is_play_blocked_by_modifier(card)` (effects.py:303-311).

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) Main-phase play-cards loop short-circuits when `modifiers.any_with_type(ModifierType::CannotPlayFromHand)` is true.

**Coverage:** `mask_cannot_play_from_hand_suppresses_all_hand_bits` in [tests/mask_main_parity.rs](../code/digimon-engine/tests/mask_main_parity.rs).

### 4.7d 🟢 FORCE_ATTACK — implemented

**Python** — [action_mask.py:227-280](../code/engine_py_legacy/engine/game/action_mask.py#L227): if any friendly Digimon has `ModifierType::FORCE_ATTACK`, every non-attack bit is zeroed and only those Digimons' attack bits remain. Falls through to the normal mask when no forced Digimon can legally act (all suspended, etc.).

**Rust** — [mask.rs](../code/digimon-engine/src/action/mask.rs) `apply_force_attack_mask_replacement` runs at the tail of the `GamePhase::Main` arm. Builds a fresh replacement mask, walks forced attackers through `can_basic_attack` + the same Raid / CanAttackUnsuspended / CannotAttackTarget filters the normal Main-phase attack loop uses, and `mask.copy_from_slice(&replacement)` when at least one attack bit was emitted. No memory gate on forced attackers (matches Python).

**Coverage:** [tests/mask_force_attack.rs](../code/digimon-engine/tests/mask_force_attack.rs) — 5 cases: non-attack bits zeroed when active, multiple forced Digimon all retain attacks, fall-through when forced attacker is suspended, CannotAttackTarget filtering, Raid-target tiebreak against unsuspended enemies.

### 4.7e 🔴 DigiXros cost-reduction — outstanding

Python's play-cost check (`action_mask.py:66-72`) computes `effective_cost = max(0, play_cost - max_reduction)` for cards with `digixros_cost`. Blocked on `CardData.digixros_cost` schema + `has_any_digixros_material` validator + ingest-pipeline data (same data-population shape as §4.5b). Own plan.

### 4.7x 🟡 Context-aware modifier queries — outstanding

Python's `has_modifier(target, type, context)` can refine the match via the modifier's `condition` closure — e.g. `CannotAttackTarget` that applies only to Red attackers, or `CannotDigivolve` that applies only when digivolving into a specific card. Rust's `ModifierEntry` ([modifiers.rs:13-19](../code/digimon-engine/src/modifiers.rs#L13)) has no condition closure, so §4.7a and §4.7b are unconditional (any active modifier blocks regardless of the attacker/digivolving_card discriminant). Adding condition closures is an architectural change worthy of its own plan.

---

## 5. Registry parity

### §5.1 Cost-reduction closures + pay_cost_fn hook — Rust-only (Phase 5)

**Status (2026-04-21):** Rust exclusively supports closure-valued cost reduction at `EffectTiming::BeforePayCost` and a synchronous `pay_cost_fn` hook on triggered effects (and at BeforePayCost dispatch). Python uses a `_temp_play_cost_reduction` instance variable that leaks across effects (Issue 24 per project memory). Rust **intentionally does not replicate** this pattern; scripts requiring dynamic reduction must use `.cost_reduction_fn`.

No Python parity — this is a strict improvement in Rust. Python will not catch up; migration targets Rust as the source of truth for these mechanics.

Cards unblocked (per audits): ~50 across Rocks (primary), some Dark Masters and TS Olympos cost-gating effects. See `.claude/plans/rust-engine-gaps-rocks.md` for the Rocks-specific list.

Rust implementation: `Game::scan_before_pay_cost_reduction` in `code/digimon-engine/src/game_actions.rs` + `pay_cost_fn` hook in `code/digimon-engine/src/effect_queue.rs::run_queued_effect`.

---

### §6.1 Player-scoped flood gates — Rust (Phase 6)

Rust adds a parallel `player_modifiers` tier to `ModifierRegistry` (`HashMap<PlayerId, Vec<PlayerModifierEntry>>`) plus 13 new `ModifierType` variants for action-category flood gates (`CannotPlayDigimonByEffect`, `CannotGainMemoryByEffect`, `CannotGainMemoryExceptFromTamers`, `CannotReducePlayCost`, `CannotActivateMainEffects`, `CannotActivateWhenDigivolvingEffects`, `CannotActivateSecurityEffects`, `CannotAddSecurityByEffect`, `CannotTrashOpponentSecurity`, `CannotReduceOpponentSecurity`, `CannotDrawByEffect`, `CannotDigivolveDigimonByEffect`, `IgnoreColorRequirement`). Gates are enforced at BOTH the action-mask layer (RL-visible suppression) and the resolver layer (defense-in-depth).

Python stores modifiers as a flat `HashMap<ModifierType, Vec<Entry>>` with closure-valued per-entry conditions. Rust v1 uses flag-based entries + card-script `.condition` closures at install-time, following DCGO's separate-class-per-restriction pattern (see `DCGO/Assets/Scripts/CardEffect/BT3/Green/BT3_046.cs` for Tamer-source-discriminated `CannotAddMemoryClass`). Phase 7 may add closure conditions to `ModifierEntry` for the would-replacement framework.

Python's `ctx.get('played_by_effect', False)` context is matched by Rust's typed `PlaySource` enum (`ByHand` / `ByEffect` / `ByDigivolve`), threaded through play/digivolve helpers — strictly cleaner than Python's dict-based context.

The `source_is_tamer` helper matches DCGO's `ICardEffect.IsTamerEffect` property; Rust uses a fast path via `source_permanent` + slow-path `card_kind` lookup. Used by `CannotGainMemoryExceptFromTamers` to pass memory gains originating from Tamer effects through the restriction gate.

Cards unblocked (per audits): ~55 across all 5 audited archetypes (Dark Masters lockout shell, Medusamon Petrification, TS Olympos Tamer-anchoring, Rocks Plug-In lockouts).

---

### 5.1 🟢 CardRegistry

Fixed in [card_registry.rs](../code/digimon-engine/src/card_registry.rs). `CardData.index` from cards.json is the source of truth in both engines. Verified by [card_registry_parity.rs](../code/digimon-engine/tests/card_registry_parity.rs) against the real 4082-card cards.json.

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

13. **§2.5 — Security effect execution** — mostly done (2026-04-24). Closed sub-items: §2.5a basic `SecuritySkill` dispatch, §2.5b `OnSecurityCheck` observer, §2.5d `DontBattleSecurityDigimon` modifier, §2.5e inherited-stack DP adjustments, §2.5f native Jamming (via Phase 3 keyword parsing), §2.5g `EffectContext` security sugar, §2.5h `SecuritySkill` condition-skip, §2.5i single-source `TriggerOrder` suppression, §2.5j selection re-entrancy via `SecurityResolutionState` + `Dispose`/`DisposeFinalize` split, §2.5k `face_up_security` cleared on reveal, §2.5l `last_security_reveal` snapshot. Remaining: **§2.5c Progress / ImmunityToOpponentEffects** — deferred; both engines currently diverge from printed rules (see [DCGO_KEYWORD_PARITY.md](DCGO_KEYWORD_PARITY.md) under "Progress"). Plus §2.5m `security_reveal` event (UI/replay only) and §2.5-harness cross-engine YAML parity harness — neither a correctness blocker for real-card ports.

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

### Curated `CardData` exposure across consumer surfaces (audited 2026-05-10)

The Rust `CardData` struct (`code/digimon-engine/src/card_data.rs`) is the
canonical card-metadata shape. Consumers expose intentionally narrowed
subsets — adding a field to `CardData` does **not** automatically propagate.
Audit performed after PR #457 flagged a Tauri build break from
`ace_overflow` / `digixros_aliases` (added in PRs #413 / #455) not being
mirrored on the desktop builder.

| Consumer | Surface | New-field policy |
|---|---|---|
| Tauri DTO | `code/src-tauri/src/engine_commands.rs::CardDto` | All `CardData` fields the desktop UI may render. `ace_overflow` and `digixros_aliases` exposed as of 2026-05-10. `card_dto` exhaustively destructures `CardData`, so the next field addition trips a compile error here. |
| PyO3 binding | `code/digimon-engine-py/src/lib.rs::PyCard` | Curated subset by design. `ace_overflow` / `digixros_aliases` deliberately not exposed — no Python caller currently reads them. Add `#[pyo3(get)]` accessors on demand when a Python consumer needs them. |
| Frontend types | `code/frontend/src/types/{game,cards}.ts` | Hosted-API path consumes `to_ui_json` (state-shape, not `CardData`-shape). Desktop frontend consumes the Tauri DTO via `invoke()` with looser typing — extra fields on `CardDto` are ignored, not rejected. |
| State filter | `code/server/state_filter.py` | Operates on `to_ui_json` output, not `CardData`. Card metadata flows in through hand/permanent shapes that already filter sensitive identity for opponents (`handIds`, `handCards` redaction per Working Rule 14). |
| RL env / observation tensor | `code/digimon_gym/digimon_gym.py`, engine tensor encoding | Tensor encodes a fixed feature schema; `ace_overflow` / `digixros_aliases` are not encoded. Adding them is a separate spec change (Working Rule 4). |

The structural drift detector for the Tauri layer is the exhaustive
destructure of `CardData` inside `card_dto`. The engine-side analog is
Rust's struct-literal exhaustiveness (which is what caught PR #457's
build break — by definition, every `CardData { ... }` constructor must
list every field). Construction-time drift detection is therefore
already free; consumption-time drift detection requires the explicit
destructure pattern. New consumers reading `CardData` should follow the
`card_dto` pattern.

---

## 8. Test strategy

For each 🔴 item, we want a paired test:

1. **Snapshot test** — construct an identical game state in both Python and Rust, step the same action, diff `to_dict()` / `to_json()`. Should be runnable from the workspace root.
2. **Unit test** — a Rust-only behavioral test in `code/digimon-engine/tests/` that locks the correct semantics in place after the fix.

`code/digimon-engine/tests/card_registry_parity.rs` is the template — it loads Python's authoritative data and asserts the Rust implementation agrees.

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
[`code/digimon-engine/src/inference/`](../code/digimon-engine/src/inference/) with
its own parity test suite (`tests/onnx_parity/`). This section covers
the heuristic side — the greedy bot — plus shape assumptions shared
across both engines.

### 10.1 🟡 Greedy heuristic re-implemented in Rust

**Python** — [`code/digimon_gym/digimon_gym.py::greedy_policy`](../code/digimon_gym/digimon_gym.py) inspects full game state (phases, hand/field, level/DP/cost) and prioritizes Digivolve > Attack > Play > Pass in Main and Hatch > Move > Pass in Breeding. Tie-breaks are deterministic by (level, DP, -cost, -hand_idx, -field_idx).

**Rust** — [`code/digimon-engine/src/policies/greedy.rs`](../code/digimon-engine/src/policies/greedy.rs) hand-ports the same baseline phase structure. `greedy_action(game, mask) -> u16` is wired into `PlayerKind::Greedy` in `code/src-tauri/src/engine_commands.rs::run_agent_steps`, replacing the pre-port `first_valid_action` placeholder. Rust intentionally refines setup sequencing: legal search/resource-flow plays and Tamers happen before keep-turn digivolves, legal Digimon plays prefer lower-level curve starters before expensive high-level hard-plays, attacks prefer security pressure before board trades after checking for lethal swings, and Breeding holds a vanilla Lv.3 when there is no matching evolution in hand. DSL-lowered effects mark resource-flow metadata from `Draw`/`AddToHandFrom*` steps, with printed text/keyword fallback for older hand-written effects. Python still sorts hand plays by raw play cost first.

**Parity hazard:** if Python's `greedy_policy()` changes (new tie-break rule, phase handling, archetype-specific logic), compare it against the Rust-owned heuristic instead of assuming exact lockstep. Any edit that should affect Rust greedy must be mirrored in Rust and covered by a deterministic behavioral test under [`code/digimon-engine/tests/policies/greedy.rs`](../code/digimon-engine/tests/policies/greedy.rs). The `self_play.rs` tripwire (20 seeds of greedy-vs-greedy to conclusion) catches gross breakage but not nuanced decision divergence.

### 10.2 🟡 ONNX inference shape contract

**Python** — [`code/engine_py_legacy/engine/onnx_policy.py`](../code/engine_py_legacy/engine/onnx_policy.py) binds input `"obs"` (shape `(1, TENSOR_SIZE)`) and output `"logits"` (shape `(1, ACTION_SPACE_SIZE)`). LSTM variant adds `h_in`/`c_in`/`h_out`/`c_out` at `(1, 1, 256)`.

**Rust** — [`code/digimon-engine/src/inference/`](../code/digimon-engine/src/inference/) binds the same names and asserts the same shapes at session-load time; the compatibility gate in [`code/src-tauri/src/models.rs`](../code/src-tauri/src/models.rs) rejects drifted models before the download starts.

**Historical drift (resolved):** pre-2026-04-18, `tools/export_onnx.py` hardcoded `obs=981 / logits=2120` — the pre-rewrite layout. Any `.onnx` on disk dated before the fix is unusable by either engine. Re-export from the original `.zip` checkpoint is mandatory; if that checkpoint was trained against the old layout, it must be retrained from scratch. The exporter now imports `TENSOR_SIZE` / `ACTION_SPACE_SIZE` from `digimon_gym.engine.game.constants` and raises before writing on any shape mismatch.

**Ongoing hazard:** any future change to [`code/digimon-engine/src/tensor.rs`](../code/digimon-engine/src/tensor.rs) (`TENSOR_SIZE`) or [`code/digimon-engine/src/action/space.rs`](../code/digimon-engine/src/action/space.rs) (`ACTION_SPACE_SIZE`) invalidates every bundled or cached `.onnx`. The compatibility gate in `models.rs` and the exporter's shape assertion together make this a loud error, not a silent regression — but re-exports of all live checkpoints are required whenever either constant changes.

## 11. Phase 10 — Tokens + De-Digivolve

### 11.1 🟢 Token creation + CardKind::Token — implemented

**Python** — `code/engine_py_legacy/engine/data/token_registry.py`: `TOKENS`
dict mapping token names to metadata; `create_token_card_source`
factory; `CardSource.is_token: bool` flag; `Permanent.is_token`
property; Game.effect_play_token for spawning.
`Player.delete_permanent` branches on `is_token` to skip trash
(`player.py:506`).

**Rust** — `code/digimon-engine/src/token_registry.rs` defines
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

**Coverage:** `code/digimon-engine/tests/cards_behavioral/tokens.rs` +
`code/digimon-engine/src/token_registry.rs` unit tests.

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

**Coverage:** `code/digimon-engine/tests/cards_behavioral/de_digivolve.rs`.

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
4. Phase 7's `REPLACEMENT_ACCEPT` did not change `ACTION_SPACE_SIZE`
   — it reuses the existing `EffectChoice` range; `PASS` (62) is
   decline. (Separately, Task S1.3 later raised `ACTION_SPACE_SIZE`
   2168 → 2192 by appending `BREEDING_SOURCE_SELECT`; see §4.)

**Coverage:** `code/digimon-engine/tests/replacements/` (55 tests across
`dispatcher_core`, `dispatcher_guard`, `deletion_replacements`,
`route_replacements`, `native_keywords`, `passive_modifier_migration`,
`enum_and_context`, `behavioral_end_to_end`).

**When Python retires:** all replacement-semantics cards (~60 from
the cross-archetype audit) become Rust-only from their first
implementation; there is no Python port and no dual-engine
parity to maintain.

---

## 13. Phase 8 Training sideways inheritance — bound-carrier refinement

### 13.1 🟡 Training `.inherited()` sideways scan — bound effects are carrier-scoped

Rust Task 6 (2026-05-02) refined Training `.inherited()` sideways scan for
bound Training Options. `OptionState::Training` stores an optional
`TrainingBinding` with the intended carrier handle plus the carrier's physical
top `CardHandle`; enqueue and queued-effect liveness both revalidate that the
current source permanent still matches the recorded carrier before resolving
the Training effect. This prevents owner-wide fan-out for bound Training,
duplicate-copy ambiguity, and stale field-index aliasing.

Remaining limitation: unbound Training (`trained: None`) keeps the earlier
compatibility behavior and can still scan same-owner battle-area timing
dispatches. The engine still lacks a first-class `TriggerSource::BreedingArea`
for generic breeding-area timing dispatch. Python side: Python implements
Training with targeted inheritance (breeding-specific); Rust now matches that
shape for explicitly bound Training but remains wider for unbound interim
state.

---

## 14. Option flow (Phase 8)

Phase 8 Option-card play flow landed 2026-04-21. See `docs/RUST_ENGINE_API.md` §Phase 8 for the full scripting surface.

### 14.1 🟢 Option subtype dispatch is now native

Rust now implements Option cards as a first-class play pipeline with
dedicated `OptionState` (Standard / Delayed / Linked / Training),
`PendingOption` single-slot pending state, and `OptionPlayResult` outcome
enum. `EffectTiming` gained `OnUseOption`, `OptionMain` (dispatched),
`DelayEffect`, `OnLink`, `OnLinkedCardTrashed`, `OnUnlink` (reserved), and
`OnTrainingTrash`. `EffectBuilder` gained `.option_main()`, `.delay(trigger)`,
`.link(cost, filter)`, `.training()`, `.linked()`.

Python's flag-based workarounds (`_option_stays_on_field`,
`_trash_option_after_resolution`, `_is_delay`, `_is_training`) are subsumed.
Python retains them for its own scripts, but new Option cards in the Rust
engine use the typed builder surface.

### 14.2 🟢 Linked cards / Plug-Ins faithful

Rust `Permanent.linked_cards: Vec<CardSource>` preserves Python's
`Permanent.linked_cards: List[CardSource]` semantics. Attach happens after
body drain via an explicit `LinkSelectHost` phase; detach-on-host-deletion
cascade routes each linked card to owner's trash (subject to constraint
§14.5). Sideways inheritance — `.linked()`-flagged effects on the attached
card fire off the host's timings — matches DCGO `OnLinkCardDiscarded`
ordering (`ICardEffect.cs:996`).

### 14.3 🟡 Cancel-semantics for non-Permanent trash-replacement subjects

Rust v1: when `WhenWouldBeTrashed` on a `Card` subject (hand-origin,
mid-Option-resolution) produces `Cancelled`, the card returns to owner's
hand. Cost was paid and `OptionMain` already fired, so the net effect is
"Option body resolved for free, card went back to hand". Python uses the
same hand-return convention (`hand_back_if_cancelled`). No engine
divergence, but the printed-rules outcome is spec-unclarified — flagged for
refinement if a real card triggers it. Tracked in API doc §Phase 8
constraint 1.

### 14.4 🟡 `Redirected(Deck)` / `Redirected(Hand)` use direct vec manipulation

Rust v1 uses `deck.insert(0, …)` / `hand.push(…)` directly on the
Card-subject commit path for `Redirected` outcomes. Spec §7.3 calls for
zone-mover helpers that fire nested observers. Python uses
`_return_card_to_hand` / `_place_at_bottom` helpers which do fire
observers. Latent divergence — no printed card today has a nested observer
on these paths, but a `WhenWouldBeReturnedToHand` installed by a card
observing its own redirected-to-hand disposal would see the miss. Tracked
in API doc §Phase 8 constraint 2; follow-up pass will migrate to helpers.

### 14.5 🟢 `OnLink` observer wired

Rust fires `OnLink` globally across both players after a Plug-In attaches
to its host. Required by Appmon-trait cards (BT21-053, BT21-054, BT21-059,
BT21-073, AD1-005) that observe "when a card is linked to a Digimon".
Python's `WhenLinked` behavior is preserved — body fires before observer,
matching DCGO ordering.

Linked-card host-deletion cascade does **not** fire `WhenWouldBeTrashed`
(too recursive during host deletion); v1 unconditionally trashes each
linked card. Python behaves identically today. Marked
`TODO(phase-8-followup)` in `combat.rs`; no printed card audited requires
the replacement window to fire on this path.

## 15. Combat interrupt completion (Phase 9)

Phase 9 combat-interrupt completion landed 2026-04-21. See
`docs/RUST_ENGINE_API.md` §Phase 9 for the full scripting surface.

### 15.1 🟢 `WhenWouldAttack` / `WhenWouldBeAttackTarget` dispatch

Both replacement timings were parsed and built in Phase 7 but reserved —
no fire-sites. Phase 9 wires dispatch at the top of `begin_attack_impl`:
`WhenWouldAttack` on the attacker, then `WhenWouldBeAttackTarget` on the
target, before `AllianceOpen`. Python fires the equivalent entry at
`combat.py:127-147` (`_emit_when_would_attack`). Parity reached.

### 15.2 🟢 `ctx.redirect_attack` + `ctx.cancel_attack`

Rust-side script helpers landed on `EffectContext` (§6.1 of spec). Both
validate `pending_attack` and return `AttackError::NoActiveAttack` /
`InvalidTarget`. `redirect_attack` fires `OnAttackTargetChange`;
`cancel_attack` short-circuits advance to `Cleanup`. Python's
`combat.py:102-125` exposes `redirect_attack` and a conceptually
equivalent cancellation path. Parity reached.

### 15.3 🟡 Counter hand-play — Rust leads

Rust Phase 9 supports three Counter-window shapes: Blast Digivolve
(pre-existing), Hand Counter Option (`.counter().option_main()` — NEW,
Phase 8 play_option_from_hand with a `CounterEffect` overlay), and Field
Counter Ability (`.counter().timing(CounterEffect)` on a permanent —
NEW). Python `combat.py:173-186` only dispatches blast-digivolve
candidates during the Counter window. Rust leads; a Python port is
required for full parity but is out of scope for the Rust pivot and
retirement-track — tracked as a parity follow-up.

### 15.4 🟡 Raid retarget — Rust leads

Rust Phase 9 adds `AttackState::PostBlock` with a Raid retarget rider:
if the attacker has `<Raid>` (native-printed OR modifier-granted — see
§15.9) AND the effective target has invalidated AND any legal retarget
exists, the engine installs a `PendingSelection::AttackRetarget`. Retarget candidate
set prefers unsuspended Digimon; suspended fallback only when no
unsuspended exist. Python does not have an equivalent retarget
interrupt. Rust leads.

Known v1 looseness: retarget candidate ordering (unsuspended-priority)
is stricter than declaration-time mask ordering. Logged in API doc
§Phase 9 constraint 4.

### 15.5 🟡 Collision MUST-block — Rust leads

Rust Phase 9 implements `<Collision>` as a mask-layer mandate: the
`AttackState::BlockOpen` mask builder flips `is_optional = false` on the
block selection and drops the PASS/no-block action bit. `CannotBlock`
still gates individual defenders before Collision elevates the choice
to mandatory (a CannotBlock defender is simply not a candidate). Python
`permanent.py:502` expands the Blocker-eligible set for `<Collision>`
but does not convert the opt-in block window into a mandatory one. Rust
leads.

### 15.6 🟢 Piercing post-battle security check

Rust Phase 9 adds `AttackState::PostBattle`: if the attacker survives,
the defender was a Digimon, the defender was wiped, and the attacker
has `<Piercing>`, enter a security check against the defending player
(standard `OnSecurityCheck` dispatch; one card). Piercing on
direct-player-attack does NOT fire — this is a post-Digimon-battle rule
only. Python has the equivalent post-battle check in
`combat.py:_resolve_piercing`. Parity reached.

### 15.7 🟢 Reboot unsuspend consumer

Rust Phase 9 wires `<Reboot>` into the opponent's unsuspend phase: at
the start of the opponent's unsuspend step, every Reboot permanent on
either battle area unsuspends, gated by `CannotUnsuspend` /
`CannotBeUnsuspendedByEffect`. Python has the equivalent consumer. Parity
reached.

### 15.8 🟡 `OnBlock` / `OnAllyAttack` / `OnOpponentAttack` dispatch

Rust Phase 9 fires all three: `OnBlock` via `TriggerSource::PlayerBattleArea`
fan-out after block declaration (both players' battle areas scanned);
`OnAllyAttack` on attacker-controller's OTHER permanents (attacker itself
filtered out structurally); `OnOpponentAttack` on opposing-controller
permanents. Python `combat.py:58-74` fires `OnAllyAttack`. `OnBlock` and
`OnOpponentAttack` do not have comparable Python fire-sites — Rust leads
on those two. Flagged for parity follow-up if Python is not retired
before cards depending on these observers migrate.

### 15.9 🟢 Native `<Raid>` keyword parsing

Printed `<Raid>` on the card face IS queryable via `has_keyword` (parsed
by Phase 3 into `CardData.keywords`; verified by
`tests/keyword_parsing.rs`). Phase 9's Raid retarget rider uses
`Game::has_keyword(pa.attacker, Keyword::Raid)` which honors both
native-printed AND modifier-granted Raid. No parity gap.

---

## 16. Refire attribution (Track K, 2026-05-10)

### 16.1 ⚪ Permanent-target refire source-card attribution

Rust `EffectContext::refire_effect_from_permanent` now routes through the
same permanent-target refire path as `refire_target_effect`. For cross-stack
callers, the refired effect's lookup identity remains the target's effect
slot, while `EffectContext::source_card` / source kind are attributed to the
grantor and `source_permanent` carries the target. Existing self-refire users
are observationally unchanged because grantor and target are the same card.

This is Rust-leading by design for BT24-102 Homeros's "activate 1 [On Play]
or [When Digivolving] effect" shape and intentionally does not dispatch fake
`OnAnyDigimonPlayed` or `OnDigivolve` events. The Python legacy engine has no
equivalent Homeros cross-card primitive.

---

## Phase 3 residue (callers still on Python engine)

These imports survived the Phase 3 cutover because the Rust counterpart
isn't in `digimon_engine` yet. Each entry is a checklist: when the
binding lands, remove the Python import and the row.

**As of Phase 4** (2026-04-25), all surface paths are rooted at
`engine_py_legacy.engine.*` — the Python engine moved to
`code/engine_py_legacy/`. The "Surface" column below uses the
unqualified shorthand (e.g., `engine.runners.headless_game.HeadlessGame`);
read it as `engine_py_legacy.engine.runners.headless_game.HeadlessGame`.
The sole exception was `engine.onnx_policy.load_onnx_policy`, which now
lives at `digimon_gym.inference.onnx_policy.load_onnx_policy` (Phase 5
relocation completed).

| Surface | Caller(s) | Rust counterpart? |
|---|---|---|
| `engine.runners.headless_game.HeadlessGame` (Python class) | `routers/state.py`, `routers/recordings.py`, `routers/games.py`, `digimon_gym.py` (Python fallback path), `agents/architect_simulator.py` | `RustHeadlessGame` exists but has a different state-shape; per-caller migration is non-trivial. |
| `engine.runners.interactive_game.InteractiveGame` | `routers/games.py`, `routers/debug_games.py`, `routers/matchmaking.py` (`# noqa: F401` re-export) | Pending — covered by the PvP bindings plan (`docs/superpowers/plans/2026-04-18-pyo3-pvp-bindings.md`). |
| `engine.runners.replay_runner.ReplayRunner` | `routers/recordings.py` | ✅ Ported as `digimon_engine::runners::replay::ReplayRunner` (Phase 3 of `add-engine-debug-mcp`). Step / seek / run-to-completion / verify-mode all implemented. See `docs/DEBUG_MCP.md`. |
| `engine.runners.scenario_runner.ScenarioRunner` | `tools/run_scenario.py`, `tools/run_qa_batch.py`, behavioral test infrastructure | Not planned (DebugRunner is the Rust-side parallel). |
| `engine.data.tensor_layout.*` | `agents/features_extractor.py` | Not planned in scope. Add later if RL trainer survives. |
| `engine.data.enums.PendingAction` | `digimon_gym.py` (Python fallback path) | Vestigial. Remove when the Python backend is retired. |
| `engine.data.enums.PlayerType` | `routers/games.py`, `routers/debug_games.py` | Server orchestration concept, not engine. Stays Python-side. |
| `engine.data.deck_loader.{validate_deck, RESTRICTED_LIST, CardRestriction}` | `db/routers/decks.py` (`no_restriction` mode passes empty `CardRestriction()` to bypass) | Rust binding's `validate_deck` always uses the official ENG list; an overload accepting a custom `CardRestriction` is not exposed. |
| `engine.data.deck_loader.RE_CARD_ID` | `tools/meta_loader.py` | Regex constant; could expose if needed. |
| `engine.data.card_features.CardFeatureVectorizer` | `tools/train_card_autoencoder.py` | Not planned. RL-training-side tool. |
| `engine.data.script_promotion.*` | `tools/promote_script.py`, `tools/archive/bootstrap_frozen_manifest.py` | Sunset — Python script lane is going away. Tools delete in Phase 4. |
| `engine.model_utils.{list_onnx_models, resolve_model_path}` | `routers/games.py`, `db/routers/admin_models.py` | Could be wrapped; low priority. |
| `engine.onnx_policy.load_onnx_policy` | `routers/games.py`, `agents/architect_simulator.py`, `tools/export_random_onnx.py` | Stays Python-side. Phase 5 relocated it to `code/digimon_gym/inference/onnx_policy.py`. |
| `engine.core.{permanent.Permanent, player.Player, card_source.CardSource}` | `engine/debug/state_injection.py` | Engine internals — don't expose. |
| `engine.events.GameEvent` (Python class) | `engine/loggers.py` | Python-engine internal. `digimon_engine` exposes events via `RustHeadlessGame.get_events_since_last_step`. |
| `engine.data.card_database.{parse_xros_req, parse_digixros_req}` | `tools/ingest_cards.py` | Could be wrapped if needed; low priority. |
| `engine.debug.state_injection.*` | `routers/debug_games.py` | Engine-internal scenario builder — don't expose. |
| `engine.game.{FIELD_SLOTS, TARGETS_PER_ATTACKER, FIELDS_PER_HAND, SECURITY_TARGET, BREEDING_SLOT}` | `digimon_gym.py` | Geometry constants. The Rust crate has these in `action::space` (renamed `MAX_FIELD_SLOTS` for `FIELD_SLOTS`); could be exposed in a follow-on. |
