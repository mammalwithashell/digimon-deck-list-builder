//! Synthetic evidence that direct-reference syntax is not a transitive certificate.
use digimon_engine::{
    card_data::EvoCost, card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, context_liveness::ignores_event_card_identity, step::run_steps},
    effect::{CardEffect, Effect}, effect_context::EffectContext,
    enums::{CardColor, CardKind, Expiry, ModifierType},
    modifiers::ModifierEntry, permanent::PermanentHandle, trigger_context::TriggerContext, Game,
};
use digimon_dsl::compiled::*;
use std::sync::Arc;

struct EventReader;
impl CardEffect for EventReader {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card).condition(|ctx| ctx.cost_is_digivolve)
            .cost_reduction_fn(|ctx| i32::from(ctx.event_card().is_some())).build()]
    }
}
fn fixture() -> Game {
    let mut base = make_test_card("CT-BASE", "Base");
    base.card_kind = CardKind::Digimon; base.level = Some(3);
    base.dp = Some(3000); base.colors = vec![CardColor::Red];
    let mut evo = make_test_card("CT-EVO", "Evolution");
    evo.card_kind = CardKind::Digimon; evo.level = Some(4);
    evo.dp = Some(5000); evo.colors = vec![CardColor::Red];
    evo.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 4 }];
    let mut reader = make_test_card("CT-READER", "Reader");
    reader.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder().add_card(base).add_card(evo).add_card(reader)
        .add_card(make_test_card("CT-EVENT", "Event"))
        .hand(0, &["CT-EVO"]).hand(1, &["CT-EVENT"])
        .deck(0, &["CT-BASE"; 8]).memory(5).start();
    r.register_effect("CT-READER", Arc::new(EventReader));
    r.place_on_field(0, "CT-READER", Some(0));
    r.place_on_field(0, "CT-BASE", Some(0));
    let mut context = TriggerContext::default();
    context.event_card = (Some(r.game.players[1].hand[0].handle())).into();
    r.game.current_trigger_context = Some(context);
    r.game
}
fn clause(process: Vec<CompiledStep>) -> CompiledTriggeredClause {
    CompiledTriggeredClause {
        when: vec![CompiledTiming::OnOwnSecurityRemoved], scope: CompiledScope::FaceUp,
        active_when: None, condition: None, trigger_if: None, timing_conditions: vec![],
        optional: true, outer_prompt: false, once_per_turn: false, max_per_turn: None,
        process, summary: None, summary_key: None,
    }
}
fn execute(game: &mut Game, steps: &[CompiledStep]) {
    let target = PermanentHandle { player: 0, index: 1 };
    let source = game.players[0].battle_area[0].top_card().handle();
    let mut bindings = Bindings::default();
    bindings.insert_permanent("target", target);
    bindings.insert_hand_index_for("candidate", 0, 0);
    run_steps(steps, &mut EffectContext::new(game, source,
        Some(PermanentHandle { player: 0, index: 0 }), 0), &mut bindings);
}

#[test]
fn named_literal_evolution_is_not_an_unconditional_projection_certificate() {
    let clause = clause(vec![CompiledStep::EffectInitiatedDigivolve {
        target: CompiledBindingRef::Named("target".into()),
        from_hand: CompiledBindingRef::Named("candidate".into()),
        cost: CompiledCostDelta::Reduce(1), ignore_requirements: false, ignore_level: false,
    }]);
    let mut original = fixture();
    let mut erased = original.clone();
    erased.current_trigger_context.as_mut().unwrap().event_card = (None).into();
    execute(&mut original, &clause.process);
    execute(&mut erased, &clause.process);
    assert_eq!(original.memory, 3, "4 minus literal 1 minus captured-event reducer 1");
    assert_eq!(erased.memory, 2, "same named-target DSL execution pays one more after erasure");
    for game in [&original, &erased] {
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[0].battle_area[1].top_card().card_id(&game.card_data), "CT-EVO");
    }
    assert!(!ignores_event_card_identity(&clause),
        "direct named bindings cannot certify transitive payment callbacks");
}

#[test]
fn literal_memory_is_not_an_unconditional_projection_certificate() {
    let clause = clause(vec![CompiledStep::GainMemory(1)]);
    let mut original = fixture();
    let target = PermanentHandle { player: 0, index: 1 };
    original.modifiers.add(target,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, 0)
            .with_until_condition(Arc::new(|game, _| game.current_trigger_context.as_ref()
                .is_some_and(|context| context.event_card.read().is_some()))));
    let mut erased = original.clone();
    erased.current_trigger_context.as_mut().unwrap().event_card = (None).into();
    execute(&mut original, &clause.process);
    execute(&mut erased, &clause.process);
    assert_eq!((original.memory, erased.memory), (6, 6));
    assert_eq!(original.modifiers.sum(target, ModifierType::ChangeDp), 1000);
    assert_eq!(erased.modifiers.sum(target, ModifierType::ChangeDp), 0);
    assert!(!ignores_event_card_identity(&clause),
        "literal memory can reevaluate a captured-context modifier predicate");
}

#[test]
fn empty_body_still_requires_context_independent_predicates() {
    let mut clause = clause(vec![]);
    assert!(ignores_event_card_identity(&clause));
    clause.condition = Some(CompiledPredicate {
        event_card_name_contains: Some("Event".into()), ..Default::default()
    });
    assert!(!ignores_event_card_identity(&clause));
}
