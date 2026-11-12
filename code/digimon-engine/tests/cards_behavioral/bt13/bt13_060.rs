//! BT13-060 Rosemon: Burst Mode
//! [When Attacking] Trash the top card of your opponent's security stack for
//! every 2 of your opponent's suspended Digimon and Tamers.
//! DCGO: BT13_060.cs:218-266 (mandatory, count = suspended (Digimon|Tamer) / 2).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const YAML: &str = include_str!("../../../cards/bt13/BT13-060.yaml");

fn runner() -> DebugRunner {
    let sec = make_test_card("SEC-FILLER", "SecurityFiller");
    let mut digi = make_test_card("OPP-DIGI", "OppDigi");
    digi.card_kind = CardKind::Digimon;
    digi.dp = Some(1000);
    digi.level = Some(3);
    let mut tamer = make_test_card("OPP-TAMER", "OppTamer");
    tamer.card_kind = CardKind::Tamer;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT13-060 YAML loads")
        .add_card(sec)
        .add_card(digi)
        .add_card(tamer)
        .security(0, &["SEC-FILLER"; 5])
        .security(1, &["SEC-FILLER"; 5])
        .memory(10)
        .build();
    r.game.turn_count = 1;
    r
}

/// Place `digis` opposing Digimon (the first is the attack target) and `tamers`
/// opposing Tamers, all suspended, then attack the first Digimon with Rosemon.
/// Returns P1's (opponent) security count before and after.
fn attack_with(digis: usize, tamers: usize, extra_unsuspended: usize) -> (usize, usize) {
    let mut r = runner();
    let attacker = r.place_on_field(0, "BT13-060", Some(0));
    let mut target = None;
    for _ in 0..digis {
        let h = r.place_on_field(1, "OPP-DIGI", None);
        r.game.players[1].battle_area[h.index as usize].is_suspended = true;
        target.get_or_insert(h);
    }
    for _ in 0..tamers {
        let h = r.place_on_field(1, "OPP-TAMER", None);
        r.game.players[1].battle_area[h.index as usize].is_suspended = true;
    }
    for _ in 0..extra_unsuspended {
        r.place_on_field(1, "OPP-DIGI", None);
    }
    let before = r.security_count(1);
    r.attack_digimon(attacker, target.expect("a suspended target"), false);
    let _ = r.auto_resolve();
    (before, r.security_count(1))
}

#[test]
fn bt13_060_when_attacking_trashes_one_security_per_two_suspended_digimon_and_tamers() {
    // 1 suspended Digimon + 1 suspended Tamer = 2 -> 1 security trashed.
    let (before, after) = attack_with(1, 1, 0);
    assert_eq!(after, before - 1, "2 suspended Digimon/Tamers -> trash 1");
}

#[test]
fn bt13_060_when_attacking_scales_and_ignores_unsuspended() {
    // 2 suspended Digimon + 2 suspended Tamers = 4 -> 2; 3 unsuspended ignored.
    let (before, after) = attack_with(2, 2, 3);
    assert_eq!(after, before - 2, "4 suspended Digimon/Tamers -> trash 2");
}

#[test]
fn bt13_060_when_attacking_odd_count_rounds_down() {
    // 1 suspended Digimon + 1 suspended Tamer + 1 more suspended Digimon = 3 -> 1.
    let (before, after) = attack_with(2, 1, 0);
    assert_eq!(after, before - 1, "3 suspended -> floor(3/2) = 1");
}

#[test]
fn bt13_060_when_attacking_trashes_nothing_below_two() {
    let (before, after) = attack_with(1, 0, 2);
    assert_eq!(after, before, "1 suspended -> floor(1/2) = 0 trashed");
}

// ─────────────────────────────────────────────────────────────────────────────
// [Burst Digivolve] (CR 6-5-1-2-3, 8-3) through the real action surface.
// Shared scenario: `crate::burst_digivolve_common`.
// ─────────────────────────────────────────────────────────────────────────────

mod burst_digivolve {
    use crate::burst_digivolve_common::*;
    use digimon_engine::enums::CardColor;

    const SPEC: BurstSpec = BurstSpec {
        card: "BT13-060",
        host_name: "Rosemon",
        host_color: CardColor::Green,
        tamer_name: "Yoshino Fujieda",
    };

    /// 8-3-3: offered via the mask with [Yoshino Fujieda] on the field; returns the
    /// Tamer, pays 0, draws 1, stacks on the [Rosemon].
    #[test]
    fn bt13_060_burst_offered_returns_tamer_costs_zero_draws_one() {
        burst_and_check(SPEC, Host::BurstOnly);
    }

    /// The burst cost can't be paid without a [Yoshino Fujieda].
    #[test]
    fn bt13_060_burst_not_offered_without_tamer() {
        assert_not_offered_without_tamer(SPEC);
    }

    /// 8-3-2-1: the top card is trashed at the end of the burst turn,
    /// tracked by permanent identity (the host shifted slots).
    #[test]
    fn bt13_060_burst_top_card_trashed_at_turn_end() {
        assert_turn_end_trash(SPEC, Host::BurstOnly);
    }

    /// Normal (cost 5) and burst routes both legal: a real player choice.
    #[test]
    fn bt13_060_burst_vs_normal_route_is_a_player_choice() {
        assert_route_choice(SPEC);
    }

    /// Choosing the burst route from the route prompt, then the turn-end trash.
    #[test]
    fn bt13_060_burst_chosen_from_route_prompt_then_trashed() {
        assert_turn_end_trash(SPEC, Host::AlsoNormal);
    }

    /// 8-3-2-2: an effect that digivolves can't burst digivolve.
    #[test]
    fn bt13_060_effect_digivolve_cannot_burst() {
        assert_effect_cannot_burst(SPEC);
    }

    /// Two qualifying Tamers: the player picks which returns.
    #[test]
    fn bt13_060_burst_two_tamers_is_a_player_choice() {
        assert_two_tamers_choice(SPEC);
    }
}
