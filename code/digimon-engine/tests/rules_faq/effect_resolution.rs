//! Effect resolution — surface R. Ledger rows MP-30/31, plus MP-11/12/21/25/29.
//!
//! "My opponent has two Digimon in play. Can I use an effect that targets two
//! of my opponent's Digimon and only choose one of them? It depends on the
//! effect. If the effect reads '2 of your opponent's Digimon,' it must affect
//! two Digimon. If the effect reads 'up to 2 ...,' you can choose any number up
//! to 2." (General Rules/FAQ, Main Phase — MP-31.) And MP-30: an effect that
//! targets two can still activate with fewer than two in play (affecting all
//! available). Rules manual §8 (mandatory effects affect as many as possible).
//!
//! Vehicle: BT24-051 Merukimon — "[On Play][When Digivolving] Suspend 2 of your
//! opponent's Digimon or Tamers." A mandatory-TWO effect.

#![allow(unused_imports)]

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, PlayerId};
use digimon_engine::selection::TriggerSource;

fn opponent(p: PlayerId) -> PlayerId {
    if p == 0 {
        1
    } else {
        0
    }
}

/// MP-31 — a "Suspend **2** of your opponent's Digimon" effect MUST affect two
/// when two are available: after picking one, PASS must NOT yet be offered.
#[test]
fn mp31_mandatory_two_target_must_affect_two_when_available() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT24-051")
        .expect("BT24-051 (Merukimon) in embedded DSL pack")
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .start();

    let p = r.turn_player();
    let opp = opponent(p);
    let src = r.place_on_field(p, "BT24-051", Some(0));
    // TWO opponent Digimon are available to suspend.
    let _o1 = r.place_on_field(opp, "AD1-001", Some(0));
    let _o2 = r.place_on_field(opp, "AD1-001", Some(0));

    r.fire_on_play(p, src.index as usize);
    r.game.drain_effect_queue();

    let sel = r
        .pending_selection()
        .expect("the 'Suspend 2 of opponent' selection should install");
    let selp = sel.selecting_player;
    let first_target = *sel
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("at least one suspend target");
    r.execute_action(selp, first_target)
        .expect("first target pick resolves");

    // After ONE pick with TWO targets available, the effect must still require
    // the SECOND pick. Early finish is signalled by `is_optional` (PASS allowed
    // once `picked >= effective_min`), not by a PASS id in `valid_action_ids`.
    match r.pending_selection() {
        Some(sel2) => assert!(
            !sel2.is_optional,
            "FAQ MP-31: 'Suspend 2' must affect two when two are available — \
             allowing an early finish after only one pick (is_optional=true) lets \
             the player suspend just one"
        ),
        None => panic!(
            "FAQ MP-31: the selection completed after a single pick — a mandatory \
             'Suspend 2' must require the second target"
        ),
    }

    // Completing the second pick suspends BOTH opponent Digimon.
    let second = *r
        .pending_selection()
        .expect("second pick still pending")
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("second target available");
    r.execute_action(selp, second).expect("second pick resolves");
    let suspended = (0..r.battle_area_size(opp))
        .filter(|&i| r.game.players[opp as usize].battle_area[i].is_suspended)
        .count();
    assert_eq!(
        suspended, 2,
        "FAQ MP-31: a mandatory 'Suspend 2' suspends both opponent Digimon"
    );
}

/// MP-30 — an effect that targets two can still activate with FEWER than two in
/// play: "Suspend 2" with only ONE opponent Digimon suspends that one (it does
/// not fizzle).
#[test]
fn mp30_mandatory_two_target_affects_the_one_available() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT24-051")
        .expect("BT24-051 (Merukimon) in embedded DSL pack")
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .start();

    let p = r.turn_player();
    let opp = opponent(p);
    let src = r.place_on_field(p, "BT24-051", Some(0));
    // Only ONE opponent Digimon is available.
    let only = r.place_on_field(opp, "AD1-001", Some(0));

    r.fire_on_play(p, src.index as usize);
    r.game.drain_effect_queue();

    let sel = r
        .pending_selection()
        .expect("FAQ MP-30: 'Suspend 2' still activates with one target available");
    let selp = sel.selecting_player;
    let target = *sel
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("the single opponent Digimon is a target");
    r.execute_action(selp, target).expect("pick resolves");

    assert!(
        r.game.players[opp as usize].battle_area[only.index as usize].is_suspended,
        "FAQ MP-30: 'Suspend 2' with one target in play suspends that one"
    );
}

/// MP-29 — "[When Digivolving] Suspend 1 of your opponent's Digimon" (no "may")
/// is MANDATORY: the engine must not let the controller decline when a target
/// exists. Vehicle: BT21-037 Lighdramon (DCGO isOptional=false; the YAML even
/// documents "optional: false when targets exist").
#[test]
fn mp29_mandatory_when_digivolving_suspend_is_not_optional() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT21-037")
        .expect("BT21-037 (Lighdramon) in embedded DSL pack")
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .start();
    let p = r.turn_player();
    let opp = opponent(p);
    let src = r.place_on_field(p, "BT21-037", Some(0));
    let _victim = r.place_on_field(opp, "AD1-001", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();

    let sel = r
        .pending_selection()
        .expect("the mandatory 'Suspend 1 opponent Digimon' selection should install");
    assert!(
        !sel.is_optional,
        "FAQ MP-29: a mandatory [When Digivolving] 'Suspend 1' (no 'may') must NOT \
         expose a decline when a target exists"
    );
}
