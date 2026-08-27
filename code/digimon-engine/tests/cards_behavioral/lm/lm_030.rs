//! LM-030 Green Scramble — Option, Cost 2, Green.
//!
//! # Card text (cards.json)
//!
//! [Main] 1 of your green Digimon may digivolve into a green Digimon card in
//! the hand with the digivolution cost reduced by 3. Then, place this card in
//! the battle area.
//!
//! [Start of Your Turn] If your opponent has a Digimon, <Delay> (By trashing
//! this card after the placing turn, activate the effect below.)
//! Return 1 green Digimon card from your trash to the top of the deck. Then,
//! if you don't have a Digimon, you may play 1 green Digimon card with 2000 DP
//! or less from your trash without paying the cost.
//!
//! Inherited: Security Effect [Security] You may play 1 green Digimon card
//! with 2000 DP or less from your trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Green/LM_030.cs
//!
//! # Patterns this test covers
//! - Clause A (Main): effect_initiated_digivolve with cost reduce 3 from an
//!   option card (main_from_hand timing, optional permanent selection)
//! - Clause B (Delay/StartOfYourTurn): `kind: delay` + `trigger:
//!   start_of_your_turn`, `active_when` opponent-has-Digimon gate;
//!   `move_trash_card_to_deck_top` returns the selected green Digimon to deck
//!   top; conditional `play_from_trash_free` of a ≤2000 DP green Digimon when
//!   you control no Digimon.
//! - Clause C (Security, inherited): on_security with select_trash (green
//!   Digimon ≤2000 DP, `dp_lte` enforced for trash subjects),
//!   play_from_trash_free, add_this_option_to_hand.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::selection::SelectionKind;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/lm/LM-030.yaml");

// ─── Helper cards ──────────────────────────────────────────────────────────────

fn make_green_digimon(id: &str, level: u8, dp: i32, cost: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost as u16;
    c.colors = vec![CardColor::Green];
    c
}

/// A minimal green Digimon with DP ≤ 2000 (eligible target for Security trash play).
fn make_small_green_digimon(id: &str) -> CardData {
    make_green_digimon(id, 3, 2000, 3)
}

/// A large green Digimon (4000 DP) — should be filtered by dp_lte: 2000.
fn make_large_green_digimon(id: &str) -> CardData {
    make_green_digimon(id, 4, 4000, 5)
}

/// A green Digimon with evo cost (digivolve target).
fn make_green_evo_target(id: &str) -> CardData {
    let mut c = make_green_digimon(id, 4, 5000, 5);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Green as u8,
        level: 3,
        memory_cost: 4,
    }];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn lm_030_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML must parse and compile")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// YAML must parse and compile without error.
#[test]
fn lm_030_yaml_parses_without_error() {
    let _runner = lm_030_runner();
}

/// Card metadata: Green Option, cost 2.
#[test]
fn lm_030_is_green_option_cost_2() {
    let runner = lm_030_runner();
    let compiled = runner.compiled_card("LM-030").expect("LM-030 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Green]);
    assert_eq!(compiled.cost, Some(2));
}

/// LM-030 has 3 compiled clauses: Main (main_from_hand), Delay
/// (start_of_your_turn), and Security (on_security inherited).
#[test]
fn lm_030_has_three_clauses_main_delay_and_security() {
    let runner = lm_030_runner();
    let compiled = runner.compiled_card("LM-030").expect("LM-030 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-030 ships Main, Delay, and Security clauses"
    );
}

/// Neither clause uses raw_rust (no placeholders in this pass).
#[test]
fn lm_030_no_raw_rust_placeholders() {
    let runner = lm_030_runner();
    let compiled = runner.compiled_card("LM-030").expect("LM-030 compiled");

    assert!(
        compiled.effects.iter().all(|clause| !matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )),
        "LM-030 must not use raw_rust placeholders"
    );
}

/// Clause A: main_from_hand triggered, optional, FaceUp scope.
#[test]
fn lm_030_main_clause_is_optional_face_up() {
    let runner = lm_030_runner();
    let compiled = runner.compiled_card("LM-030").expect("LM-030 compiled");

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
        "printed Main text says the green Digimon may digivolve"
    );
}

/// Clause A contains an EffectInitiatedDigivolve step.
#[test]
fn lm_030_main_clause_contains_effect_initiated_digivolve_step() {
    let runner = lm_030_runner();
    let compiled = runner.compiled_card("LM-030").expect("LM-030 compiled");

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

/// Clause C (Security): on_security inherited, optional, FaceUp scope.
#[test]
fn lm_030_security_clause_is_inherited_optional() {
    let runner = lm_030_runner();
    let compiled = runner.compiled_card("LM-030").expect("LM-030 compiled");

    let sec = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("Security clause must exist");

    assert_eq!(
        sec.scope,
        CompiledScope::Inherited,
        "Security clause must have Inherited scope"
    );
    assert!(
        sec.optional,
        "printed Security text says 'you may' — clause must be optional"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] digivolve with cost -3
// ═══════════════════════════════════════════════════════════════════════════════

/// When P0 has a green Digimon on field and a green Digimon in hand,
/// activating the [Main] effect installs a pending selection prompt for the
/// digivolve source (optional — "may").
#[test]
fn lm_030_main_installs_selection_when_eligible_green_digimon_on_field() {
    let src = make_green_digimon("LM030-SRC", 3, 2000, 3);
    let evo = make_green_evo_target("LM030-EVO");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(src.clone())
        .add_card(evo.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-030", "LM030-EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["LM030-SRC"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let _field_handle = runner.place_on_field(0, "LM030-SRC", None);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for LM-030 at hand index 0"
    );

    assert!(
        runner.game.pending_selection.is_some(),
        "LM-030 Main must install a pending selection when a green Digimon is on P0's field"
    );
}

/// When P0 has no green Digimon on field AND no green Digimon in hand,
/// the Main effect's optional permanent selection has no eligible targets and
/// `select_hand` also finds no matching hand cards — the effect completes
/// cleanly without installing any pending selection.
///
/// Note: when green Digimon exist in hand but not on field, the engine's
/// `run_steps_with_runtime` loop advances past the empty `select_own_permanent`
/// and runs `select_hand` sequentially (a pre-existing engine behaviour also
/// present in LM-027). This test avoids that scenario by seeding the hand with
/// only non-green filler cards.
#[test]
fn lm_030_main_no_selection_when_no_green_digimon_anywhere() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-030", "FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true");

    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "LM-030 Main must produce no selection when P0 has no green Digimon on field or in hand"
    );
}

/// When the player selects a green source Digimon and a green hand card to
/// digivolve into, the evo cost 4 reduced by 3 costs 1 memory, and the target
/// lands on the stack.
#[test]
fn lm_030_main_digivolve_applies_cost_reduction_of_3() {
    let src = make_green_digimon("LM030-BASE", 3, 2000, 3);
    let evo = make_green_evo_target("LM030-EVO-POS");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(src.clone())
        .add_card(evo.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-030", "LM030-EVO-POS"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let base_handle = runner.place_on_field(0, "LM030-BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));

    // Drive field selection (optional permanent selection for source Digimon).
    let field_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which green Digimon digivolves");
    assert_eq!(field_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "field selection must expose PASS for the printed may"
    );
    assert_eq!(
        field_view.valid_action_ids.len(),
        1,
        "only the green field Digimon should be selectable"
    );
    runner
        .execute_action(field_view.selecting_player, field_view.valid_action_ids[0])
        .expect("select green field Digimon");

    // Drive hand selection (choose digivolve target from hand).
    let hand_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which green hand card to digivolve into");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.len(),
        1,
        "only the green hand Digimon should be selectable"
    );
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("select green hand Digimon");
    runner.auto_resolve().expect("finish digivolve");

    // evo_cost 4 reduced by 3 = 1 memory spent; starting at 10, end at 9.
    assert_eq!(
        runner.memory(),
        9,
        "evo cost 4 reduced by 3 should pay 1 memory"
    );
    // The selected evo card must have left hand.
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .all(|card| card.card_id(&runner.game.card_data) != "LM030-EVO-POS"),
        "selected green evo card must leave hand"
    );
    // The evo card must be on top of the stack.
    let evolved = &runner.game.player(0).battle_area[base_handle.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "LM030-EVO-POS"
    );
}

/// Declining the optional field selection leaves field and hand unchanged.
#[test]
fn lm_030_main_decline_leaves_field_and_hand_unchanged() {
    let src = make_green_digimon("LM030-BASE-DECLINE", 3, 2000, 3);
    let evo = make_green_evo_target("LM030-EVO-DECLINE");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(src.clone())
        .add_card(evo.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-030", "LM030-EVO-DECLINE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    runner.place_on_field(0, "LM030-BASE-DECLINE", Some(0));
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
// Section 3 — Clause B: Delay ([Start of Your Turn])
// ═══════════════════════════════════════════════════════════════════════════════

use digimon_engine::enums::DelayTrigger;
use digimon_engine::permanent::OptionState;

/// Place LM-030 on P0's field as a Delay-Option whose `StartOfYourNextTurn`
/// trigger matures at the start of P0's next turn. Driven via two `end_turn()`
/// calls (P0 → P1 → P0): the start of the P0 turn fires the delay.
fn place_lm_030_as_start_delay(runner: &mut DebugRunner) {
    let handle = runner.place_on_field(0, "LM-030", Some(0));
    // P0 is the turn player; "next P0 turn" is turn_count + 2 (skip P1).
    let fire_turn = runner.game.turn_count + 2;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: fire_turn,
            trigger: DelayTrigger::StartOfYourNextTurn,
            placed_on_turn: runner.game.turn_count,
        };
}

/// Advance from P0's turn to the start of P0's next turn (P0 → P1 → P0),
/// where `StartOfYourNextTurn` delays mature.
/// Accept the §16-16-2 `<Delay>` cost confirm that now precedes the body.
///
/// Added 2026-08-24: the scheduled window used to auto-pay the trash-this-card
/// cost and run the body straight away. It is optional processing, so the
/// controller is asked first; these tests exercise the ACCEPT branch, and
/// `lm_030_delay_may_be_declined_leaving_the_option_on_the_field` covers the
/// other one.
fn accept_scheduled_delay(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("the scheduled <Delay> window must offer its optional cost");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("the accept branch must be offered");
    runner
        .execute_action(view.selecting_player, action)
        .expect("accept the <Delay> cost");
}

fn advance_to_next_p0_turn(runner: &mut DebugRunner) {
    runner.end_turn(); // P0 → P1
    runner.end_turn(); // P1 → P0 (start fires matured delays)
}

/// Push a card by id into player `p`'s trash; owned by `p`.
fn push_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_trash: unknown card_id {card_id}"));
    let card =
        digimon_engine::card_source::CardSource::new(data_idx, p, runner.game.next_card_index());
    runner.game.players[p as usize].trash.push(card);
}

/// Place an opponent (P1) Digimon on the field.
fn place_opp_digimon(runner: &mut DebugRunner, card_id: &str) {
    runner.place_on_field(1, card_id, None);
}

/// The Delay fires at the start of P0's turn when the opponent has a Digimon:
/// it installs the mandatory trash selection (return a green Digimon to the
/// top of the deck).
#[test]
fn lm_030_delay_fires_at_start_of_your_turn_when_opponent_has_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(make_green_digimon("LM030-TRASH-GREEN", 3, 2000, 3))
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // OPP-DIGI must be a Digimon for the active_when gate.
    if let Some(d) = runner
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "OPP-DIGI")
    {
        d.card_kind = CardKind::Digimon;
        d.level = Some(4);
        d.dp = Some(4000);
    }

    place_lm_030_as_start_delay(&mut runner);
    place_opp_digimon(&mut runner, "OPP-DIGI");
    push_trash(&mut runner, 0, "LM030-TRASH-GREEN");

    advance_to_next_p0_turn(&mut runner);
    accept_scheduled_delay(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("Delay must install the mandatory green-Digimon trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        !runner.pending_is_optional(),
        "step 1 of the Delay body is a mandatory selection (no PASS)"
    );
}

/// The Delay must NOT fire when the opponent controls no Digimon — its
/// `active_when` opponent-has-Digimon gate fails.
#[test]
fn lm_030_delay_does_not_fire_when_opponent_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(make_green_digimon("LM030-TRASH-GREEN", 3, 2000, 3))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    place_lm_030_as_start_delay(&mut runner);
    push_trash(&mut runner, 0, "LM030-TRASH-GREEN");
    // No opponent Digimon on the field.

    advance_to_next_p0_turn(&mut runner);

    assert!(
        runner.game.pending_selection.is_none(),
        "Delay must not fire (and not prompt) when opponent has no Digimon"
    );
}

/// Inner Delay body (1/2): the selected green Digimon is moved from trash to
/// the TOP of P0's deck. The Delay fires during `begin_turn`, before the
/// turn-start draw; once the parked Delay resolves, `begin_turn` continues and
/// the turn-start draw takes the top card — so the returned green Digimon ends
/// up in hand. Being drawn proves it was placed on the deck TOP (the draw
/// always takes the top card). The Delay cost trashes LM-030 itself.
#[test]
fn lm_030_delay_body_returns_green_digimon_to_top_of_deck() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(make_green_digimon("LM030-RET-GREEN", 3, 2000, 3))
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    if let Some(d) = runner
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "OPP-DIGI")
    {
        d.card_kind = CardKind::Digimon;
        d.level = Some(4);
        d.dp = Some(4000);
    }

    place_lm_030_as_start_delay(&mut runner);
    place_opp_digimon(&mut runner, "OPP-DIGI");
    push_trash(&mut runner, 0, "LM030-RET-GREEN");

    // P0 has a Digimon on the field so the optional play branch does NOT fire
    // — isolates step 1 (return to deck top).
    runner.place_on_field(0, "OPP-DIGI", None);

    advance_to_next_p0_turn(&mut runner);
    accept_scheduled_delay(&mut runner);

    // Pick the green Digimon in the mandatory trash selection.
    let view = runner
        .pending_selection_view()
        .expect("Delay must install the trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select green Digimon from trash");
    runner.auto_resolve().expect("finish Delay body");

    // The green Digimon must have left the trash.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "LM030-RET-GREEN"),
        "the returned green Digimon must leave the trash"
    );
    // It was placed on the deck TOP and then drawn by the turn-start draw.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM030-RET-GREEN"),
        "the returned green Digimon must be on the deck TOP — proven by the \
         turn-start draw pulling it into hand"
    );
    // The Delay cost trashes LM-030 itself.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM-030"),
        "LM-030 must be trashed as the <Delay> cost"
    );
}

/// Inner Delay body (2/2): when P0 controls NO Digimon, the conditional
/// free-play branch fires — a ≤2000 DP green Digimon is played from trash.
#[test]
fn lm_030_delay_body_play_from_trash_only_when_no_field_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(make_green_digimon("LM030-RET-GREEN", 3, 2000, 3))
        .add_card(make_small_green_digimon("LM030-PLAY-GREEN"))
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    if let Some(d) = runner
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "OPP-DIGI")
    {
        d.card_kind = CardKind::Digimon;
        d.level = Some(4);
        d.dp = Some(4000);
    }

    place_lm_030_as_start_delay(&mut runner);
    place_opp_digimon(&mut runner, "OPP-DIGI");
    push_trash(&mut runner, 0, "LM030-RET-GREEN");
    push_trash(&mut runner, 0, "LM030-PLAY-GREEN");
    // P0 controls no Digimon (only the LM-030 Delay-Option permanent).

    advance_to_next_p0_turn(&mut runner);
    accept_scheduled_delay(&mut runner);

    // Step 1: mandatory trash selection — return a green Digimon to deck top.
    let view = runner
        .pending_selection_view()
        .expect("Delay step 1 trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    // Pick the card NOT meant for play (return LM030-RET-GREEN).
    let ret_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| {
            // Both green Digimon are eligible for step 1; either works. Use
            // the first.
            true
        })
        .expect("a trash card to return");
    runner
        .execute_action(view.selecting_player, ret_action)
        .expect("return a green Digimon to deck top");

    // Step 2: optional play selection — a ≤2000 DP green Digimon from trash.
    let play_view = runner
        .pending_selection_view()
        .expect("Delay step 2 optional play selection (P0 has no Digimon)");
    assert_eq!(play_view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "step 2 play is optional ('you may')"
    );
    runner
        .execute_action(play_view.selecting_player, play_view.valid_action_ids[0])
        .expect("play a green Digimon from trash");
    runner.auto_resolve().expect("finish Delay body");

    // A green Digimon must have been played from trash onto P0's field.
    assert!(
        runner.game.players[0].battle_area.iter().any(|p| {
            let id = p.top_card().card_id(&runner.game.card_data);
            id == "LM030-PLAY-GREEN" || id == "LM030-RET-GREEN"
        }),
        "the conditional branch must play a ≤2000 DP green Digimon from trash"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause C: [Security] (Inherited) play small green Digimon from
// trash; then add this card to hand
// ═══════════════════════════════════════════════════════════════════════════════

/// Security clause fires without panic when the engine triggers it.
#[test]
fn lm_030_security_clause_no_panic_with_empty_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // Place LM-030 on P0's field as an option permanent (as it would be after
    // the Delay clause lands it there).
    let field_handle = runner.place_on_field(0, "LM-030", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // Drain any pending selections (optional clause — engine may install no
    // selection when trash is empty).
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // No panic is the primary assertion.
}

/// When the defender's trash has a green Digimon, a real attack on the
/// defender's security checks LM-030 and the inherited [Security] clause runs
/// through the proper security path: it may play the small green Digimon from
/// the defender's trash and then adds LM-030 to the defender's hand.
///
/// Driven through the real combat/security-check path (see
/// `lm_030_security_adds_card_to_hand_and_plays_small_green_digimon`) — the
/// previous `enqueue_triggered(SecuritySkill, Permanent(..))` shortcut only
/// ever "worked" because of an over-fire bug in `enqueue_from_permanent` and
/// is now a silent no-op for inherited-scope [Security] clauses.
#[test]
fn lm_030_security_installs_trash_selection_when_green_digimon_in_trash() {
    let small = make_small_green_digimon("LM030-SMALL-GREEN");
    let mut attacker = make_filler("LM030-ATK-GREEN");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(small.clone())
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM030-SMALL-GREEN"])
        .security(1, &["LM-030"])
        .start();

    // Seed the defender's trash with a small green Digimon.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("small green seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM030-ATK-GREEN", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: LM-030 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The [Security] clause must have run: LM-030 left the security stack and
    // its mandatory tail ("Then, add this card to the hand") routed LM-030 to
    // the defender's hand.
    assert_eq!(
        runner.security_count(1),
        0,
        "LM-030 left the security stack"
    );
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-030 must be routed to the defender's hand by the mandatory tail"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        hand_id, "LM-030",
        "the card added to the defender's hand must be LM-030 itself"
    );

    // The optional clause body may also have played the small green Digimon
    // from the defender's trash onto the defender's field.
    let small_on_field = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "LM030-SMALL-GREEN");
    let _ = small_on_field;
}

/// When the defender's trash is empty, a real attack on the defender's
/// security still runs LM-030's [Security] clause: the optional trash play has
/// no eligible target (nothing played), but the mandatory tail ("Then, add
/// this card to the hand") still fires.
///
/// Driven through the real combat/security-check path. The previous
/// `enqueue_triggered(SecuritySkill, Permanent(..))` shortcut became a silent
/// no-op after the `enqueue_from_permanent` over-fire fix, so the old
/// `pending_selection.is_none()` assertion passed vacuously (no selection
/// because the effect never fired at all). This rewrite asserts real
/// post-state instead.
#[test]
fn lm_030_security_no_selection_when_trash_is_empty() {
    let mut attacker = make_filler("LM030-ATK-EMPTY");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["LM-030"])
        .start();

    let attacker_handle = runner.place_on_field(0, "LM030-ATK-EMPTY", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(
        runner.trash_size(1),
        0,
        "precondition: defender trash empty"
    );
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: LM-030 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // No eligible green Digimon in the trash → nothing played.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "no green Digimon played from an empty trash"
    );
    // The mandatory tail still ran: LM-030 left security and went to hand.
    assert_eq!(
        runner.security_count(1),
        0,
        "LM-030 left the security stack"
    );
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-030's mandatory tail must add it to the defender's hand even with an empty trash"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        hand_id, "LM-030",
        "the card added to hand must be LM-030 itself"
    );
}

/// DP filter test: when the defender's trash contains ONLY a large green
/// Digimon (>2000 DP), the [Security] clause's `dp_lte: 2000` predicate
/// rejects it — the large Digimon must NOT be played from trash, while the
/// mandatory tail ("Then, add this card to the hand") still runs.
///
/// Driven through the real combat/security-check path. The previous
/// `enqueue_triggered(SecuritySkill, Permanent(..))` shortcut became a silent
/// no-op after the `enqueue_from_permanent` over-fire fix, which made the old
/// `pending_selection.is_none()` assertion pass vacuously. Through the real
/// path the `dp_lte: 2000` predicate is honored, so the large Digimon stays
/// in trash and the clause's mandatory tail still fires.
#[test]
fn lm_030_security_no_selection_when_only_large_green_digimon_in_trash() {
    let large = make_large_green_digimon("LM030-LARGE-GREEN");
    let mut attacker = make_filler("LM030-ATK-LARGE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(large.clone())
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM030-LARGE-GREEN"])
        .security(1, &["LM-030"])
        .start();

    // Seed the defender's trash with a large (>2000 DP) green Digimon.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("large green seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM030-ATK-LARGE", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: LM-030 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The mandatory tail must always run: LM-030 added to the defender's hand.
    assert_eq!(
        runner.security_count(1),
        0,
        "LM-030 left the security stack"
    );
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-030's mandatory tail must add it to the defender's hand"
    );
    // The DP filter must reject the >2000 DP Digimon: nothing played from trash.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "LM-030 Security must not play a >2000 DP Digimon from trash (G-PRED-DP-LTE)"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "the large green Digimon must remain in the defender's trash"
    );
}

/// After Security resolves and a small green Digimon is played from trash,
/// LM-030 must be added to the defender's hand ("Then, add this card to the
/// hand" — mandatory tail).
#[test]
fn lm_030_security_adds_card_to_hand_and_plays_small_green_digimon() {
    let small = make_small_green_digimon("LM030-SMALL-SEC");
    let mut attacker = make_filler("LM030-ATTACKER");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(small)
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM030-SMALL-SEC"])
        .security(1, &["LM-030"])
        .start();

    // Seed P1's trash with a small green Digimon.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("small green seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM030-ATTACKER", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "LM-030 left security");
    assert_eq!(runner.hand_size(1), 1, "LM-030 moved to defender hand");
    assert_eq!(
        runner.trash_size(1),
        0,
        "small green Digimon left trash (was played)"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "small green Digimon was played from trash"
    );
    let played_id = runner.game.players[1].battle_area[0].card_sources[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(played_id, "LM030-SMALL-SEC");
}

/// add-this-option-to-hand must still fire when the player DECLINES the optional
/// trash play. The selected Digimon must remain in trash while the mandatory
/// tail ("Then, add this card to the hand") continues.
/// G-OPTIONAL-SELECTION-CONTINUE-TAIL — closed by Phase 2 Track H
/// (install_select_trash now attaches an on_decline tail-runner for optional
/// selections, mirroring the 2026-04-29 select_material / select_own_sources fix).
#[test]
fn lm_030_security_adds_card_to_hand_even_when_trash_play_declined() {
    let small = make_small_green_digimon("LM030-SMALL-DECL");
    let mut attacker = make_filler("LM030-ATTACKER-DECL");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(small)
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM030-SMALL-DECL"])
        .security(1, &["LM-030"])
        .start();

    // Seed P1's trash with a small green Digimon.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("small green seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM030-ATTACKER-DECL", Some(0));

    let _ = runner.attack_player(attacker_handle, 1, false);

    // The optional trash selection should be pending (green Digimon in trash).
    // Decline it (PASS action).
    if let Some(view) = runner.pending_selection_view() {
        runner
            .execute_action(view.selecting_player, digimon_engine::action::space::PASS)
            .expect("decline optional Security trash play");
        runner.auto_resolve().ok();
    }

    assert_eq!(runner.security_count(1), 0, "LM-030 left security");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-030 must be added to hand even when trash play is declined"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "small Digimon stays in trash after declined play"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "no Digimon played when trash play was declined"
    );
}

/// §16-16-2: "The processing from <Delay> is optional." Reaching the scheduled
/// window must therefore OFFER the trash-this-card cost, not pay it, and
/// §15-7-2 means declining skips the linked effect too.
///
/// Declining must also leave the Option ON THE FIELD: §16-16-1 makes the Delay
/// available "while a card with this effect is in the battle area", and the
/// printed timing here is [Start of Your Turn] -- a window that comes round
/// every own turn -- so a decline is a pass on THIS window, not a forfeit.
#[test]
fn lm_030_delay_may_be_declined_leaving_the_option_on_the_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-030 YAML parses")
        .add_card(make_green_digimon("LM030-TRASH-GREEN", 3, 2000, 3))
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    if let Some(d) = runner
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "OPP-DIGI")
    {
        d.card_kind = CardKind::Digimon;
        d.level = Some(4);
        d.dp = Some(4000);
    }

    place_lm_030_as_start_delay(&mut runner);
    place_opp_digimon(&mut runner, "OPP-DIGI");
    push_trash(&mut runner, 0, "LM030-TRASH-GREEN");
    let trash_before = runner.trash_size(0);

    advance_to_next_p0_turn(&mut runner);

    // The FIRST prompt at the scheduled window is the optional cost itself.
    let outer = runner
        .pending_selection_view()
        .expect("the scheduled window must surface the optional <Delay> cost (rule 17)");
    assert!(
        outer.is_optional,
        "the <Delay> confirm must expose PASS (§16-16-2)"
    );

    runner
        .execute_action(outer.selecting_player, PASS)
        .expect("declining the <Delay> must be reachable from the action space");
    let _ = runner.auto_resolve();

    // (a) the carrier is NOT trashed -- the cost was never paid.
    assert!(
        runner.game.player(0).battle_area.iter().any(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == "LM-030"
        }),
        "declining must NOT trash the Option (the trash IS the unpaid cost)"
    );
    // (b) the linked effect did not resolve (§15-7-2): the green Digimon is
    //     still in the trash, not returned to the top of the deck.
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "§15-7-2: with the cost declined, the processing after it can't execute"
    );

    // (c) the Option is still schedulable -- it must not be stranded on the
    //     field forever with a window that can never match again.
    let still_delayed = runner.game.player(0).battle_area.iter().any(|permanent| {
        permanent.top_card().card_id(&runner.game.card_data) == "LM-030"
            && matches!(permanent.option_state, OptionState::Delayed { .. })
    });
    assert!(
        still_delayed,
        "the declined Option must remain a Delayed Option, not become inert"
    );
}

