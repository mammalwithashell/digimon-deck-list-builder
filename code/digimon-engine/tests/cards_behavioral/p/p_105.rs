//! P-105 Physical Training - Option, Cost 2, Yellow.
//!
//! # Card text (cards.json)
//!
//! [Main] Reveal the top 2 cards of your deck. Add 1 yellow card among them to
//! your hand. Place the rest at the bottom of your deck in any order. Then,
//! place this card in the battle area.
//!
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.) 1 of your Digimon may digivolve into a yellow Digimon card in
//! your hand for its digivolution cost. When it would digivolve by this effect,
//! reduce the cost by 2.
//!
//! Inherited: Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Yellow/P_105.cs
//!
//! # Patterns this test covers
//! - Training Option [Main]: reveal top 2, select 1 color-matching card of any
//!   kind, add to hand, bottom the remainder.
//! - Delay body: optional own-Digimon pick, mandatory yellow Digimon hand pick,
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

fn p_105_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-105.yaml"))
        .expect("P-105 YAML must exist at cards/p/P-105.yaml")
}

fn p_105_runner() -> DebugRunner {
    let yaml = p_105_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML must parse and compile")
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

fn yellow_evo(id: &str, cost: u16) -> CardData {
    let mut card = digimon(id, CardColor::Yellow, 4);
    card.evo_costs = vec![EvoCost {
        card_color: 2,
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

// Body-only fixtures model an Option already placed by its printed Main effect.
// Keep the real placing turn: this is not permission to activate Delay early.
fn place_delayed_fixture(
    runner: &mut DebugRunner,
    owner: u8,
    card: &str,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(owner, card, Some(owner as u16));
    let placed_on_turn = runner.game.turn_count;
    runner.game.player_mut(owner).battle_area[handle.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner,
            trash_on_turn: u16::MAX,
            trigger: digimon_engine::enums::DelayTrigger::MainPhaseActivated,
            placed_on_turn,
        };
    handle
}

#[test]
fn p_105_yaml_parses_and_compiles() {
    let _runner = p_105_runner();
}

#[test]
fn p_105_is_yellow_option_cost_2() {
    let runner = p_105_runner();
    let compiled = runner.compiled_card("P-105").expect("P-105 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(2));
}

#[test]
fn p_105_has_main_delay_and_inherited_security_clauses() {
    let runner = p_105_runner();
    let compiled = runner.compiled_card("P-105").expect("P-105 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "P-105 has printed Main, Delay, and inherited Security placement text"
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
fn p_105_main_reveal_selects_yellow_card_of_any_kind_without_pass() {
    let runner = p_105_runner();
    let compiled = runner.compiled_card("P-105").expect("P-105 compiled");
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
            CompiledStep::SelectReveal {
                filter, optional, ..
            } => {
                assert!(
                    !*optional,
                    "if a yellow card is revealed, the printed add-1 choice is mandatory"
                );
                Some(filter)
            }
            _ => None,
        })
        .expect("Main process must select from the reveal");

    assert_eq!(
        select_filter.kind, None,
        "P-105 adds any yellow card, not only yellow Digimon"
    );
    assert_eq!(select_filter.color_is, Some(CompiledColor::Yellow));
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
fn p_105_main_adds_yellow_option_from_top_2_to_hand() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(option("YELLOW-OPTION", CardColor::Yellow))
        .add_card(digimon("BLUE-DIGI", CardColor::Blue, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-105"])
        .deck(0, &["FILL", "BLUE-DIGI", "YELLOW-OPTION"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Reveal),
        "a yellow Option in the top 2 must be selectable"
    );
    assert_eq!(runner.pending_action_count(), 1);
    assert!(
        !runner.pending_is_optional(),
        "PASS must not be legal when the mandatory yellow-card pick has a candidate"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PASS as usize], 0.0, "mandatory reveal pick drops PASS");

    runner
        .auto_resolve()
        .expect("resolve reveal, bottom ordering, and self-placement");

    let hand_ids = card_ids_in_hand(&runner, 0);
    assert!(
        hand_ids.iter().any(|id| id == "YELLOW-OPTION"),
        "P-105 must add yellow cards of any kind; hand={hand_ids:?}"
    );
    // "Then, place this card in the battle area" — P-105 itself leaves the hand
    // via place_self_as_delay_option, so the net hand change is +1 (YELLOW-OPTION)
    // −1 (P-105) = 0.
    assert!(
        !hand_ids.iter().any(|id| id == "P-105"),
        "P-105 must leave the hand when placed in the battle area; hand={hand_ids:?}"
    );
    assert_eq!(runner.hand_size(0), hand_before);
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-105")
        .expect("P-105 must be placed as a battle-area delayed Option after [Main]");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "reveal 2, add 1, bottom 1 should shrink the deck by exactly 1"
    );
}

#[test]
fn p_105_main_with_no_yellow_in_top_2_adds_nothing_and_bottoms_remainder() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(digimon("BLUE-DIGI", CardColor::Blue, 3))
        .add_card(option("RED-OPTION", CardColor::Red))
        .add_card(filler("FILL"))
        .hand(0, &["P-105"])
        .deck(0, &["FILL", "RED-OPTION", "BLUE-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve bottom ordering for non-matching reveal and self-placement");

    let hand_ids = card_ids_in_hand(&runner, 0);
    // No yellow card is added, but P-105 still leaves the hand via
    // place_self_as_delay_option, so the hand shrinks by exactly 1 (just P-105).
    assert!(
        !hand_ids.iter().any(|id| id == "P-105"),
        "P-105 must leave the hand when placed in the battle area; hand={hand_ids:?}"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "no yellow card is added, but P-105 itself leaves the hand to the battle area"
    );
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-105")
        .expect("P-105 must be placed as a battle-area delayed Option after [Main]");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "both non-matching revealed cards should return to the deck bottom"
    );
}

#[test]
fn p_105_playing_main_places_it_as_delayed_option() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(digimon("YELLOW-BASE", CardColor::Yellow, 3))
        .add_card(option("YELLOW-OPTION", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &["P-105"])
        .deck(0, &["FILL", "YELLOW-OPTION"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-BASE", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve P-105 main reveal and placement");

    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-105")
        .expect("P-105 should be placed as a battle-area delayed Option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

#[test]
fn p_105_security_check_places_self_in_battle_area_as_delay_option() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Yellow, 3))
        .add_card(filler("FILL"))
        .security(1, &["P-105"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "P-105 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "P-105 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-105")
        .expect("P-105 should be placed as a battle-area delayed Option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

#[test]
fn p_105_delay_clause_filters_yellow_digimon_and_reduces_cost_by_2() {
    let runner = p_105_runner();
    let compiled = runner.compiled_card("P-105").expect("P-105 compiled");
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
        .expect("Delay must choose a yellow Digimon card in hand");
    assert_eq!(*hand_pick.0, CompiledPlayerRef::You);
    assert!(
        !*hand_pick.2,
        "after choosing a target, the evolution card pick is mandatory when eligible"
    );
    assert_eq!(hand_pick.1.kind, Some(CompiledCardKind::Digimon));
    assert_eq!(hand_pick.1.color_is, Some(CompiledColor::Yellow));

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
fn p_105_delay_body_surfaces_target_and_hand_choices_then_digivolves_cost_minus_2() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(digimon("YELLOW-BASE", CardColor::Yellow, 3))
        .add_card(yellow_evo("YELLOW-EVO", 3))
        .add_card(yellow_evo("SECOND-YELLOW-EVO", 3))
        .add_card(digimon("BLUE-EVO", CardColor::Blue, 4))
        .add_card(option("YELLOW-OPTION", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &["YELLOW-EVO", "BLUE-EVO", "YELLOW-OPTION"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "YELLOW-BASE", Some(0));
    let delay_perm = place_delayed_fixture(&mut runner, 0, "P-105");

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
        "yellow Digimon hand choice is mandatory after choosing a target"
    );
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only yellow Digimon cards, not yellow Options or blue Digimon, may be selected"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PASS as usize], 0.0, "mandatory hand pick drops PASS");
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "the yellow Digimon in hand must be the legal evolution pick"
    );
    assert_eq!(
        mask[(PLAY_HAND_START + 1) as usize],
        0.0,
        "blue Digimon must be filtered out of the yellow evolution pick"
    );
    assert_eq!(
        mask[(PLAY_HAND_START + 2) as usize],
        0.0,
        "yellow Option must be filtered out because the Delay digivolves into a Digimon"
    );
    let evo_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, evo_action)
        .expect("choose yellow Digimon evolution card");
    runner.game.drain_effect_queue();

    let carrier_top = runner.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(carrier_top, "YELLOW-EVO");
    assert_eq!(
        runner.memory(),
        9,
        "printed cost 3 reduced by 2 should pay exactly 1 memory"
    );
}

#[test]
fn p_105_delay_target_pick_can_be_declined_with_pass() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(digimon("YELLOW-BASE", CardColor::Yellow, 3))
        .add_card(yellow_evo("YELLOW-EVO", 3))
        .add_card(filler("FILL"))
        .hand(0, &["YELLOW-EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "YELLOW-BASE", Some(0));
    let delay_perm = place_delayed_fixture(&mut runner, 0, "P-105");

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
        "YELLOW-BASE",
        "declining leaves the Digimon unchanged"
    );
    assert!(
        card_ids_in_hand(&runner, 0)
            .iter()
            .any(|id| id == "YELLOW-EVO"),
        "declining leaves the evolution card in hand"
    );
}

/// PUPPETS-G009 — after the placing turn, P-105's `<Delay>` is a player-
/// visible `[Main]`-phase action. The mask exposes the `FIELD_EFFECT`
/// activation only after the placing turn; PASS stays legal. Taking the
/// action trashes P-105 as the cost and runs the digivolve body.
#[test]
fn p_105_delay_is_player_visible_main_activation_after_placing_turn() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(digimon("YELLOW-BASE", CardColor::Yellow, 3))
        .add_card(yellow_evo("YELLOW-EVO", 3))
        .add_card(filler("FILL"))
        .hand(0, &["YELLOW-EVO"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-105", Some(0));
    let carrier = runner.place_on_field(0, "YELLOW-BASE", Some(0));
    // Mark P-105 as a standard <Delay> placed on the current turn.
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

    // Same turn it was placed: the Delay activation is NOT legal (16-17-3).
    runner.game.enter_main_phase();
    assert_eq!(
        build_action_mask(&runner.game, 0)[bit],
        0.0,
        "P-105 <Delay> must not be activatable on the placing turn"
    );

    // Advance to the controller's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(10);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[bit], 1.0, "P-105 <Delay> activation is a legal action");
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
        .expect("choose yellow Digimon evolution card");
    runner.game.drain_effect_queue();

    // Body resolved: the carrier digivolved with cost reduced by 2. The
    // P-105 Option was trashed as the cost, so the carrier's field index may
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
    assert_eq!(carrier_top, "YELLOW-EVO", "Delay digivolve resolved");
    assert_eq!(
        runner.memory(),
        9,
        "printed evo cost 3 reduced by 2 pays only 1 memory"
    );
    // P-105 is trashed as the activation cost.
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "P-105 is trashed as the <Delay> activation cost after the body resolves"
    );
}

/// PUPPETS-G009 — declining (PASS) leaves a placed P-105 `<Delay>` Option on
/// the battle area for a later turn; the body never runs.
#[test]
fn p_105_declining_delay_leaves_option_on_field() {
    let yaml = p_105_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-105 YAML parses")
        .add_card(digimon("YELLOW-BASE", CardColor::Yellow, 3))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-105", Some(0));
    runner.place_on_field(0, "YELLOW-BASE", Some(0));
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
        "declined P-105 <Delay> stays parked on the battle area"
    );
    assert_eq!(runner.trash_size(0), 0, "declined P-105 is not trashed");
}

// ─── <Delay> candidate legality (select-hand audit) ────────────────────────────
//
// DCGO P/Yellow/P_105.cs:71-79 `CanSelectPermanentCondition` offers only an own
// battle-area Digimon for which SOME hand card passing `CanSelectCardCondition`
// (`IsDigimon && HasCardColor(Yellow)`, :92) satisfies
// `CanPlayCardTargetFrame(permanent.PermanentFrame, false, …)` (:79; the
// `false` is `PayCost`, so DCGO's target check is route-only); the chosen Digimon then
// goes to `CardEffectCommons.DigivolveIntoHandOrTrashCard` (payCost true,
// reduceCost 2, :144), whose hand pick (CardEffectCommons.cs:859-880)
// offers only cards with `!CanNotEvolve(target) && CanPlayCardTargetFrame(frame,
// PayCost: true, …)` under the −2 reducer. CR 4.3 8-1-3-1: the revealed card
// needs a digivolution requirement the chosen Digimon meets; CR 4.3 1-3-11-1: a
// digivolution can't be declared when its cost can't be paid. So the hand pick
// offers only cards that can digivolve onto the target for the reduced cost
// (matches DCGO). The target pick offers only Digimon that have such a card:
// deliberately STRICTER than DCGO's route-only target check (CR 4.3 1-3-11-1 —
// a Digimon whose only candidates are unpayable can't digivolve). DCGO's extra
// choice there leads to an empty hand pick, which resolves identically to PASS.
mod delay_candidate_legality {
    use std::path::Path;

    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::card_data::{CardData, EvoCost};
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
    use digimon_engine::permanent::{OptionState, PermanentHandle};
    use digimon_engine::selection::{SelectionKind, TriggerSource};

    const CARD: &str = "P-105";
    const COLOR: CardColor = CardColor::Yellow;
    const OFF: CardColor = CardColor::Red;

    fn yaml() -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-105.yaml"),
        )
        .expect("P-105 YAML")
    }

    fn digimon(id: &str, color: CardColor, level: u8) -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![color];
        c.level = Some(level);
        c.play_cost = 3;
        c
    }

    /// A Yellow Lv.`level` Digimon that digivolves from a Yellow Lv.`from` for `cost`.
    fn evo(id: &str, level: u8, from: u8, cost: u16) -> CardData {
        let mut c = digimon(id, COLOR, level);
        c.evo_costs = vec![EvoCost {
            card_color: COLOR as u8,
            level: from,
            memory_cost: cost,
        }];
        c
    }

    fn runner(hand: &[&str], memory: i16) -> DebugRunner {
        DebugRunner::builder()
            .from_dsl_yaml(&yaml())
            .expect("P-105 YAML compiles")
            .add_card(digimon("BASE", COLOR, 3))
            .add_card(digimon("OFF-BASE", OFF, 5))
            // Lv.4 from a Lv.3 for 4 (2 after the reduction): legal and payable.
            .add_card(evo("EVO", 4, 3, 4))
            // Lv.6 from a Lv.5: BASE (Lv.3) meets no requirement of it.
            .add_card(evo("NO-ROUTE", 6, 5, 3))
            // Lv.4 from a Lv.3 for 13: 11 after the reduction, more than the
            // 10 memory payable from 0 (CR 4.3 1-4-2-2).
            .add_card(evo("PRICEY", 4, 3, 13))
            .add_card(make_test_card("FILL", "FILL"))
            .hand(0, hand)
            .deck(0, &["FILL"; 4])
            .deck(1, &["FILL"; 4])
            .memory(memory)
            .start()
    }

    /// Seat the Option as a Delay permanent and activate its <Delay> body.
    fn fire_delay(runner: &mut DebugRunner) {
        let handle = runner.place_on_field(0, CARD, Some(0));
        let placed_on_turn = runner.game.turn_count;
        runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
            OptionState::Delayed {
                owner: 0,
                trash_on_turn: u16::MAX,
                trigger: DelayTrigger::MainPhaseActivated,
                placed_on_turn,
            };
        runner
            .game
            .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(handle));
        runner.game.drain_effect_queue();
    }

    fn non_pass(runner: &DebugRunner) -> Vec<u16> {
        runner
            .pending_selection_view()
            .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
            .unwrap_or_default()
    }

    fn hand_ids(runner: &DebugRunner) -> Vec<String> {
        runner.game.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect()
    }

    fn top_id(runner: &DebugRunner, handle: PermanentHandle) -> String {
        runner.game.players[0].battle_area[handle.index as usize]
            .top_card()
            .card_id(&runner.game.card_data)
            .to_string()
    }

    /// Choose the single offered Digimon, then return the offered hand cards.
    fn offered_hand_after_target(runner: &mut DebugRunner) -> Vec<String> {
        assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
        let targets = non_pass(runner);
        assert_eq!(targets.len(), 1, "only BASE is offered as the Digimon to digivolve");
        runner.execute_action(0, targets[0]).expect("choose BASE");
        assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
        let hand = hand_ids(runner);
        non_pass(runner)
            .into_iter()
            .map(|a| hand[(a - PLAY_HAND_START) as usize].clone())
            .collect()
    }

    /// Regression guard: a Yellow Digimon card whose digivolution requirement the
    /// chosen Digimon doesn't meet (CR 4.3 8-1-3-1) is not offered.
    #[test]
    fn p_105_delay_hand_pick_excludes_a_card_the_target_cannot_digivolve_into() {
        let mut runner = runner(&["NO-ROUTE", "EVO"], 10);
        let base = runner.place_on_field(0, "BASE", Some(0));
        fire_delay(&mut runner);
        assert_eq!(
            offered_hand_after_target(&mut runner),
            vec!["EVO".to_string()],
            "NO-ROUTE (Lv.6 from Lv.5) can't digivolve onto a Lv.3"
        );
        let pick = non_pass(&runner)[0];
        runner.execute_action(0, pick).expect("digivolve into EVO");
        let _ = runner.auto_resolve();
        assert_eq!(top_id(&runner, base), "EVO");
        assert_eq!(runner.memory(), 8, "4 − 2 = 2 memory paid");
    }

    /// A Yellow Digimon card whose reduced digivolution cost can't be paid
    /// (CR 4.3 1-3-11-1) is not offered; DCGO's hand pick checks `PayCost: true`.
    #[test]
    fn p_105_delay_hand_pick_excludes_a_card_whose_reduced_cost_cannot_be_paid() {
        let mut runner = runner(&["PRICEY", "EVO"], 0);
        let base = runner.place_on_field(0, "BASE", Some(0));
        fire_delay(&mut runner);
        assert_eq!(
            offered_hand_after_target(&mut runner),
            vec!["EVO".to_string()],
            "PRICEY costs 11 after the reduction; only 10 memory is payable"
        );
        let pick = non_pass(&runner)[0];
        runner.execute_action(0, pick).expect("digivolve into EVO");
        let _ = runner.auto_resolve();
        assert_eq!(top_id(&runner, base), "EVO");
    }

    /// DCGO's target pick offers only a Digimon that some Yellow hand card can
    /// digivolve onto: OFF-BASE (a Lv.5) has none, so only BASE is offered.
    #[test]
    fn p_105_delay_target_pick_excludes_a_digimon_with_no_legal_hand_card() {
        let mut runner = runner(&["EVO"], 10);
        let base = runner.place_on_field(0, "BASE", Some(0));
        let off = runner.place_on_field(0, "OFF-BASE", Some(0));
        fire_delay(&mut runner);
        assert!(runner.pending_is_optional(), "the printed 'may' keeps PASS");
        assert_eq!(offered_hand_after_target(&mut runner), vec!["EVO".to_string()]);
        let pick = non_pass(&runner)[0];
        runner.execute_action(0, pick).expect("digivolve into EVO");
        let _ = runner.auto_resolve();
        assert_eq!(top_id(&runner, base), "EVO", "the offered Digimon was BASE");
        assert_eq!(top_id(&runner, off), "OFF-BASE");
    }

    /// With only an unpayable Yellow card in hand no Digimon can digivolve, so no
    /// Digimon is offered. Stricter than DCGO, whose target check is route-only
    /// (`CanPlayCardTargetFrame(…, false, …)` = `PayCost: false`, P/Yellow/P_105.cs:79)
    /// and would offer BASE here: CR 4.3 1-3-11-1 — a Digimon with only unpayable
    /// candidates can't digivolve; DCGO's extra choice resolves identically to PASS.
    #[test]
    fn p_105_delay_with_only_an_unpayable_hand_card_offers_no_target() {
        let mut runner = runner(&["PRICEY"], 0);
        let base = runner.place_on_field(0, "BASE", Some(0));
        fire_delay(&mut runner);
        assert!(
            runner.pending_selection_view().is_none(),
            "no Digimon can digivolve, so nothing is asked"
        );
        assert_eq!(top_id(&runner, base), "BASE");
        assert_eq!(hand_ids(&runner), vec!["PRICEY".to_string()]);
    }
}
