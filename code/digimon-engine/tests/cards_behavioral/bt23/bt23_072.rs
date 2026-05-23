//! BT23-072 King Drasil_7D6.
//!
//! Covered slices:
//! - [Hand][Main] pay 3 and tuck this card under a breeding King Drasil_7D6
//!   or Mother Eater, then Draw 1.
//! - [All Turns] optional suspend-self cost when your Royal Knight/CS Digimon
//!   is played, granting Rush/Raid/Reboot/Blocker until opponent turn end.
//! - Inherited [Breeding][Start of Your Main Phase] optional King Drasil source
//!   play when the carrier has 6+ digivolution cards.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_breeding_select, encode_breeding_source_select, HAND_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(6);
    card.dp = Some(9000);
    card.play_cost = 6;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-072")
        .expect("BT23-072 YAML loads")
        .add_card(digimon("KING-HOST", "King Drasil_7D6", &["CS"]))
        .add_card(digimon("MOTHER-HOST", "Mother Eater", &["CS"]))
        .add_card(digimon("ROYAL-KNIGHT", "Magnamon", &["Royal Knight"]))
        .add_card(digimon("KING-SOURCE", "King Drasil_7D6", &["CS"]))
        .add_card(digimon("FILLER-1", "Filler 1", &[]))
        .add_card(digimon("FILLER-2", "Filler 2", &[]))
        .add_card(digimon("FILLER-3", "Filler 3", &[]))
        .add_card(digimon("FILLER-4", "Filler 4", &[]))
        .add_card(digimon("FILLER-5", "Filler 5", &[]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .memory(10)
        .start()
}

fn place_breeding_stack(runner: &mut DebugRunner, source_ids: &[&str], top_id: &str) {
    let mut ids = source_ids.to_vec();
    ids.push(top_id);
    let handle = runner.place_stack(0, &ids);
    let perm = runner.game.players[0]
        .battle_area
        .remove(handle.index as usize);
    runner.game.players[0].breeding_area = Some(perm);
}

fn card_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn find_field_card(runner: &DebugRunner, card_id: &str) -> PermanentHandle {
    let index = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} on field"));
    PermanentHandle {
        player: 0,
        index: index as u8,
    }
}

fn push_source_to_breeding(runner: &mut DebugRunner, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    let source = CardSource::new(data_idx, 0, next);
    let handle = source.handle();
    runner
        .game
        .players
        .get_mut(0)
        .unwrap()
        .breeding_area
        .as_mut()
        .unwrap()
        .card_sources
        .insert(0, source);
    handle
}

fn accept_pending_optional_trigger_or_replacement(runner: &mut DebugRunner) {
    while matches!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder | SelectionKind::Replacement)
    ) {
        let action = runner
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .find(|id| *id != PASS)
            .expect("accept action");
        runner
            .execute_action(0, action)
            .expect("accept pending optional prompt");
    }
}

#[test]
fn bt23_072_has_printed_metadata_and_three_effect_clauses() {
    let runner = runner();
    let card = runner.compiled_card("BT23-072").expect("BT23-072 present");

    assert_eq!(card.name, "King Drasil_7D6");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(9000));
    assert!(card.traits.iter().any(|name| name == "CS"));

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        3,
        "hand-main, played-Digimon observer, and inherited breeding play"
    );
    assert!(triggered
        .iter()
        .any(|clause| clause.when.contains(&CompiledTiming::MainFromHand)));
    assert!(triggered
        .iter()
        .any(|clause| clause.when.contains(&CompiledTiming::OnAllyPlayed)));
    assert!(triggered.iter().any(|clause| {
        clause.scope == CompiledScope::Inherited
            && clause.when.contains(&CompiledTiming::StartOfYourMainPhase)
    }));
}

#[test]
fn bt23_072_hand_main_pays_three_tucks_self_and_draws() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-072")
        .expect("BT23-072 YAML loads")
        .add_card(digimon("MOTHER-HOST", "Mother Eater", &["CS"]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .hand(0, &["BT23-072"])
        .deck(0, &["DRAW"])
        .memory(2)
        .start();
    runner.place_in_breeding(0, "MOTHER-HOST");
    let tucked = runner.game.players[0].hand[0].handle();

    assert_eq!(
        build_action_mask(&runner.game, 0)[HAND_EFFECT_START as usize],
        1.0,
        "[Hand][Main] should be legal with a breeding Mother Eater target"
    );
    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.game.memory, -1, "paying 3 may pass memory");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::BreedingPermanent)
    );

    runner
        .execute_action(0, encode_breeding_select(0).unwrap())
        .expect("choose Mother Eater");
    runner.auto_resolve().expect("finish tuck and draw");

    let breeding = runner.game.players[0].breeding_area.as_ref().unwrap();
    assert_eq!(breeding.card_sources[0].handle(), tucked);
    assert_eq!(
        card_ids(&runner.game.players[0].hand, &runner.game.card_data),
        vec!["DRAW".to_string()]
    );
}

#[test]
fn bt23_072_played_digimon_observer_can_be_declined_before_suspend_cost() {
    let mut runner = runner();
    let drasil = runner.place_on_field(0, "BT23-072", Some(0));
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "ROYAL-KNIGHT")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(data_idx, 0, next));

    runner.play(0, 0).expect("play Royal Knight");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline BT23-072 trigger");
    runner.auto_resolve().expect("finish decline");

    assert!(
        !runner.game.players[0].battle_area[drasil.index as usize].is_suspended,
        "declining the optional trigger must not pay the suspend cost"
    );
    let played = find_field_card(&runner, "ROYAL-KNIGHT");
    assert!(!runner.game.has_keyword(played, Keyword::Rush));
}

#[test]
fn bt23_072_played_digimon_observer_grants_four_keywords_after_suspend_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-072")
        .expect("BT23-072 YAML loads")
        .add_card(digimon("ROYAL-KNIGHT", "Magnamon", &["Royal Knight"]))
        .hand(0, &["ROYAL-KNIGHT"])
        .memory(10)
        .start();
    let drasil = runner.place_on_field(0, "BT23-072", Some(0));

    runner.play(0, 0).expect("play Royal Knight");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    let trigger_action = runner
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .iter()
        .copied()
        .find(|id| *id != PASS)
        .expect("accept action");
    runner
        .execute_action(0, trigger_action)
        .expect("accept BT23-072 trigger");
    runner.auto_resolve().expect("grant keywords");

    assert!(
        runner.game.players[0].battle_area[drasil.index as usize].is_suspended,
        "accepting the trigger pays the suspend-self cost"
    );
    let played = find_field_card(&runner, "ROYAL-KNIGHT");
    for keyword in [
        Keyword::Rush,
        Keyword::Raid,
        Keyword::Reboot,
        Keyword::Blocker,
    ] {
        assert!(
            runner.game.has_keyword(played, keyword),
            "played Royal Knight should gain {keyword:?}"
        );
    }
}

#[test]
fn bt23_072_inherited_breeding_start_main_plays_king_drasil_source_at_six_sources() {
    let mut runner = runner();
    place_breeding_stack(
        &mut runner,
        &[
            "BT23-072",
            "KING-SOURCE",
            "FILLER-1",
            "FILLER-2",
            "FILLER-3",
            "FILLER-4",
        ],
        "MOTHER-HOST",
    );
    let king_handle = runner.game.players[0]
        .breeding_area
        .as_ref()
        .unwrap()
        .card_sources[1]
        .handle();

    runner.game.enter_main_phase();
    accept_pending_optional_trigger_or_replacement(&mut runner);
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::Material | SelectionKind::CountCappedMultiSelect { .. })
        ),
        "accepted inherited trigger should prompt for a King Drasil source"
    );
    runner
        .execute_action(0, encode_breeding_source_select(0, 1).unwrap())
        .expect("choose King Drasil source");
    runner.auto_resolve().expect("play source");

    let played = find_field_card(&runner, "KING-SOURCE");
    assert_eq!(
        runner.game.players[0].battle_area[played.index as usize]
            .top_card()
            .handle(),
        king_handle
    );
}

#[test]
fn bt23_072_inherited_breeding_start_main_requires_six_sources() {
    let mut runner = runner();
    place_breeding_stack(
        &mut runner,
        &[
            "BT23-072",
            "KING-SOURCE",
            "FILLER-1",
            "FILLER-2",
            "FILLER-3",
        ],
        "MOTHER-HOST",
    );
    let before_field = runner.game.players[0].battle_area.len();

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(runner.pending_kind(), None);
    assert_eq!(runner.game.players[0].battle_area.len(), before_field);
}

#[test]
fn bt23_072_inherited_can_play_self_as_king_drasil_source() {
    let mut runner = runner();
    place_breeding_stack(
        &mut runner,
        &[
            "KING-SOURCE",
            "FILLER-1",
            "FILLER-2",
            "FILLER-3",
            "FILLER-4",
        ],
        "MOTHER-HOST",
    );
    let bt23_source = push_source_to_breeding(&mut runner, "BT23-072");

    runner.game.enter_main_phase();
    accept_pending_optional_trigger_or_replacement(&mut runner);
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::Material | SelectionKind::CountCappedMultiSelect { .. })
        ),
        "accepted inherited trigger should prompt for a King Drasil source"
    );
    runner
        .execute_action(0, encode_breeding_source_select(0, 0).unwrap())
        .expect("choose BT23-072 source itself");
    runner.auto_resolve().expect("play source itself");

    let played = find_field_card(&runner, "BT23-072");
    assert_eq!(
        runner.game.players[0].battle_area[played.index as usize]
            .top_card()
            .handle(),
        bt23_source
    );
}
