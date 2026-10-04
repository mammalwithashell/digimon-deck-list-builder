//! BT2-041 ShineGreymon — Digimon, Lv.6, Yellow, Cost 12, DP 11000,
//! Light Dragon / Vaccine. Digivolve: Yellow Lv.5, cost 4.
//!
//! Printed text (official Bandai DB, `data/card_bundles/BT2-041.md`):
//!   [When Digivolving] Suspend all of your yellow Tamers. For each Tamer you
//!   suspend this way, activate the following effect:
//!   ・1 of your opponent's Digimon gets -4000 DP for the turn.
//!   [Your Turn] This Digimon gets +1000 DP for each Tamer you have in play.
//!
//! Official Q&A: the [When Digivolving] effect activates separately for each
//! yellow Tamer you suspend, and you choose the target for each effect.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT2/Yellow/BT2_041.cs
//!   - OnEnterFieldAnyone / CanTriggerWhenDigivolving; CanActivate requires an
//!     unsuspended yellow Tamer. Collect own unsuspended yellow Tamers, suspend
//!     them all (SuspendPermanentsClass), then `actionCount = suspended count`;
//!     loop actionCount times: SelectPermanentEffect over opponent Digimon,
//!     maxCount 1, canNoSelect false → ChangeDigimonDP -4000 UntilEachTurnEnd.
//!   - Static: ChangeSelfDPStaticEffect +1000 × (all own Tamers), IsOwnerTurn.
//!
//! Patterns: when_digivolving mass own-suspend + per-suspended repeated
//! mandatory target pick (repeat_effect_choice single-label loop); [Your Turn]
//! self DP aura scaling by Tamer count.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::{EffectTiming, TriggerSource};

const CARD_ID: &str = "BT2-041";

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.dp = None;
    c.level = None;
    c
}

fn digimon(id: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT2-041 DSL card")
        .add_card(tamer("Y-TAMER-A", CardColor::Yellow))
        .add_card(tamer("Y-TAMER-B", CardColor::Yellow))
        .add_card(tamer("Y-TAMER-C", CardColor::Yellow))
        .add_card(tamer("R-TAMER", CardColor::Red))
        .add_card(digimon("Y-LV5", 5, 7000, CardColor::Yellow))
        .add_card(digimon("OPP-A", 5, 9000, CardColor::Blue))
        .add_card(digimon("OPP-B", 5, 9000, CardColor::Blue))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.player(h.player).battle_area[h.index as usize].is_suspended
}

/// Stack ShineGreymon on the yellow Lv.5 base and fire its [When
/// Digivolving] trigger through the engine queue. (DSL fixtures carry no
/// printed evo-cost rows — those come from cards.json in production — so the
/// digivolve itself is staged; the trigger path is the real one.)
fn digivolve(r: &mut DebugRunner, base: PermanentHandle) -> PermanentHandle {
    let _ = base;
    let carrier = r.place_stack(0, &["Y-LV5", CARD_ID]);
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    r.game.drain_effect_queue();
    carrier
}

// ── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn bt2_041_structure() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiles");
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(card.level, Some(6));

    let wd: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(wd.len(), 1);
    assert!(wd[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!wd[0].optional, "[When Digivolving] is mandatory");
    assert!(wd[0]
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RepeatEffectChoice { .. })));

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
        )),
        "[Your Turn] DP aura present"
    );
}

// ── Section 2/3: [When Digivolving] ────────────────────────────────────────

#[test]
fn bt2_041_wd_suspends_all_unsuspended_yellow_tamers_only() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    let ya = r.place_on_field(0, "Y-TAMER-A", Some(0));
    let yb = r.place_on_field(0, "Y-TAMER-B", Some(0));
    let red = r.place_on_field(0, "R-TAMER", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    digivolve(&mut r, base);
    let _ = r.auto_resolve();
    assert!(suspended(&r, ya), "yellow Tamer A suspended");
    assert!(suspended(&r, yb), "yellow Tamer B suspended");
    assert!(!suspended(&r, red), "red Tamer is not suspended");
}

#[test]
fn bt2_041_wd_two_suspended_tamers_give_two_separate_mandatory_picks() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.place_on_field(0, "Y-TAMER-A", Some(0));
    r.place_on_field(0, "Y-TAMER-B", Some(0));
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let ob = r.place_on_field(1, "OPP-B", Some(0));
    digivolve(&mut r, base);

    // First activation.
    let p1 = r.pending_selection().expect("first -4000 pick");
    assert_eq!(p1.selecting_player, 0);
    assert!(
        !p1.is_optional,
        "target pick is mandatory (canNoSelect false)"
    );
    assert_eq!(p1.valid_action_ids.len(), 2, "either opponent Digimon");
    let a = p1.valid_action_ids[0];
    r.execute_action(0, a).expect("pick 1");
    // Second activation, chosen independently.
    let p2 = r.pending_selection().expect("second -4000 pick");
    assert!(!p2.is_optional);
    assert_eq!(p2.valid_action_ids.len(), 2);
    let b = p2.valid_action_ids[1];
    r.execute_action(0, b).expect("pick 2");
    assert!(r.pending_selection().is_none(), "exactly 2 activations");

    let total = r.effective_dp(oa).unwrap() + r.effective_dp(ob).unwrap();
    assert_eq!(total, 18000 - 8000, "two -4000 debuffs applied in total");
    assert_eq!(r.effective_dp(oa), Some(5000));
    assert_eq!(r.effective_dp(ob), Some(5000));
}

#[test]
fn bt2_041_wd_same_target_may_be_chosen_twice() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.place_on_field(0, "Y-TAMER-A", Some(0));
    r.place_on_field(0, "Y-TAMER-B", Some(0));
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    digivolve(&mut r, base);
    for _ in 0..2 {
        let p = r.pending_selection().expect("pick");
        let a = p.valid_action_ids[0];
        r.execute_action(0, a).expect("pick OPP-A");
    }
    assert!(r.pending_selection().is_none());
    assert_eq!(r.effective_dp(oa), Some(1000), "9000 - 4000 - 4000");
}

#[test]
fn bt2_041_wd_already_suspended_yellow_tamer_does_not_count() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    let ya = r.place_on_field(0, "Y-TAMER-A", Some(0));
    let yb = r.place_on_field(0, "Y-TAMER-B", Some(0));
    r.game.player_mut(0).battle_area[yb.index as usize].is_suspended = true;
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    digivolve(&mut r, base);
    let p = r.pending_selection().expect("one pick for Tamer A");
    let a = p.valid_action_ids[0];
    r.execute_action(0, a).expect("pick");
    assert!(
        r.pending_selection().is_none(),
        "only 1 Tamer suspended this way → only 1 activation"
    );
    assert!(suspended(&r, ya));
    assert_eq!(r.effective_dp(oa), Some(5000));
}

#[test]
fn bt2_041_wd_no_unsuspended_yellow_tamer_no_activation() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    let ya = r.place_on_field(0, "Y-TAMER-A", Some(0));
    r.game.player_mut(0).battle_area[ya.index as usize].is_suspended = true;
    r.place_on_field(0, "R-TAMER", Some(0));
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    digivolve(&mut r, base);
    assert!(
        r.pending_selection().is_none(),
        "nothing suspended → no -DP pick"
    );
    assert_eq!(r.effective_dp(oa), Some(9000));
}

#[test]
fn bt2_041_wd_three_tamers_three_activations() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.place_on_field(0, "Y-TAMER-A", Some(0));
    r.place_on_field(0, "Y-TAMER-B", Some(0));
    r.place_on_field(0, "Y-TAMER-C", Some(0));
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let ob = r.place_on_field(1, "OPP-B", Some(0));
    digivolve(&mut r, base);
    let mut n = 0;
    while let Some(p) = r.pending_selection() {
        // Picks: OPP-A, OPP-A, OPP-B (keeps both alive for the DP sum).
        let a = p.valid_action_ids[if n < 2 { 0 } else { 1 }];
        r.execute_action(0, a).expect("pick");
        n += 1;
        assert!(n <= 3, "no more than 3 activations");
    }
    assert_eq!(n, 3);
    let total = r.effective_dp(oa).unwrap() + r.effective_dp(ob).unwrap();
    assert_eq!(total, 18000 - 12000);
}

#[test]
fn bt2_041_wd_no_opponent_digimon_still_suspends_tamers() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    let ya = r.place_on_field(0, "Y-TAMER-A", Some(0));
    digivolve(&mut r, base);
    assert!(r.pending_selection().is_none(), "no target → no prompt");
    assert!(suspended(&r, ya), "Tamer is still suspended");
}

#[test]
fn bt2_041_wd_debuff_expires_at_end_of_turn() {
    let mut r = builder().start();
    let base = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.place_on_field(0, "Y-TAMER-A", Some(0));
    let oa = r.place_on_field(1, "OPP-A", Some(0));
    digivolve(&mut r, base);
    let a = r.pending_selection().expect("pick").valid_action_ids[0];
    r.execute_action(0, a).expect("pick");
    assert_eq!(r.effective_dp(oa), Some(5000));
    r.end_turn();
    assert_eq!(
        r.effective_dp(oa),
        Some(9000),
        "-4000 lasts for the turn only"
    );
}

// ── Section 3: [Your Turn] DP aura ─────────────────────────────────────────

#[test]
fn bt2_041_your_turn_plus_1000_per_tamer_any_color_incl_suspended() {
    let mut r = builder().start();
    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(11000), "no Tamers → base DP");
    let ya = r.place_on_field(0, "Y-TAMER-A", Some(0));
    r.place_on_field(0, "R-TAMER", Some(0));
    r.game.player_mut(0).battle_area[ya.index as usize].is_suspended = true;
    r.game.tick_declarative_effects();
    assert_eq!(
        r.effective_dp(carrier),
        Some(13000),
        "2 Tamers (one suspended, one red) → +2000"
    );
}

#[test]
fn bt2_041_your_turn_buff_off_on_opponents_turn() {
    let mut r = builder().start();
    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "Y-TAMER-A", Some(0));
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(11000));
}

#[test]
fn bt2_041_your_turn_buff_ignores_opponent_tamers() {
    let mut r = builder().start();
    let carrier = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "Y-TAMER-A", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(11000));
}
