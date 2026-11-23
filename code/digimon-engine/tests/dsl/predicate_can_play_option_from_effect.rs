//! `can_play_from_effect_without_cost` on an Option card: the effect can use
//! it only when its colour requirement is met (CR 4.3 4-22, checked even for
//! free uses) and Option use isn't prohibited. LM-067 Gundramon filters its
//! reveal pick with it so an unusable Option is never offered.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const SELECTOR_YAML: &str = r#"
card: TEST-CPO
name: Usable Option Selector
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: pick
          optional: true
          filter: { kind: option, can_play_from_effect_without_cost: true }
          prompt: "Pick an Option you could use"
"#;

fn offered(with_blue_digimon: bool) -> usize {
    let mut blue_opt = make_test_card("BLUE-OPT", "Blue Option");
    blue_opt.card_kind = CardKind::Option;
    blue_opt.colors = vec![CardColor::Blue];
    let mut blue_mon = make_test_card("BLUE-MON", "Blue Digimon");
    blue_mon.colors = vec![CardColor::Blue];
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SELECTOR_YAML)
        .expect("selector YAML parses")
        .add_card(blue_opt)
        .add_card(blue_mon)
        .add_card(make_test_card("OTHER", "Other"))
        .deck(0, &["OTHER"; 4])
        .deck(1, &["OTHER"; 4])
        .hand(0, &["BLUE-OPT"])
        .memory(5)
        .start();
    if with_blue_digimon {
        runner.place_on_field(0, "BLUE-MON", Some(0));
    }
    let sel = runner.place_on_field(0, "TEST-CPO", Some(0));
    runner.fire_on_play(0, sel.index as usize);
    runner
        .game
        .pending_selection
        .as_ref()
        .map(|p| p.valid_action_ids.len())
        .unwrap_or(0)
}

#[test]
fn option_with_unmet_colour_requirement_is_not_offered() {
    assert_eq!(offered(false), 0);
}

#[test]
fn option_with_met_colour_requirement_is_offered() {
    assert_eq!(offered(true), 1);
}
