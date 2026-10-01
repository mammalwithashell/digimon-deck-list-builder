//! BT26-049 Rosemon — Lv.6 Green, Fairy/DATA SQUAD.
//!
//! [When Digivolving] [When Attacking] [Once Per Turn] Suspend 2 of your
//! opponent's Digimon or Tamers.
//! [All Turns] [Once Per Turn] When any of your opponent's Digimon or Tamers
//! suspend, or effects trash cards from under your Tamers, you may play or use
//! 1 play or use cost 3 or lower [DATA SQUAD] trait card from your hand without
//! paying the cost. For each suspended Digimon or Tamer, add 1 to the cost
//! maximum.
//!
//! DCGO: BT26/Green/BT26_049.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-049";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-049")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(tamer("OPP-T", "Opp Tamer", CardColor::Red, &[]))
        .add_card(digimon(
            "DS3",
            "DS three",
            CardColor::Green,
            3,
            3,
            &["DATA SQUAD"],
        ))
        .add_card(digimon(
            "DS5",
            "DS five",
            CardColor::Green,
            4,
            5,
            &["DATA SQUAD"],
        ))
        .add_card(digimon(
            "DS6",
            "DS six",
            CardColor::Green,
            5,
            6,
            &["DATA SQUAD"],
        ))
        .add_card(digimon("PLAIN3", "Plain", CardColor::Green, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Answer every prompt with its first non-PASS option until the queue settles
/// or a prompt of `stop_prompt` appears.
fn drive_until(r: &mut DebugRunner, stop_prompt: &str) -> bool {
    for _ in 0..12 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            return false;
        };
        if sel.prompt.contains(stop_prompt) {
            return true;
        }
        let p = sel.selecting_player;
        let a = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .unwrap();
        r.execute_action(p, a).unwrap();
    }
    false
}

#[test]
fn bt26_049_wd_suspends_exactly_two() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    let c = r.place_on_field(1, "OPP-T", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(!v.is_optional, "'Suspend 2' is mandatory");
    drive_until(&mut r, "play or use");
    let n = [a, b, c].iter().filter(|&&h| suspended(&r, h)).count();
    assert_eq!(n, 2);
}

#[test]
fn bt26_049_wd_clamps_to_available() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    drive_until(&mut r, "play or use");
    assert!(suspended(&r, a));
}

#[test]
fn bt26_049_wd_and_wa_share_one_use() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    let c = r.place_on_field(1, "OPP", Some(0));
    let d = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    drive_until(&mut r, "play or use");
    // Decline the reactive play so it doesn't matter here.
    let _ = r.execute_action(0, digimon_engine::action::space::PASS);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::WhenAttacking, rose);
    let _ = r.auto_resolve();
    let n = [a, b, c, d].iter().filter(|&&h| suspended(&r, h)).count();
    assert_eq!(n, 2, "[Once Per Turn] shared by WD and WA");
}

/// Rosemon's own suspension of 2 opponent permanents triggers its [All Turns]
/// reactive: cap = 3 + 2 suspended = 5, so a cost-5 [DATA SQUAD] card plays
/// free and a cost-6 one is not offered.
#[test]
fn bt26_049_reactive_cap_counts_suspended() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DS5");
    push_hand(&mut r, 0, "DS6");
    push_hand(&mut r, 0, "PLAIN3");
    let mem = r.memory();
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered");
    let v = r.pending_selection_view().unwrap();
    assert!(v.is_optional);
    let offered: Vec<u16> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    assert_eq!(offered.len(), 1, "only DS5 is within 3 + 2");
    pick_hand(&mut r, 0, "DS5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DS5".to_string()));
    assert_eq!(r.memory(), mem, "free");
}

#[test]
fn bt26_049_reactive_decline_refunds() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    // ONE opponent permanent: the engine fires on_suspend per permanent (DCGO
    // batches one OnTappedAnyone per suspend action —
    // G-ENGINE-ON-SUSPEND-BATCH), so keep this test to a single suspend.
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DS3");
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"));
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    // A later effect trash from under a Tamer re-offers it (RemoveUse).
    let t = tamer_with_face_down(&mut r, 0, "OPP-T", 1);
    let _ = t;
    // Trash via the engine surface a card effect would use.
    {
        let rc = rose_card(&r, rose);
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut r.game, rc, Some(rose), 0);
        assert!(ctx.trash_bottom_face_down_source(t));
    }
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection_view().is_some(),
        "re-offered after a decline"
    );
}

fn rose_card(r: &DebugRunner, h: PermanentHandle) -> digimon_engine::card_source::CardHandle {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .handle()
}

/// G-ENGINE-ON-SUSPEND-BATCH: suspending 2 permanents in one action is ONE
/// "when … suspend" event (rule 15-5-2; DCGO fires one OnTappedAnyone with the
/// whole list). Declining the reactive refunds its [Once Per Turn], but the
/// same batch must not re-offer it.
#[test]
fn bt26_049_two_suspends_in_one_action_trigger_once() {
    let mut r = setup();
    let rose = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    push_hand(&mut r, 0, "DS3");
    fire(&mut r, EffectTiming::WhenDigivolving, rose);
    assert!(drive_until(&mut r, "play or use"), "reactive offered once");
    pass(&mut r, 0);
    assert!(
        r.game.pending_selection.is_none(),
        "no second offer from the same 2-permanent suspend"
    );
    assert_eq!(r.hand_size(0), 1);
}
