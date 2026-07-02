//! BT25-025 Aegiochusmon: Blue — Digimon, Lv.5, Blue/Purple, DP 8000, Cost 8.
//! Traits: Shaman, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! ＜Blocker＞
//! ＜Decode ([Aegiomon])＞ (When this Digimon would leave the battle area other
//! than in battle, you may play 1 [Aegiomon] from its digivolution cards
//! without paying the cost.)
//! [On Play] [When Digivolving] ＜De-Digivolve 1＞ 1 of your opponent's Digimon.
//! Then, if you have 3 or fewer security cards, 1 of your Digimon unsuspends.
//!
//! Inherited: [All Turns] [Once Per Turn] When your security stack is removed
//! from, 1 of your [Shaman] trait Digimon may unsuspend.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_025.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H5 Blocker (own keyword)
//! - C3-adjacent special <Decode> replacement (play named source on leave)
//! - De-Digivolve 1 mandatory + security-gated unsuspend
//! - F9/inherited security-removed OPT unsuspend

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "BT25-025";

fn opp_stack_card(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 4;
    c
}

fn own_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["Shaman".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-025 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(opp_stack_card("OPP-LV4", 4))
        .add_card(opp_stack_card("OPP-LV3", 3))
        .add_card(own_digimon("OWN-SHAMAN"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_025_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Aegiochusmon: Blue");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
}

#[test]
fn bt25_025_grants_blocker() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
            if keyword.eq_ignore_ascii_case("Blocker") && *scope == CompiledScope::FaceUp
    ));
    assert!(has, "own <Blocker>");
}

#[test]
fn bt25_025_has_replacement_for_decode() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )
    });
    assert!(has, "Decode is modeled as a Replacement clause");
}

#[test]
fn bt25_025_has_inherited_security_removed_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOwnSecurityRemoved) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited security-removed clause");
    assert!(clause.once_per_turn, "OPT");
    assert!(clause.optional, "may unsuspend");
}

// ─── Section 2 — Behavior: De-Digivolve + security-gated unsuspend ───────────

/// On Play with a 2-card opponent stack present, the mandatory De-Digivolve
/// prompt installs (an opp Digimon exists).
#[test]
fn bt25_025_on_play_installs_de_digivolve_prompt() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"]) // 2 ≤ 3 → unsuspend also fires
        .memory(10)
        .start();
    // Opponent: a 2-high stack (Lv4 over Lv3) so De-Digivolve has a legal target.
    runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);

    let _perm = runner.play(0, 0).expect("play Aegiochusmon");
    assert!(
        runner.pending_selection().is_some(),
        "the mandatory De-Digivolve 1 prompt installs when an opp Digimon exists"
    );
}

/// De-Digivolve trashes the top card of the opponent's stack (sources -1).
#[test]
fn bt25_025_de_digivolve_trashes_top_of_opponent_stack() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"]) // 4 > 3 → no unsuspend
        .memory(10)
        .start();
    let opp = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    let sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    let _perm = runner.play(0, 0).expect("play Aegiochusmon");
    runner.auto_resolve().ok();

    let sources_after = runner.game.players[1]
        .battle_area
        .get(opp.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);
    assert_eq!(
        sources_after,
        sources_before - 1,
        "De-Digivolve 1 trashes the top card of the opponent's Digimon"
    );
}

/// Security gate: with 4 security (>3) the post-De-Digivolve unsuspend does NOT
/// fire, so after resolving only the De-Digivolve prompt was present.
#[test]
fn bt25_025_unsuspend_blocked_when_more_than_three_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"]) // 4 > 3
        .memory(10)
        .start();
    runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    // A suspended own Digimon that COULD be unsuspended if the gate passed.
    let own = runner.place_on_field(0, "OWN-SHAMAN", Some(0));
    runner.game.players[0].battle_area[own.index as usize].is_suspended = true;

    let _perm = runner.play(0, 0).expect("play Aegiochusmon");
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0].battle_area[own.index as usize].is_suspended,
        "with >3 security the conditional unsuspend does not fire"
    );
}

/// Security gate positive: with ≤3 security the conditional unsuspend fires and
/// the suspended own Digimon is unsuspended.
#[test]
fn bt25_025_unsuspend_fires_when_three_or_fewer_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"]) // 2 ≤ 3
        .memory(10)
        .start();
    runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);
    let own = runner.place_on_field(0, "OWN-SHAMAN", Some(0));
    runner.game.players[0].battle_area[own.index as usize].is_suspended = true;

    let _perm = runner.play(0, 0).expect("play Aegiochusmon");
    runner.auto_resolve().ok();

    assert!(
        !runner.game.players[0].battle_area[own.index as usize].is_suspended,
        "with ≤3 security the conditional unsuspend fires"
    );
}
