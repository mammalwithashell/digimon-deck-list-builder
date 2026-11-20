//! BT22-056 Guardromon -- Digimon, Lv.4, Black/Yellow, DP 5000, Cost 5,
//! traits: Machine, CS.
//!
//! # Card text (official Bandai DB, data/card_bundles/BT22-056.md)
//!
//! Digivolve Lv.3 Black / Yellow: cost 3; Lv.3 w/[CS] trait: cost 2.
//! [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for
//! the turn. Then, if this Digimon's stack has 2 or more same-level cards,
//! <De-Digivolve 1> 1 of your opponent's Digimon.
//! Inherited: [Opponent's Turn] This Digimon gets +2000 DP.
//! Official Q&A: the stack count includes the Digimon's own top card ("a level
//! 5 Digimon has a level 5 digivolution card").
//!
//! DCGO: BT22/*/BT22_056.cs. Found by the hunt-2 replay of a real game: the
//! opponent's Guardromon in hand was unimplemented (no legal digivolve), so the
//! replay stopped. New substrate: `self_stack_same_level_count_gte`
//! (G-DSL-SELF-STACK-SAME-LEVEL-COUNT).

#[path = "../bt26/support.rs"]
mod support;

use digimon_engine::action::space::ATTACK_START;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "BT22-056";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-056 must load from the embedded DSL pack")
        .add_card(filler("FILLER"))
        // Opponent Digimon: a Lv.5 (so one de-digivolve step has a level to
        // fall to) with a Lv.3 source.
        .add_card(digimon("OPP-5", "Opp Five", CardColor::Red, 5, 6, &[]))
        .add_card(digimon("OPP-3", "Opp Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP-LOW", "Opp Low", CardColor::Red, 4, 4, &[]))
        // The digivolve base: a Lv.3 [CS] Digimon (cost-2 alt route).
        .add_card(digimon("BASE-CS3", "Base CS3", CardColor::White, 3, 3, &["CS"]))
        // Stack fillers.
        .add_card(digimon("SRC-4", "Source L4", CardColor::White, 4, 4, &[]))
        .add_card(digimon("SRC-5", "Source L5", CardColor::White, 5, 5, &[]))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(10)
}

fn opp_stack(r: &mut DebugRunner) -> (PermanentHandle, PermanentHandle) {
    let big = r.place_on_field(1, "OPP-5", Some(0));
    r.push_source(big, "OPP-3");
    let small = r.place_on_field(1, "OPP-LOW", Some(0));
    (big, small)
}

fn pick_opp(r: &mut DebugRunner, h: PermanentHandle) {
    // An opponent-field selection answers with `ATTACK_START + slot`.
    let action = ATTACK_START + h.index as u16;
    let v = r.pending_selection_view().expect("opponent Digimon pick");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} must be a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(0, action).unwrap();
}

/// Digivolve BASE-CS3 into Guardromon, with `sources` pushed under the base
/// first (so they end up as digivolution cards of the Guardromon stack).
fn digivolve_guardromon(r: &mut DebugRunner, sources: &[&str]) -> PermanentHandle {
    r.set_first_player(0);
    let base = r.place_on_field(0, "BASE-CS3", Some(0));
    for s in sources {
        r.push_source(base, s);
    }
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == CARD_ID)
        .unwrap();
    assert!(
        r.game
            .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve),
        "Guardromon must digivolve onto a Lv.3 [CS] base (alt route, cost 2)"
    );
    base
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn bt22_056_digivolves_from_a_level_3_cs_digimon_for_cost_2() {
    let mut r = builder().start();
    r.set_first_player(0);
    let mem_before = r.memory();
    let base = r.place_on_field(0, "BASE-CS3", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    assert!(r
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve));
    assert_eq!(top_id(&r, base), CARD_ID);
    // The cost-2 alt route is the cheapest accepted payment; the printed
    // circles (Black / Yellow Lv.3, cost 3) do not match a White base.
    assert_eq!(r.memory(), mem_before - 2, "alt route costs 2");
}

// ─── [When Digivolving] -3000 DP ─────────────────────────────────────────────

#[test]
fn bt22_056_when_digivolving_gives_one_opponent_digimon_minus_3000_dp() {
    let mut r = builder().start();
    let (big, small) = opp_stack(&mut r);
    let big_before = r.effective_dp(big).unwrap();
    let small_before = r.effective_dp(small).unwrap();
    let _ = digivolve_guardromon(&mut r, &[]);

    // Mandatory: no PASS offered.
    let v = r.pending_selection_view().expect("-3000 DP pick");
    assert!(!v.is_optional, "the -3000 DP pick is mandatory");
    pick_opp(&mut r, small);
    let _ = r.auto_resolve();

    assert_eq!(r.effective_dp(small).unwrap(), small_before - 3000);
    assert_eq!(r.effective_dp(big).unwrap(), big_before, "only the picked Digimon");
}

#[test]
fn bt22_056_does_nothing_when_the_opponent_has_no_digimon() {
    let mut r = builder().start();
    let base = digivolve_guardromon(&mut r, &[]);
    assert!(r.pending_selection().is_none(), "no opposing Digimon");
    assert_eq!(top_id(&r, base), CARD_ID);
}

// ─── Stack same-level gate for <De-Digivolve 1> ──────────────────────────────

/// Guardromon (Lv.4 top) with a Lv.4 digivolution card: the top card counts
/// toward "2 or more same-level cards" (official Q&A), so <De-Digivolve 1>
/// follows the -3000.
#[test]
fn bt22_056_a_same_level_source_under_the_top_card_enables_de_digivolve() {
    let mut r = builder().start();
    let (big, small) = opp_stack(&mut r);
    let _ = digivolve_guardromon(&mut r, &["SRC-4"]);

    pick_opp(&mut r, small); // -3000
    // Second mandatory pick: the de-digivolve target.
    let v = r.pending_selection_view().expect("de-digivolve pick");
    assert!(!v.is_optional, "the <De-Digivolve 1> pick is mandatory");
    pick_opp(&mut r, big);
    let _ = r.auto_resolve();

    assert_eq!(
        top_id(&r, big),
        "OPP-3",
        "<De-Digivolve 1> trashed the Lv.5 top card, leaving the Lv.3 below"
    );
}

/// Two sources of one level under a different-level top also qualify (a Lv.4
/// Guardromon with two Lv.5 digivolution cards).
#[test]
fn bt22_056_two_same_level_sources_enable_de_digivolve() {
    let mut r = builder().start();
    let (big, small) = opp_stack(&mut r);
    let _ = digivolve_guardromon(&mut r, &["SRC-5", "SRC-5"]);
    pick_opp(&mut r, small);
    let _ = r.pending_selection_view().expect("de-digivolve pick");
    pick_opp(&mut r, big);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, big), "OPP-3");
}

/// All stack levels distinct (Lv.4 top over a Lv.3 base and a Lv.5 source):
/// no same-level pair -> only the -3000 resolves.
#[test]
fn bt22_056_no_same_level_pair_means_no_de_digivolve() {
    let mut r = builder().start();
    let (big, small) = opp_stack(&mut r);
    let _ = digivolve_guardromon(&mut r, &["SRC-5"]);
    pick_opp(&mut r, small);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none(), "no de-digivolve pick");
    assert_eq!(top_id(&r, big), "OPP-5", "the opponent's stack is untouched");
}

/// Face-down cards have no characteristics (CR 4.3 4-7-9): a face-down Lv.4
/// card does not pair with the Lv.4 top.
#[test]
fn bt22_056_a_face_down_same_level_card_does_not_count() {
    let mut r = builder().start();
    let (big, small) = opp_stack(&mut r);
    r.set_first_player(0);
    let base = r.place_on_field(0, "BASE-CS3", Some(0));
    r.push_source(base, "SRC-4");
    {
        let perm = &mut r.game.players[0].battle_area[base.index as usize];
        let under = perm.card_sources.len() - 1;
        for s in perm.card_sources[..under].iter_mut() {
            s.face_down = true;
        }
    }
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    assert!(r
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve));
    pick_opp(&mut r, small);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert_eq!(top_id(&r, big), "OPP-5");
}

// ─── [On Play] ───────────────────────────────────────────────────────────────

/// Played from hand there is no digivolution card under it: just the -3000.
#[test]
fn bt22_056_on_play_gives_minus_3000_dp() {
    let mut r = builder().start();
    r.set_first_player(0);
    let (big, small) = opp_stack(&mut r);
    let small_before = r.effective_dp(small).unwrap();
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    r.game.enter_main_phase();
    r.play(0, idx).expect("play Guardromon");
    pick_opp(&mut r, small);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(small).unwrap(), small_before - 3000);
    assert!(r.pending_selection().is_none(), "a lone card has no pair");
    assert_eq!(top_id(&r, big), "OPP-5");
}

// ─── Inherited [Opponent's Turn] +2000 DP ────────────────────────────────────

#[test]
fn bt22_056_inherited_effect_is_a_plus_2000_aura_on_the_opponents_turn() {
    let mut r = builder().start();
    r.skip_mulligan();
    // Bottom card = Guardromon (a digivolution card), top = a plain Lv.4.
    let host = r.place_stack(0, &[CARD_ID, "SRC-4"]);
    let own_turn = {
        r.set_first_player(0);
        r.effective_dp(host).unwrap()
    };
    r.set_first_player(1);
    let opp_turn = r.effective_dp(host).unwrap();
    assert_eq!(
        opp_turn - own_turn,
        2000,
        "[Opponent's Turn] +2000 DP while Guardromon is a digivolution card"
    );
}
