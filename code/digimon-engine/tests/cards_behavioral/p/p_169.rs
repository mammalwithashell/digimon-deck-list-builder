//! P-169 Close — Tamer, Black, Cost 4. Traits: [LIBERATOR]
//!
//! # Card text (cards.json / authoritative printed text)
//!
//! "[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory."
//!
//! "[All Turns] When effects trash digivolution cards of any of your [Mineral]
//! or [Rock] trait Digimon, by suspending this Tamer, place 1 card with the
//! [Mineral] or [Rock] trait from your trash as any of your Digimon's bottom
//! digivolution card."
//!
//! "Security Effect [Security] Play this card without paying the cost."
//!
//! # DCGO C# reference
//! DCGO submodule uninitialized — no C# reference available.
//!
//! # Patterns this test covers
//! - B1  Start-of-main tamer with conditional memory swing
//! - B3  Trigger-on-event tamer (All Turns on_digivolution_card_trashed observer)
//! - A6  Trash-to-digi-stack (place_as_bottom_source from trash, target selection)
//! - E2  Optional with cost gating (suspend self as activation cost, EX11-065 pattern)
//! - Security play (play_from_security)
//!
//! # Implementation status
//!
//! Clause 1 (Start of Main Phase): IMPLEMENTED
//! Clause 2 (All Turns on_digivolution_card_trashed): IMPLEMENTED
//! Security: IMPLEMENTED

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::PermanentHandle;

const CARD_ID: &str = "P-169";

// --- Helpers -----------------------------------------------------------------

fn make_mineral_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Mineral"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["Mineral".to_string()];
    c.play_cost = 0;
    c
}

fn make_rock_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Rock"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["Rock".to_string()];
    c.play_cost = 0;
    c
}

fn make_source_card(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-Src"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

fn make_mineral_trash_card(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id}-MineralT"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.traits = vec!["Mineral".to_string()];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Trigger on_digivolution_card_trashed by using EffectContext::trash_card_source.
/// This mirrors the EX10-063 test pattern exactly.
fn fire_source_trash(runner: &mut DebugRunner, host: PermanentHandle, source: CardHandle) {
    let top = runner.top_card(host);
    let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
    ctx.trash_card_source(host, source);
}

// =============================================================================
// Section 1 — Structural assertions
// =============================================================================

/// P-169 must compile and surface exactly 3 triggered clauses:
/// start_of_your_main_phase, on_digivolution_card_trashed, on_security.
#[test]
fn p_169_has_exactly_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-169 compiled card present in registry");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        3,
        "P-169 must have exactly 3 triggered clauses; found {}",
        triggered.len()
    );
}

/// P-169 must have an OnDigivolutionCardTrashed triggered clause (Clause 2).
#[test]
fn p_169_has_on_digivolution_card_trashed_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-169 compiled card present in registry");

    let has_clause = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed),
        _ => false,
    });

    assert!(
        has_clause,
        "P-169 must have an OnDigivolutionCardTrashed triggered clause"
    );
}

/// The [All Turns] on_digivolution_card_trashed clause must be optional.
#[test]
fn p_169_all_turns_clause_is_optional() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-169 compiled card present in registry");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OnDigivolutionCardTrashed clause must exist");

    assert!(
        clause.optional,
        "All Turns clause must be optional (printed 'by suspending this Tamer')"
    );
}

/// The [All Turns] clause must have FaceUp scope (tamer face-up effect).
#[test]
fn p_169_all_turns_clause_is_face_up_scope() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-169 compiled card present in registry");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OnDigivolutionCardTrashed clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "All Turns clause on a Tamer must be FaceUp scope"
    );
}

/// P-169 must have an OnSecurity clause.
#[test]
fn p_169_has_on_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("P-169 compiled card present in registry");

    assert!(
        compiled.effects.iter().any(|c| match c {
            CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnSecurity),
            _ => false,
        }),
        "P-169 must have an OnSecurity clause"
    );
}

// =============================================================================
// Section 2 — Clause 1 (Start of Main Phase)
// =============================================================================

#[test]
fn p_169_start_main_gains_one_memory_if_opponent_has_digimon() {
    let filler = make_filler("P-169-FILL");
    let mut opponent = make_test_card("P-169-OPP", "Opponent");
    opponent.play_cost = 3;
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses and compiles")
        .add_card(filler)
        .add_card(opponent)
        .deck(0, &["P-169-FILL"])
        .deck(1, &["P-169-FILL"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(1, "P-169-OPP", None);
    runner.game.memory = 0;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 1);
}

// =============================================================================
// Section 3 — Clause 2 (All Turns on_digivolution_card_trashed) behavioral
// =============================================================================

/// Positive: trashing a source from a Mineral Digimon with P-169 unsuspended
/// must install an optional activation prompt.
#[test]
fn p_169_all_turns_triggers_when_source_trashed_from_mineral_digimon() {
    let mineral = make_mineral_digimon("MIN-TRIG");
    let source = make_source_card("SRC-TRIG");
    let trash_card = make_mineral_trash_card("MTRC");
    let filler = make_filler("FLTRIG");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(mineral)
        .add_card(source)
        .add_card(trash_card)
        .add_card(filler)
        .deck(0, &["FLTRIG", "FLTRIG"])
        .deck(1, &["FLTRIG", "FLTRIG"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, None);
    let host = runner.place_on_field(0, "MIN-TRIG", None);
    let src_handle = runner.push_source(host, "SRC-TRIG");

    // Seed P0's trash with a Mineral card (needed for body steps).
    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MTRC")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    fire_source_trash(&mut runner, host, src_handle);

    assert!(
        runner.pending_selection().is_some(),
        "optional activation prompt must install when Mineral source trashed and P-169 is unsuspended"
    );
}

/// Positive: Rock trait also triggers the [All Turns] observer.
#[test]
fn p_169_all_turns_triggers_for_rock_trait_digimon() {
    let rock = make_rock_digimon("ROCK-TRIG");
    let source = make_source_card("SRCK");
    let mut trash_card = make_test_card("RKTR", "RockT");
    trash_card.card_kind = CardKind::Digimon;
    trash_card.level = Some(3);
    trash_card.traits = vec!["Rock".to_string()];
    let filler = make_filler("FLRK");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(rock)
        .add_card(source)
        .add_card(trash_card)
        .add_card(filler)
        .deck(0, &["FLRK"])
        .deck(1, &["FLRK"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, None);
    let host = runner.place_on_field(0, "ROCK-TRIG", None);
    let src_handle = runner.push_source(host, "SRCK");

    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "RKTR")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    fire_source_trash(&mut runner, host, src_handle);

    assert!(
        runner.pending_selection().is_some(),
        "Rock trait alone must trigger the All Turns observer"
    );
}

/// Negative: trashing a source from a non-Mineral/non-Rock Digimon must NOT
/// trigger the [All Turns] observer.
#[test]
fn p_169_all_turns_does_not_trigger_for_non_mineral_rock_digimon() {
    let mut beast = make_test_card("BEAST-NT", "BeastD");
    beast.card_kind = CardKind::Digimon;
    beast.level = Some(4);
    beast.dp = Some(4000);
    beast.traits = vec!["Beast".to_string()];
    beast.play_cost = 0;

    let source = make_source_card("SRC-BST");
    let filler = make_filler("FL-BST");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(beast)
        .add_card(source)
        .add_card(filler)
        .deck(0, &["FL-BST"])
        .deck(1, &["FL-BST"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, None);
    let host = runner.place_on_field(0, "BEAST-NT", None);
    let src_handle = runner.push_source(host, "SRC-BST");

    fire_source_trash(&mut runner, host, src_handle);

    assert!(
        runner.pending_selection().is_none(),
        "no activation prompt when source trashed from non-Mineral/Rock Digimon"
    );
}

/// Negative: when P-169 is already suspended, the condition gate prevents
/// the observer from firing.
#[test]
fn p_169_all_turns_does_not_trigger_when_close_already_suspended() {
    let mineral = make_mineral_digimon("MIN-SUSP");
    let source = make_source_card("SRC-SUS");
    let filler = make_filler("FL-SUS");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(mineral)
        .add_card(source)
        .add_card(filler)
        .deck(0, &["FL-SUS"])
        .deck(1, &["FL-SUS"])
        .memory(10)
        .start();

    let close_perm = runner.place_on_field(0, CARD_ID, None);

    // Pre-suspend P-169.
    runner.game.players[0]
        .battle_area
        .get_mut(close_perm.index as usize)
        .unwrap()
        .is_suspended = true;

    let host = runner.place_on_field(0, "MIN-SUSP", None);
    let src_handle = runner.push_source(host, "SRC-SUS");

    fire_source_trash(&mut runner, host, src_handle);

    assert!(
        runner.pending_selection().is_none(),
        "no prompt when P-169 is already suspended"
    );
}

/// Declining the optional activation prompt must NOT suspend P-169.
#[test]
fn p_169_all_turns_declining_does_not_suspend_close() {
    let mineral = make_mineral_digimon("MIN-DCL");
    let source = make_source_card("SRC-DCL");
    let trash_card = make_mineral_trash_card("MTD");
    let filler = make_filler("FLDCL");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(mineral)
        .add_card(source)
        .add_card(trash_card)
        .add_card(filler)
        .deck(0, &["FLDCL"])
        .deck(1, &["FLDCL"])
        .memory(10)
        .start();

    let close_perm = runner.place_on_field(0, CARD_ID, None);
    let host = runner.place_on_field(0, "MIN-DCL", None);
    let src_handle = runner.push_source(host, "SRC-DCL");

    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MTD")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    fire_source_trash(&mut runner, host, src_handle);

    assert!(runner.pending_selection().is_some(), "prompt must install");

    // Decline via PASS.
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .ok();

    let close_now = runner
        .game
        .player(0)
        .battle_area
        .get(close_perm.index as usize)
        .unwrap();
    assert!(
        !close_now.is_suspended,
        "P-169 must NOT be suspended when the player declines"
    );
}

/// Accepting the activation must suspend P-169.
#[test]
fn p_169_all_turns_accepting_suspends_close() {
    let mineral = make_mineral_digimon("MIN-ACP");
    let source = make_source_card("SRC-ACP");
    let trash_card = make_mineral_trash_card("MTA");
    let dest_digi = make_mineral_digimon("DEST-ACP");
    let filler = make_filler("FLACP");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(mineral)
        .add_card(source)
        .add_card(trash_card)
        .add_card(dest_digi)
        .add_card(filler)
        .deck(0, &["FLACP"])
        .deck(1, &["FLACP"])
        .memory(10)
        .start();

    let close_perm = runner.place_on_field(0, CARD_ID, None);
    let host = runner.place_on_field(0, "MIN-ACP", None);
    let src_handle = runner.push_source(host, "SRC-ACP");
    runner.place_on_field(0, "DEST-ACP", None);

    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MTA")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    fire_source_trash(&mut runner, host, src_handle);

    assert!(
        runner.pending_selection().is_some(),
        "activation prompt must install"
    );

    // Accept the activation (first valid action = accept Close).
    let act = {
        let s = runner.pending_selection().unwrap();
        s.valid_action_ids[0]
    };
    runner.execute_action(0, act).ok();
    // Resolve trash-card pick and target-Digimon pick automatically.
    runner.auto_resolve();

    let close_now = runner
        .game
        .player(0)
        .battle_area
        .get(close_perm.index as usize)
        .unwrap();
    assert!(
        close_now.is_suspended,
        "P-169 must be suspended after activation is accepted"
    );
}

/// After accepting and auto-resolving, the chosen Digimon's source count must
/// increase by 1 and the trash card must leave P0's trash.
#[test]
fn p_169_all_turns_placing_from_trash_increases_target_source_count() {
    let mineral = make_mineral_digimon("MIN-PLC");
    let source = make_source_card("SRC-PLC");
    let trash_card = make_mineral_trash_card("MTP");
    let dest_digi = make_mineral_digimon("DEST-PLC");
    let filler = make_filler("FLPLC");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(mineral)
        .add_card(source)
        .add_card(trash_card)
        .add_card(dest_digi)
        .add_card(filler)
        .deck(0, &["FLPLC"])
        .deck(1, &["FLPLC"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, None);
    let host = runner.place_on_field(0, "MIN-PLC", None);
    let src_handle = runner.push_source(host, "SRC-PLC");
    let dest_perm = runner.place_on_field(0, "DEST-PLC", None);

    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(dest_perm.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MTP")
            .unwrap();
        let ci = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(CardSource::new(idx, 0, ci));
    }

    fire_source_trash(&mut runner, host, src_handle);

    // Measure trash size AFTER fire_source_trash so SRC-PLC is already in trash.
    // After effect resolves, MTP (Mineral) leaves trash → trash_after < trash_before.
    let trash_before = runner.trash_size(0);

    assert!(
        runner.pending_selection().is_some(),
        "activation prompt must fire"
    );

    // Step 1: Accept Close (pick P-169 as the suspend-cost confirmation).
    let act = {
        let s = runner.pending_selection().unwrap();
        s.valid_action_ids[0]
    };
    runner.execute_action(0, act).ok();

    // Step 2: select_trash — pick the single Mineral card from P0's trash.
    let trash_act = {
        let s = runner
            .pending_selection()
            .expect("select_trash must install after Close is accepted");
        s.valid_action_ids[0]
    };
    runner.execute_action(0, trash_act).ok();

    // Step 3: select_own_permanent (target_digi) — pick DEST-PLC explicitly.
    // Field layout: [0]=P-169(Tamer, excluded), [1]=MIN-PLC(Digimon), [2]=DEST-PLC(Digimon).
    // encode_attack(0, field_index): valid_action_ids = [101=MIN-PLC, 102=DEST-PLC].
    // Pick index 1 (second Digimon = DEST-PLC) to verify any-target placement.
    let target_act = {
        let s = runner
            .pending_selection()
            .expect("select_own_permanent for target_digi must install after trash pick");
        *s.valid_action_ids
            .get(1)
            .expect("at least 2 own Digimon must be available for target selection")
    };
    runner.execute_action(0, target_act).ok();

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data).eq("DEST-PLC"))
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(
        sources_after > sources_before,
        "target Digimon source count must increase (before: {sources_before}, after: {sources_after})"
    );

    let trash_after = runner.trash_size(0);
    assert!(
        trash_after < trash_before,
        "the placed card must leave P0's trash (before: {trash_before}, after: {trash_after})"
    );
}

// =============================================================================
// Section 4 — Security clause
// =============================================================================

/// Security: when P-169 is in P1's security stack and P0 attacks, P-169
/// must trigger its [Security] effect and play itself onto P1's field.
#[test]
fn p_169_security_plays_itself_without_cost() {
    let mut attacker = make_test_card("ATK169", "BigAttacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(5);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let filler = make_filler("FLSEC169");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses + compiles")
        .add_card(attacker)
        .add_card(filler)
        .deck(0, &["FLSEC169"])
        .deck(1, &["FLSEC169"])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();

    let atk_perm = runner.place_on_field(0, "ATK169", Some(0));

    let p1_field_before = runner.battle_area_size(1);
    assert_eq!(runner.security_count(1), 1, "pre: P1 has 1 security card");

    runner.attack_player(atk_perm, 1, false);

    assert_eq!(
        runner.battle_area_size(1),
        p1_field_before + 1,
        "P-169 must land on P1's field after the security effect fires"
    );

    let close_on_field = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        close_on_field,
        "P-169 must be the card placed on P1's field via security effect"
    );

    assert_eq!(
        runner.security_count(1),
        0,
        "security stack must be empty after P-169 plays from security"
    );
}

// =============================================================================
// Section 5 — Compilation smoke test
// =============================================================================

#[test]
fn p_169_yaml_compiles() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-169 YAML parses and compiles")
        .start();

    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "P-169 must be present in the embedded DSL pack"
    );
}
