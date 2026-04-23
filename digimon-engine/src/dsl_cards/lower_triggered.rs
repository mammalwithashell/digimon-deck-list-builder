//! Lower `CompiledClause::Triggered` — emits one engine `Effect` per
//! entry in `clause.when` that maps to a real `EffectTiming`.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledScope, CompiledTriggeredClause};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::dsl_cards::raw_rust::EngineRawRustRegistry;
use crate::dsl_cards::step::run_step_with_raw;
use crate::dsl_cards::timing_map::compiled_timing_to_engine;
use crate::effect::{Effect, EffectBuilder};
use crate::enums::EffectTiming;

pub fn lower(
    card: CardHandle,
    clause: &CompiledTriggeredClause,
    raw: Option<Arc<EngineRawRustRegistry>>,
) -> Vec<Effect> {
    let mut out = Vec::new();
    for t in &clause.when {
        let Some(engine_timing) = compiled_timing_to_engine(*t) else {
            continue;
        };

        let process_steps = Arc::new(clause.process.clone());
        let active_when = clause.active_when.clone().map(Arc::new);
        let condition = clause.condition.clone().map(Arc::new);
        let raw_clone = raw.clone();
        let scope = clause.scope;
        let optional = clause.optional;
        let once_per_turn = clause.once_per_turn;
        let max_per_turn = clause.max_per_turn;
        let summary = clause.summary.clone();

        let mut builder = new_builder(card, engine_timing);
        if matches!(scope, CompiledScope::Inherited) {
            builder = builder.inherited();
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

        if active_when.is_some() || condition.is_some() {
            let aw = active_when.clone();
            let cc = condition.clone();
            builder = builder.condition(move |rctx| {
                if let Some(p) = &aw {
                    if !eval_predicate(p, rctx, PredicateSubject::None) {
                        return false;
                    }
                }
                if let Some(p) = &cc {
                    if !eval_predicate(p, rctx, PredicateSubject::None) {
                        return false;
                    }
                }
                true
            });
        }

        builder = builder.process(move |ctx| {
            let mut bindings = Bindings::new();
            for step in process_steps.iter() {
                run_step_with_raw(step, ctx, &mut bindings, raw_clone.as_deref());
            }
        });

        out.push(builder.build());
    }
    out
}

fn new_builder(card: CardHandle, timing: EffectTiming) -> EffectBuilder {
    match timing {
        EffectTiming::OnPlay => Effect::on_play(card),
        EffectTiming::WhenDigivolving => Effect::when_digivolving(card),
        EffectTiming::OnAttack => Effect::on_attack(card),
        EffectTiming::OnDeletion => Effect::on_deletion(card),
        EffectTiming::SecuritySkill => Effect::security(card),
        EffectTiming::BeforePayCost => Effect::before_pay_cost(card),
        EffectTiming::WhenAttacking => Effect::when_attacking(card),
        EffectTiming::EndOfAttack => Effect::end_of_attack(card),
        EffectTiming::EndOfBattle => Effect::end_of_battle(card),
        EffectTiming::StartOfYourTurn => Effect::start_of_your_turn(card),
        EffectTiming::StartOfOpponentsTurn => Effect::start_of_opponents_turn(card),
        EffectTiming::StartOfYourMainPhase => Effect::start_of_your_main_phase(card),
        EffectTiming::EndOfYourTurn => Effect::end_of_your_turn(card),
        EffectTiming::EndOfOpponentsTurn => Effect::end_of_opponents_turn(card),
        EffectTiming::OnEnterFieldAnyone => Effect::on_enter_field_anyone(card),
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
        EffectTiming::OnDigivolutionCardTrashed => Effect::on_digivolution_card_trashed(card),
        EffectTiming::OnSecurityCheck => Effect::on_security_check(card),
        EffectTiming::OnLoseSecurity => Effect::on_lose_security(card),
        other => EffectBuilder::new(card, other),
    }
}
