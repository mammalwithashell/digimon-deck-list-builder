//! BT24-007 Tsunomon — Digi-Egg Lv.2 Purple, Lesser/Titan/TS.
//!
//! Inherited: [Your Turn] [Once Per Turn] When level 4 or higher Digimon cards
//! with the [Demon] or [Titan] trait are trashed from your hand, you may play 1
//! of them with the play cost reduced by 2.
//!
//! DCGO: BT24/Purple/BT24_007.cs. G-ENGINE-DISCARDED-HAND-CARDS.

#[path = "../bt26/support.rs"]
mod support;

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use support::*;

const CARD_ID: &str = "BT24-007";

fn setup(first: u8) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-007")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .add_card(filler("FILLER"))
        .add_card(digimon("HOST3", "Host", CardColor::Purple, 3, 3, &[]))
        .add_card(digimon("TITAN4", "Titan Four", CardColor::Purple, 4, 5, &["Titan"]))
        .add_card(digimon("DEMON5", "Demon Five", CardColor::Purple, 5, 7, &["Demon"]))
        .add_card(digimon("TITAN3", "Titan Three", CardColor::Purple, 3, 4, &["Titan"]))
        .add_card(digimon("PLAIN4", "Plain Four", CardColor::Purple, 4, 5, &[]))
        .add_card(digimon("TITAN-HUGE", "Titan Huge", CardColor::Purple, 6, 18, &["Titan"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(first);
    r
}

/// Tsunomon under a Lv.3 host (inherited text is live), plus the test hand
/// trasher on the field for player 0.
fn board(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    r.place_stack(0, &[CARD_ID, "HOST3"])
}

fn trash_from_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let t = r.place_on_field(p, "T-HANDTRASH", Some(0));
    fire(r, EffectTiming::OnPlay, t);
    pick_hand(r, p, card_id);
}

#[test]
fn bt24_007_structure_single_inherited_opt_discard_observer() {
    use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
    let r = setup(0);
    let compiled = r.compiled_card(CARD_ID).expect("compiled BT24-007");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert_eq!(clause.when, vec![CompiledTiming::OnDiscardHand]);
    assert!(clause.once_per_turn);
    assert!(!clause.optional, "the pick is the optional part (decline refunds)");
}

#[test]
fn bt24_007_plays_trashed_titan_with_cost_reduced_by_two() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "TITAN4");
    trash_from_hand(&mut r, 0, "TITAN4");
    let v = r.pending_selection_view().expect("optional play pick");
    assert!(v.is_optional, "\"you may play\"");
    assert_eq!(non_pass(&r).len(), 1);
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TITAN4".to_string()));
    assert!(!trash_ids(&r, 0).contains(&"TITAN4".to_string()));
    assert_eq!(r.memory(), 5 - (5 - 2), "pays printed cost 5 reduced by 2");
}

#[test]
fn bt24_007_demon_trait_level_five_also_qualifies() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "DEMON5");
    trash_from_hand(&mut r, 0, "DEMON5");
    pick_trash(&mut r, 0, "DEMON5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DEMON5".to_string()));
    assert_eq!(r.memory(), 5 - (7 - 2));
}

#[test]
fn bt24_007_level_three_titan_does_not_trigger() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "TITAN3");
    trash_from_hand(&mut r, 0, "TITAN3");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(trash_ids(&r, 0).contains(&"TITAN3".to_string()));
    assert_eq!(r.memory(), 5);
}

#[test]
fn bt24_007_level_four_without_demon_or_titan_does_not_trigger() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "PLAIN4");
    trash_from_hand(&mut r, 0, "PLAIN4");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(trash_ids(&r, 0).contains(&"PLAIN4".to_string()));
}

#[test]
fn bt24_007_only_the_just_trashed_cards_are_candidates() {
    let mut r = setup(0);
    board(&mut r);
    // A qualifying card that was ALREADY in the trash is not "1 of them".
    push_trash(&mut r, 0, "DEMON5");
    push_hand(&mut r, 0, "TITAN4");
    trash_from_hand(&mut r, 0, "TITAN4");
    let v = r.pending_selection_view().expect("play pick");
    assert_eq!(non_pass(&r).len(), 1, "only TITAN4: {v:?}");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TITAN4".to_string()));
    assert!(!field_ids(&r, 0).contains(&"DEMON5".to_string()));
}

#[test]
fn bt24_007_does_not_trigger_on_opponents_turn() {
    let mut r = setup(1);
    board(&mut r);
    push_hand(&mut r, 0, "TITAN4");
    trash_from_hand(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"TITAN4".to_string()));
}

#[test]
fn bt24_007_does_not_trigger_on_opponents_hand_trash() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 1, "TITAN4");
    trash_from_hand(&mut r, 1, "TITAN4");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"TITAN4".to_string()));
    assert!(!field_ids(&r, 1).contains(&"TITAN4".to_string()));
}

#[test]
fn bt24_007_decline_keeps_the_once_per_turn_use() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, "DEMON5");
    trash_from_hand(&mut r, 0, "TITAN4");
    pass(&mut r, 0); // decline
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"TITAN4".to_string()));
    // Nothing was played ⇒ the [Once Per Turn] use is still available.
    trash_from_hand(&mut r, 0, "DEMON5");
    pick_trash(&mut r, 0, "DEMON5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DEMON5".to_string()));
}

#[test]
fn bt24_007_once_per_turn_after_a_play() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "TITAN4");
    push_hand(&mut r, 0, "DEMON5");
    trash_from_hand(&mut r, 0, "TITAN4");
    pick_trash(&mut r, 0, "TITAN4");
    let _ = r.auto_resolve();
    trash_from_hand(&mut r, 0, "DEMON5");
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none(), "OPT spent");
    assert!(trash_ids(&r, 0).contains(&"DEMON5".to_string()));
}

// Play-legality of the union pick: under "can't play Digimon by effects"
// (Pomumon (BT9-047)) the trashed [Titan] can't be played, so no pick opens and
// the card stays in the trash (bug scan 2026-11: the pick fed
// `play_union_bound_free` with no play-legality leaf; DCGO BT24_007.cs filters
// with `CanPlayAsNewPermanent`). The leaf is added by the DSL lowering
// (`digimon_dsl::play_candidates`).
#[test]
fn bt24_007_offers_no_pick_while_effects_cannot_play_digimon() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = setup(0);
    board(&mut r);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    push_hand(&mut r, 0, "TITAN4");
    trash_from_hand(&mut r, 0, "TITAN4");
    assert!(
        non_pass(&r).is_empty(),
        "no pick for a card the play would refuse: {:?}",
        r.pending_selection_view()
    );
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(trash_ids(&r, 0).contains(&"TITAN4".to_string()));
    assert!(!field_ids(&r, 0).contains(&"TITAN4".to_string()));
    assert_eq!(r.memory(), 5);
}

// Affordability of the paid play: "play 1 of them with the play cost reduced by 2"
// pays printed cost - 2 (DCGO BT24_007.cs:47 `CanPlayAsNewPermanent(cardSource, true,
// ..., fixedCost: cost - 2)`). A cost-18 Titan costs 16, more than the 5 + 10 the
// player can pay, so it is not offered.
#[test]
fn bt24_007_does_not_offer_a_titan_it_cannot_afford() {
    let mut r = setup(0);
    board(&mut r);
    push_hand(&mut r, 0, "TITAN-HUGE");
    trash_from_hand(&mut r, 0, "TITAN-HUGE");
    assert!(non_pass(&r).is_empty(), "unaffordable play is not offered: {:?}", r.pending_selection_view());
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(trash_ids(&r, 0).contains(&"TITAN-HUGE".to_string()));
}
