//! EX11-044 Pyramidimon — Digimon, Black, Lv.6, DP 12000, Cost 11.
//! Traits: [Mineral, LIBERATOR]
//!
//! # Card text (data/cards.json — verbatim)
//!
//! Reboot — Fragment (3)
//!
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
//! By trashing any 3 [Mineral] or [Rock] trait cards from your Digimon's
//! digivolution cards, delete 1 of your opponent's highest play cost
//! Digimon or Tamers.
//!
//! [All Turns] [Once Per Turn]
//! When effects trash any of this Digimon's digivolution cards, you may place
//! 3 [Mineral] or [Rock] trait cards from your trash as this Digimon's bottom
//! digivolution cards.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Black/EX11_044.cs
//!
//! # Gap status
//!
//! ## Clause A: BLOCKED — G-HIGHEST-PLAY-COST-SELECTOR
//!
//! `selector: highest_play_cost` is not in `CompiledFieldSelector`. Only
//! `LowestPlayCost` exists (added for G-PLAY-COST-AGGREGATE). Adding
//! `HighestPlayCost` requires changes to FieldSelector, CompiledFieldSelector,
//! compile_field_selector(), field_value(), selected_field_extreme().
//! Tests: #[ignore = "pending: G-HIGHEST-PLAY-COST-SELECTOR"]
//!
//! ## Clause B: BLOCKED — event_host_permanent_is_source
//!
//! Without `event_host_permanent_is_source: true` in the condition, an
//! on_digivolution_card_trashed face-up clause cannot gate on "the host is
//! THIS Digimon" — it would fire on all own-Digimon source trashes, violating
//! CLAUDE.md section 17 no-approximations.
//! Documented: docs/RUST_ENGINE_GAPS.md "event_host_permanent_is_source DSL
//! predicate for OnDigivolutionCardTrashed observers" (BLOCKING, 2026-05-17).
//! Fix: add `event_host_permanent_is_source: Option<bool>` to PredicateSpec /
//! CompiledPredicate; check trigger.event_host_permanent == ctx.source_permanent.
//! Tests: #[ignore = "pending: event_host_permanent_is_source"]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 YAML parses and compiles")
        .build()
}

fn make_mineral_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} MineralSrc"));
    c.traits.push("Mineral".to_string());
    c.level = Some(3);
    c
}

fn make_rock_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} RockSrc"));
    c.traits.push("Rock".to_string());
    c.level = Some(3);
    c
}

fn make_mineral_trash_card(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} MineralTrash"));
    c.traits.push("Mineral".to_string());
    c.level = Some(3);
    c
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

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (keywords)
// ═══════════════════════════════════════════════════════════════════════════════

/// Reboot and Fragment(3) keywords must be present on the compiled card.
#[test]
fn ex11_044_on_field_has_reboot_and_fragment_three() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX11-044", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Reboot));
    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [OP][WD][WA][OPT] source-trash -> delete highest play cost
//
// BLOCKED: G-HIGHEST-PLAY-COST-SELECTOR
//
// CompiledFieldSelector has LowestPlayCost but not HighestPlayCost.
// YAML would use: selector: highest_play_cost in select_opponent_permanent.
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause A shape: [OP][WD][WA][OPT], FaceUp scope.
#[test]
#[ignore = "pending: G-HIGHEST-PLAY-COST-SELECTOR (HighestPlayCost selector not in CompiledFieldSelector)"]
fn ex11_044_clause_a_has_on_play_wd_wa_opt_shape() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-044")
        .expect("EX11-044 compiled card must be present");

    let has_clause_a = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking)
                    && t.once_per_turn
        )
    });

    assert!(
        has_clause_a,
        "Clause A ([OP][WD][WA][OPT]) must be compiled as a FaceUp triggered clause"
    );
}

/// Clause A positive: two opp Digimon at costs 8 and 12; only cost-12 offered.
#[test]
#[ignore = "pending: G-HIGHEST-PLAY-COST-SELECTOR (HighestPlayCost selector not in CompiledFieldSelector)"]
fn ex11_044_clause_a_on_play_deletes_highest_play_cost_opp_permanent() {
    let src1 = make_mineral_source("EX11-044-A-SRC1");
    let src2 = make_mineral_source("EX11-044-A-SRC2");
    let src3 = make_rock_source("EX11-044-A-SRC3");
    let mut opp_cheap = make_test_card("EX11-044-A-OPP-CHEAP", "Opp Cheap");
    opp_cheap.play_cost = 8;
    let mut opp_expensive = make_test_card("EX11-044-A-OPP-EXP", "Opp Expensive");
    opp_expensive.play_cost = 12;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(src1)
        .add_card(src2)
        .add_card(src3)
        .add_card(opp_cheap)
        .add_card(opp_expensive)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyramidimon, "EX11-044-A-SRC1");
    runner.push_source(pyramidimon, "EX11-044-A-SRC2");
    runner.push_source(pyramidimon, "EX11-044-A-SRC3");
    runner.place_on_field(1, "EX11-044-A-OPP-CHEAP", None);
    runner.place_on_field(1, "EX11-044-A-OPP-EXP", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut source_picks = 0u8;
    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::TriggerOrder) => {
                let view = runner.pending_selection_view().expect("TriggerOrder view");
                let player = view.selecting_player;
                let action = view.valid_action_ids[0];
                runner
                    .game
                    .resolve_selection(player, action)
                    .expect("TriggerOrder resolves");
            }
            Some(SelectionKind::SourceMulti { picked, max, .. }) => {
                if source_picks < 3 && picked < max {
                    runner.auto_resolve().expect("pick source");
                    source_picks += 1;
                } else {
                    runner
                        .execute_action(0, digimon_engine::action::space::PASS)
                        .expect("done picking sources");
                }
            }
            Some(SelectionKind::OppField) => {
                // Must only offer the highest-play-cost Digimon (1 candidate).
                let count = runner.pending_action_count();
                assert_eq!(
                    count, 1,
                    "Highest-play-cost delete must offer exactly 1 candidate, got {count}"
                );
                runner.auto_resolve().expect("delete opp Digimon");
                break;
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 20 {
            panic!("loop did not resolve within expected iterations");
        }
    }

    // One of the two opp Digimon (cost-12) was deleted; cost-8 survives.
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "one opp Digimon (cost-8) must survive; cost-12 is deleted"
    );
}

/// Clause A OPT lockout — second trigger same turn must NOT fire.
#[test]
#[ignore = "pending: G-HIGHEST-PLAY-COST-SELECTOR (HighestPlayCost selector not in CompiledFieldSelector)"]
fn ex11_044_clause_a_opt_blocks_second_trigger_same_turn() {
    let src1 = make_mineral_source("EX11-044-OPT-S1");
    let src2 = make_mineral_source("EX11-044-OPT-S2");
    let src3 = make_rock_source("EX11-044-OPT-S3");
    let mut opp_digi = make_test_card("EX11-044-OPT-OPP", "Opp Digi OPT");
    opp_digi.play_cost = 10;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(src1)
        .add_card(src2)
        .add_card(src3)
        .add_card(opp_digi)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyramidimon, "EX11-044-OPT-S1");
    runner.push_source(pyramidimon, "EX11-044-OPT-S2");
    runner.push_source(pyramidimon, "EX11-044-OPT-S3");
    runner.place_on_field(1, "EX11-044-OPT-OPP", None);

    // First trigger.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::TriggerOrder) => {
                let view = runner.pending_selection_view().expect("view");
                let player = view.selecting_player;
                let action = view.valid_action_ids[0];
                runner.game.resolve_selection(player, action).expect("resolves");
            }
            Some(SelectionKind::SourceMulti { picked, max, .. }) => {
                if picked < 3 && picked < max {
                    runner.auto_resolve().expect("pick source");
                } else {
                    runner.execute_action(0, digimon_engine::action::space::PASS).expect("done");
                }
            }
            Some(SelectionKind::OppField) => {
                runner.auto_resolve().expect("delete opp Digimon");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 25 {
            break;
        }
    }

    // Second trigger same turn.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut saw_source_multi = false;
    let mut iterations2 = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::SourceMulti { .. }) => {
                saw_source_multi = true;
                runner.execute_action(0, digimon_engine::action::space::PASS).expect("pass");
            }
            Some(SelectionKind::OppField) => {
                runner.auto_resolve().expect("resolves");
            }
            Some(SelectionKind::TriggerOrder) => {
                let view = runner.pending_selection_view().expect("view");
                let player = view.selecting_player;
                let action = view.valid_action_ids[0];
                runner.game.resolve_selection(player, action).expect("resolves");
            }
            _ => break,
        }
        iterations2 += 1;
        if iterations2 > 15 {
            break;
        }
    }

    assert!(
        !saw_source_multi,
        "Clause A must NOT fire on the second trigger in the same turn (OPT lockout)"
    );
}

/// Clause A multi-timing: WhenDigivolving fires the clause.
#[test]
#[ignore = "pending: G-HIGHEST-PLAY-COST-SELECTOR (HighestPlayCost selector not in CompiledFieldSelector)"]
fn ex11_044_clause_a_when_digivolving_prompts_source_multi() {
    let src1 = make_mineral_source("EX11-044-WD-S1");
    let src2 = make_mineral_source("EX11-044-WD-S2");
    let src3 = make_rock_source("EX11-044-WD-S3");
    let mut opp_digi = make_test_card("EX11-044-WD-OPP", "Opp WD");
    opp_digi.play_cost = 8;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(src1)
        .add_card(src2)
        .add_card(src3)
        .add_card(opp_digi)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyramidimon, "EX11-044-WD-S1");
    runner.push_source(pyramidimon, "EX11-044-WD-S2");
    runner.push_source(pyramidimon, "EX11-044-WD-S3");
    runner.place_on_field(1, "EX11-044-WD-OPP", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut saw_source_multi = false;
    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::TriggerOrder) => {
                let view = runner.pending_selection_view().expect("view");
                let player = view.selecting_player;
                let action = view.valid_action_ids[0];
                runner.game.resolve_selection(player, action).expect("resolves");
            }
            Some(SelectionKind::SourceMulti { .. }) => {
                saw_source_multi = true;
                runner.execute_action(0, digimon_engine::action::space::PASS).expect("pass");
            }
            Some(SelectionKind::OppField) => {
                runner.auto_resolve().expect("resolves");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 20 {
            break;
        }
    }

    assert!(
        saw_source_multi,
        "WhenDigivolving must also prompt SourceMulti for Clause A cost payment"
    );
}

/// Clause A: fewer than 3 sources -> silent skip.
#[test]
#[ignore = "pending: G-HIGHEST-PLAY-COST-SELECTOR (HighestPlayCost selector not in CompiledFieldSelector)"]
fn ex11_044_clause_a_skips_when_fewer_than_3_sources_available() {
    let src1 = make_mineral_source("EX11-044-SKIP-S1");
    let src2 = make_mineral_source("EX11-044-SKIP-S2");
    let mut opp_digi = make_test_card("EX11-044-SKIP-OPP", "Opp Skip");
    opp_digi.play_cost = 8;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(src1)
        .add_card(src2)
        .add_card(opp_digi)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyramidimon, "EX11-044-SKIP-S1");
    runner.push_source(pyramidimon, "EX11-044-SKIP-S2");
    runner.place_on_field(1, "EX11-044-SKIP-OPP", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(pyramidimon));
    runner.game.drain_effect_queue();

    let mut saw_source_multi = false;
    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::SourceMulti { .. }) => {
                saw_source_multi = true;
                runner.execute_action(0, digimon_engine::action::space::PASS).expect("pass");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 10 {
            break;
        }
    }

    assert!(
        !saw_source_multi,
        "Clause A must silently skip when fewer than 3 Mineral/Rock sources are available"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "opp Digimon must survive if cost cannot be paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: [All Turns][OPT] on_digivolution_card_trashed
//
// BLOCKED: event_host_permanent_is_source predicate missing in DSL.
// See docs/RUST_ENGINE_GAPS.md "event_host_permanent_is_source DSL predicate
// for OnDigivolutionCardTrashed observers" (BLOCKING, 2026-05-17).
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B shape: on_digivolution_card_trashed, FaceUp, once_per_turn, optional.
#[test]
#[ignore = "pending: event_host_permanent_is_source (DSL predicate for OnDigivolutionCardTrashed host-self gate)"]
fn ex11_044_clause_b_has_on_digivolution_card_trashed_opt_shape() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-044")
        .expect("EX11-044 compiled card must be present");

    let has_clause_b = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed)
                    && t.once_per_turn
                    && t.optional
        )
    });

    assert!(
        has_clause_b,
        "Clause B must be compiled as a FaceUp on_digivolution_card_trashed OPT optional clause"
    );
}

/// Clause B positive: fire with THIS Pyramidimon as host; 3 picks grow sources.
#[test]
#[ignore = "pending: event_host_permanent_is_source (DSL predicate for OnDigivolutionCardTrashed host-self gate)"]
fn ex11_044_clause_b_trash_event_on_self_triggers_place_three_from_trash() {
    let src_on_stack = make_mineral_source("EX11-044-B-STACK");
    let trash1 = make_mineral_trash_card("EX11-044-B-T1");
    let mut trash2 = make_test_card("EX11-044-B-T2", "EX11-044-B-T2 Rock");
    trash2.traits.push("Rock".to_string());
    trash2.level = Some(3);
    let trash3 = make_mineral_trash_card("EX11-044-B-T3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(src_on_stack)
        .add_card(trash1)
        .add_card(trash2)
        .add_card(trash3)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyramidimon, "EX11-044-B-STACK");

    push_to_trash(&mut runner, "EX11-044-B-T1");
    push_to_trash(&mut runner, "EX11-044-B-T2");
    push_to_trash(&mut runner, "EX11-044-B-T3");

    let sources_before = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .card_sources
        .len();
    let trash_before = runner.trash_size(0);

    let host_card = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].trash[0].handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: pyramidimon,
            host_card,
            card: trashed_card,
            cause: digimon_engine::trigger_context::EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();

    let mut trash_picks = 0u8;
    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::TriggerOrder) => {
                let view = runner.pending_selection_view().expect("view");
                let player = view.selecting_player;
                let action = view.valid_action_ids[0];
                runner.game.resolve_selection(player, action).expect("resolves");
            }
            Some(SelectionKind::Trash) => {
                runner
                    .execute_action(0, digimon_engine::action::space::TRASH_EFFECT_START)
                    .expect("pick trash card");
                trash_picks += 1;
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
    let trash_after = runner.trash_size(0);

    assert_eq!(trash_picks, 3, "must have made 3 trash picks");
    assert_eq!(
        sources_after,
        sources_before + 3,
        "placing 3 from trash must grow sources by 3 (before: {sources_before}, after: {sources_after})"
    );
    assert_eq!(
        trash_after,
        trash_before - 3,
        "3 cards must leave trash (before: {trash_before}, after: {trash_after})"
    );
}

/// Clause B: must NOT fire when trashed source belongs to a different Digimon.
/// This is the correctness test requiring event_host_permanent_is_source.
#[test]
#[ignore = "pending: event_host_permanent_is_source (DSL predicate for OnDigivolutionCardTrashed host-self gate)"]
fn ex11_044_clause_b_does_not_fire_when_source_trashed_from_other_digimon() {
    let other_src = make_mineral_source("EX11-044-OTHER-SRC");
    let trash1 = make_mineral_trash_card("EX11-044-OTHER-T1");
    let trash2 = make_mineral_trash_card("EX11-044-OTHER-T2");
    let trash3 = make_mineral_trash_card("EX11-044-OTHER-T3");
    let carrier = {
        let mut c = make_test_card("EX11-044-CARRIER", "Carrier Other");
        c.traits.push("Mineral".to_string());
        c.level = Some(5);
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(other_src)
        .add_card(trash1)
        .add_card(trash2)
        .add_card(trash3)
        .add_card(carrier)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    let other_digi = runner.place_on_field(0, "EX11-044-CARRIER", None);
    runner.push_source(other_digi, "EX11-044-OTHER-SRC");

    push_to_trash(&mut runner, "EX11-044-OTHER-T1");
    push_to_trash(&mut runner, "EX11-044-OTHER-T2");
    push_to_trash(&mut runner, "EX11-044-OTHER-T3");

    // Pyramidimon is on field but NOT the host of the event.
    let _ = pyramidimon;

    let host_card = runner.game.players[0].battle_area[other_digi.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].trash[0].handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: other_digi,
            host_card,
            card: trashed_card,
            cause: digimon_engine::trigger_context::EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();

    let mut saw_trash_prompt = false;
    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                saw_trash_prompt = true;
                runner
                    .execute_action(0, digimon_engine::action::space::PASS)
                    .expect("pass");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 10 {
            break;
        }
    }

    assert!(
        !saw_trash_prompt,
        "Clause B must NOT fire when the trashed source belongs to a different Digimon's stack"
    );
}

/// Clause B OPT lockout: second trigger same turn must not re-install.
#[test]
#[ignore = "pending: event_host_permanent_is_source (DSL predicate for OnDigivolutionCardTrashed host-self gate)"]
fn ex11_044_clause_b_opt_blocks_second_trigger_same_turn() {
    let src1 = make_mineral_source("EX11-044-BOPT-S1");
    let trash1 = make_mineral_trash_card("EX11-044-BOPT-T1");
    let trash2 = make_mineral_trash_card("EX11-044-BOPT-T2");
    let trash3 = make_mineral_trash_card("EX11-044-BOPT-T3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(src1)
        .add_card(trash1)
        .add_card(trash2)
        .add_card(trash3)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyramidimon, "EX11-044-BOPT-S1");

    push_to_trash(&mut runner, "EX11-044-BOPT-T1");
    push_to_trash(&mut runner, "EX11-044-BOPT-T2");
    push_to_trash(&mut runner, "EX11-044-BOPT-T3");

    let host_card = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].trash[0].handle();

    // First trigger.
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: pyramidimon,
            host_card,
            card: trashed_card,
            cause: digimon_engine::trigger_context::EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();

    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::TriggerOrder) => {
                let view = runner.pending_selection_view().expect("view");
                let player = view.selecting_player;
                let action = view.valid_action_ids[0];
                runner.game.resolve_selection(player, action).expect("resolves");
            }
            Some(SelectionKind::Trash) => {
                runner
                    .execute_action(0, digimon_engine::action::space::TRASH_EFFECT_START)
                    .expect("pick trash");
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 15 {
            break;
        }
    }

    // Re-populate trash.
    push_to_trash(&mut runner, "EX11-044-BOPT-T1");
    push_to_trash(&mut runner, "EX11-044-BOPT-T2");
    push_to_trash(&mut runner, "EX11-044-BOPT-T3");

    // Second trigger same turn — OPT must be locked.
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: pyramidimon,
            host_card,
            card: trashed_card,
            cause: digimon_engine::trigger_context::EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();

    let mut saw_trash_prompt = false;
    let mut iterations2 = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::Trash) => {
                saw_trash_prompt = true;
                runner
                    .execute_action(0, digimon_engine::action::space::PASS)
                    .expect("pass");
            }
            _ => break,
        }
        iterations2 += 1;
        if iterations2 > 10 {
            break;
        }
    }

    assert!(
        !saw_trash_prompt,
        "Clause B OPT must not fire on the second trigger in the same turn"
    );
}

/// Clause B: declining at the first optional trash pick places 0 cards.
#[test]
#[ignore = "pending: event_host_permanent_is_source (DSL predicate for OnDigivolutionCardTrashed host-self gate)"]
fn ex11_044_clause_b_declining_places_no_cards() {
    let src = make_mineral_source("EX11-044-BDECL-SRC");
    let trash1 = make_mineral_trash_card("EX11-044-BDECL-T1");
    let trash2 = make_mineral_trash_card("EX11-044-BDECL-T2");
    let trash3 = make_mineral_trash_card("EX11-044-BDECL-T3");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 parses")
        .add_card(src)
        .add_card(trash1)
        .add_card(trash2)
        .add_card(trash3)
        .memory(10)
        .start();

    let pyramidimon = runner.place_on_field(0, "EX11-044", None);
    runner.push_source(pyramidimon, "EX11-044-BDECL-SRC");

    push_to_trash(&mut runner, "EX11-044-BDECL-T1");
    push_to_trash(&mut runner, "EX11-044-BDECL-T2");
    push_to_trash(&mut runner, "EX11-044-BDECL-T3");

    let sources_before = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .card_sources
        .len();

    let host_card = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].trash[0].handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: pyramidimon,
            host_card,
            card: trashed_card,
            cause: digimon_engine::trigger_context::EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();

    // PASS at the first optional pick — decline.
    let mut iterations = 0;
    loop {
        match runner.pending_kind() {
            Some(SelectionKind::TriggerOrder) => {
                let view = runner.pending_selection_view().expect("view");
                let player = view.selecting_player;
                let action = view.valid_action_ids[0];
                runner.game.resolve_selection(player, action).expect("resolves");
            }
            Some(SelectionKind::Trash) => {
                runner
                    .execute_action(0, digimon_engine::action::space::PASS)
                    .expect("PASS — decline");
                break;
            }
            _ => break,
        }
        iterations += 1;
        if iterations > 10 {
            break;
        }
    }

    let sources_after = runner.game.players[0].battle_area[pyramidimon.index as usize]
        .card_sources
        .len();

    assert_eq!(
        sources_after,
        sources_before,
        "declining at the first pick must leave sources unchanged"
    );
}
