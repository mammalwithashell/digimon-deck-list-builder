use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

fn digimon_card_with_trait(
    id: &str,
    dp: i32,
    trait_name: &str,
) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.traits.push(trait_name.to_string());
    card
}

#[test]
fn source_stack_dp_sum_formula_counts_matching_sources() {
    let yaml = r#"
card: TEST-SOURCE-STACK-DP-SUM
name: Source Stack DP Sum
kind: digimon
color: [yellow]
level: 6
cost: 11
dp: 11000
effects:
  - kind: aura
    target: {}
    dp_modifier_fn:
      source_stack_dp_sum:
        target: self
        filter: { trait_has: Iliad }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(digimon_card_with_trait("BT24-031", 1000, "Iliad"))
        .add_card(digimon_card_with_trait("BT24-037", 8000, "Iliad"))
        .add_card(digimon_card_with_trait("BT24-085", 9000, "Olympos XII"))
        .start();
    let stack = runner.place_stack(
        0,
        &[
            "BT24-031",
            "BT24-037",
            "BT24-085",
            "TEST-SOURCE-STACK-DP-SUM",
        ],
    );

    assert_eq!(runner.game.effective_dp(stack), Some(20000));
}

#[test]
fn source_stack_dp_sum_source_target_reads_effect_source_stack() {
    let yaml = r#"
card: TEST-SOURCE-STACK-AURA
name: Source Stack Aura
kind: digimon
color: [yellow]
level: 6
cost: 11
dp: 11000
traits: [AuraSource]
effects:
  - kind: aura
    target: { owner: you, trait: Ally, other: true }
    dp_modifier_fn:
      source_stack_dp_sum:
        target: source
        filter: { trait_has: Iliad }
"#;
    let mut ally = digimon_card_with_trait("ALLY-TARGET", 4000, "Ally");
    ally.level = Some(3);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(digimon_card_with_trait("SOURCE-ILIAD-1", 1000, "Iliad"))
        .add_card(digimon_card_with_trait("SOURCE-ILIAD-2", 8000, "Iliad"))
        .add_card(digimon_card_with_trait("TARGET-ILIAD", 2000, "Iliad"))
        .add_card(ally)
        .start();
    runner.place_stack(
        0,
        &["SOURCE-ILIAD-1", "SOURCE-ILIAD-2", "TEST-SOURCE-STACK-AURA"],
    );
    let ally = runner.place_stack(0, &["TARGET-ILIAD", "ALLY-TARGET"]);

    assert_eq!(runner.game.effective_dp(ally), Some(13000));
}
