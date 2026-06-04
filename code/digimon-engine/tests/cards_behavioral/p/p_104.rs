//! P-104 Mental Training - Option, Cost 2, Blue.
//!
//! # Card text (cards.json)
//!
//! [Main] Reveal the top 2 cards of your deck. Add 1 blue card among them to
//! your hand. Place the rest at the bottom of your deck in any order. Then,
//! place this card in the battle area.
//!
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.)
//! - 1 of your Digimon may digivolve into a blue Digimon card in your hand for
//!   its digivolution cost. When it would digivolve by this effect, reduce the
//!   cost by 2.
//!
//! Inherited: Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Blue/P_104.cs
//!
//! # Patterns this test covers
//! - Training Option [Main]: reveal top 2, select 1 blue card of any kind, add
//!   to hand, bottom the remainder, place self as a Delay Option permanent.
//! - Delay body: optional own-Digimon pick, mandatory blue Digimon hand pick,
//!   effect_initiated_digivolve with cost reduction 2.
//! - Inherited security placement through place_self_as_delay_option.
//! - Known partial: standard Delay should be a player-visible later Main
//!   activation; current generic Delay YAML uses the scheduled delay primitive.

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledPlayerRef, CompiledScope, CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

fn p_104_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-104.yaml"))
        .expect("P-104 YAML must exist at cards/p/P-104.yaml")
}

fn p_104_runner() -> DebugRunner {
    let yaml = p_104_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(level);
    card
}

fn option(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![color];
    card.level = None;
    card
}

fn blue_evo(id: &str, cost: u16) -> CardData {
    let mut card = digimon(id, CardColor::Blue, 4);
    card.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 3,
        memory_cost: cost,
    }];
    card
}

fn card_ids_in_hand(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .hand
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn p_104_yaml_parses_and_compiles() {
    let _runner = p_104_runner();
}

#[test]
fn p_104_is_blue_option_cost_2() {
    let runner = p_104_runner();
    let compiled = runner.compiled_card("P-104").expect("P-104 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(compiled.cost, Some(2));
}

#[test]
fn p_104_has_main_delay_and_inherited_security_clauses() {
    let runner = p_104_runner();
    let compiled = runner.compiled_card("P-104").expect("P-104 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "P-104 has printed Main, Delay, and inherited Security placement text"
    );

    match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => {
            assert!(triggered.when.contains(&CompiledTiming::MainFromHand));
            assert_eq!(triggered.scope, CompiledScope::FaceUp);
            assert!(
                !triggered.optional,
                "printed Main reveal text is mandatory, not a may effect"
            );
        }
        other => panic!("clause 0 must be main_from_hand; got {other:?}"),
    }

    assert!(matches!(
        compiled.effects[1],
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { .. })
    ));

    match &compiled.effects[2] {
        CompiledClause::Triggered(triggered) => {
            assert_eq!(triggered.scope, CompiledScope::Inherited);
            assert!(triggered.when.contains(&CompiledTiming::OnSecurity));
            assert_eq!(
                triggered.process,
                vec![CompiledStep::PlaceSelfAsDelayOption],
                "Security placement must use the audited delayed-option placement primitive"
            );
        }
        other => panic!("clause 2 must be inherited on_security; got {other:?}"),
    }
}

#[test]
fn p_104_main_reveal_selects_blue_card_of_any_kind() {
    let runner = p_104_runner();
    let compiled = runner.compiled_card("P-104").expect("P-104 compiled");
    let main = match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => triggered,
        other => panic!("clause 0 must be triggered; got {other:?}"),
    };

    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::RevealTopDeck {
            of: CompiledPlayerRef::You,
            count: 2,
            ..
        })
    ));

    let select_filter = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectReveal { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("Main process must select from the reveal");

    assert_eq!(
        select_filter.kind, None,
        "P-104 adds any blue card, not only blue Digimon"
    );
    assert_eq!(select_filter.color_is, Some(CompiledColor::Blue));
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. })));
    assert!(main.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaceRemainderOnDeck {
            position: CompiledStackPosition::Bottom,
            ..
        }
    )));
}

#[test]
fn p_104_main_adds_blue_option_from_top_2_to_hand() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(option("BLUE-OPTION", CardColor::Blue))
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-104"])
        .deck(0, &["FILL", "RED-DIGI", "BLUE-OPTION"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Reveal),
        "a blue Option in the top 2 must be selectable"
    );
    assert_eq!(runner.pending_action_count(), 1);
    assert!(
        !runner.pending_is_optional(),
        "the mandatory blue-card add drops PASS when a candidate exists"
    );

    runner
        .auto_resolve()
        .expect("resolve reveal and bottom ordering");

    let hand_ids = card_ids_in_hand(&runner, 0);
    assert!(
        hand_ids.iter().any(|id| id == "BLUE-OPTION"),
        "P-104 must add blue cards of any kind; hand={hand_ids:?}"
    );
    // P-104 leaves the hand (placed in battle area), and BLUE-OPTION is added:
    // net hand size is unchanged, but the contents swapped.
    assert_eq!(runner.hand_size(0), hand_before);
    assert!(
        !hand_ids.iter().any(|id| id == "P-104"),
        "P-104 is placed in the battle area, not left in hand"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P-104"),
        "Then, place this card in the battle area"
    );
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "reveal 2, add 1, bottom 1 should shrink the deck by exactly 1"
    );
}

#[test]
fn p_104_main_with_no_blue_in_top_2_adds_nothing_and_bottoms_remainder() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(option("YELLOW-OPTION", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &["P-104"])
        .deck(0, &["FILL", "YELLOW-OPTION", "RED-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve bottom ordering for non-matching reveal");

    // Nothing is added to hand (no blue among the top 2); P-104 still leaves the
    // hand because "Then, place this card in the battle area" is unconditional.
    assert!(
        !card_ids_in_hand(&runner, 0)
            .iter()
            .any(|id| id == "P-104"),
        "P-104 is placed in the battle area even when no blue card is added"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "no blue card is added; the only hand change is P-104 being placed"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "both non-matching revealed cards should return to the deck bottom"
    );
}

#[test]
fn p_104_playing_main_places_it_as_delayed_option() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3))
        .add_card(option("BLUE-OPTION", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &["P-104"])
        .deck(0, &["FILL", "BLUE-OPTION"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLUE-BASE", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve P-104 main reveal and placement");

    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-104")
        .expect("P-104 should be placed as a battle-area delayed Option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

#[test]
fn p_104_security_check_places_self_in_battle_area_as_delay_option() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Blue, 3))
        .add_card(filler("FILL"))
        .security(1, &["P-104"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "P-104 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "P-104 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-104")
        .expect("P-104 should be placed as a battle-area delayed Option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

#[test]
fn p_104_delay_clause_filters_blue_digimon_and_reduces_cost_by_2() {
    let runner = p_104_runner();
    let compiled = runner.compiled_card("P-104").expect("P-104 compiled");
    let delay = match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, process, ..
        }) => {
            assert_eq!(*trigger, CompiledTiming::Delayed);
            process
        }
        other => panic!("clause 1 must be a standard Delay; got {other:?}"),
    };

    let target_pick = delay
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectOwnPermanent {
                filter, optional, ..
            } => Some((filter, optional)),
            _ => None,
        })
        .expect("Delay must choose one of your Digimon first");
    assert!(
        *target_pick.1,
        "printed 'may digivolve' must make the target pick optional"
    );
    assert_eq!(target_pick.0.kind, Some(CompiledCardKind::Digimon));

    let hand_pick = delay
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectHand {
                of,
                filter,
                optional,
                ..
            } => Some((of, filter, optional)),
            _ => None,
        })
        .expect("Delay must choose a blue Digimon card in hand");
    assert_eq!(*hand_pick.0, CompiledPlayerRef::You);
    assert!(
        !*hand_pick.2,
        "after choosing a target, the evolution card pick is mandatory when eligible"
    );
    assert_eq!(hand_pick.1.kind, Some(CompiledCardKind::Digimon));
    assert_eq!(hand_pick.1.color_is, Some(CompiledColor::Blue));

    assert!(delay.iter().any(|step| matches!(
        step,
        CompiledStep::EffectInitiatedDigivolve {
            cost: CompiledCostDelta::Reduce(2),
            ignore_requirements: false,
            ..
        }
    )));
}

#[test]
fn p_104_delay_body_surfaces_target_and_hand_choices_then_digivolves_cost_minus_2() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3))
        .add_card(blue_evo("BLUE-EVO", 3))
        .add_card(blue_evo("SECOND-BLUE-EVO", 3))
        .add_card(digimon("RED-EVO", CardColor::Red, 4))
        .add_card(option("BLUE-OPTION", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(0, &["BLUE-EVO", "RED-EVO", "BLUE-OPTION"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-104", Some(0));
    let carrier = runner.place_on_field(0, "BLUE-BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(
        runner.pending_is_optional(),
        "target pick is the printed may"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "the delayed Option permanent itself must not be a Digimon target"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "optional target pick exposes PASS"
    );
    let target_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, target_action)
        .expect("choose Digimon to digivolve");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        !runner.pending_is_optional(),
        "blue Digimon hand choice is mandatory after choosing a target"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only blue Digimon cards, not blue Options or red Digimon, may be selected"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PASS as usize], 0.0, "mandatory hand pick drops PASS");
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "the blue Digimon in hand must be the legal evolution pick"
    );
    assert_eq!(
        mask[(PLAY_HAND_START + 1) as usize],
        0.0,
        "red Digimon must be filtered out of the blue evolution pick"
    );
    assert_eq!(
        mask[(PLAY_HAND_START + 2) as usize],
        0.0,
        "blue Option must be filtered out because the Delay digivolves into a Digimon"
    );
    let evo_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, evo_action)
        .expect("choose blue Digimon evolution card");
    runner.game.drain_effect_queue();

    let carrier_top = runner.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(carrier_top, "BLUE-EVO");
    assert_eq!(
        runner.memory(),
        9,
        "printed cost 3 reduced by 2 should pay exactly 1 memory"
    );
}

#[test]
fn p_104_delay_target_pick_can_be_declined_with_pass() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3))
        .add_card(blue_evo("BLUE-EVO", 3))
        .add_card(filler("FILL"))
        .hand(0, &["BLUE-EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-104", Some(0));
    let carrier = runner.place_on_field(0, "BLUE-BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline Delay digivolve");
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "declining the optional target pick must not continue into the hand pick"
    );
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BLUE-BASE",
        "declining leaves the Digimon unchanged"
    );
    assert!(
        card_ids_in_hand(&runner, 0)
            .iter()
            .any(|id| id == "BLUE-EVO"),
        "declining leaves the evolution card in hand"
    );
}

/// PUPPETS-G009 — after the placing turn, P-104's `<Delay>` is a player-
/// visible `[Main]`-phase action. The mask exposes the `FIELD_EFFECT`
/// activation only after the placing turn; PASS stays legal. Taking the
/// action trashes P-104 as the cost and runs the digivolve body.
#[test]
fn p_104_delay_is_player_visible_main_activation_after_placing_turn() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3))
        .add_card(blue_evo("BLUE-EVO", 3))
        .add_card(filler("FILL"))
        .hand(0, &["BLUE-EVO"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-104", Some(0));
    let carrier = runner.place_on_field(0, "BLUE-BASE", Some(0));
    // Mark P-104 as a standard <Delay> placed on the current turn.
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[delay_perm.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };
    let delay_idx = delay_perm.index as usize;
    let bit = (FIELD_EFFECT_START
        + delay_idx as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;

    // Same turn it was placed: the Delay activation is NOT legal (16-16-3).
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "P-104 <Delay> must not be activatable on the placing turn"
    );

    // Advance to the controller's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(10);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[bit], 1.0, "P-104 <Delay> activation is a legal action");
    assert_eq!(mask[PASS as usize], 1.0, "declining stays legal");

    // Take the activation — the digivolve body installs its selections.
    runner.game.decode_action(bit as u16, 0);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, target_action)
        .expect("choose Digimon to digivolve");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    let evo_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, evo_action)
        .expect("choose blue Digimon evolution card");
    runner.game.drain_effect_queue();

    // Body resolved: the carrier digivolved with cost reduced by 2. The
    // P-104 Option was trashed as the cost, so the carrier's field index may
    // have shifted — find it by Digimon kind rather than the stale handle.
    let _ = carrier;
    let carrier_top = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.is_digimon(&runner.game.card_data))
        .expect("base Digimon remains in battle area")
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(carrier_top, "BLUE-EVO", "Delay digivolve resolved");
    assert_eq!(
        runner.memory(),
        9,
        "printed evo cost 3 reduced by 2 pays only 1 memory"
    );
    // P-104 is trashed as the activation cost.
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "P-104 is trashed as the <Delay> activation cost after the body resolves"
    );
}

/// PUPPETS-G009 — declining (PASS) leaves a placed P-104 `<Delay>` Option on
/// the battle area for a later turn; the body never runs.
#[test]
fn p_104_declining_delay_leaves_option_on_field() {
    let yaml = p_104_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-104 YAML parses")
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-104", Some(0));
    runner.place_on_field(0, "BLUE-BASE", Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[delay_perm.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };

    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();

    // Decline — pass the turn instead of activating the Delay.
    runner.game.decode_action(PASS, 0);

    assert!(
        runner.game.player(0).battle_area.iter().any(|p| matches!(
            p.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        )),
        "declined P-104 <Delay> stays parked on the battle area"
    );
    assert_eq!(runner.trash_size(0), 0, "declined P-104 is not trashed");
}
