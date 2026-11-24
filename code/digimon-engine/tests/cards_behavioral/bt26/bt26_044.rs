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
    let opp_a = r.place_on_field(1, "OPP", Some(0));
    let opp_b = r.place_on_field(1, "OPP", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    pick_side_field(&mut r, 0, opp_a);
    pass(&mut r, 0); // decline the digivolve
    let _ = r.auto_resolve();
    // A second opponent suspend this turn re-offers it (DCGO RemoveUse). The
    // second Lalamon suspends the other, still-unsuspended Digimon: choosing
    // the already-suspended one is legal (Q845) but suspends nothing.
    let lala2 = r.place_stack(0, &["BT26-036", "FILLER"]);
    fire(&mut r, EffectTiming::WhenAttacking, lala2);
    pick_side_field(&mut r, 0, opp_b);
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

/// Accept the inherited save and pay with Tamer `t`. The Tamer pick is
/// declinable (owner ruling 2026-11-23, DCGO parity), so the payment is chosen
/// explicitly rather than by `auto_resolve`.
fn accept_and_pay_with(r: &mut DebugRunner, t: digimon_engine::permanent::PermanentHandle) {
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept the save");
    let id = digimon_engine::action::space::ATTACK_START + t.index as u16;
    let v = r.pending_selection_view().expect("Tamer pick");
    assert!(v.valid_action_ids.contains(&id), "{v:?}");
    r.execute_action(0, id).expect("pay with the Tamer");
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
    accept_and_pay_with(&mut r, t);
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

/// "When THIS Digimon ... would leave": another of your Digimon leaving is not
/// offered the inherited save (it leaves, no cost paid); the [DATA SQUAD]
/// carrier afterwards still is (the OPT was not spent). Audit 2026-11-11: DCGO
/// BT26_044.cs CanTriggerWhenRemoveField(hashtable, card).
#[test]
fn bt26_044_inherited_save_only_for_carrier_audit() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "DS6"]);
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 1);
    let other = r.place_on_field(0, "PLAIN6", Some(0));
    assert!(
        !delete_and_expect_replacement(&mut r, other),
        "no save for a Digimon that is not BT26-044's carrier"
    );
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&"PLAIN6".to_string()),
        "the other Digimon leaves"
    );
    assert_eq!(sources(&r, t), 1, "no cost paid");

    assert!(
        delete_and_expect_replacement(&mut r, carrier),
        "the carrier is still offered the save"
    );
    accept_and_pay_with(&mut r, t);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DS6".to_string()), "the carrier stays");
    assert_eq!(sources(&r, t), 0, "cost paid");
}

// ─── Inherited [OPT] save: DCGO parity (owner ruling 2026-11-23) ─────────────
//
// DCGO BT26_044.cs: offered only while one of your Tamers has a face-down
// card under it (`CanActivateCondition`); after the accept, a Tamer
// `SelectPermanentEffect` with canNoSelect true. No Tamer chosen ->
// `RemoveUse()` and the carrier leaves. A Tamer chosen -> `isUsed` (spent),
// its bottom face-down card is trashed and the carrier doesn't leave.
// qa/rules/mechanics-gaps-2026-10.md W14 row (f). <Evade> on the carrier keeps
// it after the would-leave window so the use can be observed on a later
// removal the same turn.

fn grant_evade(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.modifiers.grant_keyword(
        h,
        digimon_engine::enums::Keyword::Evade,
        digimon_engine::enums::Expiry::Permanent,
        0,
    );
}

/// While <Evade> can also answer the deletion, the engine first asks which
/// immediate replacement effect applies next (CR 4.3 15-8-5); pick <Evade>
/// (`evade`) or Lilamon's inherited save. With one candidate left, its own
/// accept prompt is already pending.
fn pick_replacement(r: &mut DebugRunner, evade: bool) {
    let v = r.pending_selection_view().expect("replacement prompt");
    if v.kind == SelectionKind::EffectChoice && v.prompt.contains("immediate replacement") {
        let row = v
            .effect_choices
            .as_ref()
            .and_then(|rows| rows.iter().find(|c| c.label.contains("Evade") == evade))
            .unwrap_or_else(|| panic!("replacement order rows: {:?}", v.effect_choices))
            .action_id;
        r.execute_action(v.selecting_player, row).expect("order pick");
    }
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
}

/// Delete the carrier with <Evade> also able to answer, and open the save's
/// own accept prompt.
fn delete_and_pick_the_save(
    r: &mut DebugRunner,
    carrier: digimon_engine::permanent::PermanentHandle,
) {
    r.game
        .delete_permanents_batch(vec![carrier], ReplacementCause::OpponentEffect);
    pick_replacement(r, false);
}

/// PASS is legal at the pending prompt (the action mask, not the raw ids).
fn pass_legal(r: &DebugRunner) -> bool {
    let p = r.pending_selection().expect("prompt").selecting_player;
    digimon_engine::action::build_action_mask(&r.game, p)
        [digimon_engine::action::space::PASS as usize]
        == 1.0
}

fn accept_evade(r: &mut DebugRunner) {
    pick_replacement(r, true);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept <Evade>");
    assert!(
        r.pending_selection().is_none(),
        "the same removal does not offer the declined clause again"
    );
}

fn carrier_handle(r: &DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let i = field_ids(r, 0)
        .iter()
        .position(|c| c == "DS6")
        .expect("DS6 on the field");
    digimon_engine::permanent::PermanentHandle { player: 0, index: i as u8 }
}

#[test]
fn bt26_044_inherited_outer_decline_keeps_opt() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "DS6"]);
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 2);
    grant_evade(&mut r, carrier);
    delete_and_pick_the_save(&mut r, carrier);
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the save");
    accept_evade(&mut r);
    assert_eq!(sources(&r, t), 2, "nothing trashed");
    let carrier = carrier_handle(&r);
    assert!(
        delete_and_expect_replacement(&mut r, carrier),
        "declined at the accept prompt: offered again this turn"
    );
}

#[test]
fn bt26_044_inherited_accept_then_decline_cost_keeps_opt() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "DS6"]);
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 2);
    grant_evade(&mut r, carrier);
    delete_and_pick_the_save(&mut r, carrier);
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept the save");
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField), "Tamer pick");
    assert!(
        pass_legal(&r),
        "DCGO BT26_044.cs canNoSelect: the Tamer pick may be declined after the accept"
    );
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the cost");
    assert_eq!(sources(&r, t), 2, "nothing trashed");
    accept_evade(&mut r);
    let carrier = carrier_handle(&r);
    assert!(
        delete_and_expect_replacement(&mut r, carrier),
        "DCGO RemoveUse: the declined cost keeps the [Once Per Turn]"
    );
    accept_and_pay_with(&mut r, t);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DS6".to_string()), "saved this time");
    assert_eq!(sources(&r, t), 1, "one face-down card paid");
}

#[test]
fn bt26_044_inherited_declined_cost_lets_carrier_leave() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "DS6"]);
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 1);
    assert!(delete_and_expect_replacement(&mut r, carrier));
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT)
        .expect("accept the save");
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField), "Tamer pick");
    assert!(pass_legal(&r), "the Tamer pick may be declined after the accept");
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the cost");
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"DS6".to_string()), "not paid: the carrier leaves");
    // The carrier left, so the Tamer moved down a slot.
    let _ = t;
    let tamer = field_ids(&r, 0)
        .iter()
        .position(|c| c == "TAMER")
        .expect("Tamer still on the field");
    let tamer = digimon_engine::permanent::PermanentHandle { player: 0, index: tamer as u8 };
    assert_eq!(sources(&r, tamer), 1, "nothing trashed");
}

#[test]
fn bt26_044_inherited_paid_spends_opt() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "DS6"]);
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 2);
    assert!(delete_and_expect_replacement(&mut r, carrier));
    accept_and_pay_with(&mut r, t);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1);
    let carrier = carrier_handle(&r);
    assert!(
        !delete_and_expect_replacement(&mut r, carrier),
        "spent: not offered again this turn though a face-down card remains"
    );
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"DS6".to_string()));
}

#[test]
fn bt26_044_inherited_not_offered_without_a_face_down_tamer_card() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "DS6"]);
    r.place_on_field(0, "TAMER", Some(0));
    assert!(
        !delete_and_expect_replacement(&mut r, carrier),
        "no face-down card under a Tamer: not offered (DCGO CanActivateCondition)"
    );
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"DS6".to_string()));
}

/// Q845 / Q1782: "You may suspend 1 of your opponent's Digimon or Tamers" may
/// choose one that is already suspended. It is not suspended by the effect; the
/// independent "Then" lock still follows (Q7035).
#[test]
fn bt26_044_may_choose_an_already_suspended_target_and_still_locks() {
    let mut r = setup();
    let lila = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    r.fire_on_play(0, lila.index as usize);
    let v = r.pending_selection_view().expect("optional suspend");
    assert!(v.is_optional);
    pick_first(&mut r, 0); // OPP, already suspended
    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional, "the lock is mandatory");
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
}

/// Lalamon's suspend may target an already-suspended Digimon, but nothing
/// suspends, so Lilamon's "When any of your opponent's Digimon or Tamers
/// suspend" digivolve does not trigger.
#[test]
fn bt26_044_choosing_an_already_suspended_digimon_does_not_trigger_the_digivolve() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "ROSE");
    let carrier = r.place_stack(0, &["BT26-036", "FILLER"]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let v = r
        .pending_selection_view()
        .expect("Lalamon offers the already-suspended Digimon");
    assert!(v.is_optional);
    pick_first(&mut r, 0);
    assert!(
        r.pending_selection_view().is_none(),
        "no suspend happened, so Lilamon's digivolve is not offered"
    );
}
