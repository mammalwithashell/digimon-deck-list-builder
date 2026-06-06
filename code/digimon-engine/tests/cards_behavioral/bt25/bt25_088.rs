//! BT25-088 Kyo Sawashiro — Tamer, Yellow, Cost 4. Traits: Glowing Dawn, BEATBREAK.
//!
//! # Card text (CARD IMAGE — authoritative; data/cards.json is WRONG for this
//! card and only carried the inherited [Security] line)
//!
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When your security stack is removed from, by suspending this
//!   Tamer, you may place the top 2 cards of your deck face down under this Tamer.
//! [Your Turn] [Once Per Turn] When any of your [Glowing Dawn] trait cards would
//!   be played, by trashing the bottom face-down card from under any of your
//!   Tamers, reduce the cost by 1.
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_088.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - B1 start-of-turn tamer (memory swing — set to 3)
//! - face-down tamer-stash substrate (place_as_bottom_source deck_top x2)
//! - D2 cost reduction (BeforePayCost) with pay_cost (trash face-down under Tamer)
//! - E2 OPT + optional decline (cost reduction)
//! - Tamer [Security] play-self

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-088";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn kyo() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// A non-Digimon security filler — avoids a security battle when it is broken.
fn make_security_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

/// A Glowing Dawn Digimon with a printed play cost — the cost-reduction target.
fn make_glowing_dawn(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = cost;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A non-Glowing-Dawn Digimon with a play cost — used for the negative gate.
fn make_plain(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = cost;
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_088_compiles_as_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
}

#[test]
fn bt25_088_has_start_of_turn_lose_security_and_security_triggered_clauses() {
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
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "start-of-your-turn clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnLoseSecurity]),
        "on-lose-security clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "security play-self clause present"
    );
}

#[test]
fn bt25_088_lose_security_clause_is_optional() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnLoseSecurity])
        .expect("on-lose-security clause present");
    assert!(clause.optional, "'you may place' → optional clause");
}

#[test]
fn bt25_088_has_optional_opt_cost_reduction_clause() {
    let card = compiled(CARD_ID);
    let cr = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                optional,
                once_per_turn,
                amount,
                scope,
                ..
            }) => Some((*optional, *once_per_turn, *amount, *scope)),
            _ => None,
        })
        .expect("cost_reduction declarative clause present");
    assert!(cr.0, "cost reduction is optional ('by trashing ...')");
    assert!(cr.1, "cost reduction is once per turn");
    assert_eq!(cr.2, Some(1), "reduces by 1");
    assert_eq!(cr.3, CompiledScope::FaceUp);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [Start of Your Turn] set memory to 3 (condition gating)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_088_start_of_turn_sets_low_memory_to_three() {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(1)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 1;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 3, "memory <=2 is set to 3");
}

#[test]
fn bt25_088_start_of_turn_does_not_change_high_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 5;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 5, "memory >2 is unchanged (condition gates)");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns] on security loss: suspend self → place top 2 face down
//
// Drive the security loss by having P1 attack P0's security (the bt24_001 idiom).
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_088_lose_security_suspends_self_and_places_top_two_face_down() {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .add_card(make_filler("ATK"))
        .add_card(make_security_filler("SEC-P0"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["D1", "D2"]) // D2 is the deck-top (last element); D1 next
        .deck(1, &["FILLER-DECK"])
        .security(0, &["SEC-P0"])
        .memory(8)
        .start();

    let kyo_perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn(); // → P1's turn
    let attacker = runner.place_on_field(1, "ATK", Some(0));

    let deck_before = runner.deck_size(0);
    runner.attack_player(attacker, 0, false);

    // Optional clause installs an accept/decline prompt.
    assert!(
        runner.game.pending_selection.is_some(),
        "[All Turns] on-lose-security installs an optional accept/decline prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional suspend+place");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[kyo_perm.index as usize].is_suspended,
        "Kyo must be suspended (the activation cost was paid)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "top 2 deck cards placed under Kyo"
    );
    let perm = &runner.game.players[0].battle_area[kyo_perm.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        3,
        "Kyo's own card + 2 face-down stash sources"
    );
    assert!(
        perm.card_sources[0].face_down && perm.card_sources[1].face_down,
        "the two placed sources are face-down at the bottom"
    );
}

#[test]
fn bt25_088_lose_security_decline_does_nothing() {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .add_card(make_filler("ATK"))
        .add_card(make_security_filler("SEC-P0"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["FILLER-DECK"])
        .security(0, &["SEC-P0"])
        .memory(8)
        .start();

    let kyo_perm = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    let attacker = runner.place_on_field(1, "ATK", Some(0));
    let deck_before = runner.deck_size(0);

    runner.attack_player(attacker, 0, false);
    assert!(runner.game.pending_selection.is_some());
    assert!(runner.pending_is_optional());
    runner
        .decline_optional_trigger()
        .expect("decline the optional clause");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[kyo_perm.index as usize].is_suspended,
        "declining leaves Kyo unsuspended"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no cards placed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Your Turn][OPT] Glowing Dawn play cost reduction by trashing FD
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a runner with Kyo on field carrying ONE face-down stash source under it,
/// plus a Glowing Dawn play target in hand.
fn runner_with_kyo_stash(target_cost: u16) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_glowing_dawn("GD", target_cost))
        .add_card(make_filler("STASH"))
        .hand(0, &["GD"])
        .memory(10)
        .start();
    let kyo_perm = runner.place_stack(0, &["STASH", CARD_ID]);
    runner.game.players[0].battle_area[kyo_perm.index as usize].card_sources[0].face_down = true;
    runner
}

fn hand_index(runner: &DebugRunner, player: u8, id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"))
}

#[test]
fn bt25_088_play_cost_reduction_reduces_glowing_dawn_by_one() {
    let mut runner = runner_with_kyo_stash(4);
    let gd_idx = hand_index(&runner, 0, "GD");

    let memory_before = runner.memory();
    let _ = runner.play(0, gd_idx);
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "GD"),
        "GD must be on the field"
    );
    let spent = memory_before - runner.memory();
    assert_eq!(spent, 3, "Glowing Dawn play cost 4 reduced by 1 → spent 3");
    assert_eq!(
        runner.trash_size(0),
        1,
        "the trashed face-down stash source is in trash"
    );
}

#[test]
fn bt25_088_play_cost_not_reduced_for_non_glowing_dawn_card() {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_plain("PLAIN", 4))
        .add_card(make_filler("STASH"))
        .hand(0, &["PLAIN"])
        .memory(10)
        .start();
    let kyo_perm = runner.place_stack(0, &["STASH", CARD_ID]);
    runner.game.players[0].battle_area[kyo_perm.index as usize].card_sources[0].face_down = true;

    let plain_idx = hand_index(&runner, 0, "PLAIN");
    let memory_before = runner.memory();
    let _ = runner.play(0, plain_idx);
    let _ = runner.auto_resolve();

    let spent = memory_before - runner.memory();
    assert_eq!(spent, 4, "non-Glowing-Dawn card pays full cost 4 (no reduction)");
    assert_eq!(runner.trash_size(0), 0, "no face-down stash trashed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [Security] play-self structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_088_clause_on_security_present() {
    let card = compiled(CARD_ID);
    let on_sec = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnSecurity])
    });
    assert!(on_sec, "[Security] play-self clause must compile");
}
