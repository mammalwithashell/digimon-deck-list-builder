//! EX10-033 Pyramidimon — Digimon, Lv.6, Black, DP 11000, Cost 11.
//! Traits: [Mineral, LIBERATOR]
//!
//! # Card text (data/cards.json — verbatim)
//!
//! ＜Fragment (3)＞ (When this Digimon would be deleted, by trashing any 3 of
//! its digivolution cards, it isn't deleted.)
//!
//! [When Digivolving] [When Attacking] [Once Per Turn]
//! You may place up to 3 [Mineral] or [Rock] trait cards from your trash as
//! this Digimon's bottom digivolution cards.
//!
//! [When Digivolving] [When Attacking]
//! By trashing up to 3 [Mineral] or [Rock] trait cards from any of your
//! Digimon's digivolution cards, to 1 of your opponent's Digimon, reduce
//! the play cost by 2 until their turn ends for each card trashed.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_033.cs
//!
//! # Patterns this test covers
//! - H16 Fragment (3) keyword (printed, declarative)
//! - A6 Trash→digi-stack (place_as_bottom_source from trash, target: source)
//! - E2 OPT + optional decline — Clause A is [Once Per Turn] + optional
//! - Up-to-3 multi-pick from trash (sequential optional picks)
//! - Multi-timing: both when_digivolving and when_attacking for both clauses
//! - Clause B: select_own_sources (up-to-3) + trash_selected_sources +
//!   select_opponent_permanent + per_selected add_modifier ChangePlayCost -2 each
//!
//! # TriggerOrder note
//! EX10-033 has two triggered clauses on the same timing (when_digivolving and
//! when_attacking). When both fire simultaneously, the engine installs a
//! TriggerOrder selection so the controller can sequence them. Test loops must
//! handle TriggerOrder by picking the first valid action (corresponding to
//! Clause A since it is defined first in the YAML). Pattern follows ex10_036.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS, TRASH_EFFECT_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 YAML parses and compiles")
        .build()
}

/// Resolve a TriggerOrder prompt by picking the first valid action.
/// EX10-033 has two clauses on the same timing; when both fire the engine
/// presents this ordering prompt. Returns true if a TriggerOrder was consumed.
fn drain_trigger_order_if_any(runner: &mut DebugRunner) -> bool {
    if matches!(runner.pending_kind(), Some(SelectionKind::TriggerOrder)) {
        let view = runner
            .pending_selection_view()
            .expect("TriggerOrder view present");
        let player = view.selecting_player;
        let action = view.valid_action_ids[0];
        runner
            .game
            .resolve_selection(player, action)
            .expect("TriggerOrder resolves");
        true
    } else {
        false
    }
}

/// Push a card already registered in card_data into player 0's trash.
fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered in card_data"));
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(CardSource::new(data_idx, 0, next));
}

fn make_mineral_trash(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Mineral"));
    c.traits.push("Mineral".to_string());
    c.level = Some(3);
    c
}

fn make_rock_trash(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Rock"));
    c.traits.push("Rock".to_string());
    c.level = Some(3);
    c
}

fn make_mineral_source_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} MineralSrc"));
    c.traits.push("Mineral".to_string());
    c.level = Some(4);
    c
}

fn make_rock_source_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} RockSrc"));
    c.traits.push("Rock".to_string());
    c.level = Some(4);
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// Fragment(3) keyword is present on the compiled card.
#[test]
fn ex10_033_on_field_has_fragment_three() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX10-033", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}

/// Clause A ([WD][WA][OPT]) is compiled as a triggered clause with correct
/// timings, once_per_turn, and optional flags.
#[test]
fn ex10_033_clause_a_has_wd_wa_opt_optional() {
    let runner = runner();
    let card = runner
        .compiled_card("EX10-033")
        .expect("EX10-033 compiled card must be present");

    let clause_a = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
                && t.once_per_turn
        })
        .expect("Clause A ([WD][WA][OPT]) must exist");

    assert!(clause_a.optional, "Clause A must be optional ('You may')");
    assert!(clause_a.once_per_turn, "Clause A must be OPT");
    assert_eq!(
        clause_a.scope,
        CompiledScope::FaceUp,
        "Clause A must be FaceUp (own) scope"
    );
}

/// Clause B ([WD][WA]) is compiled as a triggered clause without OPT flag.
#[test]
fn ex10_033_clause_b_has_wd_wa_no_opt() {
    let runner = runner();
    let card = runner
        .compiled_card("EX10-033")
        .expect("EX10-033 compiled card must be present");

    let clause_b = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
                && !t.once_per_turn
        })
        .expect("Clause B ([WD][WA], no OPT) must exist");

    assert!(!clause_b.once_per_turn, "Clause B must NOT be OPT");
    assert!(!clause_b.optional, "Clause B is not 'You may' (cost-by prefix)");
    assert_eq!(
        clause_b.scope,
        CompiledScope::FaceUp,
        "Clause B must be FaceUp (own) scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [WD][WA][OPT] place up to 3 from trash
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause A: placing 1 card increases this Digimon's sources by 1.
#[test]
fn ex10_033_clause_a_placing_one_card_grows_sources_by_one() {
    let min1 = make_mineral_trash("MIN-TR-ONE");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(min1)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    push_to_trash(&mut runner, "MIN-TR-ONE");

    let sources_before = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .card_sources
        .len();

    // Fire WhenDigivolving — both clauses trigger. TriggerOrder fires first.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    // Clause A fires first — pick 1 Mineral card, then PASS on remaining picks.
    // After Clause A, Clause B fires (select_own_sources with no sources → passes 0).
    let mut picked_one = false;
    let mut iterations = 0;
    loop {
        // Always drain TriggerOrder first.
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                if !picked_one {
                    runner.execute_action(0, TRASH_EFFECT_START).expect("pick first trash card");
                    picked_one = true;
                } else {
                    runner.execute_action(0, PASS).expect("PASS on 2nd trash pick");
                }
            }
            Some(SelectionKind::SourceMulti { .. }) => {
                // Clause B — PASS to skip (no sources available here)
                runner.execute_action(0, PASS).expect("PASS on Clause B SourceMulti");
            }
            Some(SelectionKind::OppField) | Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("field selection resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 15 {
            break;
        }
    }

    let sources_after = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .card_sources
        .len();

    assert_eq!(
        sources_after,
        sources_before + 1,
        "placing 1 card from trash must grow sources by 1 (before: {sources_before}, after: {sources_after})"
    );
    assert_eq!(runner.trash_size(0), 0, "placed card must leave trash");
}

/// Clause A: placing all 3 cards grows sources by 3.
#[test]
fn ex10_033_clause_a_placing_three_cards_grows_sources_by_three() {
    let min1 = make_mineral_trash("MIN-TR-3A");
    let rock1 = make_rock_trash("ROCK-TR-3B");
    let min2 = make_mineral_trash("MIN-TR-3C");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(min1)
        .add_card(rock1)
        .add_card(min2)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    push_to_trash(&mut runner, "MIN-TR-3A");
    push_to_trash(&mut runner, "ROCK-TR-3B");
    push_to_trash(&mut runner, "MIN-TR-3C");

    let sources_before = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .card_sources
        .len();

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    // Pick all 3, then handle Clause B by passing.
    let mut trash_picks = 0;
    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                if trash_picks < 3 {
                    runner.execute_action(0, TRASH_EFFECT_START).expect("pick trash card");
                    trash_picks += 1;
                } else {
                    runner.execute_action(0, PASS).expect("PASS after 3 picks");
                }
            }
            Some(SelectionKind::SourceMulti { .. }) => {
                runner.execute_action(0, PASS).expect("PASS Clause B SourceMulti");
            }
            Some(SelectionKind::OppField) | Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("field selection resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 20 {
            break;
        }
    }

    let sources_after = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .card_sources
        .len();

    assert_eq!(
        sources_after,
        sources_before + 3,
        "placing 3 cards from trash must grow sources by 3 (before: {sources_before}, after: {sources_after})"
    );
    assert_eq!(runner.trash_size(0), 0, "all 3 trash cards must be placed");
}

/// Clause A: OPT lockout — second WhenAttacking in the same turn must not
/// re-install Clause A.
#[test]
fn ex10_033_clause_a_opt_blocks_second_when_attacking_same_turn() {
    let min1 = make_mineral_trash("MIN-TR-OPT1");
    let min2 = make_mineral_trash("MIN-TR-OPT2");
    let opp_digi = make_test_card("OPP-DIGI-OPT", "Opp Digi");
    let p1_filler = make_test_card("P1-FILL-OPT", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(min1)
        .add_card(min2)
        .add_card(opp_digi)
        .add_card(p1_filler)
        .deck(1, &["P1-FILL-OPT", "P1-FILL-OPT", "P1-FILL-OPT"])
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    let opp = runner.place_on_field(1, "OPP-DIGI-OPT", None);
    push_to_trash(&mut runner, "MIN-TR-OPT1");
    push_to_trash(&mut runner, "MIN-TR-OPT2");

    // First attack — fire WhenAttacking. Drain all prompts.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                // Take first pick.
                runner.execute_action(0, TRASH_EFFECT_START).expect("pick first trash");
                // PASS subsequent picks.
                if let Some(SelectionKind::Trash) = runner.pending_kind() {
                    runner.execute_action(0, PASS).expect("pass second pick");
                }
                if let Some(SelectionKind::Trash) = runner.pending_kind() {
                    runner.execute_action(0, PASS).expect("pass third pick");
                }
            }
            Some(SelectionKind::SourceMulti { .. }) => {
                runner.execute_action(0, PASS).expect("PASS Clause B cost");
            }
            Some(SelectionKind::OppField) => {
                runner
                    .execute_action(0, encode_attack(0, opp.index as u16))
                    .expect("pick opp Digimon for cost reduction");
            }
            Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("own field resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 25 {
            break;
        }
    }

    // Second attack same turn — Clause A OPT must be locked.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    // Clause B may still fire (no OPT). Drain it.
    let mut saw_clause_a_trash = false;
    let mut iterations2 = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations2 += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                saw_clause_a_trash = true;
                runner.execute_action(0, PASS).expect("pass trash");
            }
            Some(SelectionKind::SourceMulti { .. }) => {
                runner.execute_action(0, PASS).expect("PASS Clause B cost");
            }
            Some(SelectionKind::OppField) => {
                runner
                    .execute_action(0, encode_attack(0, opp.index as u16))
                    .expect("pick opp Digimon");
            }
            Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("own field resolves");
            }
            _ => break,
        }
        iterations2 += 1;
        if iterations2 > 20 {
            break;
        }
    }

    assert!(
        !saw_clause_a_trash,
        "Clause A must NOT fire on the second WhenAttacking in the same turn (OPT lockout)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: source-trash cost reduction
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B when_digivolving: prompts SourceMulti when Mineral/Rock source present.
#[test]
fn ex10_033_clause_b_when_digivolving_prompts_source_multi_when_mineral_source_present() {
    let src_card = make_mineral_source_card("MIN-SRC-B1");
    let carrier = {
        let mut c = make_test_card("CARRIER-B1", "Carrier B1");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(src_card)
        .add_card(carrier)
        .memory(10)
        .start();

    let carrier_handle = runner.place_on_field(0, "CARRIER-B1", None);
    runner.push_source(carrier_handle, "MIN-SRC-B1");
    let pyramidimon = runner.place_on_field(0, "EX10-033", None);

    // Fire WhenDigivolving — both clauses trigger.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    // Drain TriggerOrder + Clause A (no trash, both optional picks pass immediately),
    // then look for SourceMulti from Clause B.
    let mut found_source_multi = false;
    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                // Clause A trash pick — no Mineral/Rock in trash → optional, PASS
                runner.execute_action(0, PASS).expect("pass Clause A pick");
            }
            Some(SelectionKind::SourceMulti { .. }) => {
                found_source_multi = true;
                runner.execute_action(0, PASS).expect("pass Clause B cost");
            }
            Some(SelectionKind::OppField) | Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("field resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 20 {
            break;
        }
    }

    assert!(
        found_source_multi,
        "Clause B must prompt SourceMulti when Mineral source exists"
    );
}

/// Clause B: trashing 1 source reduces opp Digimon play cost by 2.
#[test]
fn ex10_033_clause_b_trash_one_source_reduces_play_cost_by_two() {
    let src_card = make_mineral_source_card("MIN-SRC-B2");
    let carrier = {
        let mut c = make_test_card("CARRIER-B2", "Carrier B2");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };
    let mut opp_digi = make_test_card("OPP-DIGI-B2", "Opp Digi B2");
    opp_digi.play_cost = 8;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(src_card)
        .add_card(carrier)
        .add_card(opp_digi)
        .memory(10)
        .start();

    let carrier_handle = runner.place_on_field(0, "CARRIER-B2", None);
    runner.push_source(carrier_handle, "MIN-SRC-B2");
    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    let opp = runner.place_on_field(1, "OPP-DIGI-B2", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                // Clause A — no Mineral/Rock trash, pass
                runner.execute_action(0, PASS).expect("pass Clause A pick");
            }
            Some(SelectionKind::SourceMulti { max, picked, .. }) => {
                // Select 1 source (the Mineral source on CARRIER-B2).
                if picked == 0 && max >= 1 {
                    runner.auto_resolve().expect("select 1 source");
                } else {
                    runner.execute_action(0, PASS).expect("done picking sources");
                }
            }
            Some(SelectionKind::OppField) => {
                runner
                    .execute_action(0, encode_attack(0, opp.index as u16))
                    .expect("pick opp Digimon for cost reduction");
            }
            Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("own field resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 30 {
            break;
        }
    }

    let play_cost_reduction = runner
        .modifiers()
        .sum(opp, ModifierType::ChangePlayCost);

    assert_eq!(
        play_cost_reduction, -2,
        "trashing 1 Mineral/Rock source must apply -2 to opp Digimon play cost (got: {play_cost_reduction})"
    );
}

/// Clause B: trashing 3 sources reduces opp Digimon play cost by 6 total.
#[test]
fn ex10_033_clause_b_trash_three_sources_reduces_play_cost_by_six() {
    let src1 = make_mineral_source_card("MIN-SRC-3B1");
    let src2 = make_rock_source_card("ROCK-SRC-3B2");
    let src3 = make_mineral_source_card("MIN-SRC-3B3");
    let carrier1 = {
        let mut c = make_test_card("CARRIER-3B1", "Carrier 3B1");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };
    let carrier2 = {
        let mut c = make_test_card("CARRIER-3B2", "Carrier 3B2");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };
    let mut opp_digi = make_test_card("OPP-DIGI-3B", "Opp Digi 3B");
    opp_digi.play_cost = 10;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(src1)
        .add_card(src2)
        .add_card(src3)
        .add_card(carrier1)
        .add_card(carrier2)
        .add_card(opp_digi)
        .memory(10)
        .start();

    let c1 = runner.place_on_field(0, "CARRIER-3B1", None);
    let c2 = runner.place_on_field(0, "CARRIER-3B2", None);
    runner.push_source(c1, "MIN-SRC-3B1");
    runner.push_source(c1, "ROCK-SRC-3B2");
    runner.push_source(c2, "MIN-SRC-3B3");
    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    let opp = runner.place_on_field(1, "OPP-DIGI-3B", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut source_picks = 0u8;
    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                runner.execute_action(0, PASS).expect("pass Clause A pick");
            }
            Some(SelectionKind::SourceMulti { picked, max, .. }) => {
                if picked < 3 && picked < max {
                    runner.auto_resolve().expect("pick source");
                    source_picks += 1;
                } else {
                    runner.execute_action(0, PASS).expect("done picking sources");
                }
            }
            Some(SelectionKind::OppField) => {
                runner
                    .execute_action(0, encode_attack(0, opp.index as u16))
                    .expect("pick opp Digimon for cost reduction");
            }
            Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("own field resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 40 {
            break;
        }
    }

    let _ = source_picks;

    let play_cost_reduction = runner
        .modifiers()
        .sum(opp, ModifierType::ChangePlayCost);

    assert_eq!(
        play_cost_reduction, -6,
        "trashing 3 sources must reduce opp Digimon play cost by 6 total (got: {play_cost_reduction})"
    );
}

/// Clause B: play cost modifier expires after opponent's turn ends.
#[test]
fn ex10_033_clause_b_play_cost_modifier_expires_after_opponents_turn() {
    let src_card = make_mineral_source_card("MIN-SRC-EXP");
    let carrier = {
        let mut c = make_test_card("CARRIER-EXP", "Carrier EXP");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };
    let mut opp_digi = make_test_card("OPP-DIGI-EXP", "Opp Digi EXP");
    opp_digi.play_cost = 8;
    let p1_filler = make_test_card("P1-FILL-EXP", "Filler EXP");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(src_card)
        .add_card(carrier)
        .add_card(opp_digi)
        .add_card(p1_filler)
        .deck(1, &["P1-FILL-EXP", "P1-FILL-EXP", "P1-FILL-EXP"])
        .memory(10)
        .start();

    let carrier_handle = runner.place_on_field(0, "CARRIER-EXP", None);
    runner.push_source(carrier_handle, "MIN-SRC-EXP");
    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    let opp = runner.place_on_field(1, "OPP-DIGI-EXP", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                runner.execute_action(0, PASS).expect("pass Clause A pick");
            }
            Some(SelectionKind::SourceMulti { picked, max, .. }) => {
                if picked == 0 && max >= 1 {
                    runner.auto_resolve().expect("select 1 source");
                } else {
                    runner.execute_action(0, PASS).expect("done picking sources");
                }
            }
            Some(SelectionKind::OppField) => {
                runner
                    .execute_action(0, encode_attack(0, opp.index as u16))
                    .expect("pick opp Digimon for cost reduction");
            }
            Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("own field resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 30 {
            break;
        }
    }

    assert_eq!(
        runner.modifiers().sum(opp, ModifierType::ChangePlayCost),
        -2,
        "modifier must be -2 immediately after effect"
    );

    // P0's turn ends → P1's turn.
    runner.end_turn();

    assert_eq!(
        runner.modifiers().sum(opp, ModifierType::ChangePlayCost),
        -2,
        "modifier must still be -2 during opponent's turn"
    );

    // P1's turn ends → P0's turn; modifier must expire.
    runner.end_turn();

    assert_eq!(
        runner.modifiers().sum(opp, ModifierType::ChangePlayCost),
        0,
        "modifier must expire after opponent's turn ends"
    );
}

/// Clause B: passing immediately on SourceMulti (trash 0 sources) installs
/// no cost reduction modifier.
#[test]
fn ex10_033_clause_b_passing_with_zero_sources_applies_no_modifier() {
    let src_card = make_mineral_source_card("MIN-SRC-ZERO");
    let carrier = {
        let mut c = make_test_card("CARRIER-ZERO", "Carrier Zero");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };
    let mut opp_digi = make_test_card("OPP-DIGI-ZERO", "Opp Digi Zero");
    opp_digi.play_cost = 8;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(src_card)
        .add_card(carrier)
        .add_card(opp_digi)
        .memory(10)
        .start();

    let carrier_handle = runner.place_on_field(0, "CARRIER-ZERO", None);
    runner.push_source(carrier_handle, "MIN-SRC-ZERO");
    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    let opp = runner.place_on_field(1, "OPP-DIGI-ZERO", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                runner.execute_action(0, PASS).expect("pass Clause A");
            }
            Some(SelectionKind::SourceMulti { .. }) => {
                // PASS immediately — select 0 sources.
                runner.execute_action(0, PASS).expect("pass Clause B immediately");
            }
            Some(SelectionKind::OppField) | Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("field resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 20 {
            break;
        }
    }

    assert_eq!(
        runner.modifiers().sum(opp, ModifierType::ChangePlayCost),
        0,
        "passing with 0 sources must not apply any play cost modifier"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Multi-timing coverage: when_attacking fires same bodies
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B fires on when_attacking (not just when_digivolving).
#[test]
fn ex10_033_clause_b_when_attacking_reduces_play_cost() {
    let src_card = make_mineral_source_card("MIN-SRC-WA");
    let carrier = {
        let mut c = make_test_card("CARRIER-WA", "Carrier WA");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };
    let mut opp_digi = make_test_card("OPP-DIGI-WA", "Opp Digi WA");
    opp_digi.play_cost = 8;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 parses")
        .add_card(src_card)
        .add_card(carrier)
        .add_card(opp_digi)
        .memory(10)
        .start();

    let carrier_handle = runner.place_on_field(0, "CARRIER-WA", None);
    runner.push_source(carrier_handle, "MIN-SRC-WA");
    let pyramidimon = runner.place_on_field(0, "EX10-033", None);
    let opp = runner.place_on_field(1, "OPP-DIGI-WA", None);

    // Fire WhenAttacking.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut iterations = 0;
    loop {
        if drain_trigger_order_if_any(&mut runner) {
            iterations += 1;
            continue;
        }
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                runner.execute_action(0, PASS).expect("pass Clause A pick");
            }
            Some(SelectionKind::SourceMulti { picked, max, .. }) => {
                if picked == 0 && max >= 1 {
                    runner.auto_resolve().expect("select 1 source");
                } else {
                    runner.execute_action(0, PASS).expect("done picking");
                }
            }
            Some(SelectionKind::OppField) => {
                runner
                    .execute_action(0, encode_attack(0, opp.index as u16))
                    .expect("pick opp Digimon");
            }
            Some(SelectionKind::OwnField) => {
                runner.auto_resolve().expect("own field resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 30 {
            break;
        }
    }

    let play_cost_reduction = runner
        .modifiers()
        .sum(opp, ModifierType::ChangePlayCost);

    assert!(
        play_cost_reduction <= -2,
        "WhenAttacking must also reduce opp play cost (got: {play_cost_reduction})"
    );
}
