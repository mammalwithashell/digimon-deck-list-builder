//! BT26-076 Crowmon — Lv.5 Purple, Mysterious Bird/DATA SQUAD.
//!
//! [When Digivolving] Delete 1 of your opponent's level 4 or lower Digimon.
//! Then, by trashing the bottom face-down card from under any of your Tamers,
//! they trash 1 card in their hand.
//! [Your Turn] [Once Per Turn] When your opponent's hand is trashed from or
//! effects trash cards from under your Tamers, this Digimon may digivolve into
//! [Ravemon] or a [DATA SQUAD] trait Digimon card in the trash with the cost
//! reduced by 1.
//! Inherited: [On Deletion] You may play 1 play cost 5 or lower card with
//! [Avian] or [Bird] in any of its traits or the [DATA SQUAD] trait from your
//! trash without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_076.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-076";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-076")
        .dsl_card("BT26-072")
        .expect("Peckmon")
        .dsl_card("BT26-094")
        .expect("Keenan")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Purple, &["DATA SQUAD"]))
        .add_card(digimon("OPP4", "Opp4", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("JUNK", "Junk", CardColor::Red, 3, 3, &[]))
        .add_card({
            let mut c = digimon("RAVE", "Ravemon", CardColor::Purple, 6, 12, &["Cyborg"]);
            c.evo_costs = vec![digimon_engine::card_data::EvoCost {
                card_color: CardColor::Purple as u8,
                level: 5,
                memory_cost: 4,
            }];
            c
        })
        .add_card(digimon("BIRD5", "Birdy", CardColor::Purple, 4, 5, &["Mysterious Bird"]))
        .add_card(digimon("BIRD6", "Big Birdy", CardColor::Purple, 5, 6, &["Avian"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_076_wd_deletes_then_pays_for_opponent_discard() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 1);
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP4", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    push_hand(&mut r, 1, "JUNK");
    fire(&mut r, EffectTiming::WhenDigivolving, crow);
    let _ = r.pending_selection_view().expect("delete pick");
    pick_first(&mut r, 0); // the only Lv.4
    let v = r.pending_selection_view().expect("optional Tamer pick (cost)");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    assert_eq!(r.game.pending_selection.as_ref().unwrap().selecting_player, 1, "opponent chooses");
    pick_hand(&mut r, 1, "JUNK");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
    assert_eq!(sources(&r, t), 0);
    assert_eq!(r.hand_size(1), 0);
}

#[test]
fn bt26_076_wd_declining_cost_skips_discard() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP4", Some(0));
    push_hand(&mut r, 1, "JUNK");
    fire(&mut r, EffectTiming::WhenDigivolving, crow);
    pick_first(&mut r, 0);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
    assert_eq!(r.hand_size(1), 1, "no discard without the cost");
}

#[test]
fn bt26_076_opponent_hand_trash_digivolves_from_trash() {
    let mut r = setup();
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "RAVE");
    push_hand(&mut r, 1, "JUNK");
    // Peckmon's inherited [On Deletion] makes the opponent trash from hand.
    let carrier = r.place_stack(0, &["BT26-072", "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    pick_hand(&mut r, 1, "JUNK");
    r.accept_optional_trigger().expect("reactive digivolve offered (yes/no)");
    pick_first(&mut r, 0); // RAVE from trash
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[0].battle_area[crow.index as usize].top_card().card_id(&r.game.card_data),
        "RAVE"
    );
    assert_eq!(r.memory(), 5 - 3, "circle cost 4 reduced by 1");
    assert!(trash_ids(&r, 0).is_empty(), "RAVE left the trash");
}

#[test]
fn bt26_076_reactive_needs_a_trash_candidate() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 1, "JUNK");
    let carrier = r.place_stack(0, &["BT26-072", "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    pick_hand(&mut r, 1, "JUNK");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "no [Ravemon]/[DATA SQUAD] in trash ⇒ no trigger");
}

/// Crowmon's own [When Digivolving] trashes from under a Tamer, which fires
/// its own reactive digivolve mid-resolution (the engine drains
/// OnDigivolutionCardTrashed observers synchronously). The observer's prompt
/// must not be clobbered by the rest of the [When Digivolving] (the opponent's
/// discard pick) — G-DSL-TAIL-CLOBBERS-INLINE-OBSERVER-SELECTION.
#[test]
fn bt26_076_self_caused_tamer_trash_prompt_survives_the_discard() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    let crow = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "RAVE");
    push_hand(&mut r, 1, "JUNK");
    fire(&mut r, EffectTiming::WhenDigivolving, crow);
    pick_first(&mut r, 0); // Tamer (cost) — no delete target exists
    r.accept_optional_trigger().expect("reactive digivolve offered");
    pick_first(&mut r, 0); // RAVE
    // The rest of the [When Digivolving] still runs: the opponent discards.
    pick_hand(&mut r, 1, "JUNK");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(1), 0);
    assert_eq!(field_ids(&r, 0).last().map(String::as_str), Some("RAVE"));
}

#[test]
fn bt26_076_inherited_on_deletion_plays_bird_from_trash() {
    let mut r = setup();
    push_trash(&mut r, 0, "BIRD5");
    push_trash(&mut r, 0, "BIRD6"); // cost 6 — too expensive
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    let v = r.pending_selection_view().expect("optional trash pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.iter().filter(|&&a| a != digimon_engine::action::space::PASS).count(), 1);
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"BIRD5".to_string()));
}

