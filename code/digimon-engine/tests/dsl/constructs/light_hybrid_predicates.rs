//! Existing inherited-text and reduced-play-cost predicates must constrain
//! the legal choice mask, independently of any production card's coverage.
use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const SELECTOR: &str = r#"
card: TEST-INHERITED-PAID-SELECTOR
name: Inherited paid selector
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: chosen
          optional: true
          filter:
            all_of:
              - kind: tamer
              - has_inherited: {}
              - affordable_with_cost_reduce: 2
          prompt: Choose a payable inherited Tamer
      - play_from_hand: {of: you, hand_index: chosen, cost_delta: {reduce: 2}}
"#;

fn setup() -> DebugRunner {
    let mut builder = DebugRunner::builder().from_dsl_yaml(SELECTOR).unwrap();
    for (id, cost, inherited) in [
        ("TEST-FREE", 2, true),
        ("TEST-PAID", 5, true),
        ("TEST-SECURITY-ONLY", 0, false),
        ("TEST-UNPAYABLE", 16, true),
    ] {
        let mut card = make_test_card(id, id);
        card.card_kind = CardKind::Tamer;
        card.level = None;
        card.dp = None;
        card.play_cost = cost;
        card.security_text = "[Security] Play this card without paying its cost.".into();
        if inherited {
            card.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
        }
        builder = builder.add_card(card);
    }
    let mut runner = builder
        .hand(
            0,
            &[
                "TEST-INHERITED-PAID-SELECTOR",
                "TEST-FREE",
                "TEST-PAID",
                "TEST-SECURITY-ONLY",
                "TEST-UNPAYABLE",
            ],
        )
        .memory(3)
        .start();
    runner.play(0, 0).unwrap();
    runner
}

#[test]
fn light_hybrid_predicates_mask_inherited_text_and_payable_reduced_cost() {
    let runner = setup();
    let mask = build_action_mask(&runner.game, 0);
    assert!(mask[PLAY_HAND_START as usize] > 0.5);
    assert!(mask[(PLAY_HAND_START + 1) as usize] > 0.5);
    assert_eq!(
        mask[(PLAY_HAND_START + 2) as usize],
        0.0,
        "Security text is not inherited text"
    );
    assert_eq!(
        mask[(PLAY_HAND_START + 3) as usize],
        0.0,
        "16 minus 2 exceeds the available 13 memory"
    );
    assert!(mask[PASS as usize] > 0.5);
}

#[test]
fn light_hybrid_predicates_paid_choice_and_zero_floor_use_real_payment() {
    for (action, memory) in [(PLAY_HAND_START, 3), (PLAY_HAND_START + 1, 0)] {
        let mut runner = setup();
        runner.execute_action(0, action).unwrap();
        assert_eq!(runner.memory(), memory);
        assert_eq!(runner.game.players[0].battle_area.len(), 2);
        assert_eq!(runner.game.players[0].hand.len(), 3);
    }
}

#[test]
fn light_hybrid_predicates_decline_preserves_hand_and_memory() {
    let mut runner = setup();
    runner.execute_action(0, PASS).unwrap();
    assert_eq!(runner.memory(), 3);
    assert_eq!(runner.game.players[0].hand.len(), 4);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
}

#[test]
fn count_capped_trash_literal_can_choose_thirteen_existing_actions() {
    let yaml = r#"
card: TEST-THIRTEEN-CHOICES
name: Thirteen choices
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - select_count_capped_multi:
          of: you
          zone: trash
          min: 0
          max: 13
          optional_zero: true
          filter: {}
          bind_as: chosen
          prompt: Choose up to thirteen cards
      - place_cards_as_bottom_sources: {cards: chosen, target: source}
"#;
    let mut r = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
        .add_card(make_test_card("TEST-MATERIAL", "Material")).start();
    r.skip_mulligan();
    r.place_on_field(0, "TEST-THIRTEEN-CHOICES", None);
    for _ in 0..15 { r.inject_trash(0, "TEST-MATERIAL"); }
    r.fire_on_play(0, 0);
    for _ in 0..13 {
        let mask = build_action_mask(&r.game, 0);
        let action = r.pending_selection().unwrap().valid_action_ids.iter()
            .copied().find(|id| *id != PASS && mask[*id as usize] > 0.0).unwrap();
        r.execute_action(0, action).unwrap();
    }
    r.auto_resolve().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 14);
    assert_eq!(r.game.players[0].trash.len(), 2);
}

#[test]
fn self_color_count_lte_filters_over_cap_hand_cards() {
    use digimon_engine::enums::CardColor;
    let yaml = r#"
card: TEST-INHERITED-PAID-SELECTOR
name: Color bounds selector
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          optional: true
          bind_as: chosen
          prompt: Choose exactly three printed colors
          filter:
            kind: tamer
            self_color_count_gte: 3
            self_color_count_lte: 3
"#;
    let mut b = DebugRunner::builder().from_dsl_yaml(&yaml).unwrap();
    for (id, colors) in [
        ("TEST-TWO", vec![CardColor::Red, CardColor::Blue]),
        ("TEST-THREE", vec![CardColor::Red, CardColor::Blue, CardColor::Yellow]),
        ("TEST-FOUR", vec![CardColor::Red, CardColor::Blue, CardColor::Yellow, CardColor::Purple]),
    ] {
        let mut card = make_test_card(id, id);
        card.card_kind = CardKind::Tamer;
        card.colors = colors;
        b = b.add_card(card);
    }
    let mut r = b.hand(0, &["TEST-TWO", "TEST-THREE", "TEST-FOUR"]).start();
    r.skip_mulligan();
    r.place_on_field(0, "TEST-INHERITED-PAID-SELECTOR", None);
    r.fire_on_play(0, 0);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0);
    assert!(mask[(PLAY_HAND_START + 1) as usize] > 0.0);
    assert_eq!(mask[(PLAY_HAND_START + 2) as usize], 0.0);
}
