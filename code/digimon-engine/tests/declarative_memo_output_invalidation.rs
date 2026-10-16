//! Materialized output removal must invalidate the incremental declarative memo.
//! Uses the real BT25-075 aura, not a replacement effect stub.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword, ModifierType};

#[test]
fn clearing_materialized_recipient_state_rebuilds_continuous_aura_on_next_tick() {
    let mut recipient = make_test_card("AURA-RECIPIENT", "Aura Recipient");
    recipient.card_kind = CardKind::Digimon;
    recipient.level = Some(3);
    recipient.dp = Some(3000);
    recipient.traits = vec!["TS".to_string()];
    let mut runner = DebugRunner::builder().dsl_card("BT25-075").unwrap()
        .add_card(recipient).memory(3).start();
    runner.place_on_field(1, "BT25-075", None);
    let target = runner.place_on_field(1, "AURA-RECIPIENT", None);
    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(target, Keyword::Rush));
    assert!(runner.game.modifiers.has(target, ModifierType::ChangeLinkMax));
    // A registry cleanup removes derived outputs without changing board inputs.
    // The next tick must not reuse a memo describing the removed outputs.
    runner.game.modifiers.clear_permanent(target);
    assert!(!runner.game.modifiers.has_keyword(target, Keyword::Rush));
    runner.game.tick_declarative_effects();
    assert!(runner.game.has_keyword(target, Keyword::Rush));
    assert!(runner.game.modifiers.has(target, ModifierType::ChangeLinkMax));
}
