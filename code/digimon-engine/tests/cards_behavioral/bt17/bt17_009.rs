//! BT17-009 Flamemon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_009.cs
//! Covers On Play multi-bucket reveal and inherited On Deletion free Tamer play.

use digimon_dsl::compiled::{CompiledColor, CompiledCost};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn bt17_009_yaml_has_red_level_2_digivolve_path() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/bt17/BT17-009.yaml"))
        .expect("BT17-009 YAML parses");
    let compiled = compile(&spec).expect("BT17-009 YAML compiles");
    assert_eq!(compiled.alt_paths.len(), 1);
    assert_eq!(compiled.alt_paths[0].cost, Some(CompiledCost::Literal(0)));

    let from = compiled.alt_paths[0]
        .from
        .as_ref()
        .expect("digivolve path has a source predicate");
    assert!(
        from.all_of
            .iter()
            .any(|predicate| predicate.level_eq == Some(2)),
        "BT17-009 must digivolve from level 2"
    );
    assert!(
        from.all_of
            .iter()
            .any(|predicate| predicate.color_is == Some(CompiledColor::Red)),
        "BT17-009's special level-2 path must require red"
    );
}

#[test]
fn bt17_009_on_play_adds_hybrid_and_inherited_tamer_then_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-009")
        .expect("BT17-009 YAML loads")
        .add_card(make_trait_card("HYBRID", &["Hybrid"]))
        .add_card(make_inherited_tamer("TAMER-INHERITED"))
        .add_card(make_test_card("BLANK", "Blank"))
        .deck(0, &["BLANK", "TAMER-INHERITED", "HYBRID"])
        .hand(0, &["BT17-009"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Flamemon");
    assert_required_pending_pick(&runner, "Hybrid bucket");
    pick_first_pending(&mut runner, "pick Hybrid bucket");
    assert_required_pending_pick(&runner, "Tamer bucket");
    pick_first_pending(&mut runner, "pick Tamer bucket");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"HYBRID".to_string()));
    assert!(hand_ids.contains(&"TAMER-INHERITED".to_string()));
    assert_eq!(
        runner.game.players[0]
            .deck
            .first()
            .unwrap()
            .card_id(&runner.game.card_data),
        "BLANK",
        "unchosen reveal card returned to deck bottom"
    );
}

#[test]
fn bt17_009_inherited_on_deletion_may_play_tamer_with_inherited_effect_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT17-009")
        .expect("BT17-009 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_inherited_tamer("TAMER-INHERITED"))
        .hand(0, &["TAMER-INHERITED"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT17-009", "CARRIER"]);
    runner.game.delete_permanent_with_cause(
        carrier,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );

    assert!(
        runner.pending_is_optional(),
        "inherited On Deletion Tamer play should be optional"
    );
    pick_first_pending(&mut runner, "choose Tamer");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "TAMER-INHERITED"),
        "Tamer with inherited effect was played for free"
    );
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: reveal bucket should be mandatory when a matching card exists"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal before the required pick"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_inherited_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = digimon_engine::enums::CardKind::Tamer;
    card.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".to_string();
    card
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn pick_first_pending(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect(label);
}
