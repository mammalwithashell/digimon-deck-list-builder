//! BT24-088 Asuna Shiroki.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-088";

#[test]
fn bt24_088_yaml_metadata_and_clauses_match_printed_card() {
    let runner = asuna_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT24-088");

    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.traits, vec!["TS".to_string()]);

    let start = triggered(&card.effects, CompiledTiming::StartOfYourTurn);
    assert!(start.optional);
    assert_eq!(start.scope, CompiledScope::FaceUp);
    assert!(matches!(
        start.process.as_slice(),
        [
            CompiledStep::ActivationCost { .. },
            CompiledStep::SelectTrash { .. },
            CompiledStep::PlayFromTrashFree { .. },
        ]
    ));

    let on_play = triggered(&card.effects, CompiledTiming::OnPlay);
    assert!(on_play.optional);
    assert!(on_play.condition.is_some());
    assert!(matches!(
        on_play.process.as_slice(),
        [
            CompiledStep::SelectHand { .. },
            CompiledStep::TrashFromHandByIndex { .. },
            CompiledStep::Draw { count: 2, .. },
        ]
    ));

    let security = triggered(&card.effects, CompiledTiming::OnSecurity);
    assert_eq!(security.process, vec![CompiledStep::PlayFromSecurity]);
}

#[test]
fn bt24_088_start_of_turn_plays_only_asuna_or_low_level_ts_or_three_musketeers_text() {
    let mut runner = asuna_runner()
        .add_card(asuna_copy("ASUNA-COPY"))
        .add_card(ts_digimon("TS-L4", 4))
        .add_card(ts_digimon("TS-L5", 5))
        .add_card(three_musketeers_text_digimon("TEXT-L4", 4))
        .add_card(non_matching_digimon("MISS", 4))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();
    let asuna = runner.place_on_field(0, CARD_ID, Some(0));
    for id in ["ASUNA-COPY", "TS-L4", "TS-L5", "TEXT-L4", "MISS"] {
        push_to_trash(&mut runner, 0, id);
    }
    runner.game.set_memory(4);
    let memory_before = runner.memory();

    fire_start_of_turn(&mut runner, asuna);
    runner
        .accept_optional_trigger()
        .expect("accept Asuna trigger");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    let valid = runner
        .pending_selection()
        .expect("trash selection")
        .valid_action_ids
        .clone();
    assert_eq!(valid.len(), 3);
    assert!(valid.contains(&trash_action_for_id(&runner, "ASUNA-COPY")));
    assert!(valid.contains(&trash_action_for_id(&runner, "TS-L4")));
    assert!(valid.contains(&trash_action_for_id(&runner, "TEXT-L4")));
    assert!(!valid.contains(&trash_action_for_id(&runner, "TS-L5")));
    assert!(!valid.contains(&trash_action_for_id(&runner, "MISS")));

    let pick = trash_action_for_id(&runner, "TEXT-L4");
    runner
        .execute_action(0, pick)
        .expect("play Three Musketeers-text Digimon");
    runner.auto_resolve().expect("settle trash free-play");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TEXT-L4"),
        "selected trash Digimon must be played"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_from_trash_free must not charge play cost"
    );
}

#[test]
fn bt24_088_start_of_turn_decline_keeps_tamer_on_field() {
    let mut runner = asuna_runner()
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();
    let asuna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(4);

    fire_start_of_turn(&mut runner, asuna);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    assert!(runner.pending_is_optional());
    runner.execute_action(0, PASS).expect("decline trigger");
    runner.auto_resolve().expect("settle decline");

    assert!(runner
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID));
}

#[test]
fn bt24_088_on_play_trashes_matching_hand_card_and_draws_two() {
    let mut runner = asuna_runner()
        .add_card(ts_digimon("TS-DISCARD", 3))
        .add_card(three_musketeers_text_digimon("TEXT-DISCARD", 3))
        .add_card(non_matching_digimon("MISS", 3))
        .add_card(filler("DRAW-A"))
        .add_card(filler("DRAW-B"))
        .hand(0, &[CARD_ID, "MISS", "TS-DISCARD", "TEXT-DISCARD"])
        .deck(0, &["DRAW-B", "DRAW-A"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Asuna");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        runner.pending_is_optional(),
        "on-play discard cost must expose a decline choice"
    );
    let valid = runner
        .pending_selection()
        .expect("hand discard selection")
        .valid_action_ids
        .clone();
    assert_eq!(valid.len(), 2);
    assert!(valid.contains(&hand_action_for_id(&runner, "TS-DISCARD")));
    assert!(valid.contains(&hand_action_for_id(&runner, "TEXT-DISCARD")));
    assert!(!valid.contains(&hand_action_for_id(&runner, "MISS")));

    runner
        .execute_action(0, hand_action_for_id(&runner, "TS-DISCARD"))
        .expect("trash TS card from hand");
    runner.auto_resolve().expect("draw two");

    let hand_ids = zone_ids(&runner.game.player(0).hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"DRAW-A".to_string()));
    assert!(hand_ids.contains(&"DRAW-B".to_string()));
    assert!(!hand_ids.contains(&"TS-DISCARD".to_string()));
    assert_eq!(runner.deck_size(0), 0);
}

fn asuna_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-088 YAML loads")
}

fn triggered(
    effects: &[CompiledClause],
    timing: CompiledTiming,
) -> &digimon_dsl::compiled::CompiledTriggeredClause {
    effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered) if triggered.when.contains(&timing) => {
                Some(triggered)
            }
            _ => None,
        })
        .expect("triggered clause exists")
}

fn fire_start_of_turn(runner: &mut DebugRunner, source: digimon_engine::PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn asuna_copy(id: &str) -> CardData {
    let mut card = make_test_card(id, "Asuna Shiroki");
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Purple];
    card.traits = vec!["TS".to_string()];
    card.dp = None;
    card.level = None;
    card.play_cost = 3;
    card
}

fn ts_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.traits = vec!["TS".to_string()];
    card.level = Some(level);
    card.dp = Some(4000);
    card.play_cost = 6;
    card
}

fn three_musketeers_text_digimon(id: &str, level: u8) -> CardData {
    let mut card = non_matching_digimon(id, level);
    card.effect_text = "This card references Three Musketeers in its text.".to_string();
    card
}

fn non_matching_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.traits.clear();
    card.level = Some(level);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, card_index));
}

fn trash_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .player(0)
        .trash
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(TRASH_EFFECT_START + idx as u16)
        })
        .expect("trash card exists")
}

fn hand_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .player(0)
        .hand
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(PLAY_HAND_START + idx as u16)
        })
        .expect("hand card exists")
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
