//! BT25-035 Cougarmon — Digimon, Lv.4, Yellow, DP 6000, Cost 5.
//! Traits: Mammal, Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json, confirmed vs DCGO)
//! [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for
//!   the turn. Then, by trashing 2 bottom face-down cards from under any of your
//!   Tamers, this Digimon may digivolve into a [Glowing Dawn] trait Digimon card
//!   in the hand without paying the cost.            <-- "Then" half BLOCKED
//! Inherited: <Barrier>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_035.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - OnPlay/WhenDigivolving -DP debuff (turn-scoped)
//! - alt-digivolve from Glowing Dawn Lv.3
//! - H14 inherited Barrier (structural)
//!
//! # Verdict — PARTIAL
//! The "Then, by trashing 2 bottom face-down cards … may digivolve into a
//! [Glowing Dawn] card in hand for free" half is BLOCKED on
//! G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER (qa/dsl-vocab-gaps.md). Omitted rather
//! than under-charging (trash 1 instead of 2). The -3000 DP, inherited Barrier,
//! and Glowing Dawn alt-digivolve are IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-035";

fn cougarmon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
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

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_035_compiles_as_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
}

#[test]
fn bt25_035_has_onplay_whendigivolving_and_inherited_barrier() {
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
        "OnPlay/WhenDigivolving -DP clause present"
    );
    let has_inherited_keyword = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, .. })
                if *scope == CompiledScope::Inherited
        )
    });
    assert!(has_inherited_keyword, "inherited grant_keyword (Barrier) present");
}

#[test]
fn bt25_035_has_glowing_dawn_alt_digivolve() {
    let card = compiled(CARD_ID);
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve)),
        "alt-digivolve path present"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play][When Digivolving] -3000 DP to an opponent Digimon
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_035_on_play_debuffs_chosen_opponent_digimon_by_3000() {
    let mut runner = DebugRunner::builder()
        .add_card(cougarmon())
        .add_card(make_opp_digimon("OPP", 6000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");
    let cougar = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, cougar.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("a -3000 DP target selection installs (opponent Digimon present)");
    let target = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("an opponent Digimon target exists");
    runner
        .execute_action(view.selecting_player, target)
        .expect("apply -3000 DP");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 3000,
        "the chosen opponent Digimon gets -3000 DP"
    );
}

#[test]
fn bt25_035_dp_debuff_expires_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .add_card(cougarmon())
        .add_card(make_opp_digimon("OPP", 6000))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");
    let cougar = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, cougar.index as usize);
    let view = runner.pending_selection_view().expect("target prompt");
    let target = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("target");
    runner.execute_action(view.selecting_player, target).expect("apply");
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp).expect("opp dp"), dp_before - 3000);

    // End P0's turn → the for-the-turn debuff expires.
    runner.end_turn();
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before,
        "-3000 DP must expire at end of turn"
    );
}

#[test]
fn bt25_035_no_prompt_when_no_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(cougarmon())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let cougar = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, cougar.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → no -DP prompt"
    );
}
