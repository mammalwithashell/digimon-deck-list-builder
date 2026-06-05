//! Main Phase — the dense FAQ core. Ledger rows MP-01..MP-32.
//!
//! This first slice pins the three genuinely-uncovered, high-value rules the
//! coverage audit flagged (`qa/qa-reports/rules-faq.md`):
//!   MP-19  DP cannot go negative; a Digimon at 0 DP is deleted.
//!   MP-22  Memory is capped at 10; excess is lost.
//!   MP-28  Two simultaneous end-of-turn DP modifiers (+ and −) expire
//!          TOGETHER with no intermediate 0-DP deletion (CANARY vs the latent
//!          17-1-2-2 mid-expiry timing bug).
//!
//! Vehicle: AD1-001 Greymon (real implemented DSL card) — base DP read at
//! runtime so the assertions are independent of the printed value.

#![allow(unused_imports)]

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, GamePhase, ModifierType, PlayerId};
use digimon_engine::modifiers::ModifierEntry;

fn opponent(p: PlayerId) -> PlayerId {
    if p == 0 {
        1
    } else {
        0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MP-19 — "Is it possible for a Digimon's DP to be negative? No, zero is the
// minimum. A Digimon with a DP of zero is deleted." (General Rules/FAQ, Main
// Phase.) Rules manual §1-7-3 / §17-1-3-1: a Digimon at ≤0 DP is deleted by the
// state-based rules-check.
// ─────────────────────────────────────────────────────────────────────────────

/// A Digimon driven to exactly 0 DP is deleted by the state-based rules-check
/// (run during the end-of-turn effect-queue drain).
#[test]
fn mp19_digimon_at_zero_dp_is_deleted() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .memory(3)
        .start();

    let p = r.turn_player();
    let h = r.place_on_field(p, "AD1-001", Some(0));
    let base = r.effective_dp(h).expect("Greymon has a DP value");
    assert!(base > 0, "vehicle precondition: positive base DP");

    // Drive DP to exactly 0 with a permanent −base ChangeDp modifier. No
    // rules-check has run yet (place/add don't trigger one), so it is still on
    // the field at 0 DP.
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, -base, Expiry::Permanent, p as u8),
    );
    assert_eq!(r.effective_dp(h), Some(0), "DP should be exactly 0");
    assert_eq!(
        r.battle_area_size(p),
        1,
        "still present before any rules-check boundary"
    );

    // End of turn drains the effect queue, which runs the ≤0-DP state-based
    // rules-check at the outermost drain boundary.
    r.end_turn();

    assert_eq!(
        r.battle_area_size(p),
        0,
        "FAQ Main-Phase: a Digimon with 0 DP is deleted"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// MP-22 — "If a card effect gives me more than 10 memory, what happens? You
// can't have more than 10 memory. Any memory exceeding 10 is lost." (General
// Rules/FAQ, Main Phase.) Rules manual §6-2-2: the memory gauge tops out at 10.
// ─────────────────────────────────────────────────────────────────────────────

/// Memory gained past 10 is clamped to 10; the excess is lost.
#[test]
fn mp22_memory_is_capped_at_ten() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .memory(8)
        .start();

    // Turn player gains 5 more memory: 8 + 5 = 13, which must clamp to 10.
    let p = r.turn_player();
    r.game.gain_memory_for_player(p, 5);

    assert_eq!(
        r.memory(),
        10,
        "FAQ Main-Phase: memory cannot exceed 10; the excess is lost"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// MP-28 (CANARY) — "I have a Digimon with multiple effects that last 'for the
// turn'... Do these effects end in a certain order at the end of the turn?
// There's no order. The effects end simultaneously at the end of the turn. If
// you had a Digimon with two effects, one that added DP and one that subtracted
// it, even if losing the positive DP bonus would cause the Digimon to reach 0
// DP and leave play, the effects would end simultaneously, returning your
// Digimon to its original DP." (General Rules/FAQ, Main Phase.) Rules manual
// §17-1-2-2 (simultaneous expiry; no intermediate state-based check).
//
// Probes the known latent timing bug ("add_dp_modifier deletes mid-effect"):
// if end-of-turn modifier expiry interleaves a rules-check between removing the
// +DP and the −DP entry, the Digimon would be wrongly deleted. The FAQ-correct
// outcome is survival at the original DP. Discover-then-pin: assert survival.
// ─────────────────────────────────────────────────────────────────────────────

/// Two end-of-turn DP modifiers (one +, one −) chosen so that removing the
/// positive alone would drop the Digimon to ≤0 DP. They must expire together
/// with no transient deletion: the Digimon survives at its original DP.
#[test]
fn mp28_simultaneous_eot_dp_modifiers_no_intermediate_deletion() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .memory(3)
        .start();

    let p = r.turn_player();
    let h = r.place_on_field(p, "AD1-001", Some(0));
    let base = r.effective_dp(h).expect("Greymon has a DP value");

    // neg > base  ⇒ base − neg ≤ 0  (removing the + alone would delete).
    // pos chosen so base + pos − neg > 0 during the turn (Digimon is alive).
    let neg = base + 1000;
    let pos = base + 2000;
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, pos, Expiry::EndOfTurn, p as u8),
    );
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, -neg, Expiry::EndOfTurn, p as u8),
    );

    let during = base + pos - neg;
    assert!(during > 0, "vehicle precondition: alive during the turn");
    assert_eq!(r.effective_dp(h), Some(during), "net DP while both active");

    // End of turn: both EndOfTurn entries expire simultaneously.
    r.end_turn();

    assert_eq!(
        r.battle_area_size(p),
        1,
        "CANARY: simultaneous EoT expiry must NOT transiently delete the Digimon \
         (FAQ Main-Phase; rules §17-1-2-2). If this fails it is a discovered \
         engine gap — log to engine-gaps.md, do not weaken the assertion."
    );
    assert_eq!(
        r.effective_dp(h),
        Some(base),
        "Digimon returns to its original DP after both modifiers expire"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// MP-13 — "Can I attack my opponent's Digimon? You can attack your opponent's
// Digimon so long as they're suspended. You can't attack your opponent's
// unsuspended Digimon." (General Rules/FAQ, Main Phase.) Enforced at the action
// mask (`action/mask.rs` — a Digimon attack target is legal only when
// `target.is_suspended` absent a granting effect).
// ─────────────────────────────────────────────────────────────────────────────

/// A SUSPENDED opponent Digimon is a legal attack target; an UNSUSPENDED one is
/// not (asserted against the action mask, the layer that gates declarations).
#[test]
fn mp13_can_only_attack_suspended_opponent_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .start();

    let p = r.turn_player();
    let opp = opponent(p);

    // Attacker on the turn player's field, Rush-granted so summoning sickness
    // doesn't suppress the attack actions entirely.
    let atk = r.place_on_field(p, "AD1-001", Some(0));
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(ModifierType::GrantRush, 0, Expiry::Permanent, p as u8),
    );

    // Two opponent Digimon: one suspended, one not.
    let suspended = r.place_on_field(opp, "AD1-001", Some(0));
    let unsuspended = r.place_on_field(opp, "AD1-001", Some(0));
    r.game.players[opp as usize].battle_area[suspended.index as usize].is_suspended = true;
    // `unsuspended` keeps the default is_suspended == false.

    r.game.current_phase = GamePhase::Main;
    r.game.tick_declarative_effects();

    let mask = build_action_mask(&r.game, p);
    assert_eq!(
        mask[encode_attack(atk.index as u16, suspended.index as u16) as usize],
        1.0,
        "FAQ: a suspended opponent Digimon CAN be attacked"
    );
    assert_eq!(
        mask[encode_attack(atk.index as u16, unsuspended.index as u16) as usize],
        0.0,
        "FAQ: an UNSUSPENDED opponent Digimon cannot be attacked"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// MP-08 — "If a card has a play/use cost of 11 or more, can I use it if your
// memory gauge is at zero? No. From zero, you will only be able to move your
// memory counter to 10 on your opponent's side, but you need to move it to 11."
// (General Rules/FAQ, Main Phase.) The memory floor is −10 (`rules.memory_range.0`),
// so a cost-11+ card from 0 memory is masked out; with enough memory it is
// playable — isolating the memory gate on the same card.
// ─────────────────────────────────────────────────────────────────────────────

/// A cost-15 card (AD1-025 Omnimon) is unplayable from 0 memory (would require
/// −15, past the −10 floor) but becomes playable once memory affords it.
#[test]
fn mp08_cost_eleven_or_more_unplayable_from_zero_memory() {
    // Hand the card to both seats so whichever is the turn player holds it at
    // hand index 0 (turn player is seed-determined and only known post-start).
    let from_zero = DebugRunner::builder()
        .dsl_card("AD1-025")
        .expect("AD1-025 (Omnimon, cost 15) in embedded DSL pack")
        .hand(0, &["AD1-025"])
        .hand(1, &["AD1-025"])
        .memory(0)
        .start();
    let p0 = from_zero.turn_player();
    let mut g0 = from_zero;
    g0.game.current_phase = GamePhase::Main;
    let mask_zero = build_action_mask(&g0.game, p0);
    assert_eq!(
        mask_zero[0], 0.0,
        "FAQ: a cost-15 card cannot be played from 0 memory (needs −15, floor is −10)"
    );

    // Control: with 10 memory the same card is affordable (10 − 15 = −5 ≥ −10).
    let with_mem = DebugRunner::builder()
        .dsl_card("AD1-025")
        .expect("AD1-025 (Omnimon) in embedded DSL pack")
        .hand(0, &["AD1-025"])
        .hand(1, &["AD1-025"])
        .memory(10)
        .start();
    let p1 = with_mem.turn_player();
    let mut g1 = with_mem;
    g1.game.current_phase = GamePhase::Main;
    let mask_mem = build_action_mask(&g1.game, p1);
    assert_eq!(
        mask_mem[0], 1.0,
        "control: the same cost-15 card IS playable once memory affords it"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// MP-03 — "During my breeding phase, I move a Digimon from my breeding area to
// the battle area. Can I attack with it this turn? Yes. Moving a Digimon to your
// battle area doesn't count as playing it, so you're free to attack with it."
// (General Rules/FAQ.) A promoted Digimon has no summoning sickness.
// ─────────────────────────────────────────────────────────────────────────────

/// A Digimon promoted from the breeding area can attack the same turn (the
/// attack action is legal in the mask).
#[test]
fn mp03_promoted_from_breeding_can_attack_this_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX4-005")
        .expect("EX4-005 (Agumon, Lv3) in embedded DSL pack")
        .start();
    let p = r.turn_player();
    let opp = opponent(p);

    // A Lv3 in breeding, then promoted to the battle area.
    r.place_in_breeding(p, "EX4-005");
    assert!(r.game.move_from_breeding(p), "promote the Lv3 to the battle area");
    let promoted = r.perm_handle(p, 0);

    // A suspended opponent Digimon to attack.
    let target = r.place_on_field(opp, "EX4-005", Some(0));
    r.game.players[opp as usize].battle_area[target.index as usize].is_suspended = true;

    r.game.current_phase = GamePhase::Main;
    r.game.tick_declarative_effects();

    let mask = build_action_mask(&r.game, p);
    assert_eq!(
        mask[encode_attack(promoted.index as u16, target.index as u16) as usize],
        1.0,
        "FAQ MP-03: a Digimon promoted from breeding can attack the same turn"
    );
}
