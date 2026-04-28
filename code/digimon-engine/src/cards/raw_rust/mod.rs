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

pub fn build_registry() -> EngineRawRustRegistry {
    let mut r = EngineRawRustRegistry::new();
    r.register_step("ex11_012_return_trash_to_deck_bottom", ex11_012_return_trash_to_deck_bottom);
    r.register_declarative("ex11_054_all_turns_noop", ex11_054_all_turns_noop);
    r.register_declarative("bt24_012_would_leave_replacement", bt24_012_would_leave_replacement);
    r.register_step("bt16_082_on_move_noop", bt16_082_on_move_noop);
    r.register_declarative("bt5_008_opp_cannot_reduce_digivolve_cost", bt5_008_opp_cannot_reduce_digivolve_cost);
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
