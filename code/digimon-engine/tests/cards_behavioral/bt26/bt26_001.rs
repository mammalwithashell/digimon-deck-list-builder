//! BT26-001 Yokomon — Digi-Egg Lv.2 Red (Bulb / Iliad / TS).
//!
//! Inherited: [Your Turn] [Once Per Turn] When your effects add to decks, this
//! Digimon may digivolve into a Digimon card with [Chronomon] in its text in
//! the hand with the cost reduced by 1.
//!
//! DCGO: BT26/Red/BT26_001.cs (OnAddLibraryAnyone + IsOwnerEffect).
//! Engine timing: `on_add_to_deck` (G-ENGINE-ON-ADD-TO-DECK).

use super::support::*;
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-001";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-001")
        .from_dsl_yaml(DECKER_YAML)
        .expect("decker")
        .from_dsl_yaml(REVEALER_YAML)
        .expect("revealer")
        .add_card(filler("FILLER"))
        .add_card(digimon("HOST", "Host", CardColor::Red, 3, 3, &["TS"]))
        .add_card({
            let mut c = digimon("CHRONO4", "Chronomon Kid", CardColor::Red, 4, 5, &[]);
            c.evo_costs = vec![EvoCost {
                card_color: CardColor::Red as u8,
                level: 3,
                memory_cost: 3,
            }];
            c
        })
        .add_card({
            let mut c = digimon("CHRONO5", "Chronomon Teen", CardColor::Red, 5, 7, &[]);
            c.evo_costs = vec![EvoCost {
                card_color: CardColor::Red as u8,
                level: 4,
                memory_cost: 1,
            }];
            c
        })
        .add_card({
            let mut c = digimon("PLAIN4", "Plainmon", CardColor::Red, 4, 5, &[]);
            c.evo_costs = vec![EvoCost {
                card_color: CardColor::Red as u8,
                level: 3,
                memory_cost: 3,
            }];
            c
        })
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

/// Yokomon under a Lv.3 host, a trash card to return, and the decker on field.
fn board(r: &mut DebugRunner, decker_owner: u8) -> (PermanentHandle, PermanentHandle) {
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    push_trash(r, decker_owner, "FILLER");
    let decker = r.place_on_field(decker_owner, "T-DECKER", Some(0));
    (host, decker)
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

#[test]
fn bt26_001_own_deck_add_may_digivolve_into_chronomon_text_card_at_minus_one() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN4");
    push_hand(&mut r, 0, "CHRONO4");
    let (host, decker) = board(&mut r, 0);
    fire(&mut r, EffectTiming::OnPlay, decker);
    drain_first(&mut r); // decker picks the trash card (until Yokomon's prompt)
                         // Note: drain_first answered the optional prompt with its first non-PASS
                         // action (accept) and the hand pick — only CHRONO4 is a legal pick.
    assert_eq!(
        top_id(&r, host),
        "CHRONO4",
        "digivolved into the [Chronomon] card"
    );
    assert_eq!(r.memory(), 5 - 2, "cost 3 reduced by 1");
    assert!(hand_ids(&r, 0).contains(&"PLAIN4".to_string()));
}

#[test]
fn bt26_001_trigger_is_optional_and_only_offers_chronomon_text_cards() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN4");
    push_hand(&mut r, 0, "CHRONO4");
    let (host, decker) = board(&mut r, 0);
    fire(&mut r, EffectTiming::OnPlay, decker);
    // Decker's mandatory trash pick.
    pick_first(&mut r, 0);
    assert!(r.pending_is_optional(), "'may digivolve'");
    r.accept_optional_trigger().expect("accept");
    let v = r.pending_selection_view().expect("hand pick");
    let plain = digimon_engine::action::space::PLAY_HAND_START;
    assert!(
        !v.valid_action_ids.contains(&plain),
        "PLAIN4 has no [Chronomon] text"
    );
    pick_hand(&mut r, 0, "CHRONO4");
    assert_eq!(top_id(&r, host), "CHRONO4");
}

#[test]
fn bt26_001_declining_keeps_the_stack() {
    let mut r = setup();
    push_hand(&mut r, 0, "CHRONO4");
    let (host, decker) = board(&mut r, 0);
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    r.decline_optional_trigger().expect("decline");
    assert_eq!(top_id(&r, host), "HOST");
    assert_eq!(r.memory(), 5);
}

#[test]
fn bt26_001_no_candidate_no_trigger() {
    let mut r = setup();
    push_hand(&mut r, 0, "PLAIN4");
    let (host, decker) = board(&mut r, 0);
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(top_id(&r, host), "HOST");
}

#[test]
fn bt26_001_reveal_round_trip_does_not_count_as_adding_to_deck() {
    let mut r = setup();
    push_hand(&mut r, 0, "CHRONO4");
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let rev = r.place_on_field(0, "T-REVEALER", Some(0));
    fire(&mut r, EffectTiming::OnPlay, rev);
    drain_first(&mut r);
    assert_eq!(
        top_id(&r, host),
        "HOST",
        "revealed cards were already in the deck"
    );
}

#[test]
fn bt26_001_opponent_effect_does_not_trigger() {
    let mut r = setup();
    push_hand(&mut r, 0, "CHRONO4");
    let (host, decker) = board(&mut r, 1);
    fire(&mut r, EffectTiming::OnPlay, decker);
    drain_first(&mut r);
    assert_eq!(deck_ids(&r, 1).len(), 7, "opponent returned a card");
    assert_eq!(top_id(&r, host), "HOST", "only YOUR effects");
}

#[test]
fn bt26_001_only_on_your_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "CHRONO4");
    let (host, decker) = board(&mut r, 0);
    r.set_first_player(1);
    fire(&mut r, EffectTiming::OnPlay, decker);
    drain_first(&mut r);
    assert_eq!(deck_ids(&r, 0).len(), 7);
    assert_eq!(top_id(&r, host), "HOST", "[Your Turn]");
}

#[test]
fn bt26_001_once_per_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, "CHRONO4");
    push_hand(&mut r, 0, "CHRONO5");
    let (host, decker) = board(&mut r, 0);
    push_trash(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    r.accept_optional_trigger().expect("accept");
    pick_hand(&mut r, 0, "CHRONO4");
    assert_eq!(top_id(&r, host), "CHRONO4");
    // CHRONO5 is now a legal candidate on the Lv.4 stack, but the inherited
    // effect already resolved this turn.
    fire(&mut r, EffectTiming::OnPlay, decker);
    pick_first(&mut r, 0);
    assert!(r.game.pending_selection.is_none(), "[Once Per Turn]");
    assert_eq!(top_id(&r, host), "CHRONO4");
}
