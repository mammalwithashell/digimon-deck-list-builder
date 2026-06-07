//! BT25-033 Aegiomon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Traits: Shaman, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! ＜Barrier＞ (When this Digimon would be deleted in battle, by trashing your
//! top security card, it isn't deleted.)
//! [On Play] [When Digivolving] By adding your top security card to the hand,
//! 1 of your opponent's Digimon gets -5000 DP for the turn.
//!
//! Inherited: ＜Barrier＞.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_033.cs
//!
//! - Alt digivolve (None): TS-trait Lv.3 cost 2 (unprinted alt-source; printed
//!   standard Lv.3 Yellow / cost 2 authored).
//! - Barrier (own + inherited): BarrierSelfEffect.
//! - Shared OP/WD (optional, gated SecurityCards.Any()): add top security to
//!   hand, then -5000 DP to 1 opp Digimon (UntilEachTurnEnd).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - E2 optional "By [cost]" clause with security-add cost
//! - F8 Barrier (own + inherited)
//! - D1 conditional DP debuff (turn-scoped)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "BT25-033";

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-033 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(opp_digimon("OPP-DIGI", 6000))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_033_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Aegiomon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    for t in ["Shaman", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_033_grants_own_and_inherited_barrier() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let mut own = false;
    let mut inherited = false;
    for c in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) = c
        {
            if keyword.eq_ignore_ascii_case("Barrier") {
                match scope {
                    CompiledScope::FaceUp => own = true,
                    CompiledScope::Inherited => inherited = true,
                    _ => {}
                }
            }
        }
    }
    assert!(own, "own <Barrier>");
    assert!(inherited, "inherited <Barrier>");
}

#[test]
fn bt25_033_has_optional_op_wd_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD clause present");
    assert!(clause.optional, "the 'By adding...' clause is optional");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
}

// ─── Section 2 — Condition gating (security cost) ────────────────────────────

/// Negative: with no security cards the clause's `security_count_gte: 1` gate
/// blocks; no DP-debuff prompt installs.
#[test]
fn bt25_033_no_security_blocks_clause() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &[])
        .memory(8)
        .start();
    // opponent has a Digimon so a target exists if the clause fired.
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let perm = runner.play(0, 0).expect("play Aegiomon");
    assert!(
        runner.pending_selection().is_none(),
        "no security → optional clause does not install the DP prompt"
    );
}

/// Positive: with a security card the optional clause installs (the accept /
/// target prompt surfaces).
#[test]
fn bt25_033_with_security_installs_optional_prompt() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"])
        .memory(8)
        .start();
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let _perm = runner.play(0, 0).expect("play Aegiomon");
    assert!(
        runner.pending_selection().is_some(),
        "with security present the optional OP/WD clause installs a prompt"
    );
}

// ─── Section 3 — Behavioral outcome ──────────────────────────────────────────

/// Taking the effect adds the top security to hand (security -1, hand +1) and
/// applies -5000 DP to the chosen opponent Digimon for the turn.
#[test]
fn bt25_033_effect_adds_security_to_hand_and_debuffs_opponent() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"])
        .memory(8)
        .start();
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");
    let sec_before = runner.security_count(0);

    let _perm = runner.play(0, 0).expect("play Aegiomon");
    // After play, Aegiomon has left the hand. Capture hand size now; the effect
    // then adds the top security card to hand (+1).
    let hand_after_play = runner.game.players[0].hand.len();
    runner.auto_resolve().ok();

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "top security card was added to hand (cost paid)"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_after_play + 1,
        "the added security card landed in hand"
    );
    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 5000,
        "the chosen opp Digimon gets -5000 DP for the turn"
    );
}

/// The -5000 DP debuff expires at end of turn.
#[test]
fn bt25_033_debuff_expires_end_of_turn() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"])
        .memory(8)
        .start();
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");

    let _perm = runner.play(0, 0).expect("play Aegiomon");
    runner.auto_resolve().ok();
    assert_eq!(runner.effective_dp(opp).expect("dp"), dp_before - 5000);

    runner.end_turn();
    assert_eq!(
        runner.effective_dp(opp).expect("dp"),
        dp_before,
        "the -5000 DP modifier expires at end of turn"
    );
}
