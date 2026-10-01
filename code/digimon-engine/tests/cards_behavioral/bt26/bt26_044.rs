//! BT26-044 Lilamon — Lv.5 Green, Fairy/DATA SQUAD.
//!
//! [On Play] [When Digivolving] You may suspend 1 of your opponent's Digimon or
//! Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn
//! ends.
//! [Your Turn] [Once Per Turn] When any of your opponent's Digimon or Tamers
//! suspend, or effects trash cards from under your Tamers, this Digimon may
//! digivolve into a [Vegetation], [Fairy] or [DATA SQUAD] trait Digimon card in
//! the hand with the cost reduced by 1.
//! Inherited: [All Turns] [Once Per Turn] When this Digimon with [Rosemon] in
//! its name or the [DATA SQUAD] trait would leave the battle area, by trashing
//! the bottom face-down card from under any of your Tamers, it doesn't leave.
//!
//! DCGO: BT26/Green/BT26_044.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT26-044";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-044")
        .dsl_card("BT26-036")
        .expect("Lalamon")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Green, &["DATA SQUAD"]))
        .add_card(tamer("OPP-T", "Opp Tamer", CardColor::Red, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        // Green Lv.6 Fairy with a Green Lv.5 / cost 4 circle.
        .add_card({
            let mut c = digimon(
                "ROSE",
                "Rosemon",
                CardColor::Green,
                6,
                12,
                &["Fairy", "DATA SQUAD"],
            );
            c.evo_costs = vec![digimon_engine::card_data::EvoCost {
                card_color: CardColor::Green as u8,
                level: 5,
                memory_cost: 4,
            }];
            c
        })
        .add_card(digimon(
            "DS6",
            "DS carrier",
            CardColor::Green,
            6,
            12,
            &["DATA SQUAD"],
        ))
        .add_card(digimon(
            "PLAIN6",
            "Plain carrier",
            CardColor::Green,
            6,
            12,
            &["Beast"],
        ))
        // A Rosemon-NAMED carrier without the [DATA SQUAD] trait.
        .add_card(digimon(
            "ROSE-X",
            "Rosemon X",
            CardColor::Green,
            6,
            12,
            &["Fairy"],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_044_suspends_then_locks() {
    let mut r = setup();
    let lila = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let opp_t = r.place_on_field(1, "OPP-T", Some(0));
    r.fire_on_play(0, lila.index as usize);
    let v = r.pending_selection_view().expect("optional suspend");
    assert!(v.is_optional);
    pick_first(&mut r, 0); // OPP
    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional, "the lock is mandatory");
    assert_eq!(v.valid_action_ids.len(), 2, "Digimon OR Tamer");
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert!(
        r.modifiers().has(opp, ModifierType::CannotUnsuspend)
            || r.modifiers().has(opp_t, ModifierType::CannotUnsuspend)
    );
}

#[test]
fn bt26_044_declining_suspend_still_locks() {
    let mut r = setup();
    let lila = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.fire_on_play(0, lila.index as usize);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert!(
        r.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "the 'Then' leg is independent"
    );
}

#[test]
fn bt26_044_opponent_suspend_lets_it_digivolve_for_one_less() {
    let mut r = setup();
    let lila = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "ROSE");
    // Lalamon's inherited suspends an opponent's Digimon on our turn.
    let carrier = r.place_stack(0, &["BT26-036", "FILLER"]);
    r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    pick_first(&mut r, 0);
    let v = r
        .pending_selection_view()
        .expect("Lilamon's may-digivolve hand pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "ROSE");
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[0].battle_area[lila.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "ROSE"
    );
    assert_eq!(r.memory(), 5 - 3, "circle cost 4 reduced by 1");
}

#[test]
fn bt26_044_declined_digivolve_refunds_once_per_turn() {
    let mut r = setup();
    let lila = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "ROSE");
    let carrier = r.place_stack(0, &["BT26-036", "FILLER"]);
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    pick_first(&mut r, 0);
    pass(&mut r, 0); // decline the digivolve
    let _ = r.auto_resolve();
    // A second opponent suspend this turn re-offers it (DCGO RemoveUse).
    let lala2 = r.place_stack(0, &["BT26-036", "FILLER"]);
    fire(&mut r, EffectTiming::WhenAttacking, lala2);
    pick_first(&mut r, 0);
    assert!(
        r.pending_selection_view().is_some(),
        "re-offered after a decline"
    );
    pick_hand(&mut r, 0, "ROSE");
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[0].battle_area[lila.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "ROSE"
    );
}

fn delete_and_expect_replacement(
    r: &mut DebugRunner,
    carrier: digimon_engine::permanent::PermanentHandle,
) -> bool {
    r.game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    matches!(r.pending_kind(), Some(SelectionKind::Replacement))
}

#[test]
fn bt26_044_inherited_prevents_leaving_for_data_squad_carrier() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "DS6"]);
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 1);
    assert!(
        delete_and_expect_replacement(&mut r, carrier),
        "optional replacement prompt"
    );
    let v = r.pending_selection_view().unwrap();
    r.execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 0).contains(&"DS6".to_string()),
        "did not leave"
    );
    assert_eq!(sources(&r, t), 0, "cost paid");
}

#[test]
fn bt26_044_inherited_ignores_non_rosemon_non_data_squad_carrier() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "PLAIN6"]);
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    assert!(!delete_and_expect_replacement(&mut r, carrier));
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&"PLAIN6".to_string()),
        "deleted normally"
    );
}

#[test]
fn bt26_044_inherited_protects_rosemon_named_carrier() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "ROSE-X"]);
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    assert!(
        delete_and_expect_replacement(&mut r, carrier),
        "[Rosemon] in its name qualifies"
    );
}
