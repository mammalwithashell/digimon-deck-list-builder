//! ST23-15 e-Pulse — Option, White, Cost 3.
//! Traits: Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json — verbatim)
//! <Use Req. ([BEATBREAK] trait)> (Specified cards let you ignore color
//!   requirements.)
//! [Main] You may play 1 [BEATBREAK] trait card with a play cost of 4 or less
//!   from your hand or trash without paying the cost. Then, place this card in
//!   the battle area.
//! [Start of Your Main Phase] By placing this card from the battle area face
//!   down under any of your [BEATBREAK] trait Tamers, <Draw 1> and gain 1 memory.
//! Inherited: [Security] Activate this card's [Main] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/White/ST23_15.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - Option play-or-use from hand/trash free (select_union_zone +
//!   play_union_bound_free) — already-supported substrate
//! - persistent field Option ("place this card in the battle area")
//! - G-MOVE-SELF-OPTION-UNDER-PERMANENT — [Start of Your Main Phase] relocate
//!   THIS field Option face-down under a [BEATBREAK] Tamer → Draw 1 + memory
//!   (the gap closed by this card)
//! - <Use Req. [BEATBREAK]> color-ignore requirement (structural)
//! - inherited [Security] activate-Main
//!
//! # Verdict — IMPLEMENTED (2026-06-15)
//! The [Start of Your Main Phase] self-relocate is the new
//! `move_self_option_under_permanent` engine primitive + DSL verb: the field
//! Option's top card moves face-down under a chosen [BEATBREAK] Tamer, firing
//! NEITHER OnOptionTrashed NOR OnDigivolutionCardTrashed; Draw 1 + gain 1 memory
//! follow the successful relocate. Declinable (canNoSelect: true).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST23-15";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn epulse() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A [BEATBREAK] Tamer (legal relocate target).
fn bb_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c.traits = vec!["BEATBREAK".to_string()];
    c
}

/// A NON-[BEATBREAK] Tamer (illegal relocate target).
fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c.traits = vec!["Beast".to_string()];
    c
}

/// A [BEATBREAK] Digimon with play cost ≤4 (legal hand/trash free-play target).
fn bb_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::White];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["BEATBREAK".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::White];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

/// Fire `StartOfYourMainPhase` over P0's whole battle area (the engine's
/// dispatch path — `enter_main_phase` enqueues this `PlayerBattleArea` source).
fn fire_start_of_main(r: &mut DebugRunner) {
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();
}

fn process_contains_move_self_option(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::MoveSelfOptionUnderPermanent { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_move_self_option(then)
                || process_contains_move_self_option(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_move_self_option(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_15_is_white_beatbreak_option() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
}

#[test]
fn st23_15_has_main_somp_and_security_clauses() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered
            .iter()
            .any(|t| t.when.iter().any(|w| *w == CompiledTiming::MainFromHand)),
        "[Main] clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when.iter().any(|w| *w == CompiledTiming::StartOfYourMainPhase)),
        "[Start of Your Main Phase] relocate clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when.iter().any(|w| *w == CompiledTiming::OnSecurity)),
        "[Security] clause present"
    );
}

#[test]
fn st23_15_somp_clause_relocates_self_under_a_tamer() {
    let card = compiled(CARD_ID);
    let somp = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.iter().any(|w| *w == CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("SOMP clause");
    assert!(
        process_contains_move_self_option(&somp.process),
        "the SOMP clause relocates this Option under a Tamer"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [Start of Your Main Phase] relocate → Draw 1 + gain 1 memory
// ════════════════════════════════════════════════════════════════════════════

/// Build a runner with e-Pulse already on P0's battle area as a field Option,
/// plus the requested Tamers, plus a deck to draw from.
fn setup_field_option(with_bb_tamer: bool, with_plain_tamer: bool) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .add_card(epulse())
        .add_card(bb_tamer("BBT"))
        .add_card(plain_tamer("PT"))
        .add_card(bb_digimon("BBD"))
        .add_card(filler("FILLER"));
    builder = builder.deck(0, &["FILLER"; 6]).deck(1, &["FILLER"; 6]).memory(5);
    let mut runner = builder.start();
    runner.set_first_player(0);

    if with_bb_tamer {
        runner.place_on_field(0, "BBT", Some(0));
    }
    if with_plain_tamer {
        runner.place_on_field(0, "PT", Some(0));
    }
    // e-Pulse as a field Option (placed last so its index is known).
    let option = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, option)
}

fn tamer_handle(runner: &DebugRunner, id: &str) -> PermanentHandle {
    runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .unwrap_or_else(|| panic!("{id} not on field"))
}

/// POSITIVE: with a [BEATBREAK] Tamer on field, the SOMP relocate moves e-Pulse
/// face-down under the Tamer, draws 1, and gains 1 memory. The Option is no
/// longer a standalone battle-area permanent and is NOT in trash.
#[test]
fn st23_15_somp_relocates_self_under_beatbreak_tamer_and_draws_and_gains_memory() {
    let (mut runner, _option) = setup_field_option(true, false);
    let tamer = tamer_handle(&runner, "BBT");
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);

    // The relocate is optional (canNoSelect: true). Accept by resolving the
    // Tamer pick.
    let _ = runner.auto_resolve();

    // e-Pulse moved under the Tamer face-down.
    let tamer = tamer_handle(&runner, "BBT");
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "e-Pulse placed under the [BEATBREAK] Tamer"
    );
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "the placed Option is face-down"
    );
    let placed_id = runner.game.players[0].battle_area[tamer.index as usize].card_sources[0]
        .card_id(&runner.game.card_data);
    assert_eq!(placed_id, CARD_ID, "the placed source is e-Pulse");

    // The Option is no longer a standalone battle-area permanent.
    assert_eq!(
        runner.battle_area_size(0),
        ba_before - 1,
        "e-Pulse is no longer a standalone field permanent (it moved under the Tamer)"
    );
    // Draw + memory happened.
    assert_eq!(runner.hand_size(0), hand_before + 1, "<Draw 1>");
    assert_eq!(runner.memory(), mem_before + 1, "gain 1 memory");
    // It was MOVED, not trashed: no e-Pulse in trash.
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "e-Pulse is moved (not trashed) — trash unchanged"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "e-Pulse must NOT be in trash"
    );
}

/// DECLINABLE: the SOMP relocate is optional (DCGO canNoSelect: true). Declining
/// leaves e-Pulse on the field, no draw, no memory.
#[test]
fn st23_15_somp_relocate_is_declinable() {
    let (mut runner, _option) = setup_field_option(true, false);
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);

    // Decline the optional Tamer pick.
    if let Some(v) = runner.pending_selection_view() {
        assert!(v.is_optional, "the relocate is optional (canNoSelect: true)");
        runner.execute_action(v.selecting_player, PASS).expect("decline");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        ba_before,
        "declined ⇒ e-Pulse stays on the field"
    );
    assert_eq!(runner.hand_size(0), hand_before, "declined ⇒ no draw");
    assert_eq!(runner.memory(), mem_before, "declined ⇒ no memory gain");
}

/// NEGATIVE (no [BEATBREAK] Tamer): the relocate cannot happen — no prompt, no
/// draw, no memory, e-Pulse stays on the field. A NON-[BEATBREAK] Tamer is on
/// field (still no legal target).
#[test]
fn st23_15_somp_no_beatbreak_tamer_no_relocate() {
    let (mut runner, _option) = setup_field_option(false, true);
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no [BEATBREAK] Tamer ⇒ no relocate prompt"
    );
    assert_eq!(
        runner.battle_area_size(0),
        ba_before,
        "e-Pulse stays on the field"
    );
    assert_eq!(runner.hand_size(0), hand_before, "no draw");
    assert_eq!(runner.memory(), mem_before, "no memory gain");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] play-or-use a [BEATBREAK] cost≤4 card free; persist on field
// ════════════════════════════════════════════════════════════════════════════

/// [Main]: play e-Pulse from hand; play 1 [BEATBREAK] cost≤4 Digimon from hand
/// for free; e-Pulse then persists in the battle area as a field Option.
#[test]
fn st23_15_main_plays_beatbreak_free_and_persists_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-15 in embedded pack")
        .add_card(bb_digimon("BBD"))
        .add_card(filler("FILLER"))
        .hand(0, &[CARD_ID, "BBD"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let mem_before = runner.memory();
    // Play e-Pulse (the Option) from hand.
    runner.play(0, 0);
    // Resolve the [Main] play-or-use (pick the BEATBREAK Digimon to play free).
    let _ = runner.auto_resolve();

    // The BEATBREAK Digimon entered the battle area for free (no cost paid for
    // it; e-Pulse's own use cost of 3 was paid on play).
    let bbd_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BBD");
    assert!(bbd_on_field, "the [BEATBREAK] Digimon is played for free");
    // e-Pulse persists in the battle area as a field Option.
    let epulse_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(epulse_on_field, "e-Pulse persists in the battle area");
}
