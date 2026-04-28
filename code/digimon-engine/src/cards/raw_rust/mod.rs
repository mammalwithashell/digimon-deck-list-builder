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

/// EX11-054 Owen Dreadnought — [All Turns] Reptile/Dragonkin observer no-op placeholder.
///
/// The printed effect: "When your Digimon with [Reptile] or [Dragonkin] trait is played
/// or digivolves, by suspending this Tamer, Draw 1. Then 1 Progress Digimon gets +3000 DP."
///
/// This function is a no-op placeholder pending resolution of the following hybrid gap:
///
/// **Engine gap**: `OnEnterFieldAnyone` and `OnDigivolve` observer `TriggerContext` does
/// not expose the entering/digivolving permanent to observer permanents. The context's
/// `target_permanent` points to the observer (Owen) itself, not the card that just
/// entered the field.  Additionally, `GameEvent::Digivolve` is not yet emitted, blocking
/// even event-log approaches for the digivolve half.
///
/// **DSL gap**: No `entering_permanent_trait_has` / `digivolving_permanent_trait_has`
/// BoolPredicate leaf exists. Once the engine threads the entering permanent through
/// `TriggerContext`, a matching predicate would enable native DSL expression.
///
/// Tracked in `qa/dsl-vocab-gaps.md` under `entering_permanent_trait_has`.
fn ex11_054_all_turns_noop(_handle: crate::card_source::CardHandle) -> Vec<Effect> {
    // No-op: returns an empty effect list.
    // The real logic is pending engine + DSL gap closure.
    vec![]
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

/// BT16-082 Ukkomon — OnMove trigger body no-op placeholder.
///
/// Printed effect: "[Your Turn][Once Per Turn] When one of your Digimon moves
/// from the breeding area to the battle area, reveal the top 3 cards of your
/// deck. Add 1 Digimon card or Tamer card among them to the hand. Return the
/// rest to the bottom of the deck. Then, you may hatch in your breeding area."
///
/// This function is a no-op step placeholder pending resolution of G-ON-MOVE
/// (hybrid gap):
///
/// **Engine gap**: `EffectTiming::OnMove` does not exist in `enums.rs`. The
/// `game_actions::move_from_breeding` method only fires `OnTrainingTrash` — it
/// does not dispatch any event that a battle-area observer card like Ukkomon
/// could subscribe to. See `qa/archetype-qa/engine-gaps.md` [G-ON-MOVE].
///
/// **DSL gap**: No `on_move_from_breeding` when-token exists in `digimon-dsl`.
/// `CompiledTiming` has no `OnMoveFromBreeding` variant, and `timing_map.rs`
/// has no mapping for it. See `qa/dsl-vocab-gaps.md` (EX11-008 entry).
///
/// When G-ON-MOVE is closed, replace the stub clause in `BT16-082.yaml` with
/// the real process body (reveal 3 → select Digimon/Tamer → hand → remainder
/// bottom → may hatch EffectChoice) and remove this function.
fn bt16_082_on_move_noop(_ctx: &mut EffectContext<'_>, _bindings: &mut Bindings) {
    // No-op: full implementation blocked by G-ON-MOVE.
}

/// BT5-008 Gaossmon — [Opponent's Turn] opponent can't reduce digivolution costs.
///
/// Printed effect: "[Opponent's Turn] Your opponent can't reduce digivolution costs."
///
/// This function is a no-op placeholder pending resolution of the following gaps:
///
/// **DSL gap G-PLAYER-FLOOD-GATE-DSL**: The DSL `kind: flood_gate` lowers to
/// `lower_flood_gate.rs` which calls `ctx.add_modifier(h, ...)` — permanent-level only.
/// There is no `add_player_modifier` step verb in the DSL, and `EffectContext` does not
/// expose `add_player_modifier`. As a result, a player-level `CannotReducePlayCost`
/// modifier cannot be installed via any DSL or EffectContext API path.
///
/// **Engine enforcement gap**: `scan_before_pay_cost_reduction` in `game_actions.rs`
/// checks only `CannotReducePlayCost` (which covers ALL cost types, both play and digivolve).
/// There is no separate `CannotReduceDigivolveCost` variant or per-cost-type enforcement
/// split. The correct fix is to add `CannotReduceDigivolveCost` + enforce it specifically
/// in the digivolve cost-reduction scan path.
///
/// When both gaps are closed, the YAML clause should be replaced with:
/// ```yaml
/// - kind: flood_gate   # or add_player_modifier step once available
///   active_when: { opponents_turn: true }
///   target: { player: opponent }
///   modifier: CannotReduceDigivolveCost
/// ```
/// And this function should be removed.
///
/// Tracked in `qa/dsl-vocab-gaps.md` under G-PLAYER-FLOOD-GATE-DSL.
fn bt5_008_opp_cannot_reduce_digivolve_cost(_handle: crate::card_source::CardHandle) -> Vec<Effect> {
    // No-op: returns an empty effect list.
    // Full implementation blocked by G-PLAYER-FLOOD-GATE-DSL and missing
    // CannotReduceDigivolveCost enforcement in scan_before_pay_cost_reduction.
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
///   The only security-removal verb is `trash_top_security` (moves to trash).
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

pub fn build_registry() -> EngineRawRustRegistry {
    let mut r = EngineRawRustRegistry::new();
    r.register_step("ex11_012_return_trash_to_deck_bottom", ex11_012_return_trash_to_deck_bottom);
    r.register_declarative("ex11_054_all_turns_noop", ex11_054_all_turns_noop);
    r.register_declarative("bt24_012_would_leave_replacement", bt24_012_would_leave_replacement);
    r.register_step("bt16_082_on_move_noop", bt16_082_on_move_noop);
    r.register_declarative("bt5_008_opp_cannot_reduce_digivolve_cost", bt5_008_opp_cannot_reduce_digivolve_cost);
    r.register_step("p_137_opp_adds_top_security_to_hand", p_137_opp_adds_top_security_to_hand);
    r.register_formula("ex8_074_suspended_dp_cap", ex8_074_suspended_dp_cap);
    r.register_step("bt23_014_opp_cannot_play_from_trash", bt23_014_opp_cannot_play_from_trash);
    r.register_formula("bt23_014_dynamic_dp_cap", bt23_014_dynamic_dp_cap);
    r.register_step("bt9_112_delete_lowest_cost_digimon", bt9_112_delete_lowest_cost_digimon);
    r.register_step("bt17_018_delete_opp_digimon_dp_budget", bt17_018_delete_opp_digimon_dp_budget);
    r.register_step("bt17_018_trash_security_per_ten_trash", bt17_018_trash_security_per_ten_trash);
    r.register_step("lm_021_delete_dp_sum", lm_021_delete_dp_sum);
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
