use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

const SECURITY_ACTIVATE_MAIN_YAML: &str = r#"
card: TEST-SECURITY-MAIN
name: Security Main
kind: option
color: [yellow]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 3
  - when: on_discard_security
    process:
      - activate_main_effects: { source: event_card }
"#;

fn option_card() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("TEST-SECURITY-MAIN", "Security Main");
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = 0;
    card
}

#[test]
fn on_discard_security_can_activate_option_main_from_pending_security_without_hand_use() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SECURITY_ACTIVATE_MAIN_YAML)
        .expect("security-main DSL loads")
        .add_card(make_test_card("TRASHER", "Security Trasher"))
        .add_card(option_card())
        .hand(1, &["TEST-SECURITY-MAIN"])
        .security(1, &["TEST-SECURITY-MAIN"])
        .memory(0)
        .start();
    let trasher = runner.place_on_field(0, "TRASHER", Some(0));
    let source_card = runner.game.players[0].battle_area[trasher.index as usize]
        .top_card()
        .handle();

    runner.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(trasher), 0);
        assert!(ctx.trash_top_security(1));
    }
    runner.game.set_effect_source_player_for_test(None);

    assert_eq!(
        runner.memory(),
        -3,
        "effect-trashed security card should activate its OptionMain body"
    );
    assert_eq!(
        runner.hand_size(1),
        1,
        "activating from pending security must not consume the hand copy"
    );
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.game.players[1].trash.len(), 1);
}
