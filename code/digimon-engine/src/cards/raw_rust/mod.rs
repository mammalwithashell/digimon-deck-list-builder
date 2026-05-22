//! Production raw_rust function registrations for DSL long-tail cards.
//!
//! Phase 4 keeps bespoke mechanics behind named functions here instead of
//! handwritten card modules under `src/cards/<set>/`.

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::effect::Effect;
use crate::effect_context::EffectContext;

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

pub fn build_registry() -> EngineRawRustRegistry {
    let mut r = EngineRawRustRegistry::new();
    r.register_declarative(
        "bt24_012_would_leave_replacement",
        bt24_012_would_leave_replacement,
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
    // bt17_018_delete_opp_digimon_dp_budget was removed 2026-05-22 —
    // BT17-018's delete clause uses native `select_opponent_dp_budget`.
    r.register_step(
        "bt17_018_trash_security_per_ten_trash",
        bt17_018_trash_security_per_ten_trash,
    );
    // lm_021_delete_dp_sum was removed 2026-05-22 — LM-021 uses native
    // `select_opponent_dp_budget` with a `source_dp` formula budget.
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
    // bt13_040_may_play_veemon_from_hand_or_source was removed 2026-05-22 —
    // BT13-040 uses native `select_union_zone` over hand/material plus
    // `play_union_bound_free`.
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
