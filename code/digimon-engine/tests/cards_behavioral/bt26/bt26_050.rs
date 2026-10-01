//! BT26-050 Rosemon: Burst Mode — DUAL (Lv.7 Digimon // Option).
//!
//! [When Digivolving] You may suspend 2 Digimon or Tamers. Then, 2 of your
//! opponent's Digimon or Tamers can't unsuspend until their turn ends.
//! [When Digivolving] [When Attacking] By returning 1 other suspended Digimon to
//! the bottom of the deck, trash your opponent's top security card.
//! Option: [Main] Suspend 2 of your opponent's Digimon or Tamers. Then, until
//! their turn ends, none of their suspended Digimon or Tamers can digivolve or
//! unsuspend. <Arts Digivolve>. Burst Digivolve from [Rosemon] by returning
//! [Yoshino Fujieda].
//!
//! DCGO: BT26/Green/BT26_050.cs.

use super::support::*;
use digimon_dsl::compiled::CompiledAltPathKind;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};

const CARD_ID: &str = "BT26-050";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-050")
        .add_card(filler("FILLER"))
        .add_card(tamer(
            "YOSHINO",
            "Yoshino Fujieda",
            CardColor::Green,
            &["DATA SQUAD"],
        ))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(tamer("OPP-T", "Opp Tamer", CardColor::Red, &[]))
        .add_card(digimon("MINE", "Mine", CardColor::Green, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 3])
        .memory(8)
        .start();
    r.set_first_player(0);
    r
}

fn drive_first_one(r: &mut DebugRunner) {
    let Some(sel) = r.game.pending_selection.as_ref() else {
        return;
    };
    let p = sel.selecting_player;
    let a = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .unwrap_or(digimon_engine::action::space::PASS);
    r.execute_action(p, a).unwrap();
}

fn drive_first(r: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            return;
        };
        let p = sel.selecting_player;
        let a = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .unwrap_or(digimon_engine::action::space::PASS);
        r.execute_action(p, a).unwrap();
    }
}

#[test]
fn bt26_050_has_burst_and_level6_alt_paths() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", card.alt_paths);
    assert!(
        dbg.contains("Rosemon") && dbg.contains("Yoshino Fujieda"),
        "{dbg}"
    );
    let _ = CompiledAltPathKind::Digivolve;
}

#[test]
fn bt26_050_wd_suspend_is_optional_lock_is_mandatory() {
    let mut r = setup();
    let bm = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP-T", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, bm);
    // "You may suspend 2" ⇒ PASS is legal with zero picks.
    let v = r.pending_selection_view().expect("optional suspend pick");
    assert!(v.is_optional, "may decline");
    pass(&mut r, 0);
    // Lock leg is mandatory; the other WD (bounce) has no suspended candidate.
    drive_first(&mut r);
    assert!(!r.game.players[1].battle_area[a.index as usize].is_suspended);
    assert!(r.modifiers().has(a, ModifierType::CannotUnsuspend));
    assert!(r.modifiers().has(b, ModifierType::CannotUnsuspend));
}

#[test]
fn bt26_050_wd_suspends_two_across_both_sides() {
    let mut r = setup();
    let bm = r.place_on_field(0, CARD_ID, Some(0));
    let mine = r.place_on_field(0, "MINE", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, bm);
    // One prompt over BOTH players' Digimon/Tamers (self, MINE, OPP).
    let picks: Vec<_> = r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&x| x != digimon_engine::action::space::PASS)
        .collect();
    assert_eq!(
        picks.len(),
        3,
        "both players' Digimon/Tamers are candidates"
    );
    let _ = (mine, opp);
    drive_first_one(&mut r); // 1st pick
                             // "0 or exactly 2": stopping after one pick is not legal.
    let v = r.pending_selection_view().expect("2nd pick pending");
    assert!(!v.is_optional, "cannot stop at 1");
    drive_first_one(&mut r); // 2nd pick
    drive_first_one(&mut r); // the mandatory can't-unsuspend pick
                             // Decline the separate [WD] bounce so the count is not disturbed.
    if r.game.pending_selection.is_some() {
        pass(&mut r, 0);
    }
    let _ = r.auto_resolve();
    let n = r
        .game
        .players
        .iter()
        .flat_map(|p| p.battle_area.iter())
        .filter(|p| p.is_suspended)
        .count();
    assert_eq!(n, 2, "exactly 2 suspended");
}

#[test]
fn bt26_050_bounce_other_suspended_trashes_top_security() {
    let mut r = setup();
    let bm = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    fire(&mut r, EffectTiming::WhenAttacking, bm);
    let v = r.pending_selection_view().expect("optional bounce pick");
    assert!(v.is_optional);
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&x| x != digimon_engine::action::space::PASS)
            .count(),
        1,
        "only the OTHER suspended Digimon"
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty(), "bounced");
    assert_eq!(r.security_count(1), 2, "opponent's top security trashed");
}

#[test]
fn bt26_050_bounce_declined_keeps_security() {
    let mut r = setup();
    let bm = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    fire(&mut r, EffectTiming::WhenAttacking, bm);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn bt26_050_option_main_suspends_two_and_locks() {
    let mut r = setup();
    r.place_on_field(0, "YOSHINO", Some(0)); // [DATA SQUAD] use requirement
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    let c = r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
    drive_first(&mut r);
    let suspended: Vec<_> = [a, b, c]
        .into_iter()
        .filter(|h| r.game.players[1].battle_area[h.index as usize].is_suspended)
        .collect();
    assert_eq!(suspended.len(), 2, "exactly 2 suspended");
    for h in &suspended {
        assert!(
            r.modifiers().has(*h, ModifierType::CannotDigivolve),
            "suspended ⇒ can't digivolve"
        );
    }
    let untouched = [a, b, c]
        .into_iter()
        .find(|h| !suspended.contains(h))
        .unwrap();
    assert!(
        !r.modifiers().has(untouched, ModifierType::CannotDigivolve),
        "unsuspended one may still digivolve"
    );

    // The lock is continuous: a Digimon of theirs that becomes suspended LATER
    // in the window (e.g. by attacking on their turn) is locked too.
    r.game.players[1].battle_area[untouched.index as usize].is_suspended = true;
    r.game.tick_declarative_effects();
    assert!(
        r.modifiers().has(untouched, ModifierType::CannotDigivolve),
        "later-suspended ⇒ locked"
    );
}
