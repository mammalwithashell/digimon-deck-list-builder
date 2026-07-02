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
//! - Tamer [Security] play-self
//!
//! # Verdict — IMPLEMENTED (2026-06-14)
//! Clause 3 ("[Your Turn][OPT] when a [Glowing Dawn] card would be played, by
//! trashing the bottom face-down card under a Tamer, reduce the cost by 1") is
//! now IMPLEMENTED for the PLAY-FROM-HAND path. Engine gap
//! G-COST-REDUCTION-INTERACTIVE-PAY-COST is closed for that path:
//! `apply_cost_reduction_candidate(..., allow_interactive_pay_cost = true)`
//! defers the reduction credit to the parked pay_cost's success continuation
//! (`game_actions/cost.rs`), so the interactive Tamer pick parks, resolves, and
//! the -1 reduction is applied. Clauses 1, 2, 4 already IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming,
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

/// A vanilla Lv.3 [Glowing Dawn] Digimon of cost 3 — the played card whose cost
/// Kyo's clause 3 reduces.
fn make_glowing_dawn_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["Glowing Dawn".to_string(), "BEATBREAK".to_string()];
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

/// Clause 3 (the interactive-pay_cost cost reducer) now compiles as a
/// `CostReduction` declarative clause. G-COST-REDUCTION-INTERACTIVE-PAY-COST
/// is closed for the play-from-hand path.
#[test]
fn bt25_088_cost_reduction_clause_present() {
    let card = compiled(CARD_ID);
    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        has_cr,
        "the Glowing-Dawn-played cost-reduction clause must now compile"
    );
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
    assert_eq!(
        runner.memory(),
        5,
        "memory >2 is unchanged (condition gates)"
    );
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
    // `pass_turn` (memory → +3 for P1), not a raw `end_turn` (which would flip
    // memory(8) to -8, an impossible real-game state): an attack now correctly
    // ends a turn whose memory sits on the opponent's side, which would
    // otherwise rotate back to P0 and unsuspend Kyo before the assertions run.
    runner.pass_turn(); // → P1's turn
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
    // `pass_turn` (memory → +3 for P1), not a raw `end_turn`: see the sibling
    // test — an attack now ends a turn whose memory is on the opponent's side,
    // so P1 must start their turn with valid non-negative memory.
    runner.pass_turn();
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
// Section 4 — [Security] play-self structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_088_clause_on_security_present() {
    let card = compiled(CARD_ID);
    let on_sec = card.effects.iter().any(
        |c| matches!(c, CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnSecurity]),
    );
    assert!(on_sec, "[Security] play-self clause must compile");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [Your Turn][OPT] Glowing-Dawn-played cost -1 via trashing a bottom
// face-down card under a Tamer (G-COST-REDUCTION-INTERACTIVE-PAY-COST, play path)
// ═══════════════════════════════════════════════════════════════════════════════

/// Kyo on field (with one face-down stash beneath) reduces the cost of a played
/// [Glowing Dawn] Digimon by 1 — the interactive Tamer-pick pay_cost parks,
/// resolves, the face-down stash is trashed, AND the -1 reduction is credited.
#[test]
fn bt25_088_reduces_glowing_dawn_play_cost_by_trashing_face_down() {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_glowing_dawn_digimon("GD"))
        .add_card(make_filler("STASH"))
        .hand(0, &["GD"])
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(5)
        .start();

    // Kyo (top of [STASH, Kyo]) with one face-down stash source beneath it.
    let kyo_perm = runner.place_stack(0, &["STASH", CARD_ID]);
    runner.game.players[0].battle_area[kyo_perm.index as usize].card_sources[0].face_down = true;

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);
    assert_eq!(mem_before, 5);

    // Play the cost-3 Glowing Dawn Digimon from hand → installs the
    // "Use ... to reduce play cost?" optional accept/decline prompt.
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "playing a [Glowing Dawn] card installs the cost-reduction accept prompt"
    );
    assert!(runner.pending_is_optional(), "the -1 reducer is optional");
    runner
        .accept_optional_trigger()
        .expect("accept the -1 cost reduction");
    // The pay_cost parks on the single-eligible-Tamer pick — resolve it.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the bottom face-down stash source was trashed as the cost"
    );
    assert_eq!(
        runner.memory(),
        mem_before - 2,
        "cost 3 reduced by 1 → paid 2 memory (NOT 3): the reduction is credited \
         even though the pay_cost parked on the Tamer pick"
    );
}

/// Control: with NO face-down stash beneath any Tamer the pay_cost is unpayable,
/// so accepting the prompt yields no reduction — the card is played at full cost
/// and nothing is trashed. Guards against crediting a reduction that was never
/// paid for.
#[test]
fn bt25_088_no_reduction_when_no_face_down_stash() {
    let mut runner = DebugRunner::builder()
        .add_card(kyo())
        .add_card(make_glowing_dawn_digimon("GD"))
        .add_card(make_filler("STASH"))
        .hand(0, &["GD"])
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(5)
        .start();

    // Kyo on field with NO face-down stash.
    runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    // The reducer may still offer its prompt; accepting cannot pay (no stash).
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "no face-down stash → nothing trashed"
    );
    assert_eq!(
        runner.memory(),
        mem_before - 3,
        "no stash → cost paid in full (3), no reduction credited"
    );
}
