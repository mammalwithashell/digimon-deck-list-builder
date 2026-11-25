//! Source: `data/card_bundles/BT27-090.md` (printed text and official Q&A).
//!
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When any of your Digimon with [Leomon] in their names or the
//! [Boss] or [DATA SQUAD] trait are played, by placing this Tamer as any of
//! those Digimon's bottom digivolution card, your opponent's effects can't
//! return 1 of your Digimon to hands or decks, or trash its stacked cards
//! until their turn ends.
//! Inherited: [All Turns] This Digimon with [Leomon] in its name or the [Boss]
//! or [DATA SQUAD] trait gets +3000 DP.
//! [Security] Play this card without paying the cost.
//! Q&A: "can't trash stacked cards" also stops De-Digivolve.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-090";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("LEO", "BanchoLeomon", 4, 3, &[]))
        .add_card(mon("BOSS", "Bossmon", 4, 3, &["Boss"]))
        .add_card(mon("DS", "Squadmon", 4, 3, &["DATA SQUAD"]))
        .add_card(mon("PLAIN", "Plainmon", 4, 3, &[]))
        .add_card(mon("OTHER", "Othermon", 4, 3, &[]))
        .add_card(mon("ATK", "Atk", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .hand(1, &["LEO"])
        .memory(10)
        .start()
}

#[test]
fn start_of_turn_sets_two_or_less_memory_to_three() {
    for (before, after) in [(2, 3), (0, 3), (5, 5)] {
        let mut r = runner(&[]);
        r.place_on_field(0, CARD, Some(0));
        r.game.memory = before;
        r.end_turn();
        r.end_turn();
        assert_eq!(r.memory(), after, "from {before}");
    }
}

fn play_and_place(r: &mut DebugRunner, id: &str) -> digimon_engine::permanent::PermanentHandle {
    play_hand(r, id);
    let v = r.pending_selection_view().expect("optional placement");
    assert!(v.is_optional);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    if r.pending_selection_view().is_some_and(|v| v.prompt.contains("bottom digivolution")) {
        r.choose_prompt_card(CARD, None).unwrap();
    }
    find(r, id)
}

fn find(r: &DebugRunner, id: &str) -> digimon_engine::permanent::PermanentHandle {
    let i = field_ids(r, 0).iter().position(|x| x == id).expect("on field");
    digimon_engine::permanent::PermanentHandle { player: 0, index: i as u8 }
}

#[test]
fn each_matching_play_places_this_tamer_under_it_then_protects_one_own_digimon() {
    for id in ["LEO", "BOSS", "DS"] {
        let mut r = runner(&[id]);
        r.place_on_field(0, CARD, Some(0));
        r.place_on_field(0, "OTHER", Some(0));
        let played = play_and_place(&mut r, id);
        assert_eq!(source_ids(&r, played), vec![CARD.to_string()], "{id}");
        assert!(!field_ids(&r, 0).contains(&CARD.to_string()));
        let v = r.pending_selection_view().expect("protect pick");
        assert!(!v.is_optional);
        let mut c = candidates(&r);
        c.sort();
        let mut want = vec!["OTHER".to_string(), id.to_string()];
        want.sort();
        assert_eq!(c, want);
        r.choose_prompt_card("OTHER", None).unwrap();
        no_prompt(&r);
        let other = find(&r, "OTHER");
        let played = find(&r, id);
        for m in [
            ModifierType::CannotBeReturnedToHand,
            ModifierType::CannotBeReturnedToDeck,
            ModifierType::ImmuneFromStackTrashing,
            ModifierType::CannotBeDeDigivolved,
        ] {
            assert!(r.modifiers().has(other, m), "{id}: {m:?}");
        }
        assert!(!r.modifiers().has(played, ModifierType::CannotBeReturnedToHand));
    }
}

#[test]
fn protection_blocks_opponent_bounce_until_their_turn_ends() {
    let mut r = runner(&["LEO"]);
    r.place_on_field(0, CARD, Some(0));
    let played = play_and_place(&mut r, "LEO");
    r.choose_prompt_card("LEO", None).unwrap();
    no_prompt(&r);
    let opp = r.place_on_field(1, "ATK", Some(0));
    r.end_turn();
    effect_bounce(&mut r, 1, opp, played);
    assert!(field_ids(&r, 0).contains(&"LEO".to_string()), "opponent's effect");
    r.end_turn();
    assert!(!r.modifiers().has(played, ModifierType::CannotBeReturnedToHand));
    assert!(!r.modifiers().has(played, ModifierType::ImmuneFromStackTrashing));
}

#[test]
fn own_effects_may_still_return_the_protected_digimon() {
    let mut r = runner(&["LEO"]);
    r.place_on_field(0, CARD, Some(0));
    let played = play_and_place(&mut r, "LEO");
    r.choose_prompt_card("LEO", None).unwrap();
    no_prompt(&r);
    effect_bounce(&mut r, 0, played, played);
    assert!(hand_ids(&r, 0).contains(&"LEO".to_string()));
}

#[test]
fn placement_may_be_declined() {
    let mut r = runner(&["LEO"]);
    r.place_on_field(0, CARD, Some(0));
    play_hand(&mut r, "LEO");
    r.decline_prompt().unwrap();
    no_prompt(&r);
    assert!(field_ids(&r, 0).contains(&CARD.to_string()));
    let h = find(&r, "LEO");
    assert!(source_ids(&r, h).is_empty());
}

#[test]
fn non_matching_or_opponent_plays_do_not_trigger() {
    let mut r = runner(&["PLAIN"]);
    r.place_on_field(0, CARD, Some(0));
    play_hand(&mut r, "PLAIN");
    assert!(r.pending_selection_view().is_none());
    r.end_turn();
    let i = hand_index(&r, 1, "LEO");
    assert!(r.play(1, i).is_some());
    assert!(r.pending_selection_view().is_none(), "opponent's Leomon");
    assert!(field_ids(&r, 0).contains(&CARD.to_string()));
}

#[test]
fn inherited_gives_matching_carrier_3000_dp() {
    for (id, bonus) in [("LEO", 3000), ("BOSS", 3000), ("DS", 3000), ("PLAIN", 0)] {
        let mut r = runner(&[]);
        let bare = r.place_on_field(0, id, Some(0));
        let base = r.effective_dp(bare).unwrap();
        let h = r.place_stack(0, &[CARD, id]);
        assert_eq!(r.effective_dp(h).unwrap(), base + bonus, "{id}");
    }
}

#[test]
fn security_plays_this_tamer_free() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("ATK", "Atk", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &[CARD])
        .start();
    let a = r.place_on_field(0, "ATK", Some(0)).index;
    attack_security(&mut r, a);
    no_prompt(&r);
    assert!(field_ids(&r, 1).contains(&CARD.to_string()));
}
