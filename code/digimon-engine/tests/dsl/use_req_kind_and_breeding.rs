//! `<Use Req.>` (CR 16-42-3): "allows a player to ignore the color requirements
//! with specific Digimon and/or Tamers on the field". DCGO
//! `CardEffectFactory.UseRequirements` counts YOUR Digimon or Tamers in the
//! battle area OR breeding area whose top card meets the condition — an Option
//! card sitting on the field (a <Delay> Option) does not count. The printed
//! `<Use Req.>` YAMLs use this `use_requirement` shape (deck audit 2026-11-10:
//! they used to count Options, and some ignored the breeding area).
use digimon_engine::{
    action::{mask::build_action_mask, space::PLAY_HAND_START},
    card_data::CardData,
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind},
};

const YAML: &str = r#"
card: USE-REQ-OPT
name: Use Req option
kind: option
color: [black]
cost: 1
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: "Squad"
effects:
  - when: main_from_hand
    summary: "[Main] Gain 1 memory"
    process:
      - gain_memory: 1
"#;

fn red(id: &str, kind: CardKind, level: Option<u8>, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.level = level;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(red("SQ-DIGI", CardKind::Digimon, Some(4), &["Squad"]))
        .add_card(red("SQ-TAMER", CardKind::Tamer, None, &["Squad"]))
        .add_card(red("SQ-EGG", CardKind::Digimon, Some(3), &["Squad"]))
        .add_card(red("SQ-OPTION", CardKind::Option, None, &["Squad"]))
        .add_card(red("PLAIN", CardKind::Digimon, Some(4), &[]))
        .hand(0, &["USE-REQ-OPT"])
        .deck(0, &["PLAIN"; 8])
        .deck(1, &["PLAIN"; 8])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "PLAIN", None);
    r
}

fn usable(r: &mut DebugRunner) -> bool {
    r.game.enter_main_phase();
    build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0
}

#[test]
fn a_matching_digimon_or_tamer_waives_the_colour_requirement() {
    let mut r = runner();
    assert!(!usable(&mut r), "no black and no [Squad] Digimon/Tamer");

    let mut r = runner();
    r.place_on_field(0, "SQ-DIGI", None);
    assert!(usable(&mut r), "a [Squad] Digimon qualifies");

    let mut r = runner();
    r.place_on_field(0, "SQ-TAMER", None);
    assert!(usable(&mut r), "a [Squad] Tamer qualifies");
}

#[test]
fn a_matching_option_on_the_field_does_not_count() {
    let mut r = runner();
    r.place_on_field(0, "SQ-OPTION", None);
    assert!(
        !usable(&mut r),
        "a [Squad] Option in the battle area is not a Digimon or Tamer"
    );
}

#[test]
fn a_matching_digimon_in_the_breeding_area_counts() {
    let mut r = runner();
    r.place_in_breeding(0, "SQ-EGG");
    assert!(usable(&mut r), "the breeding area is part of the field for <Use Req.>");
}
