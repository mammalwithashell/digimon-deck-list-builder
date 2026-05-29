use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;

const CARD_ID: &str = "BT10-111";

const ONE_SLOT_BALLISTAMON_XROS_TARGET: &str = r#"
card: TEST-BALLISTAMON-XROS
name: Ballistamon Requirement Target
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_is: Ballistamon }
        zones: [battle_area]
        cost_delta: -2
    cost: 5
"#;

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
}

#[test]
fn bt10_111_on_play_wildcard_can_replace_one_missing_digixros_requirement_this_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-111 YAML loads without raw_rust wildcard placeholder")
        .from_dsl_yaml(ONE_SLOT_BALLISTAMON_XROS_TARGET)
        .expect("test DigiXros target YAML compiles")
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    let memory_before = runner.memory();
    let _ = runner.play(0, 0);
    if runner
        .pending_selection()
        .is_some_and(|selection| selection.valid_action_ids.contains(&PASS))
    {
        runner
            .execute_action(0, PASS)
            .expect("decline BT10-111's own DigiXros materials");
    }
    runner
        .auto_resolve()
        .expect("resolve BT10-111 on-play wildcard registration");
    let king_index = runner.game.players[0]
        .battle_area
        .iter()
        .position(|permanent| permanent.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT10-111 should be played") as u8;
    push_to_hand(&mut runner, 0, "TEST-BALLISTAMON-XROS");

    let _ = runner.play(0, 0);

    let material_prompt = runner
        .pending_selection()
        .expect("DigiXros material prompt should be installed");
    assert!(
        material_prompt
            .valid_action_ids
            .contains(&(king_index as u16)),
        "BT10-111 should be legal as a wildcard material for one missing Ballistamon requirement"
    );
    runner
        .execute_action(0, king_index as u16)
        .expect("select BT10-111 as wildcard DigiXros material");
    runner
        .execute_action(0, PASS)
        .expect("finish DigiXros material selection");

    assert_eq!(
        memory_before - runner.memory(),
        8,
        "BT10-111 costs 5, then target cost 5 is reduced by one wildcard material to 3"
    );
    let target = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == "TEST-BALLISTAMON-XROS"
        })
        .expect("DigiXros target should be played");
    assert!(
        target
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == CARD_ID),
        "BT10-111 should become the selected DigiXros source"
    );
}
