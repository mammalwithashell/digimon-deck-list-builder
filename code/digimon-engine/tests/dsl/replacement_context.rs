use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use std::sync::Arc;

#[test]
fn replacement_subject_and_source_predicates_compile_together() {
    let yaml = r#"
card: TEST-CROSS-REPLACEMENT
name: Cross Replacement Test
kind: digimon
color: [yellow]
level: 6
cost: 11
dp: 11000
effects:
  - kind: replacement
    timing: when_would_be_deleted
    active_when:
      replacement_subject_is_mine: true
      replacement_source_is_opponent: false
      replacement_cause: opponent_effect
    outcome: prevent
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("replacement compiles");
    assert_eq!(compiled.card, "TEST-CROSS-REPLACEMENT");
}

#[test]
fn replacement_active_when_trait_matches_replacement_subject_not_source() {
    let yaml = r#"
card: TEST-SUBJECT-TRAIT-PROTECTOR
name: Subject Trait Protector
kind: digimon
color: [yellow]
level: 6
cost: 11
dp: 11000
traits: [Protector]
effects:
  - kind: replacement
    timing: when_would_be_deleted
    active_when:
      all_of:
        - trait_has: Free
        - replacement_cause: opponent_effect
    process:
      - cancel_replacement: {}
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("replacement compiles");

    let mut runner = DebugRunner::builder()
        .add_card(digimon_card(
            "TEST-SUBJECT-TRAIT-PROTECTOR",
            CardColor::Yellow,
            &[],
        ))
        .add_card(digimon_card("FREE-TARGET", CardColor::Blue, &["Free"]))
        .add_card(digimon_card("PLAIN-TARGET", CardColor::Blue, &[]))
        .memory(0)
        .start();
    runner.register_effect(
        "TEST-SUBJECT-TRAIT-PROTECTOR",
        Arc::new(DslCardEffect::new(Arc::new(compiled))),
    );

    runner.place_on_field(0, "TEST-SUBJECT-TRAIT-PROTECTOR", Some(0));
    let free = runner.place_on_field(0, "FREE-TARGET", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-TARGET", Some(0));

    runner
        .game
        .delete_permanent_with_cause(free, ReplacementCause::OpponentEffect);
    assert!(
        find_permanent(&runner, 0, "FREE-TARGET").is_some(),
        "cross-permanent replacement must match the replacement subject's Free trait"
    );

    runner
        .game
        .delete_permanent_with_cause(plain, ReplacementCause::OpponentEffect);
    assert!(
        find_permanent(&runner, 0, "PLAIN-TARGET").is_none(),
        "replacement must not fire for an unqualified subject"
    );
}

fn digimon_card(card_id: &str, color: CardColor, traits: &[&str]) -> digimon_engine::CardData {
    let mut card = make_test_card(card_id, card_id);
    card.colors = vec![color];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn find_permanent(runner: &DebugRunner, player: u8, card_id: &str) -> Option<PermanentHandle> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .enumerate()
        .find(|(_, perm)| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .map(|(index, _)| PermanentHandle {
            player,
            index: index as u8,
        })
}
