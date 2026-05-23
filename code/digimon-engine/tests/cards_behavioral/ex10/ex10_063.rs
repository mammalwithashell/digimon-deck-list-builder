//! EX10-063 Close — Tamer, Black, Cost 3.
//! Traits: [LIBERATOR]
//!
//! # Card text (data/cards.json — verbatim)
//!
//! [Start of Your Main Phase] By returning this Tamer to the bottom of
//! the deck, you may play 1 [Close] from your hand without paying the
//! cost. Then, if you don't have a Digimon, you may play 1 [Sunarizamon]
//! from your trash without paying the cost.
//!
//! [All Turns] When effects trash any of your [Mineral] or [Rock] trait
//! Digimon's digivolution cards, by suspending this Tamer, gain 1 memory.
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_063.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - B1/B3: start_of_your_main_phase Tamer with activation cost (return-self
//!   to deck bottom) + conditional free-play from hand + conditional free-play
//!   from trash
//! - B3: all_turns triggered observer (on_digivolution_card_trashed) with
//!   Mineral/Rock trait gating + suspend-self cost + gain_memory
//! - Structural: 3 triggered clauses (start_of_main, on_digivolution_card_trashed,
//!   on_security), scopes, optional flags
//! - Security: structural assertion that inherited on_security clause present

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn close_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .build()
}

/// Make a Tamer named "Close" — matches Clause 1's `name_is: "Close"` filter.
fn make_close_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Close");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec!["LIBERATOR".to_string()];
    c
}

/// Make a Digimon named "Sunarizamon" — matches Clause 1's
/// `name_is: "Sunarizamon"` trash filter.
fn make_sunarizamon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Sunarizamon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 4;
    c.traits = vec!["Mineral".to_string(), "Mini".to_string()];
    c
}

/// Make an unrelated Tamer (not named "Close") to verify name filter rejects it.
fn make_other_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "TetsuoTamer");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec![];
    c
}

/// Make a Mineral Digimon to use as a carrier with Mineral digivolution sources.
fn make_mineral_host(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} MineralHost"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["Mineral".to_string()];
    c
}

/// Make a Rock Digimon.
fn make_rock_host(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} RockHost"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["Rock".to_string()];
    c
}

/// Make a source card with Mineral trait (digivolution source candidate).
fn make_mineral_source(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} MineralSrc"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.traits = vec!["Mineral".to_string()];
    c
}

/// Make a source card with Rock trait.
fn make_rock_source(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} RockSrc"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.traits = vec!["Rock".to_string()];
    c
}

/// Make a source card with no matching traits (control).
fn make_plain_source(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} PlainSrc"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.traits = vec!["Aquatic".to_string()];
    c
}

/// A generic filler card for decks.
fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Push a card already in card_data into player 0's trash.
fn push_to_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in card_data"));
    let next = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_idx, player as u8, next));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX10-063 must be in the embedded DSL pack and compile without error.
#[test]
fn ex10_063_yaml_compiles_without_error() {
    let runner = close_runner();
    assert!(
        runner.compiled_card("EX10-063").is_some(),
        "EX10-063 must be present in the embedded DSL pack"
    );
}

/// EX10-063 must have exactly 3 triggered clauses:
///   1. start_of_your_main_phase
///   2. on_digivolution_card_trashed
///   3. on_security (inherited)
#[test]
fn ex10_063_has_exactly_three_triggered_clauses() {
    let runner = close_runner();
    let card = runner
        .compiled_card("EX10-063")
        .expect("EX10-063 compiled card present");

    let triggered: Vec<_> = card
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
        "EX10-063 must have exactly 3 triggered clauses (start_of_your_main_phase, \
         on_digivolution_card_trashed, on_security); found {}",
        triggered.len()
    );
}

/// Clause 1: start_of_your_main_phase, FaceUp scope, optional ("you may").
#[test]
fn ex10_063_clause1_start_of_main_phase_face_up_optional() {
    let runner = close_runner();
    let card = runner
        .compiled_card("EX10-063")
        .expect("EX10-063 compiled present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start_of_your_main_phase clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 1 must be FaceUp scope (own tamer effect)"
    );
    assert!(
        clause.optional,
        "Clause 1 must be optional — printed 'you may' / return-to-deck cost gating"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 1 has no [Once Per Turn] — deck-return cost limits re-entry naturally"
    );
}

/// Clause 2: on_digivolution_card_trashed, FaceUp scope (all_turns), not optional
/// (the effect fires automatically when conditions met), not OPT.
#[test]
fn ex10_063_clause2_on_digivolution_card_trashed_face_up_not_opt() {
    let runner = close_runner();
    let card = runner
        .compiled_card("EX10-063")
        .expect("EX10-063 compiled present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed))
        .expect("on_digivolution_card_trashed clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 2 [All Turns] must be FaceUp scope"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 2 has no [Once Per Turn] in printed text"
    );
}

/// Clause 3: on_security, Inherited scope, not optional (security effects are
/// mandatory per RULES_CONTEXT.md §16), not OPT.
#[test]
fn ex10_063_clause3_on_security_inherited_mandatory() {
    let runner = close_runner();
    let card = runner
        .compiled_card("EX10-063")
        .expect("EX10-063 compiled present");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "Clause 3 [Security] must be Inherited scope — it is a security effect"
    );
    assert!(
        !clause.optional,
        "Security clause must be mandatory (no opt-out per RULES_CONTEXT.md §16)"
    );
    assert!(!clause.once_per_turn, "Security clause has no OPT");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 2: [All Turns] on_digivolution_card_trashed behavioral
//
// The stub test already covers the basic positive case. We add:
//  - Positive: Mineral trait triggers, tamer suspends and gains memory
//  - Positive: Rock trait triggers too
//  - Negative: Plain (non-Mineral/Rock) source — Tamer does NOT trigger
//  - Negative: Tamer already suspended — condition (is_unsuspended) prevents trigger
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive (original stub test retained & expanded):
/// When a Mineral-trait Digimon's source is trashed by an effect, the unsuspended
/// EX10-063 Tamer suspends and player gains 1 memory.
#[test]
fn ex10_063_source_trash_mineral_host_suspends_tamer_and_gains_memory() {
    let host = make_mineral_host("MINERAL-HOST-A");
    let source = make_mineral_source("MINERAL-SRC-A");
    let filler = make_filler("FILLER-A");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(host)
        .add_card(source)
        .add_card(filler)
        .deck(0, &["FILLER-A", "FILLER-A"])
        .deck(1, &["FILLER-A", "FILLER-A"])
        .memory(0)
        .build();

    let close_h = runner.place_on_field(0, "EX10-063", None);
    let host_h = runner.place_on_field(0, "MINERAL-HOST-A", None);
    let source_h = runner.push_source(host_h, "MINERAL-SRC-A");

    // Pre-condition: tamer must be unsuspended before trigger.
    assert!(
        !runner.game.players[0].battle_area[close_h.index as usize].is_suspended,
        "EX10-063 must be unsuspended before the event fires"
    );

    let top = runner.top_card(host_h);
    let memory_before = runner.memory();
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_h), 0);
        ctx.trash_card_source(host_h, source_h);
    }

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Trashing a Mineral Digimon's source must gain 1 memory for the controller"
    );
    assert!(
        runner.game.players[0].battle_area[close_h.index as usize].is_suspended,
        "EX10-063 must be suspended after the on_digivolution_card_trashed trigger fires"
    );
}

/// Positive: Rock-trait host also triggers the [All Turns] clause.
#[test]
fn ex10_063_source_trash_rock_host_suspends_tamer_and_gains_memory() {
    let host = make_rock_host("ROCK-HOST-B");
    let source = make_rock_source("ROCK-SRC-B");
    let filler = make_filler("FILLER-B");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(host)
        .add_card(source)
        .add_card(filler)
        .deck(0, &["FILLER-B", "FILLER-B"])
        .deck(1, &["FILLER-B", "FILLER-B"])
        .memory(0)
        .build();

    let close_h = runner.place_on_field(0, "EX10-063", None);
    let host_h = runner.place_on_field(0, "ROCK-HOST-B", None);
    let source_h = runner.push_source(host_h, "ROCK-SRC-B");

    let top = runner.top_card(host_h);
    let memory_before = runner.memory();
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_h), 0);
        ctx.trash_card_source(host_h, source_h);
    }

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Trashing a Rock Digimon's source must also gain 1 memory"
    );
    assert!(
        runner.game.players[0].battle_area[close_h.index as usize].is_suspended,
        "EX10-063 must be suspended after Rock-host trigger fires"
    );
}

/// Negative: plain (non-Mineral, non-Rock) host — the clause condition fails,
/// so Tamer does NOT suspend and memory does NOT change.
#[test]
fn ex10_063_source_trash_plain_host_does_not_trigger() {
    let host = make_plain_source("PLAIN-HOST-C");
    let source = make_plain_source("PLAIN-SRC-C");
    let filler = make_filler("FILLER-C");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(host)
        .add_card(source)
        .add_card(filler)
        .deck(0, &["FILLER-C", "FILLER-C"])
        .deck(1, &["FILLER-C", "FILLER-C"])
        .memory(0)
        .build();

    let close_h = runner.place_on_field(0, "EX10-063", None);
    let host_h = runner.place_on_field(0, "PLAIN-HOST-C", None);
    let source_h = runner.push_source(host_h, "PLAIN-SRC-C");

    let top = runner.top_card(host_h);
    let memory_before = runner.memory();
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_h), 0);
        ctx.trash_card_source(host_h, source_h);
    }

    assert_eq!(
        runner.memory(),
        memory_before,
        "Trashing a plain (non-Mineral/Rock) Digimon's source must NOT gain memory"
    );
    assert!(
        !runner.game.players[0].battle_area[close_h.index as usize].is_suspended,
        "EX10-063 must NOT be suspended when the host has no Mineral/Rock trait"
    );
}

/// Negative: tamer already suspended — condition `is_unsuspended: true` in the
/// YAML fails, so the trigger does not fire a second time.
#[test]
fn ex10_063_already_suspended_tamer_does_not_trigger_again() {
    let host = make_mineral_host("MINERAL-HOST-D");
    let source = make_mineral_source("MINERAL-SRC-D");
    let source2 = make_mineral_source("MINERAL-SRC-D2");
    let filler = make_filler("FILLER-D");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(host)
        .add_card(source)
        .add_card(source2)
        .add_card(filler)
        .deck(0, &["FILLER-D", "FILLER-D"])
        .deck(1, &["FILLER-D", "FILLER-D"])
        .memory(0)
        .build();

    let close_h = runner.place_on_field(0, "EX10-063", None);
    let host_h = runner.place_on_field(0, "MINERAL-HOST-D", None);
    let source_h = runner.push_source(host_h, "MINERAL-SRC-D");
    let source2_h = runner.push_source(host_h, "MINERAL-SRC-D2");

    // Fire first source trash — tamer suspends, memory +1.
    let top = runner.top_card(host_h);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_h), 0);
        ctx.trash_card_source(host_h, source_h);
    }

    assert_eq!(runner.memory(), 1, "first source trash gains 1 memory");
    assert!(
        runner.game.players[0].battle_area[close_h.index as usize].is_suspended,
        "tamer is suspended after first trigger"
    );

    // Now fire second source trash — tamer is already suspended so the
    // `is_unsuspended: true` condition in the YAML fails. Memory must NOT
    // increase again.
    let memory_after_first = runner.memory();
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host_h), 0);
        ctx.trash_card_source(host_h, source2_h);
    }

    assert_eq!(
        runner.memory(),
        memory_after_first,
        "second source trash while tamer is suspended must NOT gain additional memory \
         (is_unsuspended: true condition gates trigger)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 1: [Start of Main Phase] behavioral
//
// Activation cost: return EX10-063 to deck bottom.
// After cost: may play 1 [Close] from hand free.
// Conditional tail: if no Digimon on field, may play 1 [Sunarizamon] from trash free.
//
// Drive via end_turn + end_turn cycle (P0→P1→P0) so begin_turn fires
// StartOfYourMainPhase for P0 with the Tamer in play.
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: build a runner for start-of-main-phase tests.
/// Places EX10-063 on P0's field; both players have a non-empty deck.
fn runner_for_main_phase() -> DebugRunner {
    let filler = make_filler("FILL-MP");
    DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(filler)
        .deck(0, &["FILL-MP", "FILL-MP", "FILL-MP", "FILL-MP", "FILL-MP"])
        .deck(1, &["FILL-MP", "FILL-MP", "FILL-MP", "FILL-MP", "FILL-MP"])
        .memory(5)
        .start()
}

/// Helper: push card_id into player 0's hand.
fn push_to_hand(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in card_data"));
    let next = runner.game.next_card_index();
    runner.game.players[player]
        .hand
        .push(CardSource::new(data_idx, player as u8, next));
}

/// Positive: Clause 1 fires at the start of P0's main phase when a "Close"-named
/// card is in P0's hand. After accepting and selecting the Close, EX10-063 is no
/// longer on the field (returned to deck) and the Close-named Tamer is on field.
#[test]
fn ex10_063_clause1_returns_self_and_plays_close_from_hand_free() {
    let close_card = make_close_tamer("CLOSE-OTHER");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(close_card)
        .add_card(make_filler("FILL-C1A"))
        .deck(
            0,
            &["FILL-C1A", "FILL-C1A", "FILL-C1A", "FILL-C1A", "FILL-C1A"],
        )
        .deck(
            1,
            &["FILL-C1A", "FILL-C1A", "FILL-C1A", "FILL-C1A", "FILL-C1A"],
        )
        .memory(5)
        .start();

    // Place EX10-063 on P0's field.
    runner.place_on_field(0, "EX10-063", None);
    // Put the Close-named Tamer in P0's hand.
    push_to_hand(&mut runner, 0, "CLOSE-OTHER");

    // Cycle P0 → P1 → P0 to fire StartOfYourMainPhase.
    runner.end_turn(); // P0 → P1
    runner.end_turn(); // P1 → P0; StartOfYourMainPhase fires for P0.

    // Drive all pending selections: accept each non-PASS action until no pending.
    let mut iters = 0;
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept pending selection");
        iters += 1;
        if iters > 20 {
            break;
        }
    }
    let _ = runner.auto_resolve();

    // EX10-063 must be off-field (returned to deck bottom as activation cost).
    let close_063_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX10-063");
    assert!(
        !close_063_on_field,
        "EX10-063 must leave the field (deck-bottom cost paid)"
    );

    // The Close-named Tamer must be on P0's field (played free from hand).
    let close_other_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "CLOSE-OTHER");
    assert!(
        close_other_on_field,
        "Close-named Tamer must be on field after free play from hand"
    );
}

/// Positive: When no Digimon is on P0's field and a Sunarizamon is in trash,
/// the conditional tail fires and plays Sunarizamon from trash free.
#[test]
fn ex10_063_clause1_no_digimon_on_field_plays_sunarizamon_from_trash_free() {
    let close_card = make_close_tamer("CLOSE-C1B");
    let sunari = make_sunarizamon("SUNARI-C1B");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(close_card)
        .add_card(sunari)
        .add_card(make_filler("FILL-C1B"))
        .deck(
            0,
            &["FILL-C1B", "FILL-C1B", "FILL-C1B", "FILL-C1B", "FILL-C1B"],
        )
        .deck(
            1,
            &["FILL-C1B", "FILL-C1B", "FILL-C1B", "FILL-C1B", "FILL-C1B"],
        )
        .memory(5)
        .start();

    // Place EX10-063; no other Digimon on P0's field.
    runner.place_on_field(0, "EX10-063", None);
    // Close in hand, Sunarizamon in trash.
    push_to_hand(&mut runner, 0, "CLOSE-C1B");
    push_to_trash(&mut runner, 0, "SUNARI-C1B");

    runner.end_turn(); // P0 → P1
    runner.end_turn(); // P1 → P0; StartOfYourMainPhase fires.

    // Drive all pending: accept every non-PASS prompt.
    let mut iters = 0;
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept pending selection");
        iters += 1;
        if iters > 25 {
            break;
        }
    }
    let _ = runner.auto_resolve();

    // Sunarizamon must be on P0's battle area (played from trash, free).
    let sunari_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUNARI-C1B");
    assert!(
        sunari_on_field,
        "Sunarizamon must be on field after conditional free-play from trash when \
         no Digimon was on field"
    );
}

/// Negative (conditional gating): When a Digimon IS already on P0's field,
/// the "if you don't have a Digimon" gate fails — Sunarizamon must NOT be
/// played from trash (even if it is present in trash).
#[test]
fn ex10_063_clause1_has_digimon_on_field_sunarizamon_tail_does_not_fire() {
    let close_card = make_close_tamer("CLOSE-C1C");
    let sunari = make_sunarizamon("SUNARI-C1C");
    let digimon = make_mineral_host("DIGIMON-C1C");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(close_card)
        .add_card(sunari)
        .add_card(digimon)
        .add_card(make_filler("FILL-C1C"))
        .deck(
            0,
            &["FILL-C1C", "FILL-C1C", "FILL-C1C", "FILL-C1C", "FILL-C1C"],
        )
        .deck(
            1,
            &["FILL-C1C", "FILL-C1C", "FILL-C1C", "FILL-C1C", "FILL-C1C"],
        )
        .memory(5)
        .start();

    // Place EX10-063 AND a Digimon on P0's field.
    runner.place_on_field(0, "EX10-063", None);
    runner.place_on_field(0, "DIGIMON-C1C", None);
    // Close in hand, Sunarizamon in trash.
    push_to_hand(&mut runner, 0, "CLOSE-C1C");
    push_to_trash(&mut runner, 0, "SUNARI-C1C");

    runner.end_turn(); // P0 → P1
    runner.end_turn(); // P1 → P0; StartOfYourMainPhase fires.

    // Drive all pending: accept every non-PASS prompt (selects Close from hand).
    let mut iters = 0;
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept pending selection");
        iters += 1;
        if iters > 25 {
            break;
        }
    }
    let _ = runner.auto_resolve();

    // Sunarizamon must NOT be on P0's field (conditional tail was gated off).
    let sunari_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUNARI-C1C");
    assert!(
        !sunari_on_field,
        "Sunarizamon must NOT be played when P0 already had a Digimon on field \
         (conditional tail gated by no_permanent: {{ kind: digimon }})"
    );

    // But Sunarizamon should still be in trash (not moved).
    let sunari_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "SUNARI-C1C");
    assert!(
        sunari_in_trash,
        "Sunarizamon must remain in trash when the conditional tail did not fire"
    );
}

/// Negative: hand has no [Close]-named card — select_hand prompt either
/// has no valid actions or is optional, so no Close is played. EX10-063 still
/// leaves the field once the activation is accepted (cost is paid regardless
/// of whether the inner pick finds a candidate).
#[test]
fn ex10_063_clause1_no_close_in_hand_cost_still_paid() {
    let other_tamer = make_other_tamer("OTHER-TAMER-C1D");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(other_tamer)
        .add_card(make_filler("FILL-C1D"))
        .deck(
            0,
            &["FILL-C1D", "FILL-C1D", "FILL-C1D", "FILL-C1D", "FILL-C1D"],
        )
        .deck(
            1,
            &["FILL-C1D", "FILL-C1D", "FILL-C1D", "FILL-C1D", "FILL-C1D"],
        )
        .memory(5)
        .start();

    runner.place_on_field(0, "EX10-063", None);
    // Put a non-Close tamer in hand — won't match name_is: "Close".
    push_to_hand(&mut runner, 0, "OTHER-TAMER-C1D");

    runner.end_turn(); // P0 → P1
    runner.end_turn(); // P1 → P0; StartOfYourMainPhase fires.

    // Accept all prompts.
    let mut iters = 0;
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept pending selection");
        iters += 1;
        if iters > 20 {
            break;
        }
    }
    let _ = runner.auto_resolve();

    // EX10-063 must be off-field (cost = return-to-deck-bottom was paid once outer accepted).
    let close_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX10-063");
    assert!(
        !close_on_field,
        "EX10-063 must leave the field even when no Close-named card was in hand \
         (cost paid on outer accept)"
    );

    // Other tamer must NOT be on field (name filter excluded it).
    let other_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "OTHER-TAMER-C1D");
    assert!(
        !other_on_field,
        "Non-Close tamer must NOT be played (name_is: 'Close' filter excludes it)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3: [Security] structural
//
// Full play-from-security behavioral path is covered by the shared substrate
// tests. Here we only verify the compiled clause shape (scope = Inherited,
// timing = OnSecurity). This follows the pattern of bt22_089 and bt21_081.
// ═══════════════════════════════════════════════════════════════════════════════

/// Structural: the on_security clause must be Inherited scope and mandatory.
/// (Full behavioral path is covered by the shared play_from_security substrate.)
#[test]
fn ex10_063_clause3_security_clause_is_inherited_and_mandatory() {
    let runner = close_runner();
    let card = runner
        .compiled_card("EX10-063")
        .expect("EX10-063 compiled present");

    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist on EX10-063");

    assert_eq!(
        security.scope,
        CompiledScope::Inherited,
        "[Security] clause must be Inherited scope"
    );
    assert!(
        !security.optional,
        "[Security] play-self clause must be mandatory"
    );
}
