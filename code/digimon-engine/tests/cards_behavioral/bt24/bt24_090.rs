//! BT24-090 Abyss Sanctuary: Throne Room.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT24-090";

#[test]
fn bt24_090_metadata_use_requirement_main_security_and_auras_compile() {
    let runner = sanctuary_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT24-090");

    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert!(card.use_requirement.is_some());
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    )));
    assert_eq!(
        card.effects
            .iter()
            .filter(|clause| matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                    scope: digimon_dsl::compiled::CompiledScope::Security,
                    grant_keyword: Some(_),
                    ..
                })
            ))
            .count(),
        2,
        "security Blocker and conditional Alliance auras should compile"
    );

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(trigger)
                if trigger.when == vec![CompiledTiming::MainFromHand] =>
            {
                Some(trigger)
            }
            _ => None,
        })
        .expect("MainFromHand clause");
    assert!(matches!(
        main.process.as_slice(),
        [
            CompiledStep::AddBottomSecurityToHand { .. },
            CompiledStep::PlaceSelfOptionAtSecurity { .. },
            CompiledStep::SelectHand { .. },
            CompiledStep::PlayFromHand { .. },
        ]
    ));

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(trigger)
            if trigger.scope == digimon_dsl::compiled::CompiledScope::Inherited
                && trigger.when == vec![CompiledTiming::OnSecurity]
    )));
}

#[test]
fn bt24_090_main_replaces_bottom_security_with_self_face_up_and_plays_reduced_ts() {
    let mut runner = sanctuary_runner()
        .add_card(filler("BOTTOM"))
        .add_card(filler("TOP"))
        .add_card(ts_digimon("BLUE-TS", CardColor::Blue, 6))
        .add_card(ts_digimon("YELLOW-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .hand(0, &[CARD_ID, "BLUE-TS", "YELLOW-TS", "RED-TS"])
        .security(0, &["BOTTOM", "TOP"])
        .memory(10)
        .start();
    runner.game.enter_main_phase();
    let memory_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "no face-up security should satisfy the option's color bypass"
    );
    let hand_prompt = runner
        .pending_selection_view()
        .expect("reduced play prompt");
    assert_eq!(hand_prompt.kind, SelectionKind::Hand);
    assert!(hand_prompt.is_optional);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        CARD_ID,
        "Abyss Sanctuary should be placed as the bottom security card"
    );
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&runner.game.players[0].security[0].card_index),
        "placed security card must be face-up"
    );
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "BOTTOM"));

    let blue_action = hand_action_for_id(&runner, "BLUE-TS");
    assert!(hand_prompt.valid_action_ids.contains(&blue_action));
    assert!(!hand_prompt
        .valid_action_ids
        .contains(&hand_action_for_id(&runner, "RED-TS")));
    runner
        .execute_action(hand_prompt.selecting_player, blue_action)
        .expect("play blue TS with reduced cost");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BLUE-TS"));
    assert_eq!(
        runner.memory(),
        memory_before - 6,
        "option cost 3 plus play cost 6 reduced by 3 should spend 6 memory total"
    );
}

#[test]
fn bt24_090_security_auras_grant_blocker_and_conditional_alliance() {
    let mut runner = sanctuary_runner()
        .add_card(ts_digimon("BLUE-TS", CardColor::Blue, 4))
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .add_card(named_digimon("NEPTUNE", "Neptunemon", CardColor::Blue))
        .security(0, &[CARD_ID])
        .start();
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);
    let blue = runner.place_on_field(0, "BLUE-TS", Some(0));
    let red = runner.place_on_field(0, "RED-TS", Some(0));
    runner.place_on_field(0, "NEPTUNE", Some(0));

    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(blue, Keyword::Blocker));
    assert!(runner.game.has_keyword(blue, Keyword::Alliance));
    assert!(!runner.game.has_keyword(red, Keyword::Blocker));
    assert!(!runner.game.has_keyword(red, Keyword::Alliance));
}

#[test]
fn bt24_090_security_effect_plays_level_four_blue_or_yellow_ts_from_hand_or_trash_free() {
    let mut runner = sanctuary_runner()
        .add_card(ts_digimon("HAND-TS", CardColor::Yellow, 4))
        .add_card(ts_digimon("TRASH-TS", CardColor::Blue, 4))
        .add_card(ts_digimon("HIGH-TS", CardColor::Blue, 5))
        .add_card(attacker("ATTACKER"))
        .add_card(filler("FILL"))
        .hand(1, &["HAND-TS", "HIGH-TS"])
        .deck(1, &["TRASH-TS"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    let trash_card = runner.game.players[1].deck.pop().expect("trash seed");
    runner.game.players[1].trash.push(trash_card);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(attacker, 1, false);
    let union = runner
        .pending_selection_view()
        .expect("hand/trash union selection");
    assert!(union.is_optional);
    let chosen = union
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action != PASS)
        .expect("eligible hand or trash card");
    runner
        .execute_action(union.selecting_player, chosen)
        .expect("play eligible TS card");
    runner.auto_resolve().expect("settle security effect");

    assert!(runner.game.players[1].battle_area.iter().any(|perm| {
        let id = perm.top_card().card_id(&runner.game.card_data);
        id == "HAND-TS" || id == "TRASH-TS"
    }));
    assert_eq!(runner.memory(), memory_before, "security play is free");
    assert!(!runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "HIGH-TS"));
}

fn sanctuary_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-090 YAML loads")
}

fn hand_action_for_id(runner: &digimon_engine::debug_runner::DebugRunner, id: &str) -> u16 {
    runner
        .game
        .player(0)
        .hand
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id)
                .then_some(digimon_engine::action::space::PLAY_HAND_START + idx as u16)
        })
        .expect("hand card exists")
}

fn ts_digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.traits = vec!["TS".to_string()];
    card.level = Some(level);
    card.dp = Some(4000);
    card.play_cost = u16::from(level);
    card
}

fn named_digimon(id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = ts_digimon(id, color, 6);
    card.card_name = name.to_string();
    card
}

fn attacker(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(4);
    card.dp = Some(9000);
    card.play_cost = 4;
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}
