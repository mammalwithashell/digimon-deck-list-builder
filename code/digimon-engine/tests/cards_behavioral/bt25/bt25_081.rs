//! BT25-081 Fangmon — Digimon, Lv.4, Purple, DP 5000, Cost 5.
//! Traits: Dark Animal, BEATBREAK.
//!
//! # Card text (data/cards.json)
//! [On Play] [When Digivolving] Suspend 1 non-purple Tamer.
//! [All Turns] [Once Per Turn] When any of your opponent's Tamers suspend,
//!   gain 1 memory.
//! Inherited: <Retaliation> (When only this Digimon is deleted in battle,
//!   delete the Digimon it battled.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_081.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - OnPlay/WhenDigivolving select-target suspend (color-negated Tamer filter)
//! - B3-adjacent on-suspend event-target-gated memory gain (OPT)
//! - H-row Retaliation (inherited grant)
//!
//! # Verdict — IMPLEMENTED (all clauses)

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-081";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn fangmon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_081_compiles_as_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
}

#[test]
fn bt25_081_has_onplay_whendigivolving_and_onsuspend_clauses() {
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
            .any(|t| t.when == vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]),
        "OnPlay/WhenDigivolving suspend clause present"
    );
    let on_suspend = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSuspend])
        .expect("on_suspend memory clause present");
    assert!(on_suspend.once_per_turn, "memory gain is once per turn");
}

#[test]
fn bt25_081_has_inherited_retaliation() {
    let card = compiled(CARD_ID);
    let has_inherited_keyword = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, .. })
                if *scope == CompiledScope::Inherited
        )
    });
    assert!(
        has_inherited_keyword,
        "inherited grant_keyword clause present (Retaliation)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play][When Digivolving] suspend 1 non-purple Tamer
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_081_on_play_offers_non_purple_tamer_and_suspends_it() {
    let mut runner = DebugRunner::builder()
        .add_card(fangmon())
        .add_card(make_tamer("RED-TAMER", CardColor::Red))
        .add_card(make_tamer("PURPLE-TAMER", CardColor::Purple))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    // P0 owns a red Tamer; P1 owns a purple Tamer. Fangmon is P0's.
    let red = runner.place_on_field(0, "RED-TAMER", Some(0));
    let _purple = runner.place_on_field(1, "PURPLE-TAMER", Some(0));
    let fang = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, fang.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("a suspend-target selection must install");
    // Only the non-purple (red) Tamer is a candidate; the purple Tamer is excluded.
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the non-purple Tamer is a candidate (purple Tamer excluded)"
    );

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("suspend the non-purple Tamer");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[red.index as usize].is_suspended,
        "the chosen non-purple Tamer is suspended"
    );
}

#[test]
fn bt25_081_on_play_no_selection_when_only_purple_tamers() {
    let mut runner = DebugRunner::builder()
        .add_card(fangmon())
        .add_card(make_tamer("PURPLE-TAMER", CardColor::Purple))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    runner.place_on_field(0, "PURPLE-TAMER", Some(0));
    let fang = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, fang.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no non-purple Tamer → no suspend selection installs"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns][OPT] opponent Tamer suspends → gain 1 memory
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_081_gains_memory_when_opponent_tamer_suspends() {
    let mut runner = DebugRunner::builder()
        .add_card(fangmon())
        .add_card(make_tamer("OPP-TAMER", CardColor::Red))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0)); // Fangmon is P0's
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0)); // opponent (P1) Tamer

    let memory_before = runner.memory();
    runner.game.suspend(opp_tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "Fangmon's controller gains 1 memory when the opponent's Tamer suspends"
    );
}

#[test]
fn bt25_081_does_not_gain_memory_when_own_tamer_suspends() {
    let mut runner = DebugRunner::builder()
        .add_card(fangmon())
        .add_card(make_tamer("OWN-TAMER", CardColor::Red))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let own_tamer = runner.place_on_field(0, "OWN-TAMER", Some(0)); // P0's OWN Tamer

    let memory_before = runner.memory();
    runner.game.suspend(own_tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "suspending your OWN Tamer must not gain memory (event_target_owner: opponent gate)"
    );
}

#[test]
fn bt25_081_does_not_gain_memory_when_opponent_digimon_suspends() {
    let mut runner = DebugRunner::builder()
        .add_card(fangmon())
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0)); // opponent DIGIMON, not Tamer

    let memory_before = runner.memory();
    runner.game.suspend(opp_digi);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "suspending an opponent DIGIMON (not Tamer) must not gain memory (event_target_kind: tamer gate)"
    );
}

#[test]
fn bt25_081_memory_gain_is_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .add_card(fangmon())
        .add_card(make_tamer("OPP-TAMER-A", CardColor::Red))
        .add_card(make_tamer("OPP-TAMER-B", CardColor::Red))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-TAMER-A", Some(0));
    let b = runner.place_on_field(1, "OPP-TAMER-B", Some(0));

    let memory_before = runner.memory();
    runner.game.suspend(a);
    let _ = runner.auto_resolve();
    runner.game.suspend(b);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "OPT: only the first opponent-Tamer suspend this turn gains memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited <Retaliation> (behavioral)
//
// Fangmon as a digivolution SOURCE under a higher Digimon grants the host
// <Retaliation>: when the host (only it) is deleted losing a battle, the
// Digimon it battled is also deleted. Mirrors tests/keyword_phase_e/retaliation.rs.
// ═══════════════════════════════════════════════════════════════════════════════

fn make_vanilla(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 5;
    c
}

#[test]
fn bt25_081_inherited_retaliation_deletes_the_winner() {
    let mut runner = DebugRunner::builder()
        .add_card(fangmon())
        .add_card(make_vanilla("HOST", 3000)) // host on top of Fangmon (low DP → loses)
        .add_card(make_vanilla("ATTACKER", 6000)) // opponent attacker (wins)
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    // P0 stack: [Fangmon (bottom/inherited source), HOST (top)].
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    // P1 attacker.
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    // P1's turn so P1 can attack.
    runner.end_turn();
    runner.game.memory = 5;

    // ATTACKER (6000) attacks HOST (3000): HOST loses and is deleted. Fangmon's
    // inherited <Retaliation> fires from under HOST → the ATTACKER is also deleted.
    runner.attack_digimon(attacker, host, false);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ATTACKER"),
        "inherited <Retaliation> must delete the winning attacker"
    );
}
