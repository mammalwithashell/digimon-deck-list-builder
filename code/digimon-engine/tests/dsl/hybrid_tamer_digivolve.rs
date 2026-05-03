use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardKind, GamePhase};

const TAMER_YAML: &str = r#"
card: T5-TAMER
name: Tamer Base
kind: tamer
color: [red]
cost: 3
"#;

const HYBRID_YAML: &str = r#"
card: T5-HYBRID
name: Hybrid Result
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Hybrid]
alt_paths:
  - kind: digivolve
    from: { kind: tamer, name_is: "Tamer Base" }
    source_treated_as: level_3_red_digimon
    cost: 2
"#;

const HYBRID_MISMATCH_YAML: &str = r#"
card: T5-HYBRID-MISMATCH
name: Hybrid Mismatch
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Hybrid]
alt_paths:
  - kind: digivolve
    from: { kind: tamer, name_is: "Tamer Base" }
    source_treated_as: level_5_green_digimon
    cost: 2
"#;

const HYBRID_UNSUPPORTED_FIRST_YAML: &str = r#"
card: T5-HYBRID-ORDER
name: Hybrid Ordered Paths
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Hybrid]
alt_paths:
  - kind: digivolve
    from: { kind: tamer, name_is: "Tamer Base" }
    source_treated_as: level_3_red_digimon
    cost:
      formula: { base: 7, per: material_count, delta: 1 }
  - kind: digivolve
    from: { kind: tamer, name_is: "Tamer Base" }
    source_treated_as: level_3_red_digimon
    cost: 2
"#;

const ROOKIE_YAML: &str = r#"
card: T5-ROOKIE
name: Red Rookie
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
"#;

const HYBRID_PRINTED_AND_ALT_YAML: &str = r#"
card: T5-HYBRID-CHEAP
name: Hybrid Cheap Route
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Hybrid]
alt_paths:
  - kind: digivolve
    from: { kind: digimon, name_is: "Red Rookie" }
    source_treated_as: level_3_red_digimon
    cost: 1
"#;

fn add_red_level3_evo_cost(runner: &mut DebugRunner, card_id: &str, memory_cost: u16) {
    let card = runner
        .game
        .card_data
        .iter_mut()
        .find(|card| card.card_id == card_id)
        .expect("test card exists");
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost,
    }];
}

#[test]
fn hybrid_tamer_digivolve_mask_exposes_tamer_base_without_mutating_kind() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(TAMER_YAML)
        .expect("tamer DSL compiles")
        .from_dsl_yaml(HYBRID_YAML)
        .expect("hybrid DSL compiles")
        .hand(0, &["T5-HYBRID"])
        .memory(5)
        .start();
    add_red_level3_evo_cost(&mut runner, "T5-HYBRID", 2);

    let tamer = runner.place_on_field(0, "T5-TAMER", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    let action = encode_digivolve(0, tamer.index as u16);

    assert_eq!(
        mask[action as usize], 1.0,
        "source_treated_as should expose the Tamer as a legal Hybrid digivolve base"
    );
    assert_eq!(
        runner.game.card_data[runner.game.players[0].battle_area[tamer.index as usize]
            .top_card()
            .data_index]
            .card_kind,
        CardKind::Tamer,
        "legality must not mutate the printed Tamer kind"
    );
}

#[test]
fn hybrid_tamer_digivolve_action_uses_alt_path_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(TAMER_YAML)
        .expect("tamer DSL compiles")
        .from_dsl_yaml(HYBRID_YAML)
        .expect("hybrid DSL compiles")
        .hand(0, &["T5-HYBRID"])
        .memory(5)
        .start();
    add_red_level3_evo_cost(&mut runner, "T5-HYBRID", 2);

    let tamer = runner.place_on_field(0, "T5-TAMER", Some(0));
    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        tamer.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve
    ));

    let stack = &runner.game.players[0].battle_area[tamer.index as usize].card_sources;
    assert_eq!(
        stack.last().unwrap().card_id(&runner.game.card_data),
        "T5-HYBRID"
    );
    assert_eq!(
        stack.first().unwrap().card_id(&runner.game.card_data),
        "T5-TAMER"
    );
    assert_eq!(runner.game.memory, 3);
}

#[test]
fn source_treated_as_profile_must_match_result_card_evo_costs() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(TAMER_YAML)
        .expect("tamer DSL compiles")
        .from_dsl_yaml(HYBRID_MISMATCH_YAML)
        .expect("hybrid DSL compiles")
        .hand(0, &["T5-HYBRID-MISMATCH"])
        .memory(5)
        .start();
    add_red_level3_evo_cost(&mut runner, "T5-HYBRID-MISMATCH", 2);

    let tamer = runner.place_on_field(0, "T5-TAMER", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    let action = encode_digivolve(0, tamer.index as u16);

    assert_eq!(
        mask[action as usize], 0.0,
        "mismatched source_treated_as profiles must not expose Tamer digivolve"
    );
}

#[test]
fn unsupported_alt_path_cost_shape_does_not_block_later_supported_path() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(TAMER_YAML)
        .expect("tamer DSL compiles")
        .from_dsl_yaml(HYBRID_UNSUPPORTED_FIRST_YAML)
        .expect("hybrid DSL compiles")
        .hand(0, &["T5-HYBRID-ORDER"])
        .memory(5)
        .start();
    add_red_level3_evo_cost(&mut runner, "T5-HYBRID-ORDER", 2);

    let tamer = runner.place_on_field(0, "T5-TAMER", Some(0));
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    let action = encode_digivolve(0, tamer.index as u16);
    assert_eq!(mask[action as usize], 1.0);
    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        tamer.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve
    ));
    assert_eq!(runner.game.memory, 3);
}

#[test]
fn dsl_alt_path_can_choose_cheaper_route_than_printed_evo_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ROOKIE_YAML)
        .expect("rookie DSL compiles")
        .from_dsl_yaml(HYBRID_PRINTED_AND_ALT_YAML)
        .expect("hybrid DSL compiles")
        .hand(0, &["T5-HYBRID-CHEAP"])
        .memory(5)
        .start();
    add_red_level3_evo_cost(&mut runner, "T5-HYBRID-CHEAP", 3);

    let rookie = runner.place_on_field(0, "T5-ROOKIE", Some(0));
    runner.game.current_phase = GamePhase::Main;

    assert!(runner.game.digivolve_from_hand(
        0,
        0,
        rookie.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve
    ));
    assert_eq!(
        runner.game.memory, 4,
        "execution should use the cheaper legal DSL alt-path cost"
    );
}
