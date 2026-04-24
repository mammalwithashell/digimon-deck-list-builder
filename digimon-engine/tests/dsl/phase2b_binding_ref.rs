use digimon_dsl::compiled::CompiledBindingRef;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::binding_ref::{resolve_binding_ref, ResolvedBinding};
use digimon_engine::dsl_cards::bindings::{BindingValue, Bindings};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn self_ref_resolves_to_source_card() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let ctx = EffectContext::new(&mut runner.game, card, None, 0);
    let b = Bindings::new();
    let r = resolve_binding_ref(&CompiledBindingRef::SelfRef, &ctx, &b);
    assert_eq!(r, Some(ResolvedBinding::Card(card)));
}

#[test]
fn source_ref_resolves_to_source_permanent_when_present() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let handle = runner.place_on_field(0, "F", None);  // NOTE signature: (player, card_id, turn_override)
    let card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, card, Some(handle), 0);
    let b = Bindings::new();
    let r = resolve_binding_ref(&CompiledBindingRef::Source, &ctx, &b);
    assert_eq!(r, Some(ResolvedBinding::Permanent(handle)));
}

#[test]
fn named_ref_looks_up_in_bindings() {
    let mut runner = DebugRunner::builder().add_card(make_test_card("F", "F")).hand(0, &["F"]).build();
    let card = runner.game.players[0].hand[0].handle();
    let ctx = EffectContext::new(&mut runner.game, card, None, 0);
    let mut b = Bindings::new();
    let perm = PermanentHandle { player: 0, index: 3 };
    b.insert("tgt", BindingValue::Permanent(perm));
    let r = resolve_binding_ref(&CompiledBindingRef::Named("tgt".into()), &ctx, &b);
    assert_eq!(r, Some(ResolvedBinding::Permanent(perm)));
}

#[test]
fn named_ref_missing_returns_none() {
    let mut runner = DebugRunner::builder().add_card(make_test_card("F", "F")).hand(0, &["F"]).build();
    let card = runner.game.players[0].hand[0].handle();
    let ctx = EffectContext::new(&mut runner.game, card, None, 0);
    let b = Bindings::new();
    let r = resolve_binding_ref(&CompiledBindingRef::Named("missing".into()), &ctx, &b);
    assert!(r.is_none());
}
