use digimon_engine::action::space::{encode_source_select, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 3;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

const COST_YAML: &str = r#"
card: ROCK-COST-TAMER
name: Rocks Cost Tamer
kind: tamer
color: [purple]
cost: 0
traits: [LIBERATOR]
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, material]
          optional: true
          bind_as: cost_card
          filter:
            any_of:
              - trait_has: Mineral
              - trait_has: Rock
          prompt: "Trash a Mineral or Rock from hand or sources"
          then:
            - trash_union_bound:
                binding: cost_card
            - gain_memory: 1
"#;

fn runner_with_cost_tamer() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(COST_YAML)
        .expect("cost YAML parses")
        .add_card(digimon("MIN-HAND", "Mineral Hand", &["Mineral"]))
        .add_card(digimon("MIN-SRC", "Mineral Source", &["Mineral"]))
        .add_card(digimon("BAD-SRC", "Plain Source", &["Beast"]))
        .add_card(digimon("HOST", "Rock Host", &["Rock"]))
        .hand(0, &["ROCK-COST-TAMER", "MIN-HAND"])
        .memory(10)
        .build()
}

#[test]
fn optional_union_cost_trashes_hand_card_before_gain() {
    let mut runner = runner_with_cost_tamer();

    runner.play(0, 0).expect("play cost tamer");
    let memory_before = runner.game.memory;
    let prompt = runner
        .pending_selection_view()
        .expect("hand/source cost prompt opens");
    assert_eq!(
        prompt.kind,
        SelectionKind::UnionZone {
            zones: digimon_engine::selection::UnionZoneSet::HAND
                | digimon_engine::selection::UnionZoneSet::MATERIAL
        }
    );
    assert!(
        prompt.valid_action_ids.contains(&PLAY_HAND_START),
        "Mineral hand card must be a payable union cost candidate"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("pay cost from hand");
    runner.auto_resolve().expect("resolve cost benefit");

    assert_eq!(runner.game.memory, memory_before + 1);
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "MIN-HAND"),
        "selected hand card should be trashed as the cost"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "MIN-HAND"),
        "selected hand card must leave hand"
    );
}

#[test]
fn optional_union_cost_trashes_own_digimon_source_before_gain() {
    let mut runner = runner_with_cost_tamer();
    let host = runner.place_stack(0, &["MIN-SRC", "BAD-SRC", "HOST"]);

    runner.play(0, 0).expect("play cost tamer");
    let memory_before = runner.game.memory;
    let prompt = runner
        .pending_selection_view()
        .expect("hand/source cost prompt opens");
    let mineral_source_action =
        encode_source_select(host.index as u16, 0).expect("encode Mineral source");
    let bad_source_action = encode_source_select(host.index as u16, 1).expect("encode bad source");
    assert!(
        prompt.valid_action_ids.contains(&mineral_source_action),
        "Mineral source under an own Digimon must be a payable union cost candidate"
    );
    assert!(
        !prompt.valid_action_ids.contains(&bad_source_action),
        "non-Mineral/Rock source must be filtered out"
    );

    runner
        .execute_action(0, mineral_source_action)
        .expect("pay cost from own Digimon source");
    runner.auto_resolve().expect("resolve cost benefit");

    assert_eq!(runner.game.memory, memory_before + 1);
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "MIN-SRC"),
        "selected digivolution card should be trashed as the cost"
    );
    assert!(
        !runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "MIN-SRC"),
        "selected digivolution card must leave the source stack"
    );
}

#[test]
fn optional_union_cost_decline_skips_gain_and_trash() {
    let mut runner = runner_with_cost_tamer();

    runner.play(0, 0).expect("play cost tamer");
    let memory_before = runner.game.memory;
    runner.execute_action(0, PASS).expect("decline cost");
    runner.auto_resolve().expect("finish after decline");

    assert_eq!(
        runner.game.memory, memory_before,
        "declining the cost must skip the success benefit"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|card| card.card_id(&runner.game.card_data) != "MIN-HAND"),
        "declining must not trash the hand card"
    );
}

#[test]
fn union_cost_without_eligible_card_no_ops() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(COST_YAML)
        .expect("cost YAML parses")
        .add_card(digimon("HOST", "Plain Host", &["Beast"]))
        .hand(0, &["ROCK-COST-TAMER"])
        .memory(10)
        .build();
    runner.place_stack(0, &["HOST"]);

    runner.play(0, 0).expect("play cost tamer");
    let memory_before = runner.game.memory;
    runner.auto_resolve().expect("unpayable cost no-ops");

    assert!(
        runner.pending_selection_view().is_none(),
        "unpayable cost must not open a prompt"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "unpayable cost must not run the success benefit"
    );
}
