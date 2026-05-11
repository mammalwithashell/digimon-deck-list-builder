//! BT24-004 Wanyamon — Digi-Egg, Lv.2, Green.
//!
//! Printed text (`BT24-004.json`):
//! Inherited Effect [Your Turn] [Once Per Turn] When any of your [Iliad] trait
//! Digimon are played, <Draw 1> (Draw 1 card from your deck.).

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledPredicate, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT24-004";

#[test]
fn bt24_004_yaml_has_printed_metadata_and_inherited_iliad_play_draw_clause() {
    let runner = wanyamon_runner().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT24-004 must be present in embedded DSL pack");

    assert_eq!(card.name, "Wanyamon");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert_eq!(card.color, vec![CompiledColor::Green]);
    assert_eq!(card.cost, Some(0));
    for trait_name in ["Lesser", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT24-004 metadata must include trait {trait_name}"
        );
    }

    let clause = card
        .effects
        .iter()
        .find_map(|effect| match effect {
            CompiledClause::Triggered(clause)
                if clause.scope == CompiledScope::Inherited
                    && clause.when == vec![CompiledTiming::OnAllyPlayed] =>
            {
                Some(clause)
            }
            _ => None,
        })
        .expect("BT24-004 must have one inherited on_ally_played clause");

    assert!(clause.once_per_turn, "printed [Once Per Turn] must compile");
    assert!(
        !clause.optional,
        "printed text has no 'you may', so the draw is mandatory"
    );
    assert_eq!(
        clause.active_when,
        Some(CompiledPredicate {
            your_turn: Some(true),
            ..CompiledPredicate::default()
        }),
        "printed [Your Turn] must compile as active_when: your_turn"
    );
    assert_eq!(
        clause.condition,
        Some(CompiledPredicate {
            all_of: vec![
                CompiledPredicate {
                    event_target_kind: Some(CompiledCardKind::Digimon),
                    ..CompiledPredicate::default()
                },
                CompiledPredicate {
                    event_card_trait_has: Some("Iliad".to_string()),
                    ..CompiledPredicate::default()
                },
            ],
            ..CompiledPredicate::default()
        }),
        "trigger must require an entered Digimon card with the [Iliad] trait"
    );
    assert_eq!(
        clause.process,
        vec![CompiledStep::Draw {
            of: digimon_dsl::compiled::CompiledPlayerRef::You,
            count: 1,
        }],
        "effect body must be exactly Draw 1"
    );
}

#[test]
fn bt24_004_inherited_draws_one_when_your_iliad_digimon_is_played() {
    let mut runner = wanyamon_runner()
        .hand(0, &["ILIAD-DIGIMON"])
        .deck(0, &["DRAW"])
        .memory(10)
        .start();
    place_wanyamon_as_source(&mut runner);

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.play(0, 0).expect("play Iliad Digimon");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand nets unchanged: -1 played Iliad Digimon +1 from Wanyamon Draw 1"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Wanyamon must draw exactly 1 card"
    );
    assert!(
        runner.pending_selection().is_none(),
        "mandatory Draw 1 must not leave a player-visible pending choice"
    );
}

#[test]
fn bt24_004_inherited_does_not_draw_for_non_iliad_digimon() {
    let mut runner = wanyamon_runner()
        .hand(0, &["NON-ILIAD-DIGIMON"])
        .deck(0, &["DRAW"])
        .memory(10)
        .start();
    place_wanyamon_as_source(&mut runner);

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.play(0, 0).expect("play non-Iliad Digimon");

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "non-Iliad Digimon must not trigger Wanyamon's draw"
    );
    assert_eq!(runner.deck_size(0), deck_before);
}

#[test]
fn bt24_004_inherited_does_not_draw_for_iliad_tamer() {
    let mut runner = wanyamon_runner()
        .hand(0, &["ILIAD-TAMER"])
        .deck(0, &["DRAW"])
        .memory(10)
        .start();
    place_wanyamon_as_source(&mut runner);

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.play(0, 0).expect("play Iliad Tamer");

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "printed text says [Iliad] trait Digimon, so Iliad Tamers must not trigger"
    );
    assert_eq!(runner.deck_size(0), deck_before);
}

#[test]
fn bt24_004_inherited_does_not_draw_on_opponents_turn() {
    let mut runner = wanyamon_runner()
        .hand(1, &["ILIAD-DIGIMON"])
        .deck(0, &["DRAW"])
        .memory(10)
        .start();
    place_wanyamon_as_source(&mut runner);
    runner.game.turn_player_idx = 1;

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner.play(1, 0).expect("opponent plays Iliad Digimon");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "Wanyamon is [Your Turn] and must not draw on the opponent's turn"
    );
    assert_eq!(runner.deck_size(0), deck_before);
}

#[test]
fn bt24_004_inherited_draw_is_once_per_turn() {
    let mut runner = wanyamon_runner()
        .hand(0, &["ILIAD-DIGIMON", "ILIAD-DIGIMON-2"])
        .deck(0, &["DRAW-2", "DRAW-1"])
        .memory(10)
        .start();
    place_wanyamon_as_source(&mut runner);

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    runner.play(0, 0).expect("first Iliad Digimon play");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "first qualifying play draws 1"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "first qualifying play consumes one draw"
    );

    let second_idx =
        find_hand_index(&runner, 0, "ILIAD-DIGIMON-2").expect("second Iliad Digimon still in hand");
    let hand_after_first = runner.hand_size(0);
    let deck_after_first = runner.deck_size(0);
    runner
        .play(0, second_idx)
        .expect("second Iliad Digimon play");

    assert_eq!(
        runner.hand_size(0),
        hand_after_first - 1,
        "second qualifying play in the same turn must not draw again"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_after_first,
        "OPT lockout must prevent the second draw"
    );
}

fn wanyamon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-004 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_trait_digimon("ILIAD-DIGIMON", &["Iliad"]))
        .add_card(make_trait_digimon("ILIAD-DIGIMON-2", &["Iliad"]))
        .add_card(make_trait_digimon("NON-ILIAD-DIGIMON", &["Beast"]))
        .add_card(make_trait_tamer("ILIAD-TAMER", &["Iliad"]))
        .add_card(make_test_card("DRAW", "Draw"))
        .add_card(make_test_card("DRAW-1", "Draw 1"))
        .add_card(make_test_card("DRAW-2", "Draw 2"))
}

fn make_trait_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.play_cost = 0;
    card.level = Some(3);
    card.dp = Some(2000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn make_trait_tamer(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.play_cost = 0;
    card.level = None;
    card.dp = None;
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn place_wanyamon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &[CARD_ID, "CARRIER"])
}

fn find_hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(data) == card_id)
}
