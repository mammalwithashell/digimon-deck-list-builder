//! ST10-06 Mastemon.

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn st10_06_when_digivolving_places_yellow_or_purple_trash_digimon_on_top_security() {
    let mut runner = mastemon_runner().start();
    let mastemon = runner.place_on_field(0, "ST10-06", Some(0));
    push_to_trash(&mut runner, 0, "TRASH-YELLOW");

    fire_when_digivolving(&mut runner, mastemon, false);
    choose_first_pending(&mut runner);
    runner.auto_resolve().expect("finish security placement");

    assert_eq!(security_ids(&runner, 0), vec!["TRASH-YELLOW"]);
    assert_eq!(runner.trash_size(0), 0);
}

#[test]
fn st10_06_dna_when_digivolving_may_play_level_5_or_lower_digimon_from_security() {
    let mut runner = mastemon_runner().security(0, &["SEC-L6", "SEC-L5"]).start();
    let mastemon = runner.place_on_field(0, "ST10-06", Some(0));

    fire_when_digivolving(&mut runner, mastemon, true);
    let security = runner
        .pending_selection_view()
        .expect("DNA branch should offer a security Digimon");
    assert_eq!(security.kind, SelectionKind::Security);
    assert_eq!(
        security.valid_action_ids.len(),
        1,
        "only the level 5 or lower security Digimon should be selectable"
    );
    runner
        .execute_action(security.selecting_player, security.valid_action_ids[0])
        .expect("choose level 5 security Digimon");
    runner.auto_resolve().expect("play security Digimon");

    assert!(battle_area_contains(&runner, 0, "SEC-L5"));
    assert_eq!(security_ids(&runner, 0), vec!["SEC-L6"]);
}

#[test]
fn st10_06_effect_play_deletes_opponent_digimon_at_or_below_played_level() {
    let mut runner = mastemon_runner().start();
    let mastemon = runner.place_on_field(0, "ST10-06", Some(0));
    let played = runner.place_on_field(0, "PLAYED-L5", Some(0));
    let low = runner.place_on_field(1, "OPP-L4", Some(0));
    let _high = runner.place_on_field(1, "OPP-L6", Some(0));

    fire_ally_played(&mut runner, mastemon, played, true);
    let delete = runner
        .pending_selection_view()
        .expect("effect-play trigger should select an opponent Digimon");
    assert_eq!(
        delete.valid_action_ids,
        vec![encode_permanent(low)],
        "only opponent Digimon with level <= played Digimon's level may be selected"
    );
    runner
        .execute_action(delete.selecting_player, delete.valid_action_ids[0])
        .expect("choose deletion target");
    runner.auto_resolve().expect("delete target");

    assert!(!battle_area_contains(&runner, 1, "OPP-L4"));
    assert!(battle_area_contains(&runner, 1, "OPP-L6"));
}

fn mastemon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("ST10-06")
        .expect("ST10-06 YAML loads")
        .add_card(digimon("TRASH-YELLOW", 4, 4000, vec![CardColor::Yellow]))
        .add_card(digimon("SEC-L5", 5, 7000, vec![CardColor::Yellow]))
        .add_card(digimon("SEC-L6", 6, 11000, vec![CardColor::Purple]))
        .add_card(digimon("PLAYED-L5", 5, 7000, vec![CardColor::Yellow]))
        .add_card(digimon("OPP-L4", 4, 5000, vec![CardColor::Red]))
        .add_card(digimon("OPP-L6", 6, 11000, vec![CardColor::Red]))
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle, dna_origin: bool) {
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

fn fire_ally_played(
    runner: &mut DebugRunner,
    _source: PermanentHandle,
    played: PermanentHandle,
    effect_initiated: bool,
) {
    let card = runner.game.players[played.player as usize].battle_area[played.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnAllyPlayed,
        TriggerSource::EnteredField {
            player: played.player,
            permanent: played,
            card,
            effect_initiated,
        },
    );
    runner.game.drain_effect_queue();
}

fn choose_first_pending(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("pending selection");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose first pending action");
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card exists");
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

fn security_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn battle_area_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}

fn digimon(id: &str, level: u8, dp: i32, colors: Vec<CardColor>) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.colors = colors;
    card
}
