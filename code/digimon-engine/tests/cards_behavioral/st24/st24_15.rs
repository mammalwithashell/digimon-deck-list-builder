//! ST24-15 DNA Charge — Option, White, Cost 3.
//! Traits: DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim)
//! <Use Req. ([DATA SQUAD] trait)> (Specified cards let you ignore color
//!   requirements.)
//! [Main] You may play 1 [DATA SQUAD] trait card with a play cost of 4 or less
//!   from your hand or trash without paying the cost. Then, place this card in
//!   the battle area.
//! [Start of Your Main Phase] By placing this card from the battle area face
//!   down under any of your [DATA SQUAD] trait Tamers, <Draw 1> and gain 1 memory.
//! Inherited: [Security] Activate this card's [Main] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/White/ST24_15.cs
//!
//! # Patterns — sister to ST23-15 (e-Pulse), trait-swapped BEATBREAK → DATA SQUAD
//! - Option play-or-use from hand/trash free (select_union_zone + play_union_bound_free)
//! - persistent field Option ("place this card in the battle area")
//! - G-MOVE-SELF-OPTION-UNDER-PERMANENT — SOMP relocate self FD under a Tamer

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

const CARD_ID: &str = "ST24-15";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn dna_charge() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A [DATA SQUAD] Tamer (legal relocate target).
fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A NON-[DATA SQUAD] Tamer (illegal relocate target).
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

/// A [DATA SQUAD] Digimon with play cost ≤4 (legal hand/trash free-play target).
fn ds_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::White];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["DATA SQUAD".to_string()];
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
fn st24_15_is_white_data_squad_option() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
}

#[test]
fn st24_15_has_main_somp_and_security_clauses() {
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
fn st24_15_somp_clause_relocates_self_under_a_tamer() {
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

fn setup_field_option(with_ds_tamer: bool, with_plain_tamer: bool) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .add_card(dna_charge())
        .add_card(ds_tamer("DST"))
        .add_card(plain_tamer("PT"))
        .add_card(ds_digimon("DSD"))
        .add_card(filler("FILLER"));
    builder = builder.deck(0, &["FILLER"; 6]).deck(1, &["FILLER"; 6]).memory(5);
    let mut runner = builder.start();
    runner.set_first_player(0);

    if with_ds_tamer {
        runner.place_on_field(0, "DST", Some(0));
    }
    if with_plain_tamer {
        runner.place_on_field(0, "PT", Some(0));
    }
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

/// POSITIVE: with a [DATA SQUAD] Tamer on field, the SOMP relocate moves
/// DNA Charge face-down under the Tamer, draws 1, and gains 1 memory. The Option
/// is no longer a standalone permanent and is NOT in trash (it moved).
#[test]
fn st24_15_somp_relocates_self_under_ds_tamer_and_draws_and_gains_memory() {
    let (mut runner, _option) = setup_field_option(true, false);
    let tamer = tamer_handle(&runner, "DST");
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);
    let _ = runner.auto_resolve();

    let tamer = tamer_handle(&runner, "DST");
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "DNA Charge placed under the [DATA SQUAD] Tamer"
    );
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "the placed Option is face-down"
    );
    let placed_id = runner.game.players[0].battle_area[tamer.index as usize].card_sources[0]
        .card_id(&runner.game.card_data);
    assert_eq!(placed_id, CARD_ID, "the placed source is DNA Charge");

    assert_eq!(
        runner.battle_area_size(0),
        ba_before - 1,
        "DNA Charge is no longer a standalone field permanent"
    );
    assert_eq!(runner.hand_size(0), hand_before + 1, "<Draw 1>");
    assert_eq!(runner.memory(), mem_before + 1, "gain 1 memory");
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "DNA Charge is moved (not trashed) — trash unchanged"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "DNA Charge must NOT be in trash"
    );
}

#[test]
fn st24_15_somp_relocate_is_declinable() {
    let (mut runner, _option) = setup_field_option(true, false);
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);
    if let Some(v) = runner.pending_selection_view() {
        assert!(v.is_optional, "the relocate is optional (canNoSelect: true)");
        runner.execute_action(v.selecting_player, PASS).expect("decline");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        ba_before,
        "declined ⇒ DNA Charge stays on the field"
    );
    assert_eq!(runner.hand_size(0), hand_before, "declined ⇒ no draw");
    assert_eq!(runner.memory(), mem_before, "declined ⇒ no memory gain");
}

#[test]
fn st24_15_somp_no_ds_tamer_no_relocate() {
    let (mut runner, _option) = setup_field_option(false, true);
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no [DATA SQUAD] Tamer ⇒ no relocate prompt"
    );
    assert_eq!(runner.battle_area_size(0), ba_before, "DNA Charge stays on the field");
    assert_eq!(runner.hand_size(0), hand_before, "no draw");
    assert_eq!(runner.memory(), mem_before, "no memory gain");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] play a [DATA SQUAD] cost≤4 card free; persist on field
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_15_main_plays_data_squad_free_and_persists_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-15 in embedded pack")
        .add_card(ds_digimon("DSD"))
        .add_card(filler("FILLER"))
        .hand(0, &[CARD_ID, "DSD"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let _mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let dsd_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "DSD");
    assert!(dsd_on_field, "the [DATA SQUAD] Digimon is played for free");
    let dna_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(dna_on_field, "DNA Charge persists in the battle area");
}
