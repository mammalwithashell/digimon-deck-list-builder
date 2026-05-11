//! BT24-100 In-Between Theater.
//! Printed text: while you have a [TS] Digimon or Tamer, ignore color
//! requirements; [Main] reveal top 3, add 1 [TS], bottom the rest, then place
//! this card in the battle area; [Main] <Delay> gain 2 memory; Security places
//! this card in the battle area.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PLAY_HAND_START, SEL_REVEAL_START};
use digimon_engine::build_action_mask;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::OptionPlayResult;

fn ts_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["TS".to_string()];
    card
}

fn ts_tamer(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.dp = None;
    card.level = None;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["TS".to_string()];
    card
}

fn non_ts_digimon(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.traits.clear();
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

#[test]
fn bt24_100_yaml_metadata_and_clauses_match_printed_card() {
    let runner = DebugRunner::builder()
        .dsl_card("BT24-100")
        .expect("BT24-100 YAML loads")
        .add_card(non_ts_digimon("FILL"))
        .hand(0, &["BT24-100"])
        .deck(0, &["FILL"])
        .start();
    let compiled = runner.compiled_card("BT24-100").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::White]);
    assert_eq!(compiled.cost, Some(3));
    assert!(
        compiled.use_requirement.is_some(),
        "color-bypass use_requirement must mirror TS Digimon/Tamer board presence"
    );
    assert_eq!(compiled.effects.len(), 4);

    assert!(matches!(
        &compiled.effects[0],
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            ..
        }) if modifier == "IgnoreColorRequirement"
    ));

    let CompiledClause::Triggered(main) = &compiled.effects[1] else {
        panic!("clause 1 must be main_from_hand");
    };
    assert_eq!(main.when, vec![CompiledTiming::MainFromHand]);
    assert!(matches!(
        main.process.as_slice(),
        [
            CompiledStep::RevealTopDeck { count: 3, .. },
            CompiledStep::SelectReveal { .. },
            CompiledStep::AddToHandFromReveal { .. },
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Bottom,
                ..
            },
            CompiledStep::PlaceSelfAsDelayOption,
        ]
    ));

    assert!(matches!(
        &compiled.effects[2],
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger: CompiledTiming::EndOfYourNextTurn,
            process,
            ..
        }) if process == &vec![CompiledStep::GainMemory(2)]
    ));

    assert!(matches!(
        &compiled.effects[3],
        CompiledClause::Triggered(security)
            if security.scope == CompiledScope::Inherited
                && security.when == vec![CompiledTiming::OnSecurity]
                && security.process == vec![CompiledStep::PlaceSelfAsDelayOption]
    ));
}

#[test]
fn bt24_100_color_bypass_requires_ts_digimon_or_tamer_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-100")
        .expect("BT24-100 YAML loads")
        .add_card(ts_tamer("TS-TAMER"))
        .add_card(non_ts_digimon("FILL"))
        .hand(0, &["BT24-100"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    let before = build_action_mask(&runner.game, 0);
    assert_eq!(
        before[PLAY_HAND_START as usize], 0.0,
        "without a white source or TS board presence, the white Option is not usable"
    );

    runner.place_on_field(0, "TS-TAMER", Some(0));
    let after = build_action_mask(&runner.game, 0);
    assert_eq!(
        after[PLAY_HAND_START as usize], 1.0,
        "TS Tamer board presence should satisfy BT24-100's color-bypass requirement"
    );
}

#[test]
fn bt24_100_main_adds_ts_card_bottoms_remainder_and_places_as_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-100")
        .expect("BT24-100 YAML loads")
        .add_card(ts_digimon("TS-BOARD"))
        .add_card(ts_digimon("TS-HIT"))
        .add_card(non_ts_digimon("MISS-A"))
        .add_card(non_ts_digimon("MISS-B"))
        .hand(0, &["BT24-100"])
        .deck(0, &["MISS-B", "MISS-A", "TS-HIT"])
        .memory(10)
        .start();
    runner.place_on_field(0, "TS-BOARD", Some(0));

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let action = revealed_action_for_id(&runner, "TS-HIT");
    let view = runner.pending_selection_view().expect("TS reveal pick");
    assert!(view.valid_action_ids.contains(&action));
    runner
        .execute_action(0, action)
        .expect("select revealed TS card");
    runner.auto_resolve().expect("finish BT24-100 main");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"TS-HIT".to_string()));
    assert!(!hand_ids.contains(&"MISS-A".to_string()) && !hand_ids.contains(&"MISS-B".to_string()));
    assert_eq!(runner.deck_size(0), 2, "two misses returned to deck bottom");
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT24-100")
        .expect("BT24-100 placed as delayed Option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::EndOfYourNextTurn,
            ..
        }
    ));
}

#[test]
fn bt24_100_delay_activation_gains_two_memory_and_trashes_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-100")
        .expect("BT24-100 YAML loads")
        .add_card(ts_digimon("TS-BOARD"))
        .add_card(ts_digimon("TS-HIT"))
        .hand(0, &["BT24-100"])
        .deck(0, &["TS-HIT", "TS-HIT", "TS-HIT"])
        .deck(1, &["TS-HIT"])
        .memory(10)
        .start();
    runner.place_on_field(0, "TS-BOARD", Some(0));

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner.auto_resolve().expect("place BT24-100 as delay");
    let start_turn = runner.turn_count();

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.turn_count(), start_turn + 2);
    runner.game.enter_main_phase();
    runner.game.set_memory(0);
    runner.end_turn();

    let expected_memory = if runner.game.turn_player() == 0 {
        2
    } else {
        -2
    };
    assert_eq!(runner.memory(), expected_memory);
    assert!(
        !runner.game.player(0).battle_area.iter().any(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == "BT24-100"
                && matches!(permanent.option_state, OptionState::Delayed { .. })
        }),
        "BT24-100 should be trashed after its Delay resolves"
    );
}
