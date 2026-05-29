use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT19-057";

fn raptor_sparrowmon() -> CardData {
    let mut card = make_test_card_with_level("BT19-061", "RaptorSparrowmon", 4);
    card.colors = vec![CardColor::Black, CardColor::Purple];
    card.traits = vec![
        "Cyborg".to_string(),
        "Twilight".to_string(),
        "Xros Heart".to_string(),
    ];
    card.evo_costs = vec![EvoCost {
        card_color: 5,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn insert_source(runner: &mut DebugRunner, host: PermanentHandle, card_id: &str) -> CardHandle {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    let card = CardSource::new(data_index, host.player, instance_id);
    let handle = card.handle();
    let sources = &mut runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources;
    let insert_at = sources.len().saturating_sub(1);
    sources.insert(insert_at, card);
    handle
}

fn source_ids(runner: &DebugRunner, host: PermanentHandle) -> Vec<String> {
    runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-057 YAML loads")
        .add_card(raptor_sparrowmon())
        .add_card(tamer("XROS-TAMER"))
        .memory(10)
        .start()
}

#[test]
fn bt19_057_when_attacking_may_digivolve_from_under_tamer() {
    let mut runner = runner();
    let sparrow = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(0, "XROS-TAMER", Some(0));
    let raptor_handle = insert_source(&mut runner, tamer, "BT19-061");

    let result = runner.attack_player(sparrow, 1, true);
    assert_ne!(result, AttackResult::Invalid);

    let pending = runner
        .pending_selection()
        .expect("When Attacking should offer RaptorSparrowmon under a Tamer");
    assert!(pending.is_optional, "printed 'may digivolve' is declinable");
    let raptor_action = encode_source_select(tamer.index as u16, 0).unwrap();
    assert!(pending.valid_action_ids.contains(&raptor_action));

    runner
        .execute_action(0, raptor_action)
        .expect("choose RaptorSparrowmon under Tamer");
    runner.auto_resolve().expect("settle digivolution");

    let evolved = &runner.game.players[0].battle_area[sparrow.index as usize];
    assert_eq!(evolved.top_card().handle(), raptor_handle);
    assert_eq!(source_ids(&runner, tamer), vec!["XROS-TAMER"]);
}

#[test]
fn bt19_057_when_attacking_decline_leaves_under_tamer_card_in_place() {
    let mut runner = runner();
    let sparrow = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(0, "XROS-TAMER", Some(0));
    let raptor_handle = insert_source(&mut runner, tamer, "BT19-061");

    let result = runner.attack_player(sparrow, 1, true);
    assert_ne!(result, AttackResult::Invalid);

    let pending = runner
        .pending_selection()
        .expect("When Attacking should offer an optional source-zone digivolve");
    assert!(pending.is_optional);
    assert!(pending.valid_action_ids.contains(&PASS));
    runner.execute_action(0, PASS).expect("decline digivolve");
    runner
        .auto_resolve()
        .expect("settle declined attack trigger");

    let sparrow_after = &runner.game.players[0].battle_area[sparrow.index as usize];
    assert_eq!(
        sparrow_after.top_card().card_id(&runner.game.card_data),
        CARD_ID
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].handle(),
        raptor_handle
    );
}

#[test]
fn bt19_057_inherited_grants_collision_to_xros_heart_carrier() {
    let mut runner = runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "BT19-061"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Collision),
        "Sparrowmon inherited text should grant Collision to an Xros Heart carrier"
    );
}
