//! `offer_unpayable_deletion_payers` on a paid-deletion payer pick
//! (G-DSL-OFFER-UNPAYABLE-DELETION-PAYERS, owner ruling 2026-11-24).
//!
//! A "by deleting 1 ... Digimon, it doesn't leave" clause normally hides
//! payers whose deletion is impossible, both from the replacement's offer and
//! from the pick (CR 4.3 15-7-4). With the opt-in the clause is offered while a
//! printed match exists, an undeletable payer is listed, and choosing it
//! deletes nothing, does not cancel the removal, and keeps the use spent.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

fn clause(card: &str, offer_unpayable: bool) -> String {
    format!(
        r#"
card: {card}
name: Payer Test
kind: digimon
level: 4
color: [black]
cost: 3
dp: 4000
effects:
  - kind: replacement
    scope: inherited
    trigger: when_would_leave_battle_area
    optional: true
    once_per_turn: true
    process:
      - select_own_permanent:
          bind_as: payer
          optional: true
          continue_on_decline: true
          offer_unpayable_deletion_payers: {offer_unpayable}
          filter:
            all_of:
              - other: true
              - kind: digimon
          prompt: "Delete 1 of your other Digimon"
      - delete_permanent: {{ target: payer }}
      - cancel_replacement: {{}}
      - if:
          condition: {{ binding_absent: payer }}
          then:
            - refund_opt: {{}}
"#
    )
}

fn digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

/// Carrier (the test card under `CARRIER`) and one own Digimon that can't be
/// deleted by effects.
fn board(card: &str, offer_unpayable: bool) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&clause(card, offer_unpayable))
        .expect("fixture compiles")
        .add_card(digimon("CARRIER"))
        .add_card(digimon("PAYER"))
        .deck(0, &["PAYER"; 6])
        .deck(1, &["PAYER"; 6])
        .start();
    r.skip_mulligan();
    let carrier = r.place_stack(0, &[card, "CARRIER"]);
    let payer = r.place_on_field(0, "PAYER", Some(0));
    r.game.modifiers.add(
        payer,
        ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect, 0, Expiry::Permanent, 0),
    );
    (r, carrier, payer)
}

fn ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

#[test]
fn offer_unpayable_deletion_payers_offers_and_lists_an_undeletable_payer() {
    let (mut r, carrier, payer) = board("TEST-PAYER-OFFER", true);
    r.game.delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "offered");
    r.execute_action(0, digimon_engine::action::space::REPLACEMENT_ACCEPT).unwrap();
    let id = digimon_engine::action::space::encode_attack(0, payer.index as u16);
    let pick = r.pending_selection_view().expect("payer pick");
    assert!(pick.valid_action_ids.contains(&id), "{pick:?}");
    r.execute_action(0, id).unwrap();
    let _ = r.auto_resolve();
    let field = ids(&r);
    assert!(field.contains(&"PAYER".to_string()), "nothing was deleted");
    assert!(!field.contains(&"CARRIER".to_string()), "the removal was not cancelled");
}

#[test]
fn without_offer_unpayable_deletion_payers_the_clause_is_not_offered() {
    let (mut r, carrier, _payer) = board("TEST-PAYER-FILTER", false);
    r.game.delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection().is_none(),
        "CR 4.3 15-7-4: no payable payer, so the clause is not offered"
    );
    assert!(!ids(&r).contains(&"CARRIER".to_string()));
}
