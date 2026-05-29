//! BT9-082 Ordinemon.

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt9/BT9-082.yaml");

#[test]
fn bt9_082_dna_when_digivolving_deletes_large_and_small_digimon_then_recovers() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT9-082 YAML loads")
        .add_card(level_digimon("LV6", 6))
        .add_card(level_digimon("LV5-A", 5))
        .add_card(level_digimon("LV5-B", 5))
        .add_card(level_digimon("LV6-MATERIAL-A", 6))
        .add_card(level_digimon("LV6-MATERIAL-B", 6))
        .add_card(make_test_card("REC1", "Recover 1"))
        .add_card(make_test_card("REC2", "Recover 2"))
        .add_card(make_test_card("REC3", "Recover 3"))
        .deck(0, &["REC1", "REC2", "REC3"])
        .memory(0)
        .start();
    let ordinemon = runner.place_stack(0, &["LV6-MATERIAL-A", "LV6-MATERIAL-B", "BT9-082"]);
    runner.place_on_field(1, "LV6", Some(0));
    runner.place_on_field(1, "LV5-A", Some(0));
    runner.place_on_field(1, "LV5-B", Some(0));

    fire_digivolving(&mut runner, ordinemon, true);
    let lv6_pick = runner
        .pending_selection_view()
        .expect("level 6 selection")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, lv6_pick);
    runner.execute_action(0, lv6_pick).expect("choose level 6");
    runner
        .auto_resolve()
        .expect("deletions and recovery resolve");

    assert_eq!(runner.battle_area_size(1), 0);
    assert_eq!(runner.security_count(0), 3);
}

#[test]
fn bt9_082_on_deletion_trashes_security_to_play_itself_from_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT9-082 YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .memory(0)
        .start();
    let ordinemon = runner.place_on_field(0, "BT9-082", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ordinemon, ReplacementCause::OpponentEffect);
    let accept = runner
        .pending_selection_view()
        .expect("optional play-self trigger")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, accept);
    runner
        .execute_action(0, accept)
        .expect("accept optional play-self trigger");
    runner.auto_resolve().expect("play resolves");

    assert!(battle_area_contains(&runner, 0, "BT9-082"));
    assert_eq!(runner.security_count(0), 0);
}

fn fire_digivolving(runner: &mut DebugRunner, handle: PermanentHandle, dna_origin: bool) {
    let card = runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: handle.player,
            permanent: handle,
            card,
            effect_initiated: false,
            dna_origin,
        },
    );
    runner.game.drain_effect_queue();
}

fn level_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card
}

fn battle_area_contains(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == id)
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
