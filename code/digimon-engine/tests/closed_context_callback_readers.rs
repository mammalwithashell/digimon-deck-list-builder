//! A transitive-reader counterexample for closed-world context certificates.
//! This is synthetic and model-free. A literal evolution step is not sufficient
//! evidence that its callers can erase a captured event-card reference.
use digimon_engine::{
    card_data::EvoCost,
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect},
    enums::{CardColor, CardKind, CostDelta, PlaySource},
    permanent::PermanentHandle,
    trigger_context::TriggerContext,
};
use std::sync::Arc;

struct CapturedEventCostReader;
impl CardEffect for CapturedEventCostReader {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .condition(|ctx| ctx.cost_is_digivolve)
            .cost_reduction_fn(|ctx| i32::from(ctx.event_card().is_some()))
            .build()]
    }
}

#[test]
fn literal_evolution_cost_still_has_an_indirect_captured_context_reader() {
    let mut base = make_test_card("CTX-BASE", "Context base");
    base.card_kind = CardKind::Digimon;
    base.level = Some(3);
    base.dp = Some(3000);
    base.colors = vec![CardColor::Red];
    let mut evo = make_test_card("CTX-EVO", "Context evolution");
    evo.card_kind = CardKind::Digimon;
    evo.level = Some(4);
    evo.dp = Some(5000);
    evo.colors = vec![CardColor::Red];
    evo.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 4 }];
    let mut reducer = make_test_card("CTX-READER", "Context reader");
    reducer.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .add_card(base).add_card(evo).add_card(reducer)
        .add_card(make_test_card("CTX-EVENT", "Anonymous event"))
        .hand(0, &["CTX-EVO"]).hand(1, &["CTX-EVENT"])
        .deck(0, &["CTX-BASE"; 8]).memory(10).start();
    r.register_effect("CTX-READER", Arc::new(CapturedEventCostReader));
    r.place_on_field(0, "CTX-READER", Some(0));
    r.place_on_field(0, "CTX-BASE", Some(0));
    let event = r.game.players[1].hand[0].handle();
    let mut context = TriggerContext::default();
    context.event_card = (Some(event)).into();
    r.game.current_trigger_context = Some(context);
    let mut projected = r.game.clone();
    projected.current_trigger_context.as_mut().unwrap().event_card = (None).into();
    let target = PermanentHandle { player: 0, index: 1 };
    assert!(r.game.effect_initiated_digivolve(0, 0, target, CostDelta::Reduce(1), false, PlaySource::ByEffect));
    assert!(projected.effect_initiated_digivolve(0, 0, target, CostDelta::Reduce(1), false, PlaySource::ByEffect));
    assert_eq!(r.game.memory, 8, "printed 4 minus literal 1 minus callback 1");
    assert_eq!(projected.memory, 7, "erasing the event reference changes a real payment");
    assert_eq!(r.game.players[0].battle_area[1].top_card().card_id(&r.game.card_data), "CTX-EVO");
    assert_eq!(projected.players[0].battle_area[1].top_card().card_id(&projected.card_data), "CTX-EVO");
}
