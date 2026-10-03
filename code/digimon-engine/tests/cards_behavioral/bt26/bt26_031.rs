//! BT26-031 Murasamemon // Gonozan: Murashigure — DUAL (Digimon Lv.5
//! Yellow/Blue DP 8000 // Option Yellow use cost 4).
//! Traits: Beastkin, Glowing Dawn, BEATBREAK.
//! Digivolve: Yellow Lv.4 / 4; Blue Lv.4 / 4; [Digivolve] Lv.4 w/[Glowing Dawn]: 3.
//!
//! [When Digivolving] By trashing the top security card of 1 player with the
//! most security cards, 1 of your opponent's Digimon or Tamers can't suspend
//! until their turn ends.
//! [When Digivolving] [When Attacking] [Once Per Turn] By trashing the bottom
//! face-down card from under any of your Tamers, <Recovery +1>.
//! Option: <Use Req. ([Glowing Dawn] trait)> [Main] 1 of your opponent's
//! Digimon gets -8000 DP until their turn ends. By trashing your top security
//! card, it further gets -5000 DP. <Arts Digivolve>.
//! Official Q&A: the player that activated the effect chooses the player.
//!
//! DCGO: BT26/Yellow/BT26_031.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-031";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-031")
        .add_card(filler("FILLER"))
        .add_card(tamer("SECT", "SecTamer", CardColor::Red, &[]))
        .add_card(tamer(
            "GDT",
            "GD Tamer",
            CardColor::Yellow,
            &["Glowing Dawn"],
        ))
        .add_card({
            let mut d = digimon("BIG", "Big", CardColor::Red, 6, 8, &[]);
            d.dp = Some(20000);
            d
        })
        .add_card(tamer("OPP-T", "Opp Tamer", CardColor::Red, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["SECT", "SECT", "SECT"])
        .security(1, &["SECT", "SECT", "SECT"])
        .memory(6)
        .start();
    r.set_first_player(0);
    r
}

fn labels(r: &DebugRunner) -> usize {
    r.pending_selection()
        .and_then(|s| s.effect_choices.as_ref())
        .map(|c| c.len())
        .unwrap_or(0)
}

fn set_security(r: &mut DebugRunner, mine: usize, theirs: usize) {
    r.game.players[0].security.truncate(mine);
    r.game.players[1].security.truncate(theirs);
}

fn lock_target(r: &mut DebugRunner) -> PermanentHandle {
    r.place_on_field(1, "BIG", Some(0))
}

// ─── [When Digivolving] trash most-security top card → can't suspend ────────

#[test]
fn bt26_031_wd_tie_offers_both_players() {
    let mut r = setup();
    let opp = lock_target(&mut r);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    assert_eq!(labels(&r), 3, "own / opponent / don't");
    r.execute_branch(1).expect("trash opponent's top security");
    pick_first(&mut r, 0); // the only opponent Digimon/Tamer
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2);
    assert_eq!(r.security_count(0), 3);
    assert!(r.modifiers().has(opp, ModifierType::CannotSuspend));
}

#[test]
fn bt26_031_wd_only_the_player_with_more_security() {
    let mut r = setup();
    set_security(&mut r, 3, 1);
    let opp = lock_target(&mut r);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    assert_eq!(labels(&r), 2, "only yours (3 > 1) + don't");
    r.execute_branch(0).expect("trash own top security");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2);
    assert_eq!(r.security_count(1), 1);
    assert!(r.modifiers().has(opp, ModifierType::CannotSuspend));
}

#[test]
fn bt26_031_wd_opponent_with_more_security() {
    let mut r = setup();
    set_security(&mut r, 0, 2);
    let opp = lock_target(&mut r);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    assert_eq!(labels(&r), 2, "only the opponent's + don't");
    r.execute_branch(0).expect("trash opponent's top security");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 1);
    assert!(r.modifiers().has(opp, ModifierType::CannotSuspend));
}

#[test]
fn bt26_031_wd_lock_may_target_a_tamer() {
    let mut r = setup();
    let t = r.place_on_field(1, "OPP-T", Some(0));
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    r.execute_branch(0).expect("trash own top security");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.modifiers().has(t, ModifierType::CannotSuspend));
}

#[test]
fn bt26_031_wd_dont_trash_does_nothing() {
    let mut r = setup();
    let opp = lock_target(&mut r);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    r.execute_branch(2).expect("don't trash");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3);
    assert_eq!(r.security_count(1), 3);
    assert!(!r.modifiers().has(opp, ModifierType::CannotSuspend));
}

#[test]
fn bt26_031_wd_lock_lasts_until_end_of_opponents_turn() {
    let mut r = setup();
    let opp = lock_target(&mut r);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    r.execute_branch(1).expect("opp");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    r.pass_turn();
    assert!(
        r.modifiers().has(opp, ModifierType::CannotSuspend),
        "their turn"
    );
    r.pass_turn();
    assert!(
        !r.modifiers().has(opp, ModifierType::CannotSuspend),
        "expired"
    );
}

#[test]
fn bt26_031_wd_needs_security_somewhere() {
    let mut r = setup();
    set_security(&mut r, 0, 0);
    lock_target(&mut r);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, m);
    assert!(r.game.pending_selection.is_none());
}

// ─── [WD][WA][OPT] trash a Tamer's face-down card → <Recovery +1> ──────────

#[test]
fn bt26_031_wa_trashes_face_down_and_recovers() {
    let mut r = setup();
    set_security(&mut r, 1, 3);
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, m);
    r.accept_optional_trigger().expect("optional cost");
    pick_first(&mut r, 0); // the Tamer
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0);
    assert_eq!(r.security_count(0), 2, "<Recovery +1>");
}

#[test]
fn bt26_031_wa_declined_keeps_once_per_turn() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 2);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, m);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 2);
    assert_eq!(r.security_count(0), 3);
    fire(&mut r, EffectTiming::WhenAttacking, m);
    r.accept_optional_trigger().expect("still available");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1);
    assert_eq!(r.security_count(0), 4);
}

#[test]
fn bt26_031_wa_is_once_per_turn() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 2);
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, m);
    r.accept_optional_trigger().expect("accept");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, m);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1, "OPT spent");
}

#[test]
fn bt26_031_wa_needs_a_face_down_card() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    let m = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, m);
    assert!(r.game.pending_selection.is_none());
}

// ─── Option face ────────────────────────────────────────────────────────────

fn use_option(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
}

#[test]
fn bt26_031_option_minus_8000_then_trash_security_minus_5000_more() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0)); // [Glowing Dawn] use requirement
    let opp = lock_target(&mut r);
    use_option(&mut r);
    pick_first(&mut r, 0); // the target
    assert_eq!(r.effective_dp(opp), Some(12000), "-8000");
    assert_eq!(labels(&r), 2, "trash top security? yes / no");
    r.execute_branch(0).expect("trash");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2);
    assert_eq!(r.effective_dp(opp), Some(7000), "-8000 -5000");
    r.pass_turn();
    assert_eq!(r.effective_dp(opp), Some(7000), "until their turn ends");
    r.pass_turn();
    assert_eq!(r.effective_dp(opp), Some(20000));
}

#[test]
fn bt26_031_option_decline_security_keeps_only_minus_8000() {
    let mut r = setup();
    r.place_on_field(0, "GDT", Some(0));
    let opp = lock_target(&mut r);
    use_option(&mut r);
    pick_first(&mut r, 0);
    r.execute_branch(1).expect("don't trash");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3);
    assert_eq!(r.effective_dp(opp), Some(12000));
}

#[test]
fn bt26_031_option_without_security_has_no_follow_up() {
    let mut r = setup();
    set_security(&mut r, 0, 3);
    r.place_on_field(0, "GDT", Some(0));
    let opp = lock_target(&mut r);
    use_option(&mut r);
    pick_first(&mut r, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.effective_dp(opp), Some(12000));
}

#[test]
fn bt26_031_dual_metadata() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", c);
    assert!(dbg.contains("ArtsDigivolve"), "<Arts Digivolve>");
    assert!(dbg.contains("Glowing Dawn"));
}
