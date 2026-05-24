//! BT24-083 Hiroko Sagisaka.
//! Printed text covered here:
//! - [Start of Your Turn] memory <= 4, return-self cost, optional free play
//!   of Hiroko or a TS Digimon with 5000 DP or less from hand.
//! - [On Play] reveal top 3, add 1 TS card, bottom the rest.
//! - [Security] play this Tamer without paying the cost.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-083";

#[test]
fn bt24_083_yaml_metadata_and_clauses_match_printed_card() {
    let runner = hiroko_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT24-083");

    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.color, vec![CompiledColor::Red]);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.traits, vec!["TS".to_string()]);
    assert_eq!(card.effects.len(), 3);

    let start = triggered(&card.effects, CompiledTiming::StartOfYourTurn);
    assert!(start.optional);
    assert_eq!(start.scope, CompiledScope::FaceUp);
    assert!(start.condition.is_some());
    assert!(matches!(
        start.process.as_slice(),
        [
            CompiledStep::ActivationCost { .. },
            CompiledStep::SelectHand { .. },
            CompiledStep::PlayFromHandFree { .. },
        ]
    ));

    let on_play = triggered(&card.effects, CompiledTiming::OnPlay);
    assert!(!on_play.optional);
    assert!(matches!(
        on_play.process.as_slice(),
        [
            CompiledStep::RevealTopDeck { count: 3, .. },
            CompiledStep::SelectRevealBuckets { .. },
            CompiledStep::AddToHandFromReveal { .. },
            CompiledStep::PlaceRemainderOnDeck { .. },
        ]
    ));

    let security = triggered(&card.effects, CompiledTiming::OnSecurity);
    assert!(!security.optional);
    assert_eq!(security.process, vec![CompiledStep::PlayFromSecurity]);
}

#[test]
fn bt24_083_start_of_turn_decline_keeps_tamer_on_field() {
    let mut runner = hiroko_runner()
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();
    let hiroko = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(4);

    fire_start_of_turn(&mut runner, hiroko);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline Hiroko trigger");
    runner.auto_resolve().expect("settle decline");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "declining must leave Hiroko on the field"
    );
}

#[test]
fn bt24_083_start_of_turn_plays_only_hiroko_or_low_dp_ts_digimon() {
    let mut runner = hiroko_runner()
        .add_card(ts_digimon("TS-LOW", 5000))
        .add_card(ts_digimon("TS-HIGH", 6000))
        .add_card(non_ts_digimon("NON-TS", 3000))
        .add_card(hiroko_copy("HIROKO-COPY"))
        .add_card(filler("FILL"))
        .hand(0, &["TS-HIGH", "NON-TS", "TS-LOW", "HIROKO-COPY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(4)
        .start();
    let hiroko = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(4);
    let memory_before = runner.memory();

    fire_start_of_turn(&mut runner, hiroko);
    runner
        .accept_optional_trigger()
        .expect("accept Hiroko trigger");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let valid = runner
        .pending_selection()
        .expect("hand selection")
        .valid_action_ids
        .clone();
    assert_eq!(
        valid.len(),
        2,
        "only exact-name Hiroko and TS Digimon with DP <= 5000 may be selected"
    );
    let low_action = hand_action_for_id(&runner, "TS-LOW");
    let hiroko_action = hand_action_for_id(&runner, "HIROKO-COPY");
    assert!(valid.contains(&low_action));
    assert!(valid.contains(&hiroko_action));
    assert!(!valid.contains(&hand_action_for_id(&runner, "TS-HIGH")));
    assert!(!valid.contains(&hand_action_for_id(&runner, "NON-TS")));

    runner
        .execute_action(0, low_action)
        .expect("play the low-DP TS Digimon");
    runner.auto_resolve().expect("settle free play");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TS-LOW"),
        "selected TS Digimon must be played"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "source Hiroko must leave field as the return-self cost"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_from_hand_free must not deduct the selected Digimon's play cost"
    );
}

#[test]
fn bt24_083_start_of_turn_memory_gate_blocks_when_above_four() {
    let mut runner = hiroko_runner()
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();
    let hiroko = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.set_memory(5);

    fire_start_of_turn(&mut runner, hiroko);

    assert!(
        runner.pending_selection().is_none(),
        "memory > 4 must not offer Hiroko's optional start-of-turn trigger"
    );
    assert!(runner
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID));
}

#[test]
fn bt24_083_on_play_adds_one_ts_card_and_bottoms_the_remainder() {
    let mut runner = hiroko_runner()
        .add_card(ts_digimon("TS-HIT", 3000))
        .add_card(non_ts_digimon("MISS-A", 3000))
        .add_card(non_ts_digimon("MISS-B", 3000))
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "MISS-B", "MISS-A", "TS-HIT"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Hiroko");
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::RevealBucket { .. })
    ));
    assert!(!runner.pending_is_optional());
    let pick = revealed_action_for_id(&runner, "TS-HIT");
    runner.execute_action(0, pick).expect("select revealed TS");
    runner.auto_resolve().expect("bottom reveal remainder");

    let hand_ids = zone_ids(&runner.game.player(0).hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"TS-HIT".to_string()));
    assert!(!hand_ids.contains(&"MISS-A".to_string()));
    assert!(!hand_ids.contains(&"MISS-B".to_string()));

    let deck_ids = zone_ids(&runner.game.player(0).deck, &runner.game.card_data);
    assert_eq!(
        deck_ids[0..2],
        ["MISS-B".to_string(), "MISS-A".to_string()],
        "unchosen reveal cards must be returned to deck bottom"
    );
}

fn hiroko_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-083 YAML loads")
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

fn hiroko_copy(id: &str) -> CardData {
    let mut card = make_test_card(id, "Hiroko Sagisaka");
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["TS".to_string()];
    card.play_cost = 3;
    card.dp = None;
    card.level = None;
    card
}

fn ts_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["TS".to_string()];
    card.play_cost = 7;
    card.dp = Some(dp);
    card
}

fn non_ts_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.traits.clear();
    card.play_cost = 3;
    card.dp = Some(dp);
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn fire_start_of_turn(runner: &mut DebugRunner, source: digimon_engine::PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
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

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
