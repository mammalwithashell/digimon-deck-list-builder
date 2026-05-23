//! LM-032 Purple Scramble — Option, Cost 2, Purple.
//!
//! # Card text (cards.json)
//!
//! [Main] 1 of your purple Digimon may digivolve into a purple Digimon card in
//! the hand with the digivolution cost reduced by 3. Then, place this card in
//! the battle area.
//!
//! [Start of Your Turn] If your opponent has a Digimon, <Delay> (By trashing
//! this card after the placing turn, activate the effect below.)
//! Return 1 purple Digimon card from your trash to the top of the deck. Then,
//! if you don't have a Digimon, you may play 1 purple Digimon card with 2000 DP
//! or less from your trash without paying the cost.
//!
//! Inherited: Security Effect [Security] You may play 1 purple Digimon card
//! with 2000 DP or less from your trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Purple/LM_032.cs
//!
//! # Patterns this test covers
//! - Clause A (Main): effect_initiated_digivolve with cost reduce 3 from an
//!   option card (main_from_hand timing, optional permanent selection)
//! - Clause B (Delay/StartOfYourTurn): `kind: delay` + `trigger:
//!   start_of_your_turn`, `active_when` opponent-has-Digimon gate;
//!   `move_trash_card_to_deck_top` returns the selected purple Digimon to deck
//!   top; conditional `play_from_trash_free` of a <=2000 DP purple Digimon when
//!   you control no Digimon.
//! - Clause C (Security, inherited): on_security with select_trash (purple
//!   Digimon <=2000 DP, `dp_lte` enforced for trash subjects),
//!   play_from_trash_free, add_this_option_to_hand.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::SelectionKind;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/lm/LM-032.yaml");

// --- Helper cards -----------------------------------------------------------

fn make_purple_digimon(id: &str, level: u8, dp: i32, cost: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost as u16;
    c.colors = vec![CardColor::Purple];
    c
}

/// A minimal purple Digimon with DP <= 2000 (eligible for Security/Delay trash play).
fn make_small_purple_digimon(id: &str) -> CardData {
    make_purple_digimon(id, 3, 2000, 3)
}

/// A large purple Digimon (4000 DP) — filtered out by dp_lte: 2000.
fn make_large_purple_digimon(id: &str) -> CardData {
    make_purple_digimon(id, 4, 4000, 5)
}

/// A purple Digimon with evo cost (digivolve target for Clause A).
fn make_purple_evo_target(id: &str) -> CardData {
    let mut c = make_purple_digimon(id, 4, 5000, 5);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Purple as u8,
        level: 3,
        memory_cost: 4,
    }];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn lm_032_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML must parse and compile")
        .memory(10)
        .start()
}

// ===========================================================================
// Section 1 — Structural assertions
// ===========================================================================

/// YAML must parse and compile without error.
#[test]
fn lm_032_yaml_parses_without_error() {
    let _runner = lm_032_runner();
}

/// Card metadata: Purple Option, cost 2.
#[test]
fn lm_032_is_purple_option_cost_2() {
    let runner = lm_032_runner();
    let compiled = runner.compiled_card("LM-032").expect("LM-032 compiled");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.color, vec![CompiledColor::Purple]);
    assert_eq!(compiled.cost, Some(2));
}

/// LM-032 has 3 compiled clauses: Main (main_from_hand), Delay
/// (start_of_your_turn), and Security (on_security inherited).
#[test]
fn lm_032_has_three_clauses_main_delay_and_security() {
    let runner = lm_032_runner();
    let compiled = runner.compiled_card("LM-032").expect("LM-032 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-032 ships Main, Delay, and Security clauses"
    );
}

/// No raw_rust placeholders — all clauses are native DSL.
#[test]
fn lm_032_no_raw_rust_placeholders() {
    let runner = lm_032_runner();
    let compiled = runner.compiled_card("LM-032").expect("LM-032 compiled");

    assert!(
        compiled.effects.iter().all(|clause| !matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )),
        "LM-032 must not use raw_rust placeholders"
    );
}

/// Clause A: main_from_hand triggered, optional, FaceUp scope.
#[test]
fn lm_032_main_clause_is_optional_face_up() {
    let runner = lm_032_runner();
    let compiled = runner.compiled_card("LM-032").expect("LM-032 compiled");

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
        "printed Main text says the purple Digimon may digivolve"
    );
}

/// Clause A contains an EffectInitiatedDigivolve step.
#[test]
fn lm_032_main_clause_contains_effect_initiated_digivolve_step() {
    let runner = lm_032_runner();
    let compiled = runner.compiled_card("LM-032").expect("LM-032 compiled");

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

/// Clause C (Security): on_security inherited, optional.
#[test]
fn lm_032_security_clause_is_inherited_optional() {
    let runner = lm_032_runner();
    let compiled = runner.compiled_card("LM-032").expect("LM-032 compiled");

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

// ===========================================================================
// Section 2 — Clause A: [Main] digivolve with cost -3
// ===========================================================================

/// When P0 has a purple Digimon on field and a purple Digimon in hand,
/// activating the [Main] effect installs a pending selection prompt for the
/// digivolve source (optional — "may").
#[test]
fn lm_032_main_installs_selection_when_eligible_purple_digimon_on_field() {
    let src = make_purple_digimon("LM032-SRC", 3, 2000, 3);
    let evo = make_purple_evo_target("LM032-EVO");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(src.clone())
        .add_card(evo.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-032", "LM032-EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["LM032-SRC"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let _field_handle = runner.place_on_field(0, "LM032-SRC", None);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for LM-032 at hand index 0"
    );

    assert!(
        runner.game.pending_selection.is_some(),
        "LM-032 Main must install a pending selection when a purple Digimon is on P0's field"
    );
}

/// When P0 has no purple Digimon on field AND no purple Digimon in hand,
/// the Main effect completes cleanly without installing any pending selection.
#[test]
fn lm_032_main_no_selection_when_no_purple_digimon_anywhere() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-032", "FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true");

    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "LM-032 Main must produce no selection when P0 has no purple Digimon on field or in hand"
    );
}

/// Clause A: evo cost 4 reduced by 3 = 1 memory spent; evo card lands on top.
#[test]
fn lm_032_main_digivolve_applies_cost_reduction_of_3() {
    let src = make_purple_digimon("LM032-BASE", 3, 2000, 3);
    let evo = make_purple_evo_target("LM032-EVO-POS");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(src.clone())
        .add_card(evo.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-032", "LM032-EVO-POS"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let base_handle = runner.place_on_field(0, "LM032-BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));

    // Drive field selection.
    let field_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which purple Digimon digivolves");
    assert_eq!(field_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "field selection must expose PASS for the printed may"
    );
    runner
        .execute_action(field_view.selecting_player, field_view.valid_action_ids[0])
        .expect("select purple field Digimon");

    // Drive hand selection.
    let hand_view = runner
        .pending_selection_view()
        .expect("Main effect must ask which purple hand card to digivolve into");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("select purple hand Digimon");
    runner.auto_resolve().expect("finish digivolve");

    // evo_cost 4 reduced by 3 = 1 memory spent; starting at 10, end at 9.
    assert_eq!(
        runner.memory(),
        9,
        "evo cost 4 reduced by 3 should pay 1 memory"
    );
    // The evo card must be on top of the stack.
    let evolved = &runner.game.player(0).battle_area[base_handle.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "LM032-EVO-POS"
    );
}

/// Declining the optional field selection leaves field and hand unchanged.
#[test]
fn lm_032_main_decline_leaves_field_and_hand_unchanged() {
    let src = make_purple_digimon("LM032-BASE-DECLINE", 3, 2000, 3);
    let evo = make_purple_evo_target("LM032-EVO-DECLINE");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(src.clone())
        .add_card(evo.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-032", "LM032-EVO-DECLINE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    runner.place_on_field(0, "LM032-BASE-DECLINE", Some(0));
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

// ===========================================================================
// Section 3 — Clause B: Delay ([Start of Your Turn])
// ===========================================================================

/// Place LM-032 on P0's field as a Delay-Option whose `StartOfYourNextTurn`
/// trigger matures at the start of P0's next turn.
fn place_lm_032_as_start_delay(runner: &mut DebugRunner) {
    let handle = runner.place_on_field(0, "LM-032", Some(0));
    let fire_turn = runner.game.turn_count + 2;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: fire_turn,
            trigger: DelayTrigger::StartOfYourNextTurn,
            placed_on_turn: runner.game.turn_count,
        };
}

/// Advance from P0's turn to the start of P0's next turn (P0 -> P1 -> P0).
fn advance_to_next_p0_turn(runner: &mut DebugRunner) {
    runner.end_turn(); // P0 -> P1
    runner.end_turn(); // P1 -> P0 (start fires matured delays)
}

/// Push a card by id into player `p`'s trash.
fn push_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_trash: unknown card_id {card_id}"));
    let card = digimon_engine::card_source::CardSource::new(
        data_idx,
        p,
        runner.game.next_card_index(),
    );
    runner.game.players[p as usize].trash.push(card);
}

/// Place an opponent (P1) Digimon on the field.
fn place_opp_digimon(runner: &mut DebugRunner, card_id: &str) {
    runner.place_on_field(1, card_id, None);
}

/// The Delay fires at the start of P0's turn when the opponent has a Digimon:
/// it installs the mandatory trash selection.
#[test]
fn lm_032_delay_fires_at_start_of_your_turn_when_opponent_has_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(make_purple_digimon("LM032-TRASH-PURPLE", 3, 2000, 3))
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

    place_lm_032_as_start_delay(&mut runner);
    place_opp_digimon(&mut runner, "OPP-DIGI");
    push_trash(&mut runner, 0, "LM032-TRASH-PURPLE");

    advance_to_next_p0_turn(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("Delay must install the mandatory purple-Digimon trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        !runner.pending_is_optional(),
        "step 1 of the Delay body is a mandatory selection (no PASS)"
    );
}

/// The Delay must NOT fire when the opponent controls no Digimon.
#[test]
fn lm_032_delay_does_not_fire_when_opponent_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(make_purple_digimon("LM032-TRASH-PURPLE", 3, 2000, 3))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    place_lm_032_as_start_delay(&mut runner);
    push_trash(&mut runner, 0, "LM032-TRASH-PURPLE");
    // No opponent Digimon on the field.

    advance_to_next_p0_turn(&mut runner);

    assert!(
        runner.game.pending_selection.is_none(),
        "Delay must not fire when opponent has no Digimon"
    );
}

/// Delay body step 1: selected purple Digimon moves from trash to deck top;
/// the turn-start draw then pulls it into hand, proving deck-top placement.
/// The Delay cost trashes LM-032 itself.
#[test]
fn lm_032_delay_body_returns_purple_digimon_to_top_of_deck() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(make_purple_digimon("LM032-RET-PURPLE", 3, 2000, 3))
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

    place_lm_032_as_start_delay(&mut runner);
    place_opp_digimon(&mut runner, "OPP-DIGI");
    push_trash(&mut runner, 0, "LM032-RET-PURPLE");

    // P0 has a Digimon on the field so the conditional play branch does NOT fire.
    runner.place_on_field(0, "OPP-DIGI", None);

    advance_to_next_p0_turn(&mut runner);

    // Pick the purple Digimon in the mandatory trash selection.
    let view = runner
        .pending_selection_view()
        .expect("Delay must install the trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select purple Digimon from trash");
    runner.auto_resolve().expect("finish Delay body");

    // The purple Digimon must have left the trash.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "LM032-RET-PURPLE"),
        "the returned purple Digimon must leave the trash"
    );
    // It was placed on the deck TOP and then drawn by the turn-start draw.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM032-RET-PURPLE"),
        "the returned purple Digimon must be on the deck TOP — proven by the \
         turn-start draw pulling it into hand"
    );
    // The Delay cost trashes LM-032 itself.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM-032"),
        "LM-032 must be trashed as the <Delay> cost"
    );
}

/// Delay body step 2: when P0 controls NO Digimon, the conditional free-play
/// branch fires.
#[test]
fn lm_032_delay_body_play_from_trash_only_when_no_field_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(make_purple_digimon("LM032-RET-PURPLE", 3, 2000, 3))
        .add_card(make_small_purple_digimon("LM032-PLAY-PURPLE"))
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

    place_lm_032_as_start_delay(&mut runner);
    place_opp_digimon(&mut runner, "OPP-DIGI");
    push_trash(&mut runner, 0, "LM032-RET-PURPLE");
    push_trash(&mut runner, 0, "LM032-PLAY-PURPLE");
    // P0 controls no Digimon (only the LM-032 Delay-Option permanent).

    advance_to_next_p0_turn(&mut runner);

    // Step 1: mandatory trash selection — return a purple Digimon to deck top.
    let view = runner
        .pending_selection_view()
        .expect("Delay step 1 trash selection");
    assert_eq!(view.kind, SelectionKind::Trash);
    let ret_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|_| true)
        .expect("a trash card to return");
    runner
        .execute_action(view.selecting_player, ret_action)
        .expect("return a purple Digimon to deck top");

    // Step 2: optional play selection — a <=2000 DP purple Digimon from trash.
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
        .expect("play a purple Digimon from trash");
    runner.auto_resolve().expect("finish Delay body");

    // A purple Digimon must have been played from trash onto P0's field.
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| {
                let id = p.top_card().card_id(&runner.game.card_data);
                id == "LM032-PLAY-PURPLE" || id == "LM032-RET-PURPLE"
            }),
        "the conditional branch must play a <=2000 DP purple Digimon from trash"
    );
}

// ===========================================================================
// Section 4 — Clause C: [Security] (Inherited)
// ===========================================================================

/// Security clause fires without panic when the engine triggers it with empty
/// trash (the optional play selection has no eligible targets).
#[test]
fn lm_032_security_clause_no_panic_with_empty_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let field_handle = runner.place_on_field(0, "LM-032", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // Drain any pending selections.
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

/// When the defender's trash has a small purple Digimon, a real attack on the
/// defender's security triggers LM-032's [Security] clause: the small Digimon
/// is played from trash and LM-032 is added to the defender's hand.
#[test]
fn lm_032_security_installs_trash_selection_when_purple_digimon_in_trash() {
    let small = make_small_purple_digimon("LM032-SMALL-PURPLE");
    let mut attacker = make_filler("LM032-ATK-PURPLE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(small.clone())
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM032-SMALL-PURPLE"])
        .security(1, &["LM-032"])
        .start();

    // Seed the defender's trash with a small purple Digimon.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("small purple seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM032-ATK-PURPLE", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(runner.security_count(1), 1, "precondition: LM-032 in security");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "LM-032 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-032 must be routed to the defender's hand by the mandatory tail"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(hand_id, "LM-032");
}

/// When the defender's trash is empty, the mandatory tail still fires — LM-032
/// leaves security and goes to hand.
#[test]
fn lm_032_security_no_selection_when_trash_is_empty() {
    let mut attacker = make_filler("LM032-ATK-EMPTY");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["LM-032"])
        .start();

    let attacker_handle = runner.place_on_field(0, "LM032-ATK-EMPTY", Some(0));
    assert_eq!(runner.trash_size(1), 0, "precondition: defender trash empty");
    assert_eq!(runner.security_count(1), 1, "precondition: LM-032 in security");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.battle_area_size(1), 0, "no purple Digimon played from empty trash");
    assert_eq!(runner.security_count(1), 0, "LM-032 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-032 mandatory tail must add it to defender's hand with empty trash"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(hand_id, "LM-032");
}

/// dp_lte: 2000 filter — a large purple Digimon (>2000 DP) in trash must NOT
/// be played; the mandatory tail still fires.
#[test]
fn lm_032_security_no_selection_when_only_large_purple_digimon_in_trash() {
    let large = make_large_purple_digimon("LM032-LARGE-PURPLE");
    let mut attacker = make_filler("LM032-ATK-LARGE");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(large.clone())
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM032-LARGE-PURPLE"])
        .security(1, &["LM-032"])
        .start();

    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("large purple seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM032-ATK-LARGE", Some(0));
    assert_eq!(runner.security_count(1), 1, "precondition: LM-032 in security");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "LM-032 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-032 mandatory tail must add it to defender's hand"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "LM-032 Security must not play a >2000 DP Digimon (dp_lte: 2000)"
    );
    assert_eq!(runner.trash_size(1), 1, "large purple Digimon must remain in trash");
}

/// add_this_option_to_hand must still fire when the player DECLINES the optional
/// trash play.
#[test]
fn lm_032_security_adds_card_to_hand_even_when_trash_play_declined() {
    let small = make_small_purple_digimon("LM032-SMALL-DECL");
    let mut attacker = make_filler("LM032-ATTACKER-DECL");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(small)
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM032-SMALL-DECL"])
        .security(1, &["LM-032"])
        .start();

    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("small purple seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM032-ATTACKER-DECL", Some(0));

    let _ = runner.attack_player(attacker_handle, 1, false);

    // Decline the optional trash selection.
    if let Some(view) = runner.pending_selection_view() {
        runner
            .execute_action(view.selecting_player, digimon_engine::action::space::PASS)
            .expect("decline optional Security trash play");
        runner.auto_resolve().ok();
    }

    assert_eq!(runner.security_count(1), 0, "LM-032 left security");
    assert_eq!(
        runner.hand_size(1),
        1,
        "LM-032 must be added to hand even when trash play is declined"
    );
    assert_eq!(runner.trash_size(1), 1, "small Digimon stays in trash");
    assert_eq!(runner.battle_area_size(1), 0, "no Digimon played");
}

/// Full security play: small purple Digimon played from trash; LM-032 added to
/// the defender's hand.
#[test]
fn lm_032_security_adds_card_to_hand_and_plays_small_purple_digimon() {
    let small = make_small_purple_digimon("LM032-SMALL-SEC");
    let mut attacker = make_filler("LM032-ATTACKER");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("LM-032 YAML parses")
        .add_card(small)
        .add_card(attacker)
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["LM032-SMALL-SEC"])
        .security(1, &["LM-032"])
        .start();

    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("small purple seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "LM032-ATTACKER", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "LM-032 left security");
    assert_eq!(runner.hand_size(1), 1, "LM-032 moved to defender hand");
    assert_eq!(runner.trash_size(1), 0, "small purple Digimon left trash");
    assert_eq!(runner.battle_area_size(1), 1, "small purple Digimon played");
    let played_id = runner.game.players[1].battle_area[0].card_sources[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(played_id, "LM032-SMALL-SEC");
}
