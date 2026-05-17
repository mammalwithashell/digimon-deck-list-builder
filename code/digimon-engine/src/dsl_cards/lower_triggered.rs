//! Lower `CompiledClause::Triggered` — emits one engine `Effect` per
//! entry in `clause.when` that maps to a real `EffectTiming`.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledPredicate, CompiledScope, CompiledStep, CompiledTriggeredClause,
};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;
use crate::resource_flow::identifier_indicates_resource_flow;

pub fn lower(card: CardHandle, clause: &CompiledTriggeredClause) -> Vec<Effect> {
    lower_with_raw(card, clause, Arc::new(EngineRawRustRegistry::new()))
}

pub fn lower_with_raw(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Arc<EngineRawRustRegistry>,
) -> Vec<Effect> {
    lower_with_raw_and_option_use_requirement(card, clause, raw, None)
}

pub fn lower_with_raw_and_option_use_requirement(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Arc<EngineRawRustRegistry>,
    option_use_requirement: Option<Arc<CompiledPredicate>>,
) -> Vec<Effect> {
    lower_with_raw_and_option_use_requirement_for_kind(
        card,
        clause,
        raw,
        option_use_requirement,
        None,
    )
}

pub fn lower_with_raw_and_option_use_requirement_for_kind(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Arc<EngineRawRustRegistry>,
    option_use_requirement: Option<Arc<CompiledPredicate>>,
    card_kind: Option<CompiledCardKind>,
) -> Vec<Effect> {
    let mut out = Vec::new();
    for t in &clause.when {
        let Some(mut engine_timing) = compiled_timing_to_engine(*t) else {
            continue;
        };
        if matches!(
            card_kind,
            Some(CompiledCardKind::Option | CompiledCardKind::Dual)
        ) && engine_timing == EffectTiming::MainFromHand
        {
            engine_timing = EffectTiming::OptionMain;
        }

        let process_steps = Arc::new(clause.process.clone());
        let active_when = clause.active_when.clone().map(Arc::new);
        let condition = clause.condition.clone().map(Arc::new);
        let scope = clause.scope;
        let optional = clause.optional;
        let once_per_turn = clause.once_per_turn;
        let max_per_turn = clause.max_per_turn;
        let summary = clause.summary.clone();
        let raw_for_process = raw.clone();

        let mut builder = new_builder(card, engine_timing);
        if matches!(scope, CompiledScope::Inherited) {
            builder = builder.inherited();
        }
        if matches!(scope, CompiledScope::Linked) {
            builder = builder.linked();
        }
        if matches!(scope, CompiledScope::Security) {
            builder = builder.security_zone();
        }
        if let Some(s) = summary {
            builder = builder.name(&s);
        }
        if once_per_turn {
            builder = builder.once_per_turn();
        } else if let Some(n) = max_per_turn {
            // EffectBuilder only exposes once_per_turn today; n > 1 can't be
            // expressed. Mark once_per_turn for n == 1 so the common case
            // works; higher caps are a no-op in Phase 2a.
            if n == 1 {
                builder = builder.once_per_turn();
            }
        }
        if optional {
            builder = builder.optional();
        }
        if steps_provide_resource_flow(&clause.process) {
            builder = builder.resource_flow();
        }
        if matches!(
            engine_timing,
            EffectTiming::MainFromHand | EffectTiming::OptionMain
        ) {
            if let Some(use_req) = option_use_requirement.clone() {
                builder = builder.option_color_requirement_bypass_condition(move |rctx| {
                    eval_predicate(use_req.as_ref(), rctx, PredicateSubject::None)
                });
            }
        }

        if active_when.is_some() || condition.is_some() {
            let aw = active_when.clone();
            let cc = condition.clone();
            builder = builder.condition(move |rctx| {
                let subject = predicate_subject_for_source(rctx);
                if let Some(p) = &aw {
                    if !eval_predicate(p, rctx, subject) {
                        return false;
                    }
                }
                if let Some(p) = &cc {
                    if !eval_predicate(p, rctx, subject) {
                        return false;
                    }
                }
                true
            });
        }

        builder = builder.process(move |ctx| {
            let mut bindings = Bindings::new();
            // Phase 2b: `run_steps` drives the slice and yields control to
            // a selection step if one is encountered — installing the tail
            // as the selection's resolve callback so it continues once the
            // player picks a target.
            let runtime = StepRuntime::new(raw_for_process.clone())
                .with_dna_origin(ctx.game.current_dna_origin);
            run_steps_with_runtime(process_steps.as_slice(), ctx, &mut bindings, &runtime);
        });

        out.push(builder.build());
    }
    out
}

fn predicate_subject_for_source(
    rctx: &crate::effect_context::EffectReadContext<'_>,
) -> PredicateSubject {
    rctx.source_permanent
        .map(PredicateSubject::Permanent)
        .unwrap_or(PredicateSubject::None)
}

fn steps_provide_resource_flow(steps: &[CompiledStep]) -> bool {
    steps.iter().any(step_provides_resource_flow)
}

fn step_provides_resource_flow(step: &CompiledStep) -> bool {
    match step {
        CompiledStep::Draw { .. }
        | CompiledStep::AddToHandFromDeck { .. }
        | CompiledStep::AddToHandFromTrash { .. }
        | CompiledStep::AddToHandFromReveal { .. } => true,
        CompiledStep::RawRust { fn_name, .. } => identifier_indicates_resource_flow(fn_name),
        CompiledStep::AsSelectingPlayer { body, .. }
        | CompiledStep::ForEach { body, .. }
        | CompiledStep::PerSelected { body, .. }
        | CompiledStep::ScheduleDelayed { body, .. }
        | CompiledStep::Optional(body) => steps_provide_resource_flow(body),
        CompiledStep::If {
            then, else_branch, ..
        } => steps_provide_resource_flow(then) || steps_provide_resource_flow(else_branch),
        _ => false,
    }
}

fn new_builder(card: CardHandle, timing: EffectTiming) -> EffectBuilder {
    match timing {
        EffectTiming::OnPlay => Effect::on_play(card),
        EffectTiming::WhenDigivolving => Effect::when_digivolving(card),
        EffectTiming::OnAttack => Effect::on_attack(card),
        EffectTiming::OnDeletion => Effect::on_deletion(card),
        EffectTiming::SecuritySkill => Effect::security(card),
        EffectTiming::BeforePayCost => Effect::before_pay_cost(card),
        EffectTiming::BeforePayCostObserve => Effect::before_pay_cost_observe(card),
        EffectTiming::OptionMain => Effect::on_play(card).option_main(),
        EffectTiming::WhenAttacking => Effect::when_attacking(card),
        EffectTiming::EndOfAttack => Effect::end_of_attack(card),
        EffectTiming::EndOfBattle => Effect::end_of_battle(card),
        EffectTiming::StartOfYourTurn => Effect::start_of_your_turn(card),
        EffectTiming::StartOfOpponentsTurn => Effect::start_of_opponents_turn(card),
        EffectTiming::StartOfYourMainPhase => Effect::start_of_your_main_phase(card),
        EffectTiming::EndOfYourTurn => Effect::end_of_your_turn(card),
        EffectTiming::EndOfOpponentsTurn => Effect::end_of_opponents_turn(card),
        EffectTiming::OnEnterFieldAnyone => Effect::on_enter_field_anyone(card),
        EffectTiming::OnAllyPlayed => Effect::on_ally_played(card),
        EffectTiming::OnAnyDeletion => Effect::on_any_deletion(card),
        EffectTiming::OnDigivolve => Effect::on_digivolve(card),
        EffectTiming::OnSuspend => Effect::on_suspend(card),
        EffectTiming::OnUnsuspend => Effect::on_unsuspend(card),
        EffectTiming::OnAttackTargetChange => Effect::on_attack_target_change(card),
        EffectTiming::OnBlock => Effect::on_block(card),
        EffectTiming::OnAllyAttack => Effect::on_ally_attack(card),
        EffectTiming::OnOpponentAttack => Effect::on_opponent_attack(card),
        EffectTiming::OnHatch => Effect::on_hatch(card),
        EffectTiming::OnOpponentSecurityRemoved => Effect::on_opponent_security_removed(card),
        EffectTiming::OnOwnSecurityRemoved => Effect::on_own_security_removed(card),
        EffectTiming::OnDigivolutionCardTrashed => Effect::on_digivolution_card_trashed(card),
        EffectTiming::OnSecurityCheck => Effect::on_security_check(card),
        EffectTiming::OnLoseSecurity => Effect::on_lose_security(card),
        EffectTiming::OnDiscardSecurity => Effect::on_discard_security(card),
        EffectTiming::OnPlaceSecurity => Effect::on_place_security(card),
        other => EffectBuilder::new(card, other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use digimon_dsl::compiled::CompiledPlayerRef;

    #[test]
    fn resource_flow_catalog_detects_nested_draw_steps() {
        let steps = vec![CompiledStep::Optional(vec![CompiledStep::Draw {
            of: CompiledPlayerRef::You,
            count: 1,
        }])];

        assert!(steps_provide_resource_flow(&steps));
    }

    #[test]
    fn resource_flow_catalog_ignores_non_hand_resource_steps() {
        let steps = vec![CompiledStep::GainMemory(1)];

        assert!(!steps_provide_resource_flow(&steps));
    }
}
