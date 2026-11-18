//! BT24-073 SkullSatamon — Lv.5 Purple, Undead / Fallen Angel ((Rule) trait).
//!
//! <Blocker>
//! [When Digivolving] [On Deletion] If your opponent has 10 or fewer cards in
//! their trash, trash the top 3 cards of both players' decks. Then, if your
//! opponent has 10 or more cards in their trash, you may play 1 level 4 or
//! lower [Evil] or [Fallen Angel] trait Digimon card from your trash without
//! paying the cost.
//! Inherited: [When Attacking] [Once Per Turn] This Digimon gains <Security
//! A. +1> for the turn. If your opponent has 10 or fewer cards in their trash,
//! instead trash the top 2 cards of both players' decks.
//!
//! DCGO: BT24/Purple/BT24_073.cs. Synthetic boards only (no recorded games).

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use support::*;

const CARD_ID: &str = "BT24-073";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-073 in embedded DSL pack")
        .add_card(filler("FILLER"))
        .add_card(digimon("EVIL3", "Evil Three", CardColor::Purple, 3, 3, &["Evil"]))
        .add_card(digimon("FALLEN4", "Fallen Four", CardColor::Purple, 4, 5, &["Fallen Angel"]))
        .add_card(digimon("EVIL4", "Evil Four", CardColor::Purple, 4, 5, &["Evil"]))
        .add_card(digimon("EVIL5", "Evil Five", CardColor::Purple, 5, 7, &["Evil"]))
        .add_card(digimon("EVILDRAGON4", "Evil Dragon Four", CardColor::Purple, 4, 5, &["Evil Dragon"]))
        .add_card(digimon("BEAST4", "Beast Four", CardColor::Purple, 4, 5, &["Beast"]))
        .add_card(digimon("PURPLE4", "Purple Four", CardColor::Purple, 4, 5, &[]))
        .add_card(digimon("RED4", "Red Four", CardColor::Red, 4, 5, &[]))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(1, &["FILLER"; 4])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    r
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not on player {p}'s field"));
    PermanentHandle { player: p, index: index as u8 }
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

fn fill_opp_trash(r: &mut DebugRunner, n: usize) {
    for _ in 0..n {
        push_trash(r, 1, "FILLER");
    }
}

// ─── Structural / identity ───────────────────────────────────────────────────

#[test]
fn bt24_073_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "SkullSatamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(8));
    assert_eq!(card.dp, Some(8000));
    for t in ["Undead", "Fallen Angel"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "shared WD/OD clause + inherited [When Attacking]");
    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WD/OD clause");
    assert_eq!(shared.when.len(), 2);
    assert!(!shared.once_per_turn, "no [Once Per Turn] on the shared clause");
    assert!(!shared.optional, "the play pick is the decline surface");
    let inherited = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("inherited WA clause");
    assert!(inherited.once_per_turn);
}

#[test]
fn bt24_073_digivolves_from_purple_lv4_for_3_only() {
    let mut r = setup();
    let base = r.place_on_field(0, "PURPLE4", Some(0));
    assert!(digivolve_onto(&mut r, base));
    assert_eq!(top_id(&r, base), CARD_ID);
    assert_eq!(r.memory(), 10 - 3);

    let mut r = setup();
    let base = r.place_on_field(0, "RED4", Some(0));
    assert!(!digivolve_onto(&mut r, base), "a red Lv.4 is not a printed source");
}

#[test]
fn bt24_073_face_up_has_blocker() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(me, Keyword::Blocker));
}

// ─── [When Digivolving] ──────────────────────────────────────────────────────

#[test]
fn bt24_073_wd_with_few_opp_trash_mills_three_each_and_offers_no_play() {
    let mut r = setup();
    push_trash(&mut r, 0, "EVIL3");
    let base = r.place_on_field(0, "PURPLE4", Some(0));
    assert!(digivolve_onto(&mut r, base));
    let _ = r.auto_resolve();
    // Digivolving draws 1 first (own deck 10 -> 9), then the mill takes 3.
    assert_eq!(deck_ids(&r, 0).len(), 10 - 1 - 3, "own deck milled by 3");
    assert_eq!(deck_ids(&r, 1).len(), 7, "opponent deck milled by 3");
    assert_eq!(trash_ids(&r, 1).len(), 3);
    assert!(r.pending_selection_view().is_none(), "3 < 10: no play window");
    assert!(!field_ids(&r, 0).contains(&"EVIL3".to_string()));
}

#[test]
fn bt24_073_wd_mill_that_crosses_ten_opens_the_play_window_with_the_printed_filter() {
    let mut r = setup();
    fill_opp_trash(&mut r, 9);
    for id in ["EVIL3", "FALLEN4", "EVIL4", "EVIL5", "EVILDRAGON4", "BEAST4"] {
        push_trash(&mut r, 0, id);
    }
    let base = r.place_on_field(0, "PURPLE4", Some(0));
    assert!(digivolve_onto(&mut r, base));
    assert_eq!(deck_ids(&r, 1).len(), 7, "9 <= 10: the mill happened first");
    assert_eq!(trash_ids(&r, 1).len(), 12);
    let v = r.pending_selection_view().expect("12 >= 10: the play window opens");
    assert!(v.is_optional, "'you may play'");
    // Lv.4 or lower with the exact [Evil] / [Fallen Angel] trait: EVIL3,
    // FALLEN4, EVIL4. Not Lv.5, not [Evil Dragon] (exact match), not [Beast].
    assert_eq!(non_pass(&r).len(), 3);
    pick_trash(&mut r, 0, "EVIL4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"EVIL4".to_string()));
    assert_eq!(r.memory(), 10 - 3, "played without paying the cost");
    assert!(!trash_ids(&r, 0).contains(&"EVIL4".to_string()));
}

#[test]
fn bt24_073_wd_at_exactly_ten_both_gates_are_true() {
    let mut r = setup();
    fill_opp_trash(&mut r, 10);
    push_trash(&mut r, 0, "EVIL3");
    let base = r.place_on_field(0, "PURPLE4", Some(0));
    assert!(digivolve_onto(&mut r, base));
    assert_eq!(deck_ids(&r, 0).len(), 10 - 1 - 3, "10 or fewer: mill (after the digivolve draw)");
    assert_eq!(deck_ids(&r, 1).len(), 7);
    assert!(r.pending_selection_view().is_some(), "10 or more: play window (fresh read)");
}

#[test]
fn bt24_073_wd_with_eleven_opp_trash_skips_the_mill_but_still_plays() {
    let mut r = setup();
    fill_opp_trash(&mut r, 11);
    push_trash(&mut r, 0, "FALLEN4");
    let base = r.place_on_field(0, "PURPLE4", Some(0));
    assert!(digivolve_onto(&mut r, base));
    assert_eq!(deck_ids(&r, 0).len(), 10 - 1, "11 > 10: no mill (only the digivolve draw)");
    assert_eq!(deck_ids(&r, 1).len(), 10);
    assert!(r.pending_selection_view().expect("play window").is_optional);
    pick_trash(&mut r, 0, "FALLEN4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"FALLEN4".to_string()));
}

#[test]
fn bt24_073_wd_play_can_be_declined() {
    let mut r = setup();
    fill_opp_trash(&mut r, 11);
    push_trash(&mut r, 0, "EVIL3");
    let base = r.place_on_field(0, "PURPLE4", Some(0));
    assert!(digivolve_onto(&mut r, base));
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(trash_ids(&r, 0).contains(&"EVIL3".to_string()));
    assert!(!field_ids(&r, 0).contains(&"EVIL3".to_string()));
}

// ─── [On Deletion] ───────────────────────────────────────────────────────────

#[test]
fn bt24_073_on_deletion_runs_the_same_body() {
    let mut r = setup();
    fill_opp_trash(&mut r, 11);
    push_trash(&mut r, 0, "EVIL3");
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("[On Deletion] play window");
    assert!(v.is_optional);
    pick_trash(&mut r, 0, "EVIL3");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["EVIL3".to_string()]);
}

#[test]
fn bt24_073_on_deletion_mills_when_opp_trash_is_small() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 0).len(), 7);
    assert_eq!(deck_ids(&r, 1).len(), 7);
}

#[test]
fn bt24_073_on_deletion_can_replay_its_own_lv4_evil_digivolution_card() {
    // Rule 25: [On Deletion] resolves post-trash, so a Lv.4 [Evil] card that
    // was under this Digimon is already in the trash and a legal pick.
    let mut r = setup();
    fill_opp_trash(&mut r, 11);
    let me = r.place_stack(0, &["EVIL4", CARD_ID]);
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    assert_eq!(non_pass(&r).len(), 1);
    pick_trash(&mut r, 0, "EVIL4");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["EVIL4".to_string()]);
}

// ─── Inherited [When Attacking] [Once Per Turn] ──────────────────────────────

#[test]
fn bt24_073_inherited_with_few_opp_trash_mills_two_each_instead_of_security_plus() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "PURPLE4"]);
    r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 0).len(), 8);
    assert_eq!(deck_ids(&r, 1).len(), 8);
    assert_eq!(r.security_count(1), 4 - 1, "'instead': a single security check");
}

#[test]
fn bt24_073_inherited_with_many_opp_trash_gains_security_attack_plus_and_does_not_mill() {
    let mut r = setup();
    fill_opp_trash(&mut r, 11);
    let host = r.place_stack(0, &[CARD_ID, "PURPLE4"]);
    r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 0).len(), 10, "11 > 10: nothing milled");
    assert_eq!(deck_ids(&r, 1).len(), 10);
    assert_eq!(r.security_count(1), 4 - 2, "<Security A. +1>: two checks");
}

#[test]
fn bt24_073_inherited_is_once_per_turn() {
    let mut r = setup();
    let host = r.place_stack(0, &[CARD_ID, "PURPLE4"]);
    r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 0).len(), 8);
    r.game.players[0].battle_area[host.index as usize].is_suspended = false;
    r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 0).len(), 8, "the second attack does not mill again");
}

#[test]
fn bt24_073_face_up_has_no_inherited_effect() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(me, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 0).len(), 10, "an inherited effect does not work face-up");
    assert_eq!(deck_ids(&r, 1).len(), 10);
}
