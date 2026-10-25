use digimon_engine::debug_runner::{make_test_card, DebugRunner};
const YAML: &str = r#"
card: RECEIPT-TOP
name: Receipt Top
kind: digimon
color: [yellow]
level: 4
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - security_place_top_stacked_card:
          carrier: source
          of: you
          position: top
          face: down
          bind_success_as: paid
      - gain_memory: { binding_value: paid }
"#;
#[test]
fn successful_top_placement_publishes_payment() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(make_test_card("LOWER", "Lower"))
        .start();
    r.skip_mulligan();
    r.game.memory = 3;
    r.place_stack(0, &["LOWER", "RECEIPT-TOP"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    r.fire_on_play(0, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security.last().unwrap().handle(), top);
    assert_eq!(r.game.memory, 4);
}
use digimon_engine::{
    action::{
        build_action_mask,
        space::{PASS, REPLACEMENT_ACCEPT},
    },
    card_source::CardHandle,
    effect::{CardEffect, Effect, EffectBuilder},
    enums::{EffectTiming, Expiry, ModifierType, Zone},
    modifiers::PlayerModifierEntry,
};
use std::sync::Arc;

fn runner(yaml: &str) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("LOWER", "Lower"))
        .add_card(make_test_card("OBSERVER", "Observer"))
        .start();
    r.skip_mulligan();
    r.game.memory = 3;
    r.place_stack(0, &["LOWER", "RECEIPT-TOP"]);
    r
}
#[derive(Clone, Copy)]
enum Replacement {
    Cancel,
    Trash,
    Security,
    Custom,
}
struct Replace {
    outcome: Replacement,
    optional: bool,
}
impl CardEffect for Replace {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let outcome = self.outcome;
        let b = EffectBuilder::new(c, EffectTiming::WhenWouldPlaceInSecurity).replacement_process(
            move |ctx| match outcome {
                Replacement::Cancel => ctx.cancel(),
                Replacement::Trash => ctx.redirect_to(Zone::Trash),
                Replacement::Security => ctx.redirect_to(Zone::Security),
                Replacement::Custom => ctx.handled(),
            },
        );
        vec![if self.optional {
            b.optional().build()
        } else {
            b.build()
        }]
    }
}
fn observer(r: &mut DebugRunner, outcome: Replacement, optional: bool) {
    r.register_effect("OBSERVER", Arc::new(Replace { outcome, optional }));
    r.place_on_field(0, "OBSERVER", None);
}
fn fire(r: &mut DebugRunner) {
    r.fire_on_play(0, 0);
}
fn finish(r: &mut DebugRunner) {
    r.auto_resolve().unwrap();
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn prohibition_and_empty_under_stack_publish_zero() {
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
    fire(&mut r);
    finish(&mut r);
    assert_eq!(r.game.memory, 3);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
    let mut r = runner(YAML);
    r.game.players[0].battle_area[0].card_sources.remove(0);
    fire(&mut r);
    finish(&mut r);
    assert_eq!(r.game.memory, 3);
}
#[test]
fn synchronous_cancel_redirect_and_custom_only_pay_for_security() {
    for (outcome, paid) in [
        (Replacement::Cancel, false),
        (Replacement::Trash, false),
        (Replacement::Security, true),
        (Replacement::Custom, false),
    ] {
        let mut r = runner(YAML);
        observer(&mut r, outcome, false);
        fire(&mut r);
        finish(&mut r);
        assert_eq!(r.game.memory, 3 + i16::from(paid));
        assert_eq!(r.game.players[0].security.len(), usize::from(paid));
    }
}
#[test]
fn optional_replacement_clones_resume_receipt_after_accept_and_decline() {
    for outcome in [
        Replacement::Cancel,
        Replacement::Trash,
        Replacement::Security,
    ] {
        let mut r = runner(YAML);
        observer(&mut r, outcome, true);
        fire(&mut r);
        assert!(r.game.pending_selection.is_some());
        assert_eq!(r.game.memory, 3, "tail waits for actual payment");
        let saved = r.game.clone();
        for (action, paid) in [
            (PASS, true),
            (REPLACEMENT_ACCEPT, matches!(outcome, Replacement::Security)),
        ] {
            r.game = saved.clone();
            assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
            r.game.resolve_selection(0, action).unwrap();
            finish(&mut r);
            assert_eq!(r.game.memory, 3 + i16::from(paid));
            assert_eq!(r.game.players[0].security.len(), usize::from(paid));
            assert_eq!(
                r.game.verification_digest(),
                r.game.clone().verification_digest()
            );
        }
    }
}
#[test]
fn duplicate_printed_ids_have_independent_physical_receipts() {
    let mut r = runner(YAML);
    r.place_stack(0, &["LOWER", "RECEIPT-TOP"]);
    let a = r.game.players[0].battle_area[0].top_card().handle();
    let b = r.game.players[0].battle_area[1].top_card().handle();
    assert_ne!(a, b);
    fire(&mut r);
    finish(&mut r);
    r.fire_on_play(0, 1);
    finish(&mut r);
    assert_eq!(r.game.memory, 5);
    assert_eq!(
        r.game.players[0]
            .security
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>(),
        vec![a, b]
    );
}
#[test]
fn opt_in_owner_destination_preserves_default_controller_destination() {
    for to_owner in [false, true] {
        let yaml = YAML.replace(
            "bind_success_as: paid",
            &format!("bind_success_as: paid\n          to_owner: {to_owner}"),
        );
        let mut r = runner(&yaml);
        r.game.players[0].battle_area[0]
            .card_sources
            .last_mut()
            .unwrap()
            .owner = 1;
        let top = r.game.players[0].battle_area[0].top_card().handle();
        fire(&mut r);
        finish(&mut r);
        let player = usize::from(to_owner);
        assert_eq!(
            r.game.players[player].security.last().unwrap().handle(),
            top
        );
        assert_eq!(r.game.memory, 4);
    }
}
#[test]
fn completed_and_pending_receipts_leave_no_replay_reset_history() {
    let mut reused = runner(YAML);
    observer(&mut reused, Replacement::Cancel, true);
    fire(&mut reused);
    assert!(reused.game.pending_selection.is_some());
    reused.game.reset_for_replay();
    assert!(
        reused.game.pending_selection_resume.is_none(),
        "reset discards parked typed selection continuation"
    );
    assert!(
        reused.game.after_selection_resume_hooks.0.is_empty(),
        "reset discards parked after-selection hooks"
    );
    let mut fresh = runner(YAML);
    observer(&mut fresh, Replacement::Cancel, true);
    fresh.game.reset_for_replay();
    assert_eq!(
        reused.game.verification_digest(),
        fresh.game.verification_digest()
    );
    for r in [&mut reused, &mut fresh] {
        r.game.current_phase = digimon_engine::enums::GamePhase::Main;
        r.game.memory = 3;
        r.place_stack(0, &["LOWER", "RECEIPT-TOP"]);
        r.place_on_field(0, "OBSERVER", None);
        fire(r);
    }
    assert_eq!(
        build_action_mask(&reused.game, 0),
        build_action_mask(&fresh.game, 0)
    );
    assert_eq!(
        reused.game.verification_digest(),
        fresh.game.verification_digest()
    );
    let mut cloned = DebugRunner::wrap(reused.game.clone());
    for r in [&mut reused, &mut fresh, &mut cloned] {
        r.game.resolve_selection(0, PASS).unwrap();
        finish(r);
        assert_eq!(r.game.memory, 4, "exactly one new receipt pays after reset");
        assert_eq!(r.game.players[0].security.len(), 1);
    }
    assert_eq!(
        reused.game.verification_digest(),
        fresh.game.verification_digest()
    );
    assert_eq!(
        reused.game.verification_digest(),
        cloned.game.verification_digest()
    );
}
#[test]
fn receipt_absent_preserves_legacy_unconditional_tail() {
    let yaml = YAML
        .replace("          bind_success_as: paid\n", "")
        .replace("{ binding_value: paid }", "1");
    let mut r = runner(&yaml);
    observer(&mut r, Replacement::Cancel, false);
    fire(&mut r);
    finish(&mut r);
    assert_eq!(r.game.memory, 4);
    assert!(r.game.players[0].security.is_empty());
}
#[test]
fn later_failed_attempt_overwrites_prior_success_binding() {
    let yaml = YAML.replace("      - gain_memory:", "      - security_place_top_stacked_card:\n          carrier: source\n          of: you\n          position: top\n          face: down\n          bind_success_as: paid\n      - gain_memory:");
    let mut r = runner(&yaml);
    fire(&mut r);
    finish(&mut r);
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(
        r.game.memory, 3,
        "second attempt has no card below it and must publish zero"
    );
}
#[test]
fn visible_top_alias_uses_same_receipt_continuation() {
    let yaml = YAML.replace(
        "security_place_top_stacked_card:",
        "security_place_top_card:",
    );
    let mut r = runner(&yaml);
    observer(&mut r, Replacement::Cancel, true);
    fire(&mut r);
    assert_eq!(r.game.memory, 3);
    r.game.resolve_selection(0, PASS).unwrap();
    finish(&mut r);
    assert_eq!(r.game.memory, 4);
    assert_eq!(r.game.players[0].security.len(), 1);
}
#[test]
fn pending_request_does_not_pay_for_another_physical_copy_with_same_printed_id() {
    let mut r = runner(YAML);
    r.place_stack(0, &["LOWER", "RECEIPT-TOP"]);
    observer(&mut r, Replacement::Cancel, true);
    let original = r.game.players[0].battle_area[0].top_card().handle();
    let other = r.game.players[0].battle_area[1].top_card().handle();
    assert_ne!(original, other);
    fire(&mut r);
    // Model a nested replacement's intervening movement: the current top is
    // now another physical copy. The captured placement must not follow it.
    let a = r.game.players[0].battle_area[0].card_sources.pop().unwrap();
    let b = r.game.players[0].battle_area[1].card_sources.pop().unwrap();
    r.game.players[0].battle_area[0].card_sources.push(b);
    r.game.players[0].battle_area[1].card_sources.push(a);
    r.game.resolve_selection(0, PASS).unwrap();
    finish(&mut r);
    assert_eq!(r.game.memory, 3);
    assert!(r.game.players[0].security.is_empty());
    assert_eq!(r.game.players[0].battle_area[0].top_card().handle(), other);
    assert_eq!(
        r.game.players[0].battle_area[1].top_card().handle(),
        original
    );
}

#[test]
fn source_carrier_is_pinned_after_prior_synchronous_field_compaction() {
    let yaml = YAML.replace(
        "      - security_place_top_stacked_card:",
        "      - delete_all_permanents: {over: {of: you, name_is: Lower}}\n      - security_place_top_stacked_card:",
    ) + "\n      - if:\n          condition: {equals: [paid, 1]}\n          then:\n            - unsuspend: {target: source}\n";
    let mut r = runner(&yaml);
    r.place_on_field(0, "LOWER", None);
    r.game.players[0].battle_area.swap(0, 1);
    r.game.players[0].battle_area[1].is_suspended = true;
    observer(&mut r, Replacement::Cancel, true);
    r.fire_on_play(0, 1);
    assert!(r.pending_selection().is_some());
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "RECEIPT-TOP");
    let saved = r.game.clone();
    for (action, paid) in [(PASS, true), (REPLACEMENT_ACCEPT, false)] {
        r.game = saved.clone();
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.0);
        r.game.resolve_selection(0, action).unwrap();
        finish(&mut r);
        assert_eq!(r.game.players[0].battle_area[0].is_suspended, !paid);
        assert_eq!(r.game.players[0].security.len(), usize::from(paid));
    }
}
