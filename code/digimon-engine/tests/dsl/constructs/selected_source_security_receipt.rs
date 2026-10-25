//! Completed selected-source payments, independent of the paging migration.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{PASS, REPLACEMENT_ACCEPT},
    },
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::{CardEffect, Effect, EffectBuilder},
    enums::{EffectTiming, Zone},
};
use std::sync::Arc;

const YAML: &str = r#"
card: SELECTED-RECEIPT
name: Selected Receipt
kind: digimon
color: [yellow]
level: 5
cost: 5
dp: 6000
effects:
  - when: on_play
    process:
      - select_material:
          of_permanent: source
          bind_as: chosen
          filter: {}
          prompt: Choose the source to place in security
      - security_place_stacked_card:
          carrier: source
          source: chosen
          of: you
          position: top
          face: down
          to_owner: true
          bind_success_as: paid
      - gain_memory: {binding_value: paid}
"#;

fn runner(yaml: &str) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("SELECTED-LOWER", "Lower"))
        .add_card(make_test_card("SELECTED-OBSERVER", "Observer"))
        .hand(0, &["SELECTED-LOWER"])
        .start();
    r.skip_mulligan();
    r.game.memory = 3;
    r.place_stack(0, &["SELECTED-LOWER", "SELECTED-RECEIPT"]);
    r
}

fn choose(r: &mut DebugRunner) {
    let host = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "SELECTED-RECEIPT")
        .unwrap();
    r.fire_on_play(0, host);
    let selection = r.pending_selection_view().expect("explicit source choice");
    let mask = build_action_mask(&r.game, 0);
    let action = *selection
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    assert!(mask[action as usize] > 0.5);
    r.execute_action(0, action).unwrap();
}

#[derive(Clone, Copy)]
enum Outcome {
    Cancel,
    Trash,
    Security,
    Custom,
}
struct Replace {
    outcome: Outcome,
    optional: bool,
}
impl CardEffect for Replace {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let outcome = self.outcome;
        let b = EffectBuilder::new(card, EffectTiming::WhenWouldPlaceInSecurity)
            .replacement_process(move |ctx| match outcome {
                Outcome::Cancel => ctx.cancel(),
                Outcome::Trash => ctx.redirect_to(Zone::Trash),
                Outcome::Security => ctx.redirect_to(Zone::Security),
                Outcome::Custom => ctx.handled(),
            });
        vec![if self.optional {
            b.optional().build()
        } else {
            b.build()
        }]
    }
}
fn observer(r: &mut DebugRunner, outcome: Outcome, optional: bool) {
    r.register_effect("SELECTED-OBSERVER", Arc::new(Replace { outcome, optional }));
    r.place_on_field(0, "SELECTED-OBSERVER", None);
}

#[test]
fn selected_source_receipt_pays_for_the_chosen_card_and_its_actual_owner() {
    let mut r = runner(YAML);
    r.game.players[0].battle_area[0].card_sources[0].owner = 1;
    let source = r.game.players[0].battle_area[0].card_sources[0].handle();
    let before = r.game.players[0].security.len();
    choose(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 4);
    assert_eq!(r.game.players[1].security.last().unwrap().handle(), source);
    assert_eq!(r.game.players[0].security.len(), before);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}

#[test]
fn selected_source_receipt_requires_completed_security_movement() {
    for (outcome, paid) in [
        (Outcome::Cancel, false),
        (Outcome::Trash, false),
        (Outcome::Security, true),
        (Outcome::Custom, false),
    ] {
        let mut r = runner(YAML);
        observer(&mut r, outcome, false);
        choose(&mut r);
        r.auto_resolve().unwrap();
        assert_eq!(r.game.memory, 3 + i16::from(paid));
    }
}

#[test]
fn selected_source_pending_receipt_survives_clone_and_accept_or_decline() {
    let mut r = runner(YAML);
    observer(&mut r, Outcome::Cancel, true);
    choose(&mut r);
    assert!(r.game.pending_selection.is_some());
    let fork = r.game.clone();
    for (action, paid) in [(REPLACEMENT_ACCEPT, false), (PASS, true)] {
        r.game = fork.clone();
        r.game.resolve_selection(0, action).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.memory, 3 + i16::from(paid));
    }
}

#[test]
fn invalid_selected_source_publishes_zero() {
    let yaml = YAML.replace("      - select_material:\n          of_permanent: source\n          bind_as: chosen\n          filter: {}\n          prompt: Choose the source to place in security\n", "")
        .replace("          source: chosen", "          source_index_from_top: 99");
    let mut r = runner(&yaml);
    r.fire_on_play(0, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 3);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
}

#[test]
fn a_bound_card_outside_the_carrier_cannot_pay_its_source_cost() {
    let yaml = YAML
        .replace("select_material:", "select_hand:")
        // This is a selector-argument rewrite, not an of_permanent predicate
        // test. Keep the Rust rewrite fragment out of YAML-leaf attribution.
        .replace(concat!("of_permanent", ": source"), "of: you");
    let mut r = runner(&yaml);
    let hand = r.game.players[0].hand[0].handle();
    choose(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 3);
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|card| card.handle() == hand));
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
}

#[test]
fn selected_source_prohibition_reports_unpaid_and_keeps_the_source() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = runner(YAML);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );
    choose(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 3);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
}

#[test]
fn an_invalid_later_attempt_cannot_reuse_a_previous_paid_result() {
    let yaml = YAML.replace(
        "      - gain_memory: {binding_value: paid}",
        r#"      - gain_memory: {binding_value: paid}
      - security_place_stacked_card:
          carrier: source
          source_index_from_top: 99
          of: you
          position: top
          face: down
          bind_success_as: paid
      - gain_memory: {binding_value: paid}"#,
    );
    let mut r = runner(&yaml);
    choose(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 4);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}

#[test]
fn pending_selected_source_payment_survives_carrier_field_compaction() {
    let mut r = runner(YAML);
    r.place_stack(0, &["SELECTED-LOWER"]);
    r.game.players[0].battle_area.swap(0, 1);
    observer(&mut r, Outcome::Cancel, true);
    let source = r.game.players[0].battle_area[1].card_sources[0].handle();
    choose(&mut r);
    assert!(r.game.pending_selection.is_some());
    r.game.players[0].battle_area.remove(0);
    r.game.resolve_selection(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 4);
    assert_eq!(r.game.players[0].security.last().unwrap().handle(), source);
}

#[test]
fn pending_selected_source_payment_survives_compaction_within_its_stack() {
    let mut r = runner(YAML);
    let old = r.game.players[0].battle_area.remove(0);
    r.game.players[0].trash.extend(old.card_sources);
    r.place_stack(0, &["SELECTED-LOWER", "SELECTED-LOWER", "SELECTED-RECEIPT"]);
    observer(&mut r, Outcome::Cancel, true);
    let source = r.game.players[0].battle_area[0].card_sources[1].handle();
    r.fire_on_play(0, 0);
    let selection = r.pending_selection_view().unwrap();
    let action = selection
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .nth(1)
        .unwrap();
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.execute_action(0, action).unwrap();
    assert!(r.game.pending_selection.is_some());
    r.game.players[0].battle_area[0].card_sources.remove(0);
    r.game.resolve_selection(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 4);
    assert_eq!(r.game.players[0].security.last().unwrap().handle(), source);
}

#[test]
fn selected_active_top_retains_owned_whole_permanent_placement_semantics() {
    let yaml = YAML.replace("      - select_material:\n          of_permanent: source\n          bind_as: chosen\n          filter: {}\n          prompt: Choose the source to place in security\n", "")
        .replace("          source: chosen", "          source: self");
    for (action, paid) in [
        (None, true),
        (Some(PASS), true),
        (Some(REPLACEMENT_ACCEPT), false),
    ] {
        let mut r = runner(&yaml);
        let top = r.game.players[0].battle_area[0].top_card().handle();
        let lower = r.game.players[0].battle_area[0].card_sources[0].handle();
        if action.is_some() {
            observer(&mut r, Outcome::Cancel, true);
        }
        r.fire_on_play(0, 0);
        if let Some(action) = action {
            assert!(r.game.pending_selection.is_some());
            r.game.resolve_selection(0, action).unwrap();
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.game.memory, 3 + i16::from(paid));
        assert_eq!(
            r.game.players[0].security.iter().any(|c| c.handle() == top),
            paid
        );
        assert_eq!(
            r.game.players[0].trash.iter().any(|c| c.handle() == lower),
            paid
        );
    }
}
