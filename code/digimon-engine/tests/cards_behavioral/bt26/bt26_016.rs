//! BT26-016 Chronomon: Holy Mode — Lv.6 Red/Yellow (Shaman / Iliad / TS).
//!
//! <Piercing> <Engage>. [On Play] [When Digivolving] [When Attacking] [Once Per
//! Turn] You may delete 1 of your opponent's Digimon with as much DP as this
//! Digimon or less. Then, by returning 3 cards in trashes to the bottom of the
//! deck, <Recovery +1>.
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area, by
//! returning your top security card to the bottom of the deck, it doesn't
//! leave.
//!
//! DCGO: BT26/Red/BT26_016.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT26-016";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-016")
        .add_card(filler("FILLER"))
        .add_card(filler("TR"))
        .add_card({
            let mut c = digimon("BIG", "Big", CardColor::Blue, 7, 14, &[]);
            c.dp = Some(13000);
            c
        })
        .add_card(digimon("OPP6", "Opp Six", CardColor::Blue, 6, 9, &[]))
        .add_card(digimon("MINE", "Mine", CardColor::Red, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn on_field(r: &DebugRunner, id: &str) -> bool {
    field_ids(r, 0).iter().any(|c| c == id)
}

/// Pick the first `n` legal non-PASS actions of a multi-select, then PASS to
/// finish it (when the prompt is still open).
fn pick_n_then_finish(r: &mut DebugRunner, n: usize) {
    for _ in 0..n {
        pick_first(r, 0);
    }
    if r.pending_selection_view().is_some() {
        let v = r.pending_selection_view().unwrap();
        if v.valid_action_ids
            .contains(&digimon_engine::action::space::PASS)
        {
            pass(r, 0);
        }
    }
}

fn chrono(r: &mut DebugRunner) -> PermanentHandle {
    r.place_on_field(0, CARD_ID, Some(0))
}

#[test]
fn bt26_016_keywords_and_alt_path() {
    let mut r = setup();
    let h = chrono(&mut r);
    assert!(r.game.has_keyword(h, Keyword::Piercing));
    assert!(r.game.has_keyword(h, Keyword::Engage));
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("TS") && alt.contains("level_eq: Some(5)"),
        "{alt}"
    );
}

#[test]
fn bt26_016_on_play_deletes_up_to_own_dp_then_three_own_trash_recover() {
    let mut r = setup();
    r.place_on_field(1, "BIG", Some(0));
    let opp6 = r.place_on_field(1, "OPP6", Some(0));
    for _ in 0..3 {
        push_trash(&mut r, 0, "TR");
    }
    let h = chrono(&mut r);
    fire(&mut r, EffectTiming::OnPlay, h);
    let v = r.pending_selection_view().expect("delete pick");
    assert!(v.is_optional, "'you may delete'");
    // Only OPP6 (6000 <= 12000) — BIG (13000) is not a legal pick.
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        1
    );
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 1), vec!["BIG"]);
    let _ = opp6;
    // Own trash: pick all 3.
    pick_n_then_finish(&mut r, 3);
    assert!(r.game.pending_selection.is_none());
    assert!(trash_ids(&r, 0).is_empty());
    assert_eq!(&deck_ids(&r, 0)[..3], &["TR", "TR", "TR"], "bottom of deck");
    assert_eq!(r.security_count(0), 4, "<Recovery +1>");
}

#[test]
fn bt26_016_mixed_trashes_one_own_two_opponent() {
    let mut r = setup();
    push_trash(&mut r, 0, "TR");
    push_trash(&mut r, 1, "TR");
    push_trash(&mut r, 1, "TR");
    let h = chrono(&mut r);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    pick_n_then_finish(&mut r, 1); // 1 from own trash
    let v = r.pending_selection_view().expect("opponent trash pick");
    assert!(v.is_optional, "0 or exactly 2");
    pick_n_then_finish(&mut r, 2);
    assert!(trash_ids(&r, 0).is_empty());
    assert!(trash_ids(&r, 1).is_empty());
    assert_eq!(
        deck_ids(&r, 0).len(),
        6 + 1 - 1,
        "+1 returned, -1 recovered"
    );
    assert_eq!(deck_ids(&r, 0)[0], "TR");
    assert_eq!(deck_ids(&r, 1).len(), 8, "each card to its owner's deck");
    assert_eq!(r.security_count(0), 4);
}

#[test]
fn bt26_016_partial_return_pays_nothing_and_refunds_opt() {
    let mut r = setup();
    push_trash(&mut r, 0, "TR");
    push_trash(&mut r, 1, "TR");
    push_trash(&mut r, 1, "TR");
    let h = chrono(&mut r);
    fire(&mut r, EffectTiming::OnPlay, h);
    pick_n_then_finish(&mut r, 1);
    pass(&mut r, 0); // decline the opponent share → only 1 of 3
    assert!(r.game.pending_selection.is_none());
    assert_eq!(trash_ids(&r, 0), vec!["TR"], "nothing returned");
    assert_eq!(r.security_count(0), 3, "no recovery");
    // Nothing deleted, nothing returned → OPT not spent.
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_some(), "RemoveUse");
}

#[test]
fn bt26_016_shared_once_per_turn_after_use() {
    let mut r = setup();
    r.place_on_field(1, "OPP6", Some(0));
    r.place_on_field(1, "OPP6", Some(0));
    let h = chrono(&mut r);
    fire(&mut r, EffectTiming::OnPlay, h);
    pick_first(&mut r, 0);
    // Trash too small for the cost — the select no-ops.
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn] across OP/WD/WA"
    );
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

#[test]
fn bt26_016_no_target_and_small_trash_does_not_activate() {
    let mut r = setup();
    r.place_on_field(1, "BIG", Some(0));
    push_trash(&mut r, 0, "TR");
    push_trash(&mut r, 1, "TR");
    let h = chrono(&mut r);
    fire(&mut r, EffectTiming::OnPlay, h);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt26_016_would_leave_returns_top_security_and_stays_once_per_turn() {
    let mut r = setup();
    let h = chrono(&mut r);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_some(), "replacement offered");
    let v = r.pending_selection_view().unwrap();
    assert!(v.is_optional);
    let a = *v
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .unwrap();
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert!(on_field(&r, CARD_ID), "it doesn't leave");
    assert_eq!(r.security_count(0), 2);
    assert_eq!(deck_ids(&r, 0).len(), 7, "top security → deck bottom");
    // Once per turn.
    let h = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .unwrap();
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert!(!on_field(&r, CARD_ID));
}

#[test]
fn bt26_016_would_leave_declined_or_no_security_leaves() {
    let mut r = setup();
    let h = chrono(&mut r);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!on_field(&r, CARD_ID));
    assert_eq!(r.security_count(0), 3);

    let mut r = setup();
    r.game.players[0].security.clear();
    let h = chrono(&mut r);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_none());
    assert!(!on_field(&r, CARD_ID));
}

#[test]
fn bt26_016_replacement_only_protects_itself() {
    let mut r = setup();
    chrono(&mut r);
    let mine = r.place_on_field(0, "MINE", Some(0));
    r.game
        .delete_permanent_with_cause(mine, ReplacementCause::OpponentEffect);
    assert!(r.game.pending_selection.is_none(), "only 'this Digimon'");
    assert!(!on_field(&r, "MINE"));
}

/// [Once Per Turn] must stay spent after a completed return, whichever way
/// the three cards were split between the two trashes. CR 4.3 15-14-1-1: "each
/// activation counts toward 1 use"; 15-14-1-2: once used, "it won't trigger
/// again during that turn". DCGO BT26_016.cs shares one `BT26_016_OPT` hash
/// (maxCountPerTurn 1) across On Play / When Digivolving / When Attacking and
/// calls `RemoveUse()` only when nothing was deleted AND nothing was returned.
///
/// Bug-scan repro (2026-11): one Chronomon: Holy Mode (BT26-016) resolved the
/// whole effect at On Play and again at When Attacking in the same turn
/// whenever the return went through the opponent-trash prompt. The return was
/// recorded in the nested prompt's bindings and never reached the parked
/// outer tail, so the trailing "nothing deleted and nothing returned" guard
/// read a stale log and refunded the use.
fn split_return_then_second_trigger(own: usize, opp: usize) {
    let mut r = setup();
    for _ in 0..own {
        push_trash(&mut r, 0, "TR");
    }
    for _ in 0..opp {
        push_trash(&mut r, 1, "TR");
    }
    let h = chrono(&mut r);
    fire(&mut r, EffectTiming::OnPlay, h);
    // No opposing Digimon: the first prompt is the own-trash share.
    pick_n_then_finish(&mut r, own);
    if opp > 0 {
        assert!(
            r.pending_selection_view().is_some(),
            "opponent-trash share prompt ({own} own + {opp} opponent)"
        );
        pick_n_then_finish(&mut r, opp);
    }
    assert!(r.game.pending_selection.is_none());
    assert!(trash_ids(&r, 0).is_empty() && trash_ids(&r, 1).is_empty());
    assert_eq!(r.security_count(0), 4, "<Recovery +1> ({own} own + {opp} opponent)");

    // Same turn, fresh material for a second return: the effect must not
    // trigger again.
    for _ in 0..3 {
        push_trash(&mut r, 0, "TR");
        push_trash(&mut r, 1, "TR");
    }
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn] is spent after a {own} own + {opp} opponent return"
    );
    assert_eq!(r.security_count(0), 4, "no second <Recovery +1>");
    assert_eq!(trash_ids(&r, 0).len(), 3);
    assert_eq!(trash_ids(&r, 1).len(), 3);
}

#[test]
fn bt26_016_once_per_turn_holds_after_three_own_return() {
    split_return_then_second_trigger(3, 0);
}

#[test]
fn bt26_016_once_per_turn_holds_after_two_own_one_opponent_return() {
    split_return_then_second_trigger(2, 1);
}

#[test]
fn bt26_016_once_per_turn_holds_after_one_own_two_opponent_return() {
    split_return_then_second_trigger(1, 2);
}

#[test]
fn bt26_016_once_per_turn_holds_after_three_opponent_return() {
    split_return_then_second_trigger(0, 3);
}

/// Regression (`raw-20261124T071011Z#g29` step 53): even when your own trash is
/// non-empty, `BT26-016`'s own-trash share must expose PASS (`optional_zero: true`)
/// so the player can choose 0 cards from their own trash and all 3 cards from
/// their opponent's trash for `<Recovery +1>`.
#[test]
fn bt26_016_zero_own_three_opponent_when_own_trash_non_empty() {
    let mut r = setup();
    for _ in 0..4 {
        push_trash(&mut r, 0, "TR");
        push_trash(&mut r, 1, "TR");
    }
    let h = chrono(&mut r);
    fire(&mut r, EffectTiming::WhenDigivolving, h);

    let own_prompt = r
        .pending_selection_view()
        .expect("own-trash share prompt should open when own trash is non-empty");
    assert!(
        own_prompt.is_optional,
        "own-trash share must allow PASS (0 cards) to take all 3 from opponent's trash"
    );
    pass(&mut r, 0);

    let opp_prompt = r
        .pending_selection_view()
        .expect("passing own trash must open the 3-card opponent-trash prompt");
    assert!(opp_prompt.is_optional, "opponent share is 0 or 3");
    pick_n_then_finish(&mut r, 3);

    assert!(r.game.pending_selection.is_none());
    assert_eq!(trash_ids(&r, 0).len(), 4, "own trash untouched");
    assert_eq!(trash_ids(&r, 1).len(), 1, "3 returned from opponent trash");
    assert_eq!(deck_ids(&r, 1).len(), 9, "3 returned to opponent deck bottom");
    assert_eq!(r.security_count(0), 4, "<Recovery +1>");

    // OPT must be spent after the 0-own + 3-opponent return.
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn] must stay spent after 0-own + 3-opponent return"
    );
}
