//! BT26-078 Cherubimon — Lv.6 Purple/Green Digimon (Cherub / Titan / TS).
//!
//! [Trash] [Your Turn] When any of your [Chronomon] text or [Titan] trait
//! Digimon are played, if your opponent has 5 or more memory, by returning
//! this card to the bottom of the deck, 1 of them gains <Rush> and <Execute>
//! for the turn.
//! [On Play] [When Digivolving] By deleting this Digimon, you may play 1 play
//! cost 12 or lower [Chronomon] text or [Titan] trait card from your trash
//! without paying the cost.
//!
//! DCGO: BT26/Green/BT26_078.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-078";

fn setup(memory: i16) -> DebugRunner {
    let mut texted = digimon("TEXTED", "Texted", CardColor::Purple, 5, 8, &[]);
    texted.effect_text = "[When Digivolving] If you have [Chronomon] ...".to_string();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-078")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "TITAN",
            "Titan Guy",
            CardColor::Purple,
            5,
            8,
            &["Titan"],
        ))
        .add_card(digimon(
            "BIG-TITAN",
            "Big",
            CardColor::Purple,
            7,
            14,
            &["Titan"],
        ))
        .add_card(texted)
        .add_card(tamer(
            "TITAN-T",
            "Titan Tamer",
            CardColor::Purple,
            &["Titan"],
        ))
        .add_card(digimon("PLAIN", "Plain", CardColor::Purple, 5, 8, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(memory)
        .start();
    r.set_first_player(0);
    r
}

fn has_kw(r: &DebugRunner, h: digimon_engine::permanent::PermanentHandle, k: Keyword) -> bool {
    r.game.has_keyword(h, k)
}

#[test]
fn bt26_078_traits_and_alt_path() {
    let r = setup(0);
    let c = r.compiled_card(CARD_ID).unwrap();
    for t in ["Cherub", "Titan", "TS"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
    assert!(format!("{:?}", c.alt_paths).contains("TS"));
}

#[test]
fn bt26_078_trash_titan_played_grants_rush_execute() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    assert!(r.pending_is_optional(), "by returning — optional");
    r.accept_optional_trigger().unwrap();
    // The memory is on the opponent's side, so the turn ends inside the
    // resolution and the granted <Execute> offers its attack (§16-38-2);
    // stop there, while the "for the turn" grants still apply.
    resolve_until_end_of_turn_keyword_decision(&mut r);
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(deck_ids(&r, 0).first().map(String::as_str), Some(CARD_ID));
    assert!(has_kw(&r, t, Keyword::Rush));
    assert!(has_kw(&r, t, Keyword::Execute));
}

#[test]
fn bt26_078_trash_chronomon_text_counts() {
    let mut r = setup(-6);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TEXTED", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    r.accept_optional_trigger().unwrap();
    // The memory is on the opponent's side, so the turn ends inside the
    // resolution and the granted <Execute> offers its attack (§16-38-2);
    // stop there, while the "for the turn" grants still apply.
    resolve_until_end_of_turn_keyword_decision(&mut r);
    assert!(has_kw(&r, t, Keyword::Execute));
}

#[test]
fn bt26_078_trash_needs_opponent_five_memory() {
    let mut r = setup(-4);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    assert!(r.pending_selection_view().is_none());
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt26_078_trash_ignores_non_matching_digimon() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "PLAIN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    assert!(r.pending_selection_view().is_none());
}

#[test]
fn bt26_078_trash_decline_keeps_card() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    r.decline_optional_trigger().unwrap();
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert!(!has_kw(&r, t, Keyword::Rush));
}

#[test]
fn bt26_078_on_play_delete_self_then_play_titan_free() {
    let mut r = setup(3);
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN");
    push_trash(&mut r, 0, "BIG-TITAN"); // cost 14 — ineligible
    push_trash(&mut r, 0, "TITAN-T");
    push_trash(&mut r, 0, "PLAIN"); // no Titan/Chronomon — ineligible
    fire(&mut r, EffectTiming::OnPlay, c);
    assert!(r.pending_is_optional(), "may");
    r.accept_optional_trigger().unwrap();
    // Self deleted as the cost; then optional trash pick (TITAN, TITAN-T).
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        2
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    let f = field_ids(&r, 0);
    assert_eq!(f.len(), 1);
    assert!(f[0] == "TITAN" || f[0] == "TITAN-T");
    assert_eq!(r.memory(), 3, "without paying the cost");
}

#[test]
fn bt26_078_on_play_decline_keeps_cherubimon() {
    let mut r = setup(3);
    let c = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TITAN");
    fire(&mut r, EffectTiming::WhenDigivolving, c);
    r.decline_optional_trigger().unwrap();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn bt26_078_trash_only_on_your_turn() {
    let mut r = setup(5); // opponent's turn: the gauge from P0's view is -5
    push_trash(&mut r, 0, CARD_ID);
    r.game.turn_player_idx = 1;
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, true, false);
    assert!(r.pending_selection_view().is_none(), "[Your Turn] only");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

/// Settle prompts (first valid choice) until an end-of-turn attack keyword's
/// decision is pending.
fn resolve_until_end_of_turn_keyword_decision(r: &mut DebugRunner) {
    for _ in 0..16 {
        if r.pending_is_end_of_turn_keyword_decision() {
            return;
        }
        let Some(sel) = r.pending_selection() else {
            panic!("no end-of-turn keyword decision was offered");
        };
        let player = sel.selecting_player;
        let action = sel
            .valid_action_ids
            .first()
            .copied()
            .unwrap_or(digimon_engine::action::space::PASS);
        r.execute_action(player, action).unwrap();
    }
    panic!("no end-of-turn keyword decision after 16 prompts");
}

fn move_cherubimon_from_trash_to_deck_bottom(r: &mut DebugRunner) {
    let at = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == CARD_ID)
        .expect("a Cherubimon (BT26-078) in the trash");
    let card = r.game.players[0].trash.remove(at);
    r.game.players[0].deck.insert(0, card);
}

fn cherubimon_in_trash(r: &DebugRunner) -> usize {
    trash_ids(r, 0).iter().filter(|id| *id == CARD_ID).count()
}

/// CR 4.3 15-4-4-3: "When a card with an effect that's pending activation
/// becomes a new card before the effect activates, the effect can no longer
/// be activated." 15-14-3-1: a {Trash} effect can trigger and activate "while
/// the card with the effect is in your trash". DCGO BT26_078.cs
/// `CanActivateCondition` requires `IsExistOnTrashActivate(card, ...)`.
///
/// Bug-scan repro (2026-11): two Cherubimon (BT26-078) triggered on a
/// Chronomon: Holy Mode (BT26-016) play; Chronomon's own cost returned one of
/// them to the deck, and that copy's pending trigger was still offered and
/// granted <Rush> and <Execute> with nothing returned.
#[test]
fn bt26_078_pending_trash_trigger_is_lost_when_the_card_leaves_the_trash() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    assert!(
        r.pending_selection().is_some(),
        "both Cherubimon triggers are pending"
    );
    // Another effect returns BOTH copies to the deck before either activates.
    move_cherubimon_from_trash_to_deck_bottom(&mut r);
    move_cherubimon_from_trash_to_deck_bottom(&mut r);
    let deck_before = deck_ids(&r, 0);

    // Answer the prompt that was already on screen (order pick or the first
    // copy's "you may"), accepting.
    let sel = r.pending_selection().unwrap();
    let (player, action) = (
        sel.selecting_player,
        sel.valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .expect("an accepting action"),
    );
    r.execute_action(player, action).unwrap();

    assert!(
        !r.pending_is_end_of_turn_keyword_decision(),
        "no <Execute> was granted: neither card was in the trash to be returned"
    );
    assert!(
        !r.pending_is_optional(),
        "the second copy's trigger can no longer activate either"
    );
    assert_eq!(deck_ids(&r, 0), deck_before, "nothing was returned");
    assert!(!has_kw(&r, t, Keyword::Rush));
    assert!(!has_kw(&r, t, Keyword::Execute));
}

/// Same rule, one copy surviving: the copy still in the trash pays and
/// grants; the copy that left does not get a second, free activation.
#[test]
fn bt26_078_only_the_copy_still_in_the_trash_activates() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, t.index as usize, false, false);
    // Whatever is on screen now (the order pick, or one copy's "you may") was
    // offered while both copies were in the trash. Another effect then returns
    // the copy that is NOT being asked about to the deck.
    let on_screen = r.pending_selection().expect("triggers pending").source_card;
    let at = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == CARD_ID && c.handle() != on_screen)
        .expect("a second Cherubimon (BT26-078) in the trash");
    let moved = r.game.players[0].trash.remove(at);
    let moved_handle = moved.handle();
    r.game.players[0].deck.insert(0, moved);
    assert_eq!(cherubimon_in_trash(&r), 1);

    let mut offers_for_the_moved_copy = 0;
    for _ in 0..8 {
        if r.pending_is_end_of_turn_keyword_decision() {
            break;
        }
        let Some(sel) = r.pending_selection() else {
            break;
        };
        if sel.is_optional && sel.source_card == moved_handle {
            offers_for_the_moved_copy += 1;
        }
        let (player, action) = (
            sel.selecting_player,
            sel.valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != digimon_engine::action::space::PASS)
                .expect("an accepting action"),
        );
        r.execute_action(player, action).unwrap();
    }
    assert_eq!(
        offers_for_the_moved_copy, 0,
        "the copy that left the trash is never offered"
    );
    assert_eq!(cherubimon_in_trash(&r), 0, "the surviving copy paid its cost");
    assert!(has_kw(&r, t, Keyword::Rush));
    assert!(has_kw(&r, t, Keyword::Execute));
}

/// ONE Cherubimon (BT26-078) in the trash with TWO of its triggers pending:
/// a second [Titan] Digimon was played before the first trigger resolved (in
/// a game, by the effect of the first one). The first activation pays the
/// cost, so the card is in the deck when the second trigger's turn comes.
/// CR 4.3 15-4-4-3: that trigger can no longer be activated; 15-14-3-1: a
/// {Trash} effect activates "while the card with the effect is in your
/// trash". DCGO BT26_078.cs `IsExistOnTrashActivate`.
///
/// This is the shape a recorded Chronomon mirror game hit (the
/// `chronomon-security-history` fixture of tests/determinization, decision
/// 58): the second trigger was offered, returned nothing, and still granted
/// <Rush> and <Execute>.
#[test]
fn bt26_078_second_pending_trigger_of_one_copy_is_lost_after_the_first_returns_it() {
    let mut r = setup(-5);
    push_trash(&mut r, 0, CARD_ID);
    let first = r.place_on_field(0, "TITAN", None);
    let second = r.place_on_field(0, "TITAN", None);
    r.fire_play_event_triggers(0, first.index as usize, false, false);
    assert!(r.pending_is_optional(), "the first trigger's \"by returning\"");
    // The second play happens while the first trigger is still pending.
    r.fire_play_event_triggers(0, second.index as usize, false, false);

    let mut offers = 0;
    for _ in 0..8 {
        if r.pending_is_end_of_turn_keyword_decision() {
            break;
        }
        let Some(sel) = r.pending_selection() else {
            break;
        };
        if sel.is_optional {
            offers += 1;
        }
        let (player, action) = (
            sel.selecting_player,
            sel.valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != digimon_engine::action::space::PASS)
                .expect("an accepting action"),
        );
        r.execute_action(player, action).unwrap();
    }
    assert_eq!(offers, 1, "one card in the trash pays once: one \"by returning\" offer");
    assert_eq!(cherubimon_in_trash(&r), 0);
    assert_eq!(
        deck_ids(&r, 0).iter().filter(|id| *id == CARD_ID).count(),
        1,
        "the card was returned once"
    );
    let granted = [first, second]
        .iter()
        .filter(|h| has_kw(&r, **h, Keyword::Rush) && has_kw(&r, **h, Keyword::Execute))
        .count();
    assert_eq!(granted, 1, "\"1 of them\": only the Digimon the paid trigger was for");
    assert!(
        !(has_kw(&r, first, Keyword::Rush) && has_kw(&r, second, Keyword::Rush)),
        "the unpaid trigger granted nothing"
    );
}
