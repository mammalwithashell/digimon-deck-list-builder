//! Production raw_rust function registrations for DSL long-tail cards.
//!
//! Phase 4 keeps bespoke mechanics behind named functions here instead of
//! handwritten card modules under `src/cards/<set>/`.

use crate::dsl_cards::bindings::{BindingValue, Bindings};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect::Effect;
use crate::effect_context::EffectContext;

/// EX11-012 Medusamon — [When Digivolving][End of Attack] trash-return step.
///
/// Reads the `TrashIndex` binding named `"r"` (installed by the preceding
/// `select_trash` step) and moves that card from the opponent's trash to the
/// bottom of the opponent's deck.  If the binding is absent or the index is
/// out-of-range the function is a no-op (defensive).
fn ex11_012_return_trash_to_deck_bottom(ctx: &mut EffectContext<'_>, bindings: &mut Bindings) {
    if let Some(BindingValue::TrashIndex(owner, idx)) = bindings.get("r") {
        let idx = idx as usize;
        let owner = owner as usize;
        if idx < ctx.game.players[owner].trash.len() {
            let card = ctx.game.players[owner].trash.remove(idx);
            // Bottom of deck = index 0 (deck is stored front=bottom, back=top).
            ctx.game.players[owner].deck.insert(0, card);
        }
    }
}

/// BT24-012 Dimetromon — [All Turns] "protect other Reptile/Dragonkin ally by bouncing self"
/// replacement — no-op placeholder.
///
/// Printed clause (b):
/// "[All Turns] When any of your OTHER Digimon with the [Reptile] or [Dragonkin] trait
/// would leave the battle area by your opponent's effects, by returning this Digimon to
/// your hand, they don't leave."
///
/// This is a cross-permanent replacement effect: the carrier (Dimetromon) intercepts a
/// *different* permanent leaving and cancels that departure by paying a cost (return self
/// to hand). The standard `kind: replacement` + `cancel_replacement` DSL path is blocked
/// by the `subject_matches` guard in `lower_replacement.rs` (line 83–91), which only fires
/// when the carrier IS the leaving subject.
///
/// The full implementation requires:
///
/// **Gap G-EVENT-TARGET-OWNER** — no predicate in `ReplacementContext` gates on whether
/// the leaving permanent is controlled by the same player as the carrier. Additionally,
/// removal-cause attribution ("by your opponent's effects") is not threaded into the
/// replacement context — the engine would need `ReplacementContext::caused_by_opponent`
/// populated from game-action callsites. Until this is wired, any implementation would
/// over-fire (fires for own-effect removal too), violating the no-approximations policy.
///
/// **subject_matches architecture gap** — `lower_replacement.rs` enforces that replacement
/// effects only fire when `rctx.effect.source_permanent == Some(subject_h)`. Lifting this
/// restriction to allow "protect others" patterns requires a targeted change to
/// `lower_replacement.rs`.
///
/// Until both gaps are closed this function returns an empty `Vec<Effect>`, preserving
/// no-op behavior while the YAML clause documents the intent.
///
/// When implemented, the fn must:
///   1. Build a `WhenWouldLeaveBattleArea` replacement effect scoped to the carrier.
///   2. In the replacement predicate: check subject != carrier, subject.controller == carrier.controller,
///      subject has Reptile or Dragonkin trait, and carrier is on the battle area.
///   3. Present optional prompt ("Accept/Decline"); on accept: return carrier to hand via
///      `ctx.return_to_hand(carrier)` and set outcome to Cancelled.
///
/// Tracked under G-EVENT-TARGET-OWNER in `qa/archetype-qa/engine-gaps.md`.
fn bt24_012_would_leave_replacement(_handle: crate::card_source::CardHandle) -> Vec<Effect> {
    // No-op: returns an empty effect list.
    // Full implementation blocked by G-EVENT-TARGET-OWNER (removal cause attribution
    // + cross-permanent replacement) and the subject_matches gate in lower_replacement.rs.
    vec![]
}

/// P-137 Flamedramon — [Your Turn][OPT] opponent adds their top security to hand.
///
/// Printed effect: "[Your Turn][Once Per Turn] When this Digimon's attack target
/// is switched, your opponent adds the top card of their security stack to the hand."
///
/// DCGO analysis (P_137.cs EffectTiming.OnAttackTargetChanged):
///   - `CardObjectController.AddHandCards` — moves the top security card to the
///     opponent's hand.
///   - `IReduceSecurity.ReduceSecurity` — fires the security-loss event chain
///     (OnLoseSecurity for the defender, OnOpponentSecurityRemoved for the attacker).
///
/// This function is the `raw_rust:` bridge pending resolution of the hybrid gap
/// [G-ADD-TOP-SECURITY-TO-HAND]:
///
/// **DSL gap**: No `add_top_security_to_hand` verb exists in `digimon-dsl/src/step.rs`.
///   The security-removal verbs are `trash_top_security` (moves top card to trash) and `trash_bottom_security` (moves bottom card to trash).
///   `add_top_security_to_hand` would lower to a new `EffectContext` method that
///   moves the card to the owner's hand instead of trash.
///
/// **Engine gap**: `EffectContext` has no `add_top_security_to_hand` method.
///   The closest is `trash_top_security` which routes through the WhenWouldBeTrashed
///   replacement chain and fires zone-transfer events to trash. The hand-transfer
///   variant needs the same `IReduceSecurity`-equivalent event firing but to hand.
///
/// Implementation strategy here:
///   1. Pop the top security card from the opponent's security stack.
///   2. Push it to the opponent's hand.
///   3. Fire `OnLoseSecurity` from the defender's security-revealed context so
///      observer cards (e.g., BT21-001 Gigimon) see the security loss.
///   4. Fire `OnOpponentSecurityRemoved` from the controller's battle area so
///      archetype observers (e.g., BT21-008 Elizamon) see the opponent's loss.
///
/// When [G-ADD-TOP-SECURITY-TO-HAND] is closed, replace the raw_rust step in
/// `P-137.yaml` with the native DSL verb and remove this function.
///
/// Tracked under [G-ADD-TOP-SECURITY-TO-HAND] in qa/dsl-vocab-gaps.md and
/// qa/archetype-qa/engine-gaps.md.
fn p_137_opp_adds_top_security_to_hand(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    use crate::enums::EffectTiming;
    use crate::selection::TriggerSource;

    let opponent = ctx.opponent_id();

    // No-op if opponent has no security.
    if ctx.game.player(opponent).security.is_empty() {
        return;
    }

    // Pop the top security card (security is stored front=bottom, back=top;
    // `pop()` removes from the back = the top of the stack).
    let card = match ctx.game.player_mut(opponent).security.pop() {
        Some(c) => c,
        None => return,
    };
    let card_handle = card.handle();

    // Move the card to the opponent's hand.
    ctx.game.player_mut(opponent).hand.push(card);

    // Fire OnLoseSecurity so cards watching the defender's security loss can react.
    // Use SecurityRevealed trigger source to mirror the normal security-loss path.
    ctx.game.enqueue_triggered(
        EffectTiming::OnLoseSecurity,
        TriggerSource::SecurityRevealed {
            defender: opponent,
            card: card_handle,
        },
    );
    ctx.game.drain_effect_queue();

    // Fire OnOpponentSecurityRemoved so the controller's archetype observers react.
    let controller = ctx.player;
    ctx.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::PlayerBattleArea(controller),
    );
    ctx.game.drain_effect_queue();
}

/// EX8-074 MedievalGallantmon — Dynamic DP cap formula for the [When Digivolving]
/// delete sub-clause.
///
/// Printed text: "delete 1 of your opponent's 8000 DP or lower Digimon. For each
/// other suspended Digimon, add 3000 to this DP deletion effect's maximum."
///
/// Formula: `8000 + 3000 × (count of OTHER suspended Digimon in the controller's
/// battle area, excluding the source permanent itself)`.
///
/// The `target` argument is the candidate being filtered (an opponent's Digimon);
/// `rctx.source_permanent` is the handle to EX8-074 on the field.
/// `rctx.player` is the controller (who owns EX8-074).
///
/// Used in `dp_lte: { formula: { raw_rust: ex8_074_suspended_dp_cap } }`.
/// The formula is called once per candidate during predicate evaluation.
///
/// NOTE G-PRED-DP-LTE: dp_lte with a formula is parsed but the predicate
/// evaluator does not yet invoke the formula on permanents in non-security zones.
/// Until that gap closes, this formula is compiled and registered but not called
/// at runtime — all opponent Digimon pass the dp_lte filter unconditionally.
fn ex8_074_suspended_dp_cap(
    rctx: &crate::effect_context::EffectReadContext<'_>,
    _target: crate::permanent::PermanentHandle,
) -> i32 {
    let controller = rctx.player;
    let source_handle = rctx.source_permanent;

    // Count suspended Digimon belonging to the controller, excluding the source permanent.
    let suspended_count = rctx
        .game
        .player(controller)
        .battle_area
        .iter()
        .enumerate()
        .filter(|(idx, perm)| {
            // Exclude the source permanent (EX8-074 itself).
            let perm_handle = crate::permanent::PermanentHandle {
                player: controller,
                index: *idx as u8,
            };
            let is_source = source_handle.map_or(false, |sh| sh == perm_handle);
            !is_source && perm.is_suspended
        })
        .count() as i32;

    8000 + 3000 * suspended_count
}

/// BT23-014 Gallantmon — [On Play][When Digivolving] opponent can't play Digimon or Tamers
/// from trash until their turn ends.
///
/// Printed effect:
/// "[On Play][When Digivolving] Until your opponent's turn ends, their effects can't play
/// Digimon or Tamers from the trash."
///
/// DCGO analysis (BT23_014.cs, `SharedFloodGateActivateCoroutine`):
///   - Creates a `CanNotPutFieldClass` that blocks cards satisfying:
///     `IsExistInAnyTrash(cardSource) && (cardSource.IsDigimon || cardSource.IsTamer)`
///   - The effect source must belong to the opponent: `cardEffect.EffectSourceCard.Owner == card.Owner.Enemy`
///   - Added to `card.Owner.Enemy.UntilOwnerTurnEndEffects` — expires at end of opponent's turn.
///
/// Implementation:
///   1. Install `CannotPlayDigimonByEffect` as a player-scoped modifier on the opponent with
///      `Expiry::EndOfOpponentsTurn`. This blocks effect-initiated Digimon plays from any zone
///      (hand or trash) via the gate in `play_from_hand_with_cost` and `play_from_trash_with_cost`.
///   2. Install `CannotPlayTamerByEffect` (added in this batch) as a parallel player-scoped
///      modifier on the opponent with the same expiry.
///
/// `EndOfOpponentsTurn` expiry uses `source_player == ctx.player` (the Gallantmon controller),
/// so the modifier is cleared when the OTHER player's turn ends — i.e., the opponent's turn.
fn bt23_014_opp_cannot_play_from_trash(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    use crate::enums::{Expiry, ModifierType};
    use crate::modifiers::PlayerModifierEntry;

    let opponent = ctx.opponent_id();
    let source_player = ctx.player;

    // Block opponent from playing Digimon by effect (covers hand + trash zones).
    ctx.game.modifiers.add_player_modifier(
        opponent,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::EndOfOpponentsTurn,
            None,
            source_player,
        ),
    );

    // Block opponent from playing Tamers by effect (covers hand + trash zones).
    ctx.game.modifiers.add_player_modifier(
        opponent,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayTamerByEffect,
            0,
            Expiry::EndOfOpponentsTurn,
            None,
            source_player,
        ),
    );
}

/// BT23-014 Gallantmon — dynamic DP cap formula for the delete clause.
///
/// Printed text: "Delete 1 of your opponent's Digimon with 8000 DP or less.
/// For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum."
///
/// Formula: `8000 + 2000 × (total permanents in opponent's battle area)`.
/// "Their Digimon and Tamers" = all permanents in opponent's battle area
/// (since only Digimon and Tamers occupy battle area slots).
///
/// The `target` argument is the candidate being filtered (an opponent's Digimon).
/// `rctx.player` is the controller (who owns BT23-014).
///
/// NOTE G-PRED-DP-LTE: dp_lte with a formula is parsed but the predicate
/// evaluator does not yet invoke the formula on permanents in non-security zones.
/// Until that gap closes, this formula is registered but not called at runtime.
fn bt23_014_dynamic_dp_cap(
    rctx: &crate::effect_context::EffectReadContext<'_>,
    _target: crate::permanent::PermanentHandle,
) -> i32 {
    let opponent = 1 - rctx.player; // player 0 or 1

    // Count all permanents in opponent's battle area (Digimon + Tamers).
    let count = rctx.game.player(opponent).battle_area.len() as i32;

    8000 + 2000 * count
}

/// BT9-112 DeathXmon — [End of Opponent's Turn][Once Per Turn]
/// delete all opponent Digimon with the lowest play cost.
///
/// Printed effect: "[End of Opponent's Turn] [Once Per Turn]
/// Delete all of your opponent's Digimon with the lowest play cost."
///
/// DCGO analysis (BT9_112.cs, `ActivateClass(EffectTiming.OnEndTurn)`):
///   - `CardEffectCommons.IsMinCost(perm, Enemy, true)` — finds the minimum
///     play cost among the opponent's Digimon (isDigimon=true), then destroys
///     all Digimon at that minimum cost via `DestroyPermanentsClass`.
///
/// Implementation:
///   1. Collect all opponent Digimon from `ctx.game.player(opponent).battle_area`.
///   2. Find the minimum `play_cost` among them.
///   3. Collect `PermanentHandle`s of all opponent Digimon at that minimum cost.
///   4. Delete each via `ctx.delete_permanent(handle)`.
///
/// GAP G-PLAY-COST-LTE: The DSL aggregate formula supports `lowest_dp`/`highest_dp`/
/// `lowest_level` but not `lowest_play_cost`. This step implements the gap logic
/// directly in Rust. When G-PLAY-COST-LTE is closed, replace the `raw_rust` step
/// in BT9-112.yaml with a native DSL expression and remove this function.
///
/// GAP G-OPT-TRIGGERED: `once_per_turn: true` on triggered clauses compiles to
/// `Effect::max_per_turn=1` but triggered OPT enforcement is not yet wired in
/// `run_queued_effect_inner`. The clause will over-fire until that gap closes.
///
/// Tracked in `qa/dsl-vocab-gaps.md` under G-PLAY-COST-LTE.
fn bt9_112_delete_lowest_cost_digimon(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    use crate::enums::CardKind;
    use crate::permanent::PermanentHandle;

    let opponent = ctx.opponent_id();

    // Collect (handle, play_cost) for every opponent Digimon.
    let digimon_costs: Vec<(PermanentHandle, u16)> = ctx
        .game
        .player(opponent)
        .battle_area
        .iter()
        .enumerate()
        .filter_map(|(idx, perm)| {
            let top = perm.top_card();
            let data = &ctx.game.card_data[top.data_index];
            if data.card_kind == CardKind::Digimon {
                let handle = PermanentHandle {
                    player: opponent,
                    index: idx as u8,
                };
                Some((handle, data.play_cost))
            } else {
                None
            }
        })
        .collect();

    // Nothing to do if opponent has no Digimon.
    if digimon_costs.is_empty() {
        return;
    }

    // Find the minimum play cost.
    let min_cost = digimon_costs
        .iter()
        .map(|(_, cost)| *cost)
        .min()
        .unwrap_or(0);

    // Collect handles of all Digimon at the minimum cost.
    let targets: Vec<PermanentHandle> = digimon_costs
        .into_iter()
        .filter(|(_, cost)| *cost == min_cost)
        .map(|(handle, _)| handle)
        .collect();

    // Delete all matching Digimon (iterate in reverse index order so earlier
    // deletions don't shift the indices of later targets).
    let mut sorted_targets = targets;
    sorted_targets.sort_by(|a, b| b.index.cmp(&a.index));
    for handle in sorted_targets {
        ctx.delete_permanent(handle);
    }
}

/// BT17-018 Gallantmon: Crimson Mode — [On Play][When Digivolving] DP-budget multi-delete.
///
/// Printed effect: "Choose any number of your opponent's Digimon whose total DP
/// adds up to 15000 or less and delete them."
///
/// DCGO analysis (BT17_018.cs, `SetUpActivateClass SharedActivateCoroutine`):
///   - `canTargetConditionByPreSelectedList`: filters candidates based on running
///     DP sum — after each pick, candidates whose DP would push the total above
///     15000 are removed from the selectable list.
///   - `canEndSelectCondition`: final validation that total DP ≤ 15000.
///   - `canNoSelect: false`: must pick ≥1 when eligible targets exist.
///   - `canEndNotMax: true`: can stop picking before all valid candidates are selected.
///
/// BLOCKED: G-DP-BUDGET-MULTI-SELECT
///   The engine has no `select_opponent_permanent_multi_dp_budget` primitive.
///   `select_count_capped_multi` caps pick COUNT, not DP sum. A proper
///   implementation requires:
///     1. Initial candidate list: all opponent Digimon.
///     2. After each pick: subtract that Digimon's DP from remaining budget (15000 -
///        sum_picked), re-filter candidates to those whose DP ≤ remaining budget.
///     3. On PASS (or when no candidates remain): delete all picked Digimon.
///   This multi-round incremental selection is not currently supported by
///   `PendingSelection` — it would require a new selection kind or a looping
///   raw_rust harness that re-installs selection each round.
///
/// Current approximation (no-approximations policy violation — noted for tracking):
///   This function installs a SINGLE mandatory selection of ONE opponent Digimon
///   with DP ≤ 15000 (i.e., effectively treats the 15000 budget as a filter on
///   the single target). It does NOT support multi-pick. This violates the
///   no-approximations policy for the multi-select aspect, but is the least bad
///   option until G-DP-BUDGET-MULTI-SELECT closes.
///
/// When G-DP-BUDGET-MULTI-SELECT is closed, replace this function with a proper
/// multi-round selection harness or a new engine primitive, and update BT17-018.yaml.
///
/// Tracked in qa/archetype-qa/engine-gaps.md under G-DP-BUDGET-MULTI-SELECT.
fn bt17_018_delete_opp_digimon_dp_budget(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    use crate::enums::CardKind;

    // Install a single mandatory selection of ONE opponent Digimon with DP ≤ 15000.
    // BLOCKED: G-DP-BUDGET-MULTI-SELECT — full multi-pick with running DP-sum cap
    // is not yet supported by the engine. This is a single-pick approximation only.
    ctx.select_opponent_permanent(
        "Select 1 of your opponent's Digimon to delete (DP ≤ 15000 budget; multi-pick pending G-DP-BUDGET-MULTI-SELECT)",
        false, // mandatory (canNoSelect: false)
        |game, h| {
            // Filter: must be Digimon with DP ≤ 15000.
            let perm = &game.player(h.player).battle_area[h.index as usize];
            let top = perm.top_card();
            let data = &game.card_data[top.data_index];
            data.card_kind == CardKind::Digimon && data.dp.unwrap_or(0) <= 15000
        },
        |ctx, selected| {
            ctx.delete_permanent(selected);
        },
    );
}

/// BT17-018 Gallantmon: Crimson Mode — [When Attacking][Once Per Turn] security trash loop.
///
/// Printed effect: "[When Attacking] [Once Per Turn] For every 10 cards in both players'
/// trash, trash 1 card from the top of your opponent's security stack."
///
/// DCGO analysis (BT17_018.cs, `ActivateClass(EffectTiming.OnAttack)`):
///   - `count = (TrashCards.Count + Enemy.TrashCards.Count) / 10`
///   - Loops `IDestroySecurity(enemyPlayer, count)` — trashes `count` security cards.
///
/// Implementation:
///   1. Sum `ctx.trash(player).len() + ctx.trash(opponent).len()`.
///   2. Compute `iterations = combined_trash_count / 10` (integer floor division).
///   3. Loop `iterations` times calling `ctx.trash_top_security(opponent)`.
///      Each call fires the `WhenWouldBeTrashed` replacement chain and returns `false`
///      if the opponent's security is empty (early-exit guard).
///
/// DSL gap note: The `lose_count_bound` DSL verb described in the spec is not yet
/// implemented in `digimon-dsl/src/step.rs`. When that verb is added, this raw_rust
/// step can be replaced with a native DSL expression:
///   ```yaml
///   - lose_count_bound:
///       count: { floor_div: [{ card_count_in_zone: { zone: trash, of: any } }, 10] }
///       of: opponent
///   ```
/// Tracked in `qa/dsl-vocab-gaps.md`.
///
/// Once-per-turn enforcement: the [Once Per Turn] constraint is compiled into
/// `once_per_turn: true` on the `WhenAttacking` clause. The DSL lowers this to
/// `Effect::max_per_turn = 1`, but G-OPT-TRIGGERED means the engine does not
/// enforce this limit for triggered effects. The step itself fires unconditionally
/// until that gap closes.
fn bt17_018_trash_security_per_ten_trash(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    let player = ctx.player;
    let opponent = ctx.opponent_id();

    // Sum both players' trash sizes.
    let combined_trash = ctx.trash(player).len() + ctx.trash(opponent).len();

    // floor(combined / 10) iterations.
    let iterations = combined_trash / 10;

    for _ in 0..iterations {
        // Bail early if opponent has no security left.
        if ctx.game.player(opponent).security.is_empty() {
            break;
        }
        ctx.trash_top_security(opponent);
    }
}

/// LM-021 Agumon - Bond of Bravery — [On Play][When Digivolving] DP-sum delete.
///
/// Printed effect: "Delete any number of your opponent's Digimon whose total DP
/// adds up to equal or less than this Digimon's DP." (This Digimon has 14000 DP.)
///
/// DCGO analysis (LM_021.cs, `SelectPermanentEffect.SetUp`):
///   - `canTargetConditionByPreSelectedList`: running DP-sum filter — after each
///     pick, candidates whose DP would push the total above this Digimon's DP (14000)
///     are removed from the selectable list.
///   - `canEndSelectCondition`: final validation that total DP ≤ 14000.
///   - `canNoSelect: false`: must pick ≥1 when eligible targets exist.
///   - `canEndNotMax: true`: can stop picking before max candidates.
///
/// BLOCKED: G-MULTI-SELECT-OPP-DP-SUM (same root as G-DP-BUDGET-MULTI-SELECT)
///   No DSL verb or engine primitive supports multi-select of opponent battle-area
///   permanents with a running DP-sum cap. This function is a single-pick fallback
///   (approximation: player picks one opponent Digimon and it is deleted).
///
/// When G-MULTI-SELECT-OPP-DP-SUM is closed, replace this raw_rust step in
/// LM-021.yaml with a native DSL expression and remove this function.
///
/// Tracked in qa/archetype-qa/engine-gaps.md under G-MULTI-SELECT-OPP-DP-SUM.
fn lm_021_delete_dp_sum(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    use crate::enums::CardKind;

    let opponent = ctx.opponent_id();

    // Check at least one eligible target (Digimon with DP ≤ 14000).
    let has_target = ctx.game.player(opponent).battle_area.iter().any(|perm| {
        let top = perm.top_card();
        let data = &ctx.game.card_data[top.data_index];
        data.card_kind == CardKind::Digimon && data.dp.unwrap_or(0) <= 14000
    });

    if !has_target {
        return;
    }

    // Single mandatory pick — fallback for G-MULTI-SELECT-OPP-DP-SUM.
    // Full multi-pick with running DP-sum cap pending engine primitive support.
    ctx.select_opponent_permanent(
        "Select 1 of your opponent's Digimon to delete (DP ≤ 14000 budget; multi-pick pending G-MULTI-SELECT-OPP-DP-SUM)",
        false, // mandatory (canNoSelect: false)
        |game, h| {
            let perm = &game.player(h.player).battle_area[h.index as usize];
            let top = perm.top_card();
            let data = &game.card_data[top.data_index];
            data.card_kind == CardKind::Digimon && data.dp.unwrap_or(0) <= 14000
        },
        |ctx, selected| {
            ctx.delete_permanent(selected);
        },
    );
}

// bt20_016_dna_on_deletion was removed 2026-05-21 (main #503) — BT20-016's
// [All Turns] cross-permanent DNA-on-deletion observer no longer needs a
// raw_rust placeholder.
//
// lm_027_delay_start_of_turn_noop was removed 2026-05-21 — LM-027's
// [Start of Your Turn] <Delay> clause is now a native `kind: delay` clause
// (G-DELAY-START-OF-TURN and G-ZONE-SELECTED-TRASH-TO-DECK-TOP both resolved).

/// LM-027 Red Scramble — legacy shim for "add this card to hand".
///
/// The printed Security effect ends with "Then, add this card to the hand."
/// DCGO implements this via `CardEffectCommons.AddThisCardToHand(card, activateClass)`
/// which moves the currently-resolving option card from security-resolution
/// staging back to the controller's hand.
///
/// Prefer the native DSL `add_this_option_to_hand: {}` step for new scripts.
fn lm_027_add_self_to_hand(ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    ctx.add_pending_security_to_hand();
}

/// BT21-093 Raging Serpentine — Main + Security clause: delete 1 of opponent's
/// highest-DP Digimon (mandatory if any eligible target).
///
/// No-op stub: the highest-DP-aggregate selection requires either a new DSL
/// `aggregate: highest_dp` evaluator (G-PRED-DP-LTE family) or a manually
/// installed `select_opponent_permanent` selection from raw Rust. Behavioral
/// tests for this card are structural-only pending that work.
fn bt21_093_delete_highest_dp_opponent(_ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    // No-op: pending G-PRED-DP-LTE (highest-DP aggregate predicate) +
    // a raw_rust-driven selection installer.
}

/// BT24-062 MasterBlimpmon — inherited [Your Turn] target-lock declarative.
///
/// Printed inherited text: "[Your Turn] This Digimon's attack target can't change."
///
/// Refreshes `ModifierType::CannotSwitchAttackTarget` on the host permanent
/// (the active top card of the digivolution stack containing this card source)
/// every declarative tick when the host's controller is the turn player. The
/// modifier is auto-cleared between ticks because `add_declarative_modifier`
/// sets `materialized_declarative: true`, and Track D's combat consult sites
/// (Block window early-return, Raid retarget early-return, and the unified
/// `apply_attack_target_substitution` no-op) read the modifier directly.
///
/// Two declaratives are emitted to cover both scopes the Rust engine's tick
/// model exposes — `face_up` (BT24-062 is the active top card) and
/// `inherited` (BT24-062 is a digivolution source under another Digimon).
/// In both cases `source_permanent` resolves to the host, so the body is
/// identical; only the `.inherited()` flag differs. Without the face-up
/// emission the modifier would not install when BT24-062 IS the host —
/// the tick walks the top card with `inherited_source = false` and skips
/// effects whose `inherited` flag is set.
fn bt24_062_attack_target_lock(card: crate::card_source::CardHandle) -> Vec<Effect> {
    use crate::enums::{Expiry, ModifierType};

    fn install(ctx: &mut EffectContext<'_>) {
        let Some(host) = ctx.source_permanent else {
            return;
        };
        if ctx.game.turn_player() != host.player {
            return;
        }
        ctx.add_declarative_modifier(
            host,
            ModifierType::CannotSwitchAttackTarget,
            0,
            Expiry::Permanent,
        );
    }

    vec![
        Effect::declarative(card)
            .name("[Your Turn] target lock — face up")
            .materializes_declarative_state()
            .process(install)
            .build(),
        Effect::declarative(card)
            .name("[Your Turn] target lock — inherited source")
            .inherited()
            .materializes_declarative_state()
            .process(install)
            .build(),
    ]
}

/// BT13-040 Magnamon — would-leave observer tail.
///
/// Printed text: "When this Digimon would leave the battle area, <Draw 1>.
/// Then, you may play 1 [Veemon] from your hand or this Digimon's digivolution
/// cards without paying the cost."
///
/// This is intentionally a non-cancelling replacement-process step: the
/// surrounding DSL replacement observer performs the draw, then this helper
/// installs one optional pending selection containing both hand and material
/// action IDs. No action-space expansion is needed; hand picks reuse
/// `PLAY_HAND_START + index`, source picks reuse `SOURCE_SELECT_START +
/// field * SOURCES_PER_FIELD + source_index`.
fn bt13_040_may_play_veemon_from_hand_or_source(
    ctx: &mut EffectContext<'_>,
    _bindings: &mut Bindings,
) {
    use crate::action::space::{
        encode_source_select, PLAY_HAND_END, PLAY_HAND_START, SOURCE_SELECT_END,
        SOURCE_SELECT_START,
    };
    use crate::enums::{CostDelta, GamePhase};
    use crate::selection::{PendingSelection, SelectionKind, UnionZoneSet};

    let Some(source_permanent) = ctx.source_permanent else {
        return;
    };

    let player = ctx.player;
    let mut valid_action_ids = Vec::new();

    for (idx, card) in ctx.game.player(player).hand.iter().enumerate() {
        if idx >= crate::action::space::HAND_MAIN_LIMIT {
            break;
        }
        if card
            .card_names(&ctx.game.card_data)
            .iter()
            .any(|name| name.contains("Veemon"))
        {
            valid_action_ids.push(PLAY_HAND_START + idx as u16);
        }
    }

    if let Some(perm) = ctx
        .game
        .player(source_permanent.player)
        .battle_area
        .get(source_permanent.index as usize)
    {
        let source_count = perm.card_sources.len().saturating_sub(1);
        for source_index in 0..source_count.min(crate::action::space::SOURCES_PER_FIELD as usize) {
            let source = &perm.card_sources[source_index];
            if source
                .card_names(&ctx.game.card_data)
                .iter()
                .any(|name| name.contains("Veemon"))
            {
                if let Some(action_id) =
                    encode_source_select(source_permanent.index as u16, source_index as u16)
                {
                    valid_action_ids.push(action_id);
                }
            }
        }
    }

    if valid_action_ids.is_empty() {
        return;
    }

    let previous_phase = ctx.game.current_phase;
    let selecting_player = ctx.override_selecting_player.unwrap_or(player);
    let source_card = ctx.source_card;
    let source_kind = ctx.source_kind;
    let override_pin = ctx.override_selecting_player;
    ctx.game.current_phase = GamePhase::SelectUnion;
    ctx.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::MATERIAL,
        },
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: true,
        prompt: "You may play 1 [Veemon] from hand or this Digimon's sources".to_string(),
        effect_choices: None,
        source_card,
        source_permanent: Some(source_permanent),
        source_kind,
        callback: Box::new(move |game, action_id| {
            let mut cb_ctx = EffectContext::new_with_source_kind_and_override(
                game,
                source_card,
                Some(source_permanent),
                source_kind,
                player,
                override_pin,
            );
            if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) {
                let hand_index = (action_id - PLAY_HAND_START) as usize;
                let _ = cb_ctx.play_from_hand_free(player, hand_index);
            } else if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action_id) {
                let (_, source_index) = crate::action::space::decode_source_select(action_id);
                let _ = cb_ctx.play_from_materials(
                    source_permanent,
                    source_index as usize,
                    CostDelta::Free,
                );
            }
        }),
        on_decline: None,
    });
}

pub fn build_registry() -> EngineRawRustRegistry {
    let mut r = EngineRawRustRegistry::new();
    r.register_step(
        "ex11_012_return_trash_to_deck_bottom",
        ex11_012_return_trash_to_deck_bottom,
    );
    r.register_declarative(
        "bt24_012_would_leave_replacement",
        bt24_012_would_leave_replacement,
    );
    r.register_step(
        "p_137_opp_adds_top_security_to_hand",
        p_137_opp_adds_top_security_to_hand,
    );
    r.register_formula("ex8_074_suspended_dp_cap", ex8_074_suspended_dp_cap);
    r.register_step(
        "bt23_014_opp_cannot_play_from_trash",
        bt23_014_opp_cannot_play_from_trash,
    );
    r.register_formula("bt23_014_dynamic_dp_cap", bt23_014_dynamic_dp_cap);
    r.register_step(
        "bt9_112_delete_lowest_cost_digimon",
        bt9_112_delete_lowest_cost_digimon,
    );
    r.register_step(
        "bt17_018_delete_opp_digimon_dp_budget",
        bt17_018_delete_opp_digimon_dp_budget,
    );
    r.register_step(
        "bt17_018_trash_security_per_ten_trash",
        bt17_018_trash_security_per_ten_trash,
    );
    r.register_step("lm_021_delete_dp_sum", lm_021_delete_dp_sum);
    // bt20_102_boardwipe_and_return was removed 2026-05-20 — BT20-102's
    // [On Play][When Digivolving] board-wipe + return clause is now pure DSL
    // (for_each + binding_absent/not_in_binding exclusion; G-FOR-EACH-EXCLUDE-BINDING).
    // bt20_016_dna_on_deletion was removed 2026-05-21 (main #503).
    // lm_027_delay_start_of_turn_noop was removed 2026-05-21 — LM-027's Delay
    // clause is now native DSL (G-ZONE-SELECTED-TRASH-TO-DECK-TOP resolved).
    r.register_step("lm_027_add_self_to_hand", lm_027_add_self_to_hand);
    // p_206_add_self_to_hand was removed 2026-05-17 (Phase 2 Track E) —
    // P-206 now uses native DSL `add_this_option_to_hand`.
    // bt21_093_cost_reduction_amount was removed 2026-05-17 (Phase 2 Track G)
    // — BT21-093 now uses the native `opponent_security_count_lte: 3`
    // predicate over the existing fixed `amount: 4` cost-reduction slot.
    r.register_step(
        "bt21_093_delete_highest_dp_opponent",
        bt21_093_delete_highest_dp_opponent,
    );
    r.register_declarative("bt24_062_attack_target_lock", bt24_062_attack_target_lock);
    r.register_step(
        "bt13_040_may_play_veemon_from_hand_or_source",
        bt13_040_may_play_veemon_from_hand_or_source,
    );
    r
}

pub fn raw_rust_budget_status(raw_fn_count: usize, dsl_card_count: usize) -> Result<(), String> {
    if dsl_card_count == 0 {
        return Ok(());
    }

    let pct = (raw_fn_count as f64 / dsl_card_count as f64) * 100.0;
    if pct > 3.0 {
        Err(format!(
            "raw_rust budget exceeded: {raw_fn_count} raw_rust fns for \
             {dsl_card_count} DSL cards ({pct:.1}%)"
        ))
    } else {
        Ok(())
    }
}
