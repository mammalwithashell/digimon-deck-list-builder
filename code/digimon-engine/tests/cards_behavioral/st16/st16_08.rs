//! ST16-08 Garurumon — Lv.4 Purple, cost 6, 5000 DP.
//!
//! Printed text (data/card_bundles/ST16-08.md, official Bandai DB):
//!
//! ```text
//! [Digivolve] Lv.3 w/[Gabumon] in name: Cost 2
//! [Security] You may play 1 [Gabumon] or 1 Tamer card with [Matt Ishida] in its
//!   name from your hand or trash without paying the cost.
//! [When Digivolving] <Draw 1>. Then, trash 1 card in your hand.
//! Inherited: [When Attacking][Once Per Turn] <Draw 1>. Then, trash 1 card in
//!   your hand.
//! ```
//!
//! Official Q&A: a bracketed `[Gabumon]` is the exact name, so a Digimon that
//! merely has Gabumon in its name cannot be played by the [Security] effect.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST16/Purple/ST16_08.cs. Boards here are
//! synthetic (no recorded game data).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

/// P1 defends with ST16-08 as its only security card; P0's 6000-DP attacker
/// attacks the player so the security effect fires for P1.
fn security_runner(p1_hand: &[&str], p1_trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST16-08")
        .expect("ST16-08 YAML compiles into the embedded pack")
        .add_card(digimon("T-GAB", "Gabumon", 3, 1000))
        .add_card(digimon("T-GABX", "Gabumon X", 3, 1000))
        .add_card(tamer("T-MATT", "Matt Ishida"))
        .add_card(digimon("T-MATT-DIGI", "Matt Ishida Fan", 3, 1000))
        .add_card(tamer("T-TK", "Takeru Takaishi"))
        .add_card(digimon("T-ATK", "Attacker", 4, 6000))
        .add_card(make_test_card("FILL", "Filler"))
        .memory(10)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .hand(1, p1_hand)
        .security(1, &["ST16-08"])
        .start();
    for id in p1_trash {
        runner.inject_trash(1, id);
    }
    let attacker = runner.place_on_field(0, "T-ATK", Some(0));
    (runner, attacker)
}

fn p1_has_on_field(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

fn union_candidates(runner: &DebugRunner) -> Vec<u16> {
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the [Security] union-zone pick is parked");
    assert_eq!(
        sel.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        },
        "the [Security] effect is a hand/trash union pick"
    );
    sel.valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect()
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn st16_08_has_gabumon_alt_digivolve_security_and_inherited_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("ST16-08")
        .expect("ST16-08 compiles")
        .start();
    let compiled = runner.compiled_card("ST16-08").expect("compiled");
    assert_eq!(compiled.cost, Some(6));
    assert_eq!(compiled.alt_paths.len(), 1, "one [Digivolve] Lv.3 [Gabumon] path");
    assert!(
        compiled
            .effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited)),
        "the [When Attacking][OPT] draw/trash is inherited"
    );
}

// ─── [Security] ──────────────────────────────────────────────────────────────

/// Only the EXACT name [Gabumon] qualifies (official Q&A): "Gabumon X" and a
/// non-Tamer named "Matt Ishida Fan" are not candidates, so with only those
/// cards available the effect offers nothing and the attack resolves.
#[test]
fn st16_08_security_offers_no_inexact_names_or_non_tamers() {
    let (mut runner, attacker) = security_runner(&["T-GABX", "T-MATT-DIGI", "T-TK"], &[]);
    let _ = runner.attack_player(attacker, 1, false);
    assert!(
        !matches!(
            runner.game.pending_selection.as_ref().map(|s| &s.kind),
            Some(SelectionKind::UnionZone { .. })
        ),
        "no eligible [Gabumon] / [Matt Ishida] Tamer: no union pick is offered"
    );
}

/// [Gabumon] in the trash is played free; nothing is paid.
#[test]
fn st16_08_security_plays_gabumon_from_trash_free() {
    let (mut runner, attacker) = security_runner(&["T-GABX"], &["T-GAB"]);
    let memory_before = runner.game.memory;
    let _ = runner.attack_player(attacker, 1, false);
    let candidates = union_candidates(&runner);
    assert_eq!(candidates.len(), 1, "only the exact-name Gabumon in trash: {candidates:?}");
    runner
        .execute_action(1, candidates[0])
        .expect("pick the trash Gabumon");
    runner.auto_resolve().expect("finish the security check");
    assert!(p1_has_on_field(&runner, "T-GAB"), "[Gabumon] was played");
    assert_eq!(runner.game.memory, memory_before, "played without paying the cost");
}

/// A Tamer with [Matt Ishida] in its name is a candidate from the HAND too.
#[test]
fn st16_08_security_plays_matt_ishida_tamer_from_hand() {
    let (mut runner, attacker) = security_runner(&["T-MATT", "T-GABX"], &[]);
    let _ = runner.attack_player(attacker, 1, false);
    let candidates = union_candidates(&runner);
    assert_eq!(candidates.len(), 1, "only the Matt Ishida Tamer: {candidates:?}");
    runner.execute_action(1, candidates[0]).expect("pick the Tamer");
    runner.auto_resolve().expect("finish the security check");
    assert!(p1_has_on_field(&runner, "T-MATT"), "the Matt Ishida Tamer was played");
}

/// Hand AND trash are both offered in one union prompt.
#[test]
fn st16_08_security_offers_hand_and_trash_together() {
    let (mut runner, attacker) = security_runner(&["T-GAB"], &["T-MATT"]);
    let _ = runner.attack_player(attacker, 1, false);
    assert_eq!(union_candidates(&runner).len(), 2, "hand Gabumon + trash Matt Ishida Tamer");
}

/// "You may": declining plays nothing.
#[test]
fn st16_08_security_decline_plays_nothing() {
    let (mut runner, attacker) = security_runner(&[], &["T-GAB"]);
    let _ = runner.attack_player(attacker, 1, false);
    let _ = union_candidates(&runner);
    runner.execute_action(1, PASS).expect("decline the optional play");
    runner.auto_resolve().expect("finish the security check");
    assert!(!p1_has_on_field(&runner, "T-GAB"), "declined: nothing played");
}

// ─── [Digivolve] Lv.3 w/[Gabumon] in name + [When Digivolving] ───────────────

fn digivolve_runner(base_id: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST16-08")
        .expect("ST16-08 compiles")
        .add_card(digimon("T-GABX", "Gabumon X", 3, 1000))
        .add_card(digimon("T-AGU", "Agumon X", 3, 1000))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["ST16-08", "FILL"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, base_id, Some(0));
    runner.game.enter_main_phase();
    (runner, base)
}

#[test]
fn st16_08_digivolves_from_lv3_with_gabumon_in_name_for_2_then_draws_and_trashes() {
    let (mut runner, base) = digivolve_runner("T-GABX");
    let memory_before = runner.game.memory;
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);
    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "ST16-08")
        .expect("ST16-08 in hand");
    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "Lv.3 [Gabumon X] is a legal base at cost 2 (any colour)");
    runner.game.drain_effect_queue();
    assert_eq!(memory_before - runner.game.memory, 2, "digivolve cost 2");

    // [When Digivolving] <Draw 1>, then a MANDATORY 1-card trash.
    let view = runner
        .pending_selection_view()
        .expect("the trash-1-card pick is parked after the draw");
    assert!(!view.is_optional, "the discard is mandatory (DCGO canNoSelect false)");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash a card");
    runner.auto_resolve().expect("finish");

    // -1 (ST16-08 stacked) +1 (rule draw) +1 (<Draw 1>) -1 (trashed) = unchanged.
    assert_eq!(runner.hand_size(0), hand_before);
    assert_eq!(runner.deck_size(0), deck_before - 2, "rule draw + <Draw 1>");
    assert_eq!(runner.trash_size(0), trash_before + 1, "exactly one card trashed");
}

#[test]
fn st16_08_cannot_digivolve_from_lv3_without_gabumon_in_name() {
    let (mut runner, base) = digivolve_runner("T-AGU");
    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "ST16-08")
        .expect("ST16-08 in hand");
    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(!ok, "a red Lv.3 without Gabumon in its name is not a legal base");
}
