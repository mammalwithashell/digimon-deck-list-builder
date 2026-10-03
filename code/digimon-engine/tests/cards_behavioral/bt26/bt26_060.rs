//! BT26-060 Chronomon: Destroy Mode — Lv.7 Black/Red.
//!
//! <Security A. +1> <Reboot> <Blocker> <Succession (Lv.6 w/[Chronomon] in name)>
//! [On Play] [When Digivolving] Return the top 5 stacked cards of 3 of your
//! opponent's Digimon to the top of the deck.
//! [All Turns] [Once Per Turn] When your effects add to decks, you may delete 1
//! of your opponent's Digimon.
//!
//! DCGO: BT26/Black/BT26_060.cs. G-ENGINE-RETURN-TOP-N-STACKED-TO-DECK.

use super::support::*;
use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT26-060";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-060")
        .dsl_card("BT26-016")
        .expect("BT26-016 Chronomon: Holy Mode")
        .add_card(filler("FILLER"))
        .add_card(digimon("A1", "A One", CardColor::Blue, 3, 3, &[]))
        .add_card(digimon("A2", "A Two", CardColor::Blue, 4, 4, &[]))
        .add_card(digimon("A3", "A Three", CardColor::Blue, 5, 5, &[]))
        .add_card(digimon("A4", "A Four", CardColor::Blue, 6, 6, &[]))
        .add_card(digimon("A5", "A Five", CardColor::Blue, 6, 7, &[]))
        .add_card(digimon("A6", "A Six", CardColor::Blue, 7, 8, &[]))
        .add_card(digimon("B1", "B One", CardColor::Blue, 3, 3, &[]))
        .add_card(digimon("B2", "B Two", CardColor::Blue, 4, 4, &[]))
        .add_card(digimon("C1", "C One", CardColor::Blue, 3, 3, &[]))
        .add_card(digimon("OLY", "Lv6 Other", CardColor::Black, 6, 9, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn pending_kind(r: &DebugRunner) -> Option<SelectionKind> {
    r.game.pending_selection.as_ref().map(|s| s.kind.clone())
}

/// Answer the 3-Digimon pick with every opponent Digimon offered.
fn pick_all_targets(r: &mut DebugRunner) {
    for _ in 0..3 {
        if !matches!(pending_kind(r), Some(SelectionKind::OppField)) {
            break;
        }
        let a = non_pass(r)[0];
        r.execute_action(0, a).unwrap();
    }
}

fn source_action(field: u8, stack_index: u16) -> u16 {
    encode_source_select(field as u16, stack_index).unwrap()
}

#[test]
fn bt26_060_returns_top_5_stacked_in_chosen_order_keeping_one() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    // A: 6-card stack (top A6) → 5 return, A1 stays. B: 2 cards → B2 returns.
    // C: a lone card → nothing returns.
    let a = r.place_stack(1, &["A1", "A2", "A3", "A4", "A5", "A6"]);
    let b = r.place_stack(1, &["B1", "B2"]);
    let _c = r.place_on_field(1, "C1", Some(0));
    let deck_before = r.game.players[1].deck.len();
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_all_targets(&mut r);

    // The order pick: the opponent's cards, ids in the SOURCE_SELECT range,
    // zone owner = the opponent, top card included.
    let sel = r.game.pending_selection.as_ref().expect("order pick");
    assert!(matches!(
        sel.kind,
        SelectionKind::OrderedPermutation { remaining: 6 }
    ));
    assert_eq!(r.game.current_phase, GamePhase::SelectPermutation);
    assert_eq!(sel.zone_owner, Some(1));
    assert!(!sel.is_optional, "every returned card is ordered");
    assert!(
        sel.valid_action_ids.contains(&source_action(a.index, 5)),
        "top card A6"
    );
    assert!(
        !sel.valid_action_ids.contains(&source_action(a.index, 0)),
        "A1 stays"
    );
    assert!(
        sel.valid_action_ids.contains(&source_action(b.index, 1)),
        "top card B2"
    );

    // Choose: B2 first (drawn first), then A3, then the rest as offered.
    r.execute_action(0, source_action(b.index, 1)).unwrap();
    r.execute_action(0, source_action(a.index, 2)).unwrap();
    while matches!(
        pending_kind(&r),
        Some(SelectionKind::OrderedPermutation { .. })
    ) {
        let x = non_pass(&r)[0];
        r.execute_action(0, x).unwrap();
    }
    // Decline the [All Turns] delete this own-effect deck add offers.
    while r.game.pending_selection.is_some() {
        pass(&mut r, 0);
    }
    let _ = r.auto_resolve();

    let deck = deck_ids(&r, 1);
    assert_eq!(deck.len(), deck_before + 6);
    assert_eq!(deck[deck.len() - 1], "B2", "1st pick is drawn first");
    assert_eq!(deck[deck.len() - 2], "A3", "2nd pick is next");
    assert_eq!(source_ids(&r, a).len() + 1, 1, "A keeps exactly one card");
    assert_eq!(top_id(&r, a), "A1");
    assert_eq!(top_id(&r, b), "B1");
    assert!(
        field_ids(&r, 1).contains(&"C1".to_string()),
        "a lone card never returns"
    );
}

#[test]
fn bt26_060_single_returned_card_needs_no_order_pick() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let b = r.place_stack(1, &["B1", "B2"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_all_targets(&mut r);
    assert!(
        !matches!(
            pending_kind(&r),
            Some(SelectionKind::OrderedPermutation { .. })
        ),
        "DCGO asks for an order only with 2+ cards"
    );
    while r.game.pending_selection.is_some() {
        pass(&mut r, 0);
    }
    let _ = r.auto_resolve();
    assert_eq!(deck_ids(&r, 1).last().map(String::as_str), Some("B2"));
    assert_eq!(top_id(&r, b), "B1");
}

#[test]
fn bt26_060_skips_a_digimon_whose_cards_cant_be_returned() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let b = r.place_stack(1, &["B1", "B2"]);
    // "Your opponent's effects can't return its cards to decks" (BT26-029's
    // grant shape): an opponent-effect-scoped CannotBeReturnedToDeck.
    r.game.modifiers.add(
        b,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToDeck,
            Expiry::EndOfTurn,
            1,
        ),
    );
    let deck_before = r.game.players[1].deck.len();
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_all_targets(&mut r);
    while r.game.pending_selection.is_some() {
        pass(&mut r, 0);
    }
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[1].deck.len(), deck_before);
    assert_eq!(top_id(&r, b), "B2");
}

#[test]
fn bt26_060_own_deck_add_offers_the_optional_delete_once_per_turn() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_stack(1, &["B1", "B2"]);
    let victim = r.place_on_field(1, "C1", Some(0));
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_all_targets(&mut r);
    // [All Turns][OPT] — outer "use this effect?" prompt, then the pick.
    let v = r.pending_selection_view().expect("optional delete offered");
    assert!(v.is_optional);
    let accept = non_pass(&r)[0];
    r.execute_action(0, accept).unwrap();
    let v = r.pending_selection_view().expect("delete pick");
    assert!(v.is_optional, "you MAY delete");
    let c1 = digimon_engine::action::space::encode_attack(0, victim.index as u16);
    assert!(v.valid_action_ids.contains(&c1));
    r.execute_action(0, c1).unwrap();
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 1).contains(&"C1".to_string()), "deleted");
    let _ = PermanentHandle {
        player: 0,
        index: 0,
    };
}

#[test]
fn bt26_060_declined_delete_refunds_the_once_per_turn() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_stack(1, &["B1", "B2"]);
    r.place_stack(1, &["A1", "A2"]);
    fire(&mut r, EffectTiming::OnPlay, me);
    pick_all_targets(&mut r);
    // The order pick for the two returned cards, then the outer prompt.
    while matches!(
        pending_kind(&r),
        Some(SelectionKind::OrderedPermutation { .. })
    ) {
        let x = non_pass(&r)[0];
        r.execute_action(0, x).unwrap();
    }
    let accept = non_pass(&r)[0];
    r.execute_action(0, accept).unwrap();
    // Decline the pick itself → RemoveUse.
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    // A second own-effect deck add this turn (fresh stacks to return from)
    // offers the delete again.
    r.place_stack(1, &["A3", "A4"]);
    r.place_stack(1, &["A5", "A6"]);
    fire(&mut r, EffectTiming::WhenDigivolving, me);
    pick_all_targets(&mut r);
    while matches!(
        pending_kind(&r),
        Some(SelectionKind::OrderedPermutation { .. })
    ) {
        let x = non_pass(&r)[0];
        r.execute_action(0, x).unwrap();
    }
    assert!(
        r.pending_selection_view().is_some_and(|v| v.is_optional),
        "the refunded once-per-turn is available again"
    );
}

#[test]
fn bt26_060_succession_adopts_chronomon_holy_mode() {
    let mut r = setup();
    let c = r.place_stack(0, &["BT26-016", CARD_ID]);
    assert!(
        r.game.has_keyword(c, Keyword::Piercing),
        "Holy Mode's <Piercing>"
    );
    assert!(
        r.game.has_keyword(c, Keyword::Engage),
        "Holy Mode's <Engage>"
    );
    let other = r.place_stack(0, &["OLY", CARD_ID]);
    assert!(
        !r.game.has_keyword(other, Keyword::Piercing),
        "no Lv.6 [Chronomon] source"
    );
}

#[test]
fn bt26_060_keywords_and_alt_paths() {
    let mut r = setup();
    let c = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(c, Keyword::Reboot));
    assert!(r.game.has_keyword(c, Keyword::Blocker));
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Chronomon") && alt.contains("Giant Slayer"),
        "{alt}"
    );
}
