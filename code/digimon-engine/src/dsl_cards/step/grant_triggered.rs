//! Track H §3 — `grant_triggered_effect` step handler.
//!
//! Walks battle areas for permanents matching the step's `target`
//! predicate; for each match, calls `EffectContext::grant_triggered_effect`
//! with a body closure that executes the compiled-step list via
//! `run_steps`. DCGO `AddSkillClass.cs` analog.
//!
//! Selection-driving bodies (RESOLVED — `G-ENGINE-GRANTED-EFFECT-SELECTION-BODY`,
//! round-2 gap closure): the old v1 note here claimed a granted body installing a
//! `PendingSelection` "will not compose … requires extending `QueuedEffect` with a
//! granted-effect discriminator". That extension LANDED with the Phase 4i queue
//! dispatch (`QueuedEffect::granted_effect_id` at `effect_queue.rs`): granted
//! bodies now ride the standard `QueuedEffect` queue and PARK selections exactly
//! like printed effects, so the drain stops (`drain_effect_queue_inner` returns
//! while `pending_selection.is_some()`), the player resolves, and the body's
//! selection continues. The BT23-032 / EX10-034 shape — a granted
//! `[Start of Your Main Phase] force_attack: { attacker: carrier }` (a MANDATORY
//! attack, DCGO `SetCanNotSelectNotAttack`) — parks correctly, COMPOSES across
//! multiple granted bodies (a `TriggerOrder` first, then each parks in turn), is
//! CLONE-SAFE (the selection carries a `BeginAttack` resume frame — pure data —
//! so a forked `Game` resolves it via the resumable VM, never the panic-stub
//! closure; rule 28), and is cleaned up on expiry (`end_of_opponents_turn` drains
//! the grant + prunes the body registry). Proof:
//! `tests/dsl/granted_effect_selection_body.rs`. EX1-068 Ice Wall! and other
//! non-selection bodies ("lose N memory" / "gain X") continue to work as before.
//!
//! Clone-safety contract (rule 28): the granted body itself is an `Arc<dyn Fn>`
//! that clones as an Arc pointer, and any selection it installs must be driven
//! through the resumable VM (a `ResumeFrame`) — which every DSL selection step
//! (`force_attack`, `select_*`, …) already does. A granted body must NOT install
//! a bespoke closure-only `pending_selection` with no resume frame, or a cloned
//! `Game` would hit the panic-stub on resolve.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledModifierTarget, CompiledStep};

use crate::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
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
    bindings: &mut Bindings,
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

    let matches = resolve_target_permanents(target, ctx, bindings);

    // Install one granted entry per matching carrier. Body closure
    // captures the compiled step list (Arc-wrapped for cheap clone)
    // and runs it through `run_steps` with a fresh `Bindings`. The
    // grantor identity (source_card / source_permanent) flows through
    // the inner ctx automatically — the engine sets them at fire time
    // per `Game::fire_granted_triggered_effects`.
    let body_arc: Arc<Vec<CompiledStep>> = Arc::new(body.clone());
    for carrier in matches {
        let body_for_closure = body_arc.clone();
        ctx.grant_triggered_effect(carrier, engine_timing, engine_expiry, move |inner_ctx| {
            let mut bindings = Bindings::default();
            let _ = run_steps(&body_for_closure, inner_ctx, &mut bindings);
        });
    }
    true
}

fn resolve_target_permanents(
    target: &CompiledModifierTarget,
    ctx: &EffectContext<'_>,
    bindings: &Bindings,
) -> Vec<PermanentHandle> {
    match target {
        CompiledModifierTarget::Binding(binding) => {
            match resolve_binding_ref(binding, ctx, bindings) {
                Some(ResolvedBinding::Permanent(handle)) => vec![handle],
                _ => Vec::new(),
            }
        }
        CompiledModifierTarget::Filter(predicate) => {
            // Snapshot matching permanents from a read borrow before installing
            // (the install path takes &mut and can't co-exist with the read
            // borrow).
            let target_arc = Arc::new(predicate.clone());
            let mut matches: Vec<PermanentHandle> = Vec::new();
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
            matches
        }
    }
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
        "on_block" => T::OnBlock,
        "on_attack_target_change" => T::OnAttackTargetChange,

        _ => return None,
    })
}
