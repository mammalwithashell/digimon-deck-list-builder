//! BT13-110 Royal Knights of the Purge - Option, White, Royal Knight.
//!
//! Supported slice:
//! - [Main] Draw 1; optional hand Digimon tuck under breeding King Drasil;
//!   place this card in the battle area.
//! - [Main]<Delay> play 1 Royal Knight from breeding sources with On Play
//!   suppression and Rush.
//! - [Security] Place this card in the battle area.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_breeding_select, encode_breeding_source_select, EFFECTS_PER_PERMANENT,
    FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectSourceKind, Keyword};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::SelectionKind;

const ON_PLAY_GAIN: i16 = 1;

struct OnPlayGainMemory;

impl CardEffect for OnPlayGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("On Play: gain memory")
            .process(|ctx| ctx.gain_memory(ON_PLAY_GAIN))
            .build()]
    }
}

fn digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 10;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT13-110")
        .expect("BT13-110 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &[]))
        .add_card(digimon("MAGNAMON", "Magnamon", &["Royal Knight"]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .memory(10)
        .start()
}

fn install_breeding_stack(runner: &mut DebugRunner, source_ids: &[&str]) {
    let mut stack = source_ids.to_vec();
    stack.push("KING");
    let handle = runner.place_stack(0, &stack);
    let perm = runner.game.players[0]
        .battle_area
        .remove(handle.index as usize);
    runner.game.players[0].breeding_area = Some(perm);
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

fn delayed_option_main_bit(field_index: usize) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

#[test]
fn bt13_110_has_printed_metadata() {
    let runner = runner();
    let card = runner.compiled_card("BT13-110").expect("BT13-110 present");
    assert_eq!(card.name, "Royal Knights of the Purge");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Royal Knight"));
}

#[test]
fn bt13_110_main_draws_one_and_places_self() {
    let runner = runner();
    let card = runner.compiled_card("BT13-110").expect("BT13-110 present");
    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] clause exists");
    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::Draw { count: 1, .. })
    ));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn bt13_110_main_may_place_hand_digimon_under_king_drasil() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-110")
        .expect("BT13-110 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &[]))
        .add_card(digimon("MAGNAMON", "Magnamon", &["Royal Knight"]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .hand(0, &["BT13-110", "MAGNAMON"])
        .deck(0, &["DRAW"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "KING");
    let magnamon_handle = runner.game.players[0].hand[1].handle();

    let _ = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::BreedingPermanent)
    );
    runner
        .execute_action(0, encode_breeding_select(0).unwrap())
        .expect("choose King Drasil");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("choose Magnamon from hand");

    let breeding = runner.game.players[0].breeding_area.as_ref().unwrap();
    assert_eq!(breeding.card_sources[0].handle(), magnamon_handle);
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| matches!(perm.option_state, OptionState::Delayed { .. })),
        "the Option is placed as a Delay permanent after the optional tuck"
    );
}

#[test]
fn bt13_110_delay_plays_royal_knight_from_breeding_sources_with_rush() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-110")
        .expect("BT13-110 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &[]))
        .add_card(digimon("MAGNAMON", "Magnamon", &["Royal Knight"]))
        .hand(0, &["BT13-110"])
        .memory(10)
        .start();
    runner.register_effect("MAGNAMON", Arc::new(OnPlayGainMemory));
    install_breeding_stack(&mut runner, &["MAGNAMON"]);

    let _ = runner.game.play_option_from_hand(0, 0);
    runner
        .execute_action(0, PASS)
        .expect("decline optional hand tuck");
    let delay_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|perm| matches!(perm.option_state, OptionState::Delayed { .. }))
        .expect("Delay option on field");
    let placed_on_turn = match runner.game.players[0].battle_area[delay_idx].option_state {
        OptionState::Delayed { placed_on_turn, .. } => placed_on_turn,
        _ => unreachable!("Delay option on field"),
    };
    for _ in 0..4 {
        if runner.game.turn_count > placed_on_turn {
            break;
        }
        runner.end_turn();
        runner.game.enter_main_phase();
    }
    runner.game.set_memory(0);

    let action = delayed_option_main_bit(delay_idx);
    let delay_handle = PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    let delay_card = runner.game.players[0].battle_area[delay_idx]
        .top_card()
        .handle();
    let delay_effects = runner
        .game
        .effects_for_card("BT13-110", delay_card)
        .expect("BT13-110 effects");
    let delay_effect = delay_effects
        .iter()
        .find(|effect| effect.delay_trigger == Some(DelayTrigger::MainPhaseActivated))
        .expect("Delay effect");
    let read_ctx = EffectReadContext::new_with_source_kind(
        &runner.game,
        delay_card,
        Some(delay_handle),
        EffectSourceKind::Option,
        0,
    );
    assert!(
        delay_effect
            .condition
            .as_ref()
            .is_none_or(|condition| condition(&read_ctx)),
        "Delay active_when should match breeding Royal Knight sources"
    );
    assert_eq!(
        build_action_mask(&runner.game, 0)[action as usize],
        1.0,
        "Delay activation should be legal when breeding has a Royal Knight source"
    );
    assert!(runner.game.activate_delayed_option_main(delay_handle));

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::BreedingPermanent)
    );
    runner
        .execute_action(0, encode_breeding_select(0).unwrap())
        .expect("choose breeding carrier");
    runner
        .execute_action(0, encode_breeding_source_select(0, 0).unwrap())
        .expect("choose Royal Knight source");

    let played = find_field_card(&runner, "MAGNAMON");
    assert!(
        runner.game.has_keyword(played, Keyword::Rush),
        "the played Royal Knight gains Rush"
    );
    assert_eq!(
        runner.game.memory, 0,
        "played Royal Knight's own On Play is suppressed"
    );
}

#[test]
fn bt13_110_security_places_self() {
    let runner = runner();
    let card = runner.compiled_card("BT13-110").expect("BT13-110 present");
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnSecurity)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption))
        }
        _ => false,
    }));
}
