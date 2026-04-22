use digimon_engine::card_source::CardHandle;
use digimon_engine::effect::Effect;
use digimon_engine::enums::EffectTiming;

#[test]
fn effect_builder_exposes_cost_reduction_fn() {
    let effect = Effect::before_pay_cost(CardHandle(0))
        .name("test reduction")
        .condition(|_ctx| true)
        .cost_reduction_fn(|_ctx| 3)
        .build();
    assert!(effect.cost_reduction_fn.is_some());
}

#[test]
fn effect_builder_exposes_pay_cost_fn() {
    let effect = Effect::on_play(CardHandle(0))
        .name("test pay cost")
        .process(|_ctx| {})
        .pay_cost_fn(|_ctx| true)
        .build();
    assert!(effect.pay_cost_fn.is_some());
}

#[test]
fn before_pay_cost_constructor_sets_correct_timing() {
    let effect = Effect::before_pay_cost(CardHandle(0))
        .name("timing check")
        .build();
    assert_eq!(effect.timing, EffectTiming::BeforePayCost);
}
