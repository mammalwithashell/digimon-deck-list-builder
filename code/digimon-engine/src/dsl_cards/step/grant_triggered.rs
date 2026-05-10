//! Track H §3 — `grant_triggered_effect` step handler.
//!
//! Walks battle areas for permanents matching the step's `target`
//! predicate; for each match, calls `EffectContext::grant_triggered_effect`
//! with a body closure that executes the compiled-step list via
//! `run_steps`. DCGO `AddSkillClass.cs` analog.
//!
//! v1 limitation: granted bodies fire inline AFTER the carrier's
//! printed observers drain (per Phase 4b's drain-hook). Bodies that
//! install a `PendingSelection` will not compose with the surrounding
//! firing sequence — the proper shape requires extending `QueuedEffect`
//! with a granted-effect discriminator. EX1-068 Ice Wall! and similar
//! "lose N memory" / "gain X" non-selection bodies work today.

use std::sync::Arc;

use digimon_dsl::compiled::CompiledStep;

use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::expiry_map::lookup_expiry;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::step::run_steps;
use crate::effect_context::EffectContext;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;

pub(crate) fn try_run(
    step: &CompiledStep,
    ctx: &mut EffectContext<'_>,
    _bindings: &mut Bindings,
) -> bool {
    let CompiledStep::GrantTriggeredEffect {
        target,
        timing,
        expiry,
        body,
    } = step
    else {
        return false;
    };

    // Resolve string-tagged enum values. Unknown timing/expiry strings
    // no-op silently — same convention as the rest of the step DSL.
    // The DSL validator catches authoring mistakes upstream; this is
    // defensive against post-validator drift.
    let Some(engine_timing) = compiled_timing_string_to_effect_timing(timing) else {
        #[cfg(debug_assertions)]
        eprintln!(
            "[debug] dsl::grant_triggered_effect: unknown timing {:?} — step is a no-op.",
            timing
        );
        return true;
    };
    let Some(engine_expiry) = lookup_expiry(expiry) else {
        #[cfg(debug_assertions)]
        eprintln!(
            "[debug] dsl::grant_triggered_effect: unknown expiry {:?} — step is a no-op.",
            expiry
        );
        return true;
    };

    // Snapshot matching permanents from a read borrow before installing
    // (the install path takes &mut and can't co-exist with the read
    // borrow).
    let target_arc = Arc::new(target.clone());
    let mut matches: Vec<PermanentHandle> = Vec::new();
    {
        let rctx = ctx.as_read();
        let n_players = rctx.game.players.len() as PlayerId;
        for p in 0..n_players {
            let m = rctx.game.player(p).battle_area.len();
            for i in 0..m {
                let handle = PermanentHandle {
                    player: p,
                    index: i as u8,
                };
                if eval_predicate(&target_arc, &rctx, PredicateSubject::Permanent(handle)) {
                    matches.push(handle);
                }
            }
        }
    }

    // Install one granted entry per matching carrier. Body closure
    // captures the compiled step list (Arc-wrapped for cheap clone)
    // and runs it through `run_steps` with a fresh `Bindings`. The
    // grantor identity (source_card / source_permanent) flows through
    // the inner ctx automatically — the engine sets them at fire time
    // per `Game::fire_granted_triggered_effects`.
    let body_arc: Arc<Vec<CompiledStep>> = Arc::new(body.clone());
    for carrier in matches {
        let body_for_closure = body_arc.clone();
        ctx.grant_triggered_effect(
            carrier,
            engine_timing,
            engine_expiry,
            move |inner_ctx| {
                let mut bindings = Bindings::default();
                let _ = run_steps(&body_for_closure, inner_ctx, &mut bindings);
            },
        );
    }
    true
}

/// Convert a snake_case timing string to the engine's `EffectTiming`.
/// Reuses the DSL's existing timing-map by parsing through
/// `digimon_dsl::compiled::CompiledTiming` first when possible. For
/// timings that the DSL doesn't surface (e.g. `OnPlaceSecurity` or
/// engine-internal observers), we fall back to a hand-written match.
fn compiled_timing_string_to_effect_timing(s: &str) -> Option<crate::enums::EffectTiming> {
    use crate::enums::EffectTiming as T;
    Some(match s {
        // Player-action timings
        "on_play" => T::OnPlay,
        "on_digivolve" => T::OnDigivolve,
        "on_dna_digivolve" => T::OnDnaDigivolve,
        "when_digivolving" => T::WhenDigivolving,

        // Combat timings
        "when_attacking" => T::WhenAttacking,
        "on_attack" => T::OnAttack,
        "end_of_attack" => T::EndOfAttack,
        "end_of_battle" => T::EndOfBattle,

        // Lifecycle timings
        "on_deletion" => T::OnDeletion,
        "on_any_deletion" => T::OnAnyDeletion,
        "on_enter_field" => T::OnEnterField,
        "on_enter_field_anyone" => T::OnEnterFieldAnyone,

        // Permanent-state timings
        "on_suspend" => T::OnSuspend,
        "on_unsuspend" => T::OnUnsuspend,

        // Turn-boundary timings
        "start_of_your_turn" => T::StartOfYourTurn,
        "start_of_opponents_turn" => T::StartOfOpponentsTurn,
        "start_of_your_main_phase" => T::StartOfYourMainPhase,
        "end_of_your_turn" => T::EndOfYourTurn,
        "end_of_opponents_turn" => T::EndOfOpponentsTurn,

        // Observer timings
        "on_ally_played" => T::OnAllyPlayed,
        "on_ally_attack" => T::OnAllyAttack,
        "on_opponent_attack" => T::OnOpponentAttack,
        "on_attack_target_change" => T::OnAttackTargetChange,

        _ => return None,
    })
}

