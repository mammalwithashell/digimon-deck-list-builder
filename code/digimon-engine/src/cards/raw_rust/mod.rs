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

pub fn build_registry() -> EngineRawRustRegistry {
    let mut r = EngineRawRustRegistry::new();
    r.register_step("ex11_012_return_trash_to_deck_bottom", ex11_012_return_trash_to_deck_bottom);
    r.register_declarative("ex11_054_all_turns_noop", ex11_054_all_turns_noop);
    r.register_declarative("bt24_012_would_leave_replacement", bt24_012_would_leave_replacement);
    r.register_step("bt16_082_on_move_noop", bt16_082_on_move_noop);
    r.register_declarative("bt5_008_opp_cannot_reduce_digivolve_cost", bt5_008_opp_cannot_reduce_digivolve_cost);
    r.register_step("p_137_opp_adds_top_security_to_hand", p_137_opp_adds_top_security_to_hand);
    r.register_formula("ex8_074_suspended_dp_cap", ex8_074_suspended_dp_cap);
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
