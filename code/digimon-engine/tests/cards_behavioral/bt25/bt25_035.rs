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
//! # Verdict — IMPLEMENTED (2026-06-15)
//! The "Then, by trashing 2 bottom face-down cards … may digivolve into a
//! [Glowing Dawn] card in hand for free" half is now IMPLEMENTED: DSL-vocab gap
//! G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER is closed by the new verb
//! `trash_bottom_face_down_sources_under_tamers: { of, count }` (multi-Tamer
//! N-total bottom face-down trash, surfacing each Tamer pick as a real
//! PendingSelection), composed with `select_own_permanent` (the carrier itself)
//! + `select_hand` (the "may", carries decline) + `effect_initiated_digivolve`
//! `{ target: source, cost: free }`. The -3000 DP, inherited Barrier, and
//! Glowing Dawn alt-digivolve were already IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

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

/// A Lv.5 Yellow [Glowing Dawn] Digimon — the legal hand digivolve target for
/// the "may digivolve into a [Glowing Dawn] card for free" half. Carries a
/// printed digivolution cost from Lv.4 so the FREE digivolve is observable as a
/// zero-memory cost.
fn gd_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "GD Lv5");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 7;
    c.traits = vec!["Glowing Dawn".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 2, // Yellow
        level: 4,
        memory_cost: 4,
    }];
    c
}

/// A Lv.5 Yellow NON-Glowing-Dawn Digimon (same route) — illegal pick.
fn non_gd_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Non GD Lv5");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 7;
    c.traits = vec!["Mammal".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 4,
        memory_cost: 4,
    }];
    c
}

/// A vanilla Tamer to host face-down stash sources for the trash-2 cost.
fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

/// Total face-down digivolution sources under the player's Tamers.
fn face_down_source_count(runner: &DebugRunner, p: u8) -> usize {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .map(|perm| perm.card_sources.iter().filter(|s| s.face_down).count())
        .sum()
}

/// Build a runner with Cougarmon on the field as a battle-area top (the carrier
/// that will digivolve), plus the requested digivolve targets in hand, plus
/// the requested set of Tamers each carrying `face_down_each` face-down sources.
fn cougar_runner(
    targets: &[&str],
    tamers_fd: &[usize],
) -> (DebugRunner, PermanentHandle, Vec<PermanentHandle>) {
    let mut builder = DebugRunner::builder()
        .add_card(cougarmon())
        .add_card(make_opp_digimon("OPP", 6000))
        .add_card(make_filler("FILLER"))
        .add_card(gd_lv5("GD5"))
        .add_card(non_gd_lv5("NONGD5"))
        .add_card(tamer("TAMER"));
    builder = builder
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10);
    let mut runner = builder.start();
    runner.set_first_player(0);

    // Cougarmon as a Lv.4 battle-area top (place a Lv.3 filler underneath so it
    // is a genuine stack with the carrier on top).
    let cougar = runner.place_stack(0, &["FILLER", CARD_ID]);

    // Tamers, each with `n` face-down stash sources beneath.
    let mut tamers = Vec::new();
    for &n in tamers_fd {
        let mut names: Vec<&str> = vec!["FILLER"; n];
        names.push("TAMER");
        let t = runner.place_stack(0, &names);
        for i in 0..n {
            runner.game.players[0].battle_area[t.index as usize].card_sources[i].face_down = true;
        }
        tamers.push(t);
    }

    for &tgt in targets {
        push_to_hand(&mut runner, 0, tgt);
    }
    (runner, cougar, tamers)
}

fn process_contains_trash_n_under_tamers(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::TrashBottomFaceDownSourcesUnderTamers { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_trash_n_under_tamers(then)
                || process_contains_trash_n_under_tamers(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_trash_n_under_tamers(body)
        }
        _ => false,
    })
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
    assert!(
        has_inherited_keyword,
        "inherited grant_keyword (Barrier) present"
    );
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

#[test]
fn bt25_035_then_clause_uses_multi_tamer_trash_2_cost() {
    let card = compiled(CARD_ID);
    let shared = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared OnPlay/WhenDigivolving clause");
    assert!(
        process_contains_trash_n_under_tamers(&shared.process),
        "the 'Then' half pays a multi-Tamer trash-2 face-down cost"
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
    runner
        .execute_action(view.selecting_player, target)
        .expect("apply");
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

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — "Then, by trashing 2 bottom face-down cards … may digivolve into a
//             [Glowing Dawn] card in the hand without paying the cost."
//
// These tests fire the shared clause with NO opponent Digimon present, so the
// mandatory -3000 DP target prompt is skipped and the "Then" half resolves on
// its own. `cougar_runner` places Cougarmon as a battle-area top.
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE — 2 face-down sources under ONE Tamer: pick the Tamer (twice over),
/// trash 2 from its bottom, then Cougarmon digivolves into the [Glowing Dawn]
/// Lv.5 hand card for FREE (no memory paid).
#[test]
fn bt25_035_trash_2_from_one_tamer_then_free_digivolve() {
    let (mut runner, cougar, tamers) = cougar_runner(&["GD5", "NONGD5"], &[2]);
    let tamer = tamers[0];
    let mem_before = runner.memory();
    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "2 face-down sources staged"
    );

    runner.fire_on_play(0, cougar.index as usize);

    // "may digivolve" — the optional hand pick installs first. Only GD5 is legal.
    let view = runner
        .pending_selection_view()
        .expect("'may digivolve' hand pick installs");
    assert!(view.is_optional, "'may digivolve' ⇒ declinable");
    assert_eq!(
        view.valid_action_ids
            .iter()
            .filter(|&&id| id != PASS)
            .count(),
        1,
        "only the Lv.5 [Glowing Dawn] hand card is a legal pick; got {:?}",
        view.valid_action_ids
    );
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .unwrap();
    runner.execute_action(0, pick).expect("choose GD5");

    // Now the trash-2 cost: a Tamer pick installs (the only eligible Tamer).
    // Resolve every follow-up Tamer pick.
    let _ = runner.auto_resolve();

    let perm = &runner.game.player(0).battle_area[cougar.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "GD5",
        "Cougarmon digivolves into the [Glowing Dawn] Lv.5 hand card"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        0,
        "both face-down sources under the Tamer are trashed (cost)"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "the digivolve is FREE — no memory paid for the digivolution"
    );
    // The trashed Tamer-stash sources land in trash.
    assert!(
        runner.trash_size(0) >= 2,
        "the 2 trashed face-down sources are in the trash"
    );
}

/// POSITIVE — 1 face-down source under EACH of TWO Tamers: the cost is paid by
/// trashing 1 from each (the distributed branch). Both Tamer picks surface.
#[test]
fn bt25_035_trash_1_from_each_of_two_tamers_then_free_digivolve() {
    let (mut runner, cougar, tamers) = cougar_runner(&["GD5"], &[1, 1]);
    assert_eq!(tamers.len(), 2);
    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "1 face-down under each Tamer"
    );
    let mem_before = runner.memory();

    runner.fire_on_play(0, cougar.index as usize);
    let view = runner.pending_selection_view().expect("hand pick installs");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .unwrap();
    runner.execute_action(0, pick).expect("choose GD5");

    // First Tamer pick.
    let v1 = runner
        .pending_selection_view()
        .expect("first Tamer pick installs (multi-Tamer trash)");
    assert!(
        v1.valid_action_ids.iter().filter(|&&id| id != PASS).count() >= 2,
        "both Tamers are eligible for the first trash pick"
    );
    runner
        .execute_action(
            0,
            v1.valid_action_ids
                .iter()
                .copied()
                .find(|&id| id != PASS)
                .unwrap(),
        )
        .expect("pick first Tamer");

    // Second Tamer pick (the other Tamer, since the first now has no face-down).
    let v2 = runner
        .pending_selection_view()
        .expect("second Tamer pick installs (1+1 distribution)");
    runner
        .execute_action(
            0,
            v2.valid_action_ids
                .iter()
                .copied()
                .find(|&id| id != PASS)
                .unwrap(),
        )
        .expect("pick second Tamer");
    let _ = runner.auto_resolve();

    let perm = &runner.game.player(0).battle_area[cougar.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "GD5",
        "Cougarmon digivolves into the [Glowing Dawn] Lv.5 hand card"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        0,
        "1 face-down source trashed from each of the two Tamers"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "free digivolve — no memory paid"
    );
}

/// DECLINABLE ("may"): PASS on the hand pick leaves Cougarmon as-is, no trash,
/// no digivolve.
#[test]
fn bt25_035_then_clause_is_declinable() {
    let (mut runner, cougar, _tamers) = cougar_runner(&["GD5"], &[2]);
    let mem_before = runner.memory();

    runner.fire_on_play(0, cougar.index as usize);
    let view = runner
        .pending_selection_view()
        .expect("optional hand pick surfaces");
    assert!(view.is_optional);
    runner.execute_action(0, PASS).expect("declining is legal");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.player(0).battle_area[cougar.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "declined ⇒ still Cougarmon (no digivolve)"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "declined ⇒ no face-down source trashed"
    );
    assert_eq!(runner.memory(), mem_before, "declined ⇒ nothing paid");
}

/// NEGATIVE (cost gate): only 1 total face-down source across all Tamers ⇒ the
/// trash-2 cost is unpayable, so the "Then" half never offers the digivolve.
#[test]
fn bt25_035_then_clause_blocked_when_fewer_than_2_face_down_sources() {
    let (mut runner, cougar, _tamers) = cougar_runner(&["GD5"], &[1]);
    assert_eq!(
        face_down_source_count(&runner, 0),
        1,
        "only 1 face-down source"
    );
    let mem_before = runner.memory();

    runner.fire_on_play(0, cougar.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "trash-2 cost unpayable (only 1 face-down) ⇒ no stuck digivolve prompt"
    );
    assert_eq!(
        runner.game.player(0).battle_area[cougar.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "no digivolve when the trash-2 cost is unpayable"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        1,
        "the single face-down source is NOT trashed"
    );
    assert_eq!(runner.memory(), mem_before);
}

/// CLONE-FAITHFULNESS (make-engine-cloneable): the multi-Tamer trash-2 cost is
/// driven by the resumable data VM, so a `Game::clone` taken BETWEEN the first
/// and second Tamer pick is faithful — resolving the clone's second pick runs
/// the free digivolve on the clone without touching the original, and the
/// original then replays to the identical state. This is the property MCTS /
/// AlphaZero search relies on (cloning at a mid-cost decision point). Before the
/// installer flip the first pick was closure-only, so the clone would hit the
/// panic-stub `PendingSelection::clone`.
#[test]
fn bt25_035_trash_n_clones_faithfully_between_tamer_picks() {
    let (mut runner, cougar, _tamers) = cougar_runner(&["GD5"], &[1, 1]);
    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "1 face-down under each Tamer"
    );
    let mem_before = runner.memory();

    runner.fire_on_play(0, cougar.index as usize);
    let view = runner.pending_selection_view().expect("hand pick installs");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .unwrap();
    runner.execute_action(0, pick).expect("choose GD5");

    // First Tamer pick → resolve it (trashes 1, re-parks the second pick).
    let v1 = runner
        .pending_selection_view()
        .expect("first Tamer pick installs");
    let t1 = v1
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .unwrap();
    runner.execute_action(0, t1).expect("pick first Tamer");

    // BETWEEN picks: the second Tamer pick is installed AND resume-driven, so
    // cloning here is faithful. The digivolve is deferred to the final pick.
    assert!(
        runner.game.pending_selection.is_some(),
        "second Tamer pick installed between picks"
    );
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "the multi-pick trash must be resume-driven between picks (clone-safe)"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        1,
        "exactly one face-down source trashed after the first pick"
    );
    assert_eq!(
        runner.game.player(0).battle_area[cougar.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "still Cougarmon between picks (digivolve deferred to the final pick)"
    );

    // Clone, then resolve the second pick on the CLONE only.
    let v2 = runner
        .pending_selection_view()
        .expect("second Tamer pick installs");
    let t2 = v2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != PASS)
        .unwrap();
    let mut clone = runner.game.clone();
    clone
        .resolve_selection(0, t2)
        .expect("clone's second pick resolves");
    assert_eq!(
        clone.player(0).battle_area[cougar.index as usize]
            .top_card()
            .card_id(&clone.card_data),
        "GD5",
        "clone: Cougarmon digivolves into GD5 after the final pick"
    );

    // INDEPENDENCE: the original is untouched by resolving the clone.
    assert!(
        runner.game.pending_selection.is_some(),
        "original's second pick survives cloning + resolving the clone"
    );
    assert_eq!(
        runner.game.player(0).battle_area[cougar.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "original is still mid-cost (Cougarmon) after the clone resolves"
    );

    // REPLAYS IDENTICALLY: resolving the same pick on the original reaches the
    // clone's state.
    runner
        .execute_action(0, t2)
        .expect("original's second pick resolves");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.player(0).battle_area[cougar.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "GD5",
        "original reaches the same GD5 digivolve as the clone"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        0,
        "both sources trashed"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "free digivolve — no memory paid"
    );
}

/// NEGATIVE (target filter): no [Glowing Dawn] Lv.5 in hand ⇒ clean no-op even
/// with 2 face-down sources available.
#[test]
fn bt25_035_then_clause_no_glowing_dawn_in_hand_clean_noop() {
    let (mut runner, cougar, _tamers) = cougar_runner(&["NONGD5"], &[2]);

    runner.fire_on_play(0, cougar.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no legal [Glowing Dawn] hand pick ⇒ no stuck selection"
    );
    assert_eq!(
        runner.game.player(0).battle_area[cougar.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID,
        "nothing digivolved"
    );
    assert_eq!(
        face_down_source_count(&runner, 0),
        2,
        "no digivolve ⇒ no face-down sources trashed"
    );
}
