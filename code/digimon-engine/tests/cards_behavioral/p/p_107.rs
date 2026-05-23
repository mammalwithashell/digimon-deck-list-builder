//! P-107 Defense Training - Option, Cost 2, Black.
//!
//! # Card text (cards.json / printed)
//!
//! [Main] Reveal the top 2 cards of your deck. Add 1 black card among them to
//! your hand. Place the rest at the bottom of your deck in any order. Then,
//! place this card in the battle area.
//!
//! [Main] <Delay> (By trashing this card after the placing turn, activate the
//! effect below.)
//! 1 of your Digimon may digivolve into a black Digimon card in your hand for
//! its digivolution cost. When it would digivolve by this effect, reduce the
//! cost by 2.
//!
//! Inherited: Security Effect [Security] Place this card in the battle area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Black/P_107.cs
//!
//! # Patterns this test covers
//! - A1 Reveal top-2 + select-by-color + add to hand (black variant of P-105)
//! - A2 place_remainder_on_deck(bottom) with ordered remainder placement
//! - Standard <Delay> (PUPPETS-G009): effect_initiated_digivolve cost -2 via
//!   player-visible [Main]-phase activation
//! - Inherited security placement through place_self_as_delay_option

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCostDelta, CompiledDeclarativeClause,
    CompiledScope, CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

fn p_107_yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-107.yaml"))
        .expect("P-107 YAML must exist at cards/p/P-107.yaml")
}

fn p_107_runner() -> DebugRunner {
    let yaml = p_107_yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML must parse and compile")
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

fn black_evo(id: &str, cost: u16) -> CardData {
    let mut card = digimon(id, CardColor::Black, 4);
    card.evo_costs = vec![EvoCost {
        card_color: 5, // black
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

// ─── Structural tests ────────────────────────────────────────────────────────

#[test]
fn p_107_yaml_parses_and_compiles() {
    let _runner = p_107_runner();
}

#[test]
fn p_107_is_black_option_cost_2() {
    let runner = p_107_runner();
    let compiled = runner.compiled_card("P-107").expect("P-107 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert_eq!(compiled.cost, Some(2));
}

#[test]
fn p_107_has_main_delay_and_inherited_security_clauses() {
    let runner = p_107_runner();
    let compiled = runner.compiled_card("P-107").expect("P-107 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "P-107 has printed Main, Delay, and inherited Security placement text"
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
fn p_107_main_reveal_selects_black_card_structurally() {
    let runner = p_107_runner();
    let compiled = runner.compiled_card("P-107").expect("P-107 compiled");
    let main = match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => triggered,
        other => panic!("clause 0 must be triggered; got {other:?}"),
    };

    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::RevealTopDeck { count: 2, .. })
    ));

    let select_filter = main
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::SelectReveal { filter, .. } => Some(filter),
            _ => None,
        })
        .expect("Main process must select from the reveal");

    assert_eq!(select_filter.color_is, Some(CompiledColor::Black));
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
fn p_107_delay_clause_is_main_phase_activated_structurally() {
    let runner = p_107_runner();
    let compiled = runner.compiled_card("P-107").expect("P-107 compiled");
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
            trigger, process, ..
        }) => {
            assert_eq!(
                *trigger,
                CompiledTiming::Delayed,
                "printed 'By trashing this card after the placing turn' is standard <Delay> \
                 (PUPPETS-G009: MainPhaseActivated, not auto-fire at turn end)"
            );

            // Target pick is optional ("may digivolve")
            let target_pick = process
                .iter()
                .find_map(|step| match step {
                    CompiledStep::SelectOwnPermanent {
                        optional, filter, ..
                    } => Some((optional, filter)),
                    _ => None,
                })
                .expect("Delay must include an optional own-Digimon pick");
            assert!(
                *target_pick.0,
                "printed 'may digivolve' makes the target pick optional"
            );
            assert_eq!(target_pick.1.kind, Some(CompiledCardKind::Digimon));

            // Evolution hand pick: black Digimon
            let hand_pick = process
                .iter()
                .find_map(|step| match step {
                    CompiledStep::SelectHand {
                        filter, optional, ..
                    } => Some((filter, optional)),
                    _ => None,
                })
                .expect("Delay must include a black Digimon hand pick");
            assert_eq!(hand_pick.0.kind, Some(CompiledCardKind::Digimon));
            assert_eq!(hand_pick.0.color_is, Some(CompiledColor::Black));
            let _ = hand_pick.1;

            // Digivolve step with cost -2
            assert!(process.iter().any(|step| matches!(
                step,
                CompiledStep::EffectInitiatedDigivolve {
                    cost: CompiledCostDelta::Reduce(2),
                    ignore_requirements: false,
                    ..
                }
            )));
        }
        other => panic!("clause 1 must be a standard Delay; got {other:?}"),
    }
}

// ─── Behavioral: security placement ─────────────────────────────────────────

#[test]
fn p_107_security_check_places_self_in_battle_area_as_delay_option() {
    let yaml = p_107_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Black, 3))
        .add_card(filler("FILL"))
        .security(1, &["P-107"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "P-107 should leave security");
    assert_eq!(
        runner.trash_size(1),
        0,
        "P-107 should be placed in battle area instead of trashed"
    );
    let placed = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-107")
        .expect("P-107 should be placed as a battle-area Option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

// ─── Behavioral: main clause adds black card to hand ────────────────────────

#[test]
fn p_107_main_adds_black_card_from_top_2_to_hand() {
    let yaml = p_107_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML parses")
        .add_card(digimon("BLACK-DIGI", CardColor::Black, 3))
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-107"])
        .deck(0, &["FILL", "RED-DIGI", "BLACK-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve reveal and bottom ordering");

    let hand_ids = card_ids_in_hand(&runner, 0);
    assert!(
        hand_ids.iter().any(|id| id == "BLACK-DIGI"),
        "P-107 must add the black card to hand; hand={hand_ids:?}"
    );
    assert_eq!(runner.hand_size(0), hand_before + 1);
    assert_eq!(
        deck_before - runner.deck_size(0),
        1,
        "reveal 2, add 1, bottom 1 should shrink the deck by exactly 1"
    );
}

#[test]
fn p_107_main_with_no_black_in_top_2_adds_nothing_and_bottoms_remainder() {
    let yaml = p_107_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML parses")
        .add_card(digimon("RED-DIGI", CardColor::Red, 3))
        .add_card(digimon("BLUE-DIGI", CardColor::Blue, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-107"])
        .deck(0, &["FILL", "BLUE-DIGI", "RED-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .auto_resolve()
        .expect("resolve bottom ordering for non-matching reveal");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no black card in the top 2 means nothing is added to hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "both non-matching revealed cards should return to the deck bottom"
    );
}

// ─── Behavioral: playing main places P-107 as delayed Option ────────────────

#[test]
fn p_107_playing_main_places_it_as_delayed_option() {
    let yaml = p_107_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML parses")
        .add_card(digimon("BLACK-BASE", CardColor::Black, 3))
        .add_card(digimon("BLACK-DIGI", CardColor::Black, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-107"])
        .deck(0, &["FILL", "BLACK-DIGI"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLACK-BASE", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    runner
        .auto_resolve()
        .expect("resolve P-107 main reveal and placement");

    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "P-107")
        .expect("P-107 should be placed as a battle-area delayed Option");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed {
            trigger: DelayTrigger::MainPhaseActivated,
            ..
        }
    ));
}

// ─── Behavioral: Delay body digivolves into black Digimon cost -2 ───────────

#[test]
fn p_107_delay_body_digivolves_into_black_with_cost_minus_2() {
    let yaml = p_107_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML parses")
        .add_card(digimon("BLACK-BASE", CardColor::Black, 3))
        .add_card(black_evo("BLACK-EVO", 3))
        .add_card(digimon("RED-EVO", CardColor::Red, 4))
        .add_card(filler("FILL"))
        .hand(0, &["BLACK-EVO", "RED-EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-107", Some(0));
    let carrier = runner.place_on_field(0, "BLACK-BASE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::DelayEffect,
        TriggerSource::Permanent(delay_perm),
    );
    runner.game.drain_effect_queue();

    // Optional target pick
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(
        runner.pending_is_optional(),
        "target pick is the printed 'may'"
    );
    let target_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, target_action)
        .expect("choose Digimon to digivolve");

    // Mandatory black Digimon hand pick
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only black Digimon eligible; RED-EVO must be excluded"
    );
    let evo_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, evo_action)
        .expect("choose black Digimon evolution card");
    runner.game.drain_effect_queue();

    let carrier_top = runner.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(carrier_top, "BLACK-EVO");
    assert_eq!(
        runner.memory(),
        9,
        "printed cost 3 reduced by 2 should pay exactly 1 memory"
    );
}

#[test]
fn p_107_delay_target_pick_can_be_declined_with_pass() {
    let yaml = p_107_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML parses")
        .add_card(digimon("BLACK-BASE", CardColor::Black, 3))
        .add_card(black_evo("BLACK-EVO", 3))
        .add_card(filler("FILL"))
        .hand(0, &["BLACK-EVO"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-107", Some(0));
    let carrier = runner.place_on_field(0, "BLACK-BASE", Some(0));

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
        "BLACK-BASE",
        "declining leaves the Digimon unchanged"
    );
    assert!(
        card_ids_in_hand(&runner, 0)
            .iter()
            .any(|id| id == "BLACK-EVO"),
        "declining leaves the evolution card in hand"
    );
}

/// PUPPETS-G009 — after the placing turn, P-107's `<Delay>` is a player-
/// visible `[Main]`-phase action. The mask exposes the `FIELD_EFFECT`
/// activation only after the placing turn; PASS stays legal. Taking the
/// action trashes P-107 as the cost and runs the digivolve body.
#[test]
fn p_107_delay_is_player_visible_main_activation_after_placing_turn() {
    let yaml = p_107_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("P-107 YAML parses")
        .add_card(digimon("BLACK-BASE", CardColor::Black, 3))
        .add_card(black_evo("BLACK-EVO", 3))
        .add_card(filler("FILL"))
        .hand(0, &["BLACK-EVO"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let delay_perm = runner.place_on_field(0, "P-107", Some(0));
    let carrier = runner.place_on_field(0, "BLACK-BASE", Some(0));
    // Mark P-107 as a standard <Delay> placed on the current turn.
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
        "P-107 <Delay> must not be activatable on the placing turn"
    );

    // Advance to the controller's next main phase.
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    runner.game.set_memory(10);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[bit], 1.0, "P-107 <Delay> activation is a legal action");
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
        .expect("choose black Digimon evolution card");
    runner.game.drain_effect_queue();

    let _ = carrier;
    let carrier_top = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.is_digimon(&runner.game.card_data))
        .expect("base Digimon remains in battle area")
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(carrier_top, "BLACK-EVO", "Delay digivolve resolved");
    assert_eq!(
        runner.memory(),
        9,
        "printed evo cost 3 reduced by 2 pays only 1 memory"
    );
    // P-107 is trashed as the activation cost.
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| matches!(p.option_state, OptionState::Delayed { .. })),
        "P-107 is trashed as the <Delay> activation cost after the body resolves"
    );
}
