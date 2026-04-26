//! Phase F §F2 — `Keyword::Iceclad` combat-resolution branch tests.
//!
//! Iceclad (RULES_CONTEXT 16-34): when a Digimon with Iceclad is in a
//! Digimon-vs-Digimon battle, both sides compare digivolution-card stack
//! lengths (`card_sources.len()`) instead of effective DP. Higher count
//! wins; equal count = mutual destruction. Security-Digimon battles are
//! NOT affected — those route through `resolve_player_security_loop`
//! which compares DP directly.
//!
//! Iceclad does NOT auto-install via `keyword_to_auto_effect`; it is
//! consumed at the combat resolver site (`Game::resolve_battle`) via a
//! direct `has_keyword` query — collapsing DCGO's `IcecladStaticEffect`
//! registration into a binary swap of comparator inputs.
//!
//! Source-stacking pattern: `place_on_field` returns a permanent with a
//! single `CardSource` (the top). Tests push extra `CardSource`s onto
//! `permanent.card_sources` directly to simulate a digivolution stack of
//! arbitrary depth. Mirrors the pattern in
//! `tests/cost_hooks/cost_reduction_fn.rs` and `tests/zone_manipulation.rs`.

use digimon_engine::card_source::CardSource;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;

use super::helpers::{digimon_with_keywords, plain_digimon};

/// Push `extra` additional `CardSource`s of the given filler card under the
/// top card of a placed permanent. After this call, `card_sources.len() ==
/// 1 + extra` (top + extras). The extras live UNDER the top (inserted at
/// position 0), mirroring how digivolve-up would lay them down.
fn stack_extra_sources(r: &mut DebugRunner, handle: PermanentHandle, filler_id: &str, extra: usize) {
    let game = r.game_mut();
    let filler_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == filler_id)
        .unwrap_or_else(|| panic!("stack_extra_sources: unknown card_id {filler_id}"));
    for _ in 0..extra {
        let ci = game.next_card_index();
        let src = CardSource::new(filler_idx, handle.player, ci);
        // Insert UNDER the top — position 0 puts it at the bottom of the
        // stack (matches `Permanent::push_under`). For source-count tests
        // the position is irrelevant; only `card_sources.len()` matters.
        game.players[handle.player as usize].battle_area[handle.index as usize]
            .card_sources
            .insert(0, src);
    }
}

fn iceclad_card(id: &str, dp: i32) -> CardData {
    digimon_with_keywords(id, 5, dp, vec![Keyword::Iceclad])
}

// ─── Test 1: Iceclad attacker wins despite lower DP via higher source count ──

/// Iceclad attacker (1000 DP, 3 sources total) vs plain defender (8000 DP,
/// 1 source). With Iceclad the comparator is source count: 3 > 1 →
/// attacker wins; defender deleted; attacker survives.
#[test]
fn iceclad_higher_card_count_wins_despite_lower_dp() {
    let atk = iceclad_card("ICE_ATK", 1000);
    let def = {
        let mut c = plain_digimon("BIG_DEF");
        c.dp = Some(8000);
        c
    };
    let filler = plain_digimon("FILLER");

    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(def)
        .add_card(filler)
        .start();

    let atk_h = r.place_on_field(0, "ICE_ATK", Some(0));
    let def_h = r.place_on_field(1, "BIG_DEF", Some(0));

    // Stack 2 extra sources under ICE_ATK → 3 total; BIG_DEF stays at 1.
    stack_extra_sources(&mut r, atk_h, "FILLER", 2);
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources.len(),
        3,
        "ICE_ATK should have 3 stacked sources before battle"
    );
    assert_eq!(
        r.game.players[1].battle_area[0].card_sources.len(),
        1,
        "BIG_DEF should have 1 source before battle"
    );

    let result = r.attack_digimon(atk_h, def_h, false);

    assert_eq!(result, AttackResult::AttackerWins);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "ICE_ATK survives — it has more sources despite less DP"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "BIG_DEF deleted — fewer sources loses under Iceclad"
    );
}

// ─── Test 2: Iceclad with equal source counts → mutual destruction ───────────

/// Iceclad attacker and plain defender, equal source counts. Iceclad
/// active → comparator is source count: 1 == 1 → mutual destruction.
#[test]
fn iceclad_equal_count_mutual_destruction() {
    // Use disparate DP values to prove DP is NOT the comparator: if Iceclad
    // were ignored, attacker (1000) < defender (5000) and we'd see
    // DefenderWins; with Iceclad active and equal source counts, we get
    // MutualDestruction.
    let atk = iceclad_card("ICE_ATK", 1000);
    let def = {
        let mut c = plain_digimon("DEF");
        c.dp = Some(5000);
        c
    };

    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(def)
        .start();

    let atk_h = r.place_on_field(0, "ICE_ATK", Some(0));
    let def_h = r.place_on_field(1, "DEF", Some(0));

    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 1);

    let result = r.attack_digimon(atk_h, def_h, false);

    assert_eq!(result, AttackResult::MutualDestruction);
    assert_eq!(r.game.players[0].battle_area.len(), 0);
    assert_eq!(r.game.players[1].battle_area.len(), 0);
}

// ─── Test 3: Iceclad attacker loses with fewer sources despite higher DP ─────

/// High-DP Iceclad attacker (8000 DP, 1 source) vs plain defender (1000
/// DP, 3 sources). With Iceclad, comparator is sources: 1 < 3 →
/// defender wins; attacker deleted.
#[test]
fn iceclad_lower_count_loses_despite_higher_dp() {
    let atk = iceclad_card("ICE_ATK", 8000);
    let def = {
        let mut c = plain_digimon("STACKED_DEF");
        c.dp = Some(1000);
        c
    };
    let filler = plain_digimon("FILLER");

    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(def)
        .add_card(filler)
        .start();

    let atk_h = r.place_on_field(0, "ICE_ATK", Some(0));
    let def_h = r.place_on_field(1, "STACKED_DEF", Some(0));

    stack_extra_sources(&mut r, def_h, "FILLER", 2);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 3);

    let result = r.attack_digimon(atk_h, def_h, false);

    assert_eq!(result, AttackResult::DefenderWins);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "ICE_ATK deleted — fewer sources loses under Iceclad even with higher DP"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "STACKED_DEF survives via source-count advantage"
    );
}

// ─── Test 4: security battle path is NOT affected by Iceclad ─────────────────

/// Iceclad attacker (low DP, many sources) attacks the player. The
/// security check pops a security Digimon with high DP and 1 source.
/// The security-battle path (`resolve_player_security_loop` →
/// `SecurityPhase::BattleResolved`) compares attacker DP vs revealed
/// security card DP — Iceclad is NOT consulted there.
///
/// Expectation: attacker loses to the security Digimon by DP comparison
/// — proving the security battle path ignored Iceclad. If Iceclad
/// leaked into the security path, attacker would win on source count
/// (3 > 1) instead.
#[test]
fn iceclad_does_not_affect_security_battle() {
    let atk = iceclad_card("ICE_ATK", 1000);
    let strong_sec = {
        let mut c = plain_digimon("STRONG_SEC");
        c.dp = Some(10_000);
        c
    };
    let filler = plain_digimon("FILLER");

    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(strong_sec)
        .add_card(filler)
        .security(1, &["STRONG_SEC"])
        .start();

    let atk_h = r.place_on_field(0, "ICE_ATK", Some(0));
    stack_extra_sources(&mut r, atk_h, "FILLER", 2);
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources.len(),
        3,
        "ICE_ATK has 3 sources"
    );

    let result = r.attack_player(atk_h, 1, false);

    // Security battle uses DP only: 1000 < 10_000 → attacker deleted.
    // If Iceclad had leaked into this path, we'd see SecurityCheckSurvived
    // (attacker wins source-count compare 3 > 1).
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "Security-battle path must compare DP, not Iceclad source counts"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "ICE_ATK deleted by security Digimon (DP comparison)"
    );
    assert_eq!(r.security_count(1), 0, "security stack drained");
}
