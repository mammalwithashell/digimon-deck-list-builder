//! LM-031 Black Scramble - Option, Cost 2, Black.
//!
//! # Card text (cards.json)
//!
//! [Main] 1 of your black Digimon may digivolve into a black Digimon card in
//! the hand with the digivolution cost reduced by 3. Then, place this card in
//! the battle area.
//!
//! [Start of Your Turn] If your opponent has a Digimon, <Delay> (By trashing
//! this card after the placing turn, activate the effect below.)
//! Return 1 black Digimon card from your trash to the top of the deck. Then,
//! if you don't have a Digimon, you may play 1 black Digimon card with 2000 DP
//! or less from your trash without paying the cost.
//!
//! Inherited: Security Effect [Security] You may play 1 black Digimon card
//! with 2000 DP or less from your trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Black/LM_031.cs
//!
//! # Patterns this test covers
//! - Clause A (Main): effect_initiated_digivolve with cost reduction 3 from an
//!   Option card (main_from_hand timing, optional permanent + hand selections).
//! - Clause B (Delay, start_of_your_turn): mandatory move_trash_card_to_deck_top
//!   (black Digimon), then conditional optional free play from trash gated on
//!   the controller holding no battle-area Digimon; active_when gates on the
//!   opponent controlling >=1 Digimon; `dp_lte: 2000` filter.
//! - Clause C (Security, inherited): on_security with select_trash (black
//!   Digimon DP <= 2000), play_from_trash_free, mandatory add_this_option_to_hand
//!   tail. Positive (a valid <=2000 DP candidate) and negative (an over-DP
//!   candidate excluded) coverage of the DP filter.
//! - PendingSelection/action-mask visibility for field, hand, and trash choices.
//!
//! # Closed gaps exercised
//! - **G-ZONE-SELECTED-TRASH-TO-DECK-TOP**: `move_trash_card_to_deck_top` moves
//!   a selected trash card to the deck top (Vec end = drawn first).
//! - **G-PRED-DP-LTE**: `dp_lte: 2000` is enforced for card-zone (trash) subjects.
//! - **G-OPTIONAL-SELECTION-CONTINUE-TAIL**: declining the optional select_trash
//!   still runs the mandatory add_this_option_to_hand tail.
//! - **G-DELAY-START-OF-TURN**: `kind: delay` + `trigger: start_of_your_turn`
//!   lowers to DelayTrigger::StartOfYourNextTurn.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const YAML: &str = include_str!("../../../cards/lm/LM-031.yaml");

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = play_cost;
    card
}

fn black_digimon(id: &str, level: u8, dp: i32, play_cost: u16) -> CardData {
    digimon(id, CardColor::Black, level, dp, play_cost)
}

/// A black Digimon with DP <= 2000 — eligible for the Security / Delay trash play.
fn black_small(id: &str) -> CardData {
    black_digimon(id, 3, 2000, 3)
}

/// A black Digimon with DP > 2000 — excluded by the `dp_lte: 2000` filter.
fn black_large(id: &str) -> CardData {
    black_digimon(id, 5, 8000, 8)
}

fn black_evo(id: &str) -> CardData {
    let mut card = black_digimon(id, 4, 5000, 5);
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Black as u8,
        level: 3,
        memory_cost: 4,
    }];
    card
}

fn lm_031_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML must parse and compile")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lm_031_yaml_parses_and_compiles() {
    let _runner = lm_031_runner();
}

#[test]
fn lm_031_is_black_option_cost_2() {
    let runner = lm_031_runner();
    let compiled = runner.compiled_card("LM-031").expect("LM-031 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Black]);
    assert_eq!(compiled.cost, Some(2));
}

/// LM-031 ships three clauses: Main (main_from_hand), Delay
/// (start_of_your_turn), and Security (on_security inherited).
#[test]
fn lm_031_has_main_delay_and_security_clauses_without_raw_rust() {
    let runner = lm_031_runner();
    let compiled = runner.compiled_card("LM-031").expect("LM-031 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-031 ships the Main, Delay, and Security clauses"
    );
    assert!(
        compiled.effects.iter().all(|clause| !matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )),
        "LM-031 must not use raw_rust placeholders"
    );

    let delay = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                scope,
                trigger,
                ..
            }) => Some((scope, trigger)),
            _ => None,
        })
        .expect("Delay clause must exist");
    assert_eq!(*delay.0, CompiledScope::FaceUp);
    assert_eq!(
        *delay.1,
        CompiledTiming::StartOfYourTurn,
        "the Scramble Delay is a start-of-your-turn Delay"
    );

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("Main clause must exist");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(
        main.optional,
        "printed Main text says the black Digimon may digivolve"
    );

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("Security clause must exist");
    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(
        security.optional,
        "printed Security text says 'you may' — the clause is optional"
    );
}

/// Clause A contains an EffectInitiatedDigivolve step.
#[test]
fn lm_031_main_clause_contains_effect_initiated_digivolve_step() {
    let runner = lm_031_runner();
    let compiled = runner.compiled_card("LM-031").expect("LM-031 compiled");

    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("Main clause must exist");

    assert!(
        main.process
            .iter()
            .any(|step| matches!(step, CompiledStep::EffectInitiatedDigivolve { .. })),
        "Main clause must contain EffectInitiatedDigivolve step"
    );
}

/// Clause C (Security) contains both the optional trash play and the mandatory
/// add-this-option-to-hand tail.
#[test]
fn lm_031_security_clause_contains_trash_play_and_add_to_hand_steps() {
    let runner = lm_031_runner();
    let compiled = runner.compiled_card("LM-031").expect("LM-031 compiled");

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("Security clause must exist");

    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayFromTrashFree { .. })),
        "Security clause must contain a PlayFromTrashFree step for the optional trash play"
    );
    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::AddThisOptionToHand { .. })),
        "Security clause must contain the mandatory AddThisOptionToHand tail"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] digivolve with cost -3
// ═══════════════════════════════════════════════════════════════════════════════

/// Condition gating (positive): with a black Digimon on the field and a black
/// Digimon in hand, the Main effect installs the field-selection prompt.
#[test]
fn lm_031_main_exposes_black_field_then_black_hand_choices_in_mask() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(black_digimon("BLACK-BASE", 3, 2000, 3))
        .add_card(digimon("BLUE-BASE", CardColor::Blue, 3, 2000, 3))
        .add_card(black_evo("BLACK-EVO"))
        .add_card(digimon("BLUE-HAND", CardColor::Blue, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-031", "BLACK-EVO", "BLUE-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BLACK-BASE", Some(0));
    runner.place_on_field(0, "BLUE-BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));

    let field_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which black Digimon digivolves");
    assert_eq!(field_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "field selection must expose PASS for the printed may"
    );
    assert_eq!(
        field_view.valid_action_ids.len(),
        1,
        "only the black field Digimon should be selectable"
    );
    let mask = build_action_mask(&runner.game, field_view.selecting_player);
    assert_eq!(mask[field_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(field_view.selecting_player, field_view.valid_action_ids[0])
        .expect("select black field Digimon");

    let hand_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which black hand card to digivolve into");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.len(),
        1,
        "only the black hand Digimon should be selectable"
    );
    let mask = build_action_mask(&runner.game, hand_view.selecting_player);
    assert_eq!(mask[hand_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("select black hand Digimon");
    runner.auto_resolve().expect("finish digivolve");

    assert_eq!(
        runner.memory(),
        9,
        "evo cost 4 reduced by 3 should pay 1 memory"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .all(|card| card.card_id(&runner.game.card_data) != "BLACK-EVO"),
        "selected black evo card must leave hand"
    );
    let evolved = &runner.game.player(0).battle_area[base.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "BLACK-EVO"
    );
}

/// Condition gating (negative): declining the optional Main digivolve leaves the
/// field and hand unchanged.
#[test]
fn lm_031_main_decline_leaves_field_and_hand_unchanged() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(black_digimon("BLACK-BASE-DECLINE", 3, 2000, 3))
        .add_card(black_evo("BLACK-EVO-DECLINE"))
        .add_card(filler("FILL"))
        .hand(0, &["LM-031", "BLACK-EVO-DECLINE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "BLACK-BASE-DECLINE", Some(0));
    let hand_before = runner.hand_size(0);
    let stack_before = runner.game.player(0).battle_area[0].card_sources.len();

    assert!(runner.game.activate_hand_main(0, 0));
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline optional Main digivolve");

    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(
        runner.game.player(0).battle_area[0].card_sources.len(),
        stack_before
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause C: [Security] (inherited) optional black DP<=2000 trash
// play, then mandatory add-this-option-to-hand
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive DP filter: when the defender's trash holds a black Digimon with
/// DP <= 2000, the [Security] clause exposes the optional trash play, plays the
/// small black Digimon when chosen, and then runs the mandatory tail that adds
/// LM-031 to the defender's hand.
#[test]
fn lm_031_security_plays_small_black_digimon_from_trash_then_adds_to_hand() {
    let mut attacker = filler("ATTACKER");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(attacker)
        .add_card(black_small("BLACK-SMALL"))
        .add_card(filler("FILL"))
        .security(1, &["LM-031"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["BLACK-SMALL"])
        .memory(0)
        .start();

    // Seed the defender's trash with a black Digimon DP <= 2000.
    let small = runner.game.players[1]
        .deck
        .pop()
        .expect("small black seed in deck");
    runner.game.players[1].trash.push(small);

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: LM-031 in security"
    );

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The optional trash play happened: the small black Digimon is on the
    // defender's field.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "small black Digimon was played from trash"
    );
    let played_id = runner.game.players[1].battle_area[0].card_sources[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(played_id, "BLACK-SMALL");
    assert_eq!(
        runner.trash_size(1),
        0,
        "small black Digimon left the trash when played"
    );

    // The mandatory tail must always run: LM-031 left security and went to hand.
    assert_eq!(
        runner.security_count(1),
        0,
        "LM-031 left the security stack"
    );
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-031 must be added to the defender's hand by the mandatory tail"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        hand_id, "LM-031",
        "the card added to hand must be LM-031 itself"
    );
}

/// Negative DP filter: when the defender's trash contains ONLY a black Digimon
/// with DP > 2000, the `dp_lte: 2000` predicate rejects it — nothing is played
/// from the trash, while the mandatory add-this-option-to-hand tail still runs.
#[test]
fn lm_031_security_does_not_play_over_dp_black_digimon_from_trash() {
    let mut attacker = filler("ATTACKER-LARGE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(attacker)
        .add_card(black_large("BLACK-LARGE"))
        .add_card(filler("FILL"))
        .security(1, &["LM-031"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["BLACK-LARGE"])
        .memory(0)
        .start();

    // Seed the defender's trash with a black Digimon DP > 2000.
    let large = runner.game.players[1]
        .deck
        .pop()
        .expect("large black seed in deck");
    runner.game.players[1].trash.push(large);

    let attacker = runner.place_on_field(0, "ATTACKER-LARGE", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: LM-031 in security"
    );

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The DP filter must reject the >2000 DP Digimon: nothing played.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "LM-031 Security must not play a >2000 DP Digimon from trash (G-PRED-DP-LTE)"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "the large black Digimon must remain in the defender's trash"
    );

    // The mandatory tail must always run: LM-031 added to the defender's hand.
    assert_eq!(
        runner.security_count(1),
        0,
        "LM-031 left the security stack"
    );
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-031's mandatory tail must add it to the defender's hand"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        hand_id, "LM-031",
        "the card added to hand must be LM-031 itself"
    );
}

/// Non-black trash candidates are excluded by the `color_is: black` filter,
/// and the Security clause produces no panic when the trash mixes colors.
#[test]
fn lm_031_security_excludes_non_black_trash_candidates() {
    let mut attacker = filler("ATTACKER-MIX");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(attacker)
        .add_card(digimon("BLUE-SMALL", CardColor::Blue, 3, 2000, 3))
        .add_card(filler("FILL"))
        .security(1, &["LM-031"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["BLUE-SMALL"])
        .memory(0)
        .start();

    // Seed the defender's trash with a NON-black Digimon DP <= 2000.
    let blue = runner.game.players[1]
        .deck
        .pop()
        .expect("blue seed in deck");
    runner.game.players[1].trash.push(blue);

    let attacker = runner.place_on_field(0, "ATTACKER-MIX", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The blue Digimon is not black -> not an eligible candidate, nothing played.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "a non-black Digimon must not be played from trash"
    );
    assert!(
        runner
            .game
            .player(1)
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BLUE-SMALL"),
        "non-black trash card must remain in trash"
    );

    // The mandatory tail still runs.
    assert_eq!(
        runner.security_count(1),
        0,
        "LM-031 left the security stack"
    );
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-031's mandatory tail must add it to the defender's hand"
    );
}

/// Empty trash: the optional trash play has no candidate (nothing played), but
/// the mandatory add-this-option-to-hand tail still fires.
#[test]
fn lm_031_security_adds_this_option_to_hand_with_empty_trash() {
    let mut attacker = filler("ATTACKER-EMPTY");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(attacker)
        .add_card(filler("FILL"))
        .security(1, &["LM-031"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER-EMPTY", Some(0));
    assert_eq!(
        runner.trash_size(1),
        0,
        "precondition: defender trash empty"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: LM-031 in security"
    );

    let result = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "no Digimon played from an empty trash"
    );
    assert_eq!(runner.security_count(1), 0, "LM-031 left security");
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "LM-031"),
        "LM-031 must be added to hand even with an empty trash"
    );
}

/// Optionality: declining the optional trash play leaves the eligible black
/// Digimon in trash, while the mandatory add-this-option-to-hand tail still
/// fires (G-OPTIONAL-SELECTION-CONTINUE-TAIL).
#[test]
fn lm_031_security_decline_trash_play_still_adds_this_option_to_hand() {
    let mut attacker = filler("ATTACKER-DECLINE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(attacker)
        .add_card(black_small("BLACK-SMALL-DECLINE"))
        .add_card(filler("FILL"))
        .security(1, &["LM-031"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["BLACK-SMALL-DECLINE"])
        .memory(0)
        .start();

    // Seed the defender's trash with an eligible black Digimon DP <= 2000.
    let small = runner.game.players[1]
        .deck
        .pop()
        .expect("small black seed in deck");
    runner.game.players[1].trash.push(small);

    let attacker = runner.place_on_field(0, "ATTACKER-DECLINE", Some(0));

    let _ = runner.attack_player(attacker, 1, false);

    // The optional trash selection must be exposed (no auto-pick) — decline it.
    let view = runner
        .pending_selection_view()
        .expect("optional Security trash play must surface a pending selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "the printed 'you may' must expose PASS on the trash selection"
    );
    runner
        .execute_action(view.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline optional Security trash play");
    runner.auto_resolve().expect("finish security effect");

    // Declining leaves the black Digimon in trash, nothing played.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "declining the optional play must not play a Digimon"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "declined black Digimon remains in trash"
    );

    // The mandatory tail still runs.
    assert_eq!(runner.security_count(1), 0, "LM-031 left security");
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "LM-031"),
        "LM-031 is still added to hand after declining the optional play"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause B: [Start of Your Turn] Delay
// ═══════════════════════════════════════════════════════════════════════════════

/// Push a card_id into a player's trash by registry lookup.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            data_idx, player, card_index,
        ));
}

/// Seat LM-031 (at player 0's `hand[0]`) as a Delay Option through the real
/// Option pipeline. With no black digivolve base on the field the Main clause
/// has nothing to do, so the Option disposes straight to a Delay permanent in
/// the battle area.
fn seat_lm_031_as_delay_option(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        digimon_engine::selection::OptionPlayResult::Trashed,
        "LM-031 disposes straight to a Delay Option permanent on Main resolution"
    );
}

fn lm_031_is_delayed_option(runner: &DebugRunner) -> bool {
    runner.game.player(0).battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "LM-031"
            && matches!(
                perm.option_state,
                digimon_engine::permanent::OptionState::Delayed { .. }
            )
    })
}

/// [Main] "Then, place this card in the battle area." Adding a `kind: delay`
/// clause makes the engine classify LM-031 as a Delay Option, so the Option
/// pipeline auto-seats it in the battle area on resolution.
#[test]
fn lm_031_main_places_this_card_in_battle_area_after_resolution() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(black_digimon("BLACK-ANCHOR", 3, 2000, 3))
        .add_card(filler("FILL"))
        .hand(0, &["LM-031"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    // A black Digimon in breeding satisfies LM-031's color requirement
    // without putting a Digimon on the battle area.
    runner.place_in_breeding(0, "BLACK-ANCHOR");

    seat_lm_031_as_delay_option(&mut runner);

    assert!(
        lm_031_is_delayed_option(&runner),
        "LM-031 must be seated in the battle area as a Delay Option after the Main clause resolves"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "exactly LM-031 should be on the battle area"
    );
}

/// [Start of Your Turn] If your opponent has a Digimon, <Delay>: trash this
/// card, then return 1 black Digimon from your trash to the TOP of the deck.
/// The delay fires automatically at the start of the owner's next turn. Because
/// the card is returned to the deck TOP (Vec end = drawn first), that turn's
/// Draw phase draws it straight back into hand.
#[test]
fn lm_031_delay_fires_at_start_of_your_turn_and_returns_black_trash_to_deck_top() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(black_digimon("BLACK-TRASH", 3, 2000, 3))
        .add_card(black_digimon("BLACK-ANCHOR", 3, 2000, 3))
        .add_card(digimon("OPP-DIGIMON", CardColor::Red, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-031"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();

    // Opponent must control a Digimon for the Delay's active_when gate.
    runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    // A black Digimon in breeding satisfies LM-031's color requirement.
    runner.place_in_breeding(0, "BLACK-ANCHOR");
    // A black Digimon waits in player 0's trash.
    push_to_trash(&mut runner, 0, "BLACK-TRASH");

    seat_lm_031_as_delay_option(&mut runner);
    assert!(lm_031_is_delayed_option(&runner));

    // P0 ends turn -> P1's turn -> P1 ends turn -> begin_turn for P0 fires
    // the StartOfYourNextTurn Delay.
    runner.end_turn();
    runner.end_turn();
    accept_scheduled_delay(&mut runner);
    assert_eq!(runner.game.turn_player(), 0, "back on player 0's turn");

    // Step (a) is a mandatory trash pick of a black Digimon.
    let view = runner
        .pending_selection_view()
        .expect("Delay must prompt a mandatory black-Digimon trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        !runner.pending_is_optional(),
        "returning a black Digimon to deck top is mandatory per printed text"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the black Digimon in trash should be selectable"
    );
    let mask = build_action_mask(&runner.game, view.selecting_player);
    assert_eq!(mask[view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return the black Digimon to the deck top");
    runner.auto_resolve().expect("finish the Delay body");

    // LM-031 was trashed as the Delay cost.
    assert!(
        !lm_031_is_delayed_option(&runner),
        "LM-031 must leave the battle area — trashed as the Delay activation cost"
    );
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM-031"),
        "LM-031 must be in the trash after paying the Delay cost"
    );

    // The black Digimon left the trash.
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "BLACK-TRASH"),
        "the returned black Digimon must have left the trash"
    );
    // It was placed on the deck TOP, so this turn's Draw phase drew it back
    // into hand. The deck (8 FILL) would never surface it from the bottom.
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BLACK-TRASH"),
        "deck top (drawn first) must surface the returned black Digimon into hand \
         on the turn-start draw — proof the verb appends to the deck end, not index 0"
    );
}

/// [Start of Your Turn] Delay tail: "Then, if you don't have a Digimon, you
/// may play 1 black Digimon card with 2000 DP or less from your trash without
/// paying the cost." With no Digimon controlled the optional play is offered,
/// the candidate set enforces the `2000 DP or less` filter, and taking it
/// free-plays the card.
#[test]
fn lm_031_delay_then_optionally_plays_small_black_digimon_if_you_have_no_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        // Trash pool: BLACK-SMALL (DP 2000, playable in step b),
        // BLACK-RETURN (DP 2000, returned in step a),
        // BLACK-BIG (DP 8000, too big for step b's DP filter).
        .add_card(black_digimon("BLACK-SMALL", 3, 2000, 3))
        .add_card(black_digimon("BLACK-RETURN", 3, 2000, 3))
        .add_card(black_digimon("BLACK-BIG", 5, 8000, 8))
        .add_card(black_digimon("BLACK-ANCHOR", 3, 2000, 3))
        .add_card(digimon("OPP-DIGIMON", CardColor::Red, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-031"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    // A black Digimon in breeding satisfies LM-031's color requirement
    // without putting a Digimon on the battle area, so step (b)'s
    // "if you don't have a Digimon" gate stays true.
    runner.place_in_breeding(0, "BLACK-ANCHOR");
    push_to_trash(&mut runner, 0, "BLACK-SMALL");
    push_to_trash(&mut runner, 0, "BLACK-RETURN");
    push_to_trash(&mut runner, 0, "BLACK-BIG");

    seat_lm_031_as_delay_option(&mut runner);

    runner.end_turn();
    runner.end_turn();
    accept_scheduled_delay(&mut runner);
    assert_eq!(runner.game.turn_player(), 0);

    // Step (a): mandatory trash pick. All three black Digimon are eligible
    // (step a has no DP filter). Pick BLACK-RETURN.
    let view = runner
        .pending_selection_view()
        .expect("Delay step (a) black-Digimon trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert_eq!(
        view.valid_action_ids.len(),
        3,
        "all three black Digimon in trash are eligible to return to deck top"
    );
    let return_trash_index = runner
        .game
        .player(0)
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BLACK-RETURN")
        .expect("BLACK-RETURN is in the trash");
    let return_action =
        digimon_engine::action::space::TRASH_EFFECT_START + return_trash_index as u16;
    assert!(
        view.valid_action_ids.contains(&return_action),
        "BLACK-RETURN must be a selectable trash candidate"
    );
    runner
        .execute_action(view.selecting_player, return_action)
        .expect("return BLACK-RETURN to deck top");

    // Player 0 controls no battle-area Digimon (LM-031 was trashed as the
    // Delay cost; the anchor is in breeding) -> step (b)'s optional play fires.
    let tail = runner
        .pending_selection_view()
        .expect("with no Digimon controlled, the optional small-black trash play must be offered");
    assert_eq!(tail.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "the 'you may play' tail is optional"
    );
    // Only BLACK-SMALL (DP 2000) qualifies — BLACK-BIG (DP 8000) is filtered
    // out by `dp_lte: 2000`. LM-031 in trash is an Option, not a Digimon.
    assert_eq!(
        tail.valid_action_ids.len(),
        1,
        "only the DP-2000 black Digimon qualifies; the DP-8000 one is filtered out"
    );
    runner
        .execute_action(tail.selecting_player, tail.valid_action_ids[0])
        .expect("take the optional free play");
    runner.auto_resolve().expect("finish the Delay body");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the optional free play should put exactly one black Digimon on the field"
    );
    assert_eq!(
        runner.game.player(0).battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "BLACK-SMALL",
        "the played card is the DP-2000 black Digimon"
    );
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "BLACK-SMALL"),
        "the free-played black Digimon must have left the trash"
    );
}

/// Negative branch of the Delay tail: when the controller already has a
/// Digimon on the field the "if you don't have a Digimon" optional play is
/// gated off entirely — only step (a) runs.
#[test]
fn lm_031_delay_skips_optional_play_when_you_already_have_a_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-031 YAML parses")
        .add_card(black_digimon("BLACK-RETURN", 3, 2000, 3))
        .add_card(black_digimon("BLACK-SMALL", 3, 2000, 3))
        .add_card(black_digimon("BLACK-ANCHOR", 3, 2000, 3))
        .add_card(digimon("BLUE-DIGIMON", CardColor::Blue, 4, 5000, 5))
        .add_card(digimon("OPP-DIGIMON", CardColor::Red, 4, 5000, 5))
        .add_card(filler("FILL"))
        .hand(0, &["LM-031"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    push_to_trash(&mut runner, 0, "BLACK-RETURN");
    push_to_trash(&mut runner, 0, "BLACK-SMALL");
    // A black Digimon in breeding satisfies LM-031's color requirement
    // without being a black battle-area digivolve base for the Main clause.
    runner.place_in_breeding(0, "BLACK-ANCHOR");
    // Player 0 keeps a (non-black) Digimon on the battle area through the
    // Delay so step (b)'s "if you don't have a Digimon" gate stays false.
    runner.place_on_field(0, "BLUE-DIGIMON", Some(0));

    seat_lm_031_as_delay_option(&mut runner);

    runner.end_turn();
    runner.end_turn();
    accept_scheduled_delay(&mut runner);
    assert_eq!(runner.game.turn_player(), 0);

    let view = runner
        .pending_selection_view()
        .expect("Delay step (a) mandatory trash pick still runs");
    assert_eq!(view.kind, SelectionKind::Trash);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return a black Digimon to deck top");
    runner.auto_resolve().expect("finish the Delay body");

    assert_eq!(
        runner.pending_kind(),
        None,
        "the optional trash play must NOT be offered when you control a Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "only the pre-existing Digimon remains; no extra card was played"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .all(|perm| { perm.top_card().card_id(&runner.game.card_data) == "BLUE-DIGIMON" }),
        "no black Digimon from trash was played"
    );
    // Step (a) still ran: one black Digimon left the trash; LM-031 (Delay
    // cost) plus the remaining black Digimon stay in the trash.
    assert_eq!(
        runner.trash_size(0),
        2,
        "step (a) returned one black Digimon to deck top; LM-031 and the \
         second black Digimon remain in the trash"
    );
}


/// Accept the §16-16-2 `<Delay>` cost confirm that now precedes the body.
///
/// Added 2026-08-24: the scheduled window used to auto-pay the trash-this-card
/// cost and run the body straight away. It is optional processing, so the
/// controller is asked first.
fn accept_scheduled_delay(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("the scheduled <Delay> window must offer its optional cost");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("the accept branch must be offered");
    runner
        .execute_action(view.selecting_player, action)
        .expect("accept the <Delay> cost");
}
