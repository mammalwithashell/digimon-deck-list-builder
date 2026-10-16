//! Recorder controls: these inspect real movement, not card-effect stand-ins.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{PASS, REPLACEMENT_ACCEPT},
};
use digimon_engine::enums::{Expiry, GamePhase, Keyword};
use digimon_engine::{
    debug_runner::DebugRunner, effect_context::EffectContext, permanent::PermanentHandle,
};
fn fixture() -> DebugRunner {
    let mut data = digimon_engine::card_data::CardData::load_from_str(include_str!(
        "../../../data/cards.json"
    ))
    .unwrap();
    let mut b = DebugRunner::builder();
    for id in ["BT9-109", "BT9-062", "ST1-03", "ST5-15"] {
        b = b.add_card(data.remove(id).unwrap());
    }
    let mut r = b
        .deck(0, &["ST1-03"; 12])
        .deck(1, &["ST1-03"; 12])
        .memory(0)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn stack(r: &mut DebugRunner, ids: &[&str]) -> PermanentHandle {
    let h = r.place_on_field(1, ids[0], None);
    for id in &ids[1..] {
        let i = r.add_to_hand(1, id);
        let c = r.game.players[1].hand.remove(i);
        r.game.players[1].battle_area[h.index as usize]
            .card_sources
            .push(c);
    }
    r.game.tick_declarative_effects();
    h
}
fn expose(r: &mut DebugRunner, h: PermanentHandle) {
    let i = r.add_to_hand(0, "ST5-15");
    let c = r.game.players[0].hand[i].handle();
    assert_eq!(
        EffectContext::new(&mut r.game, c, None, 0).de_digivolve(h, Some(3), Some(1)),
        1
    );
    r.game.drain_effect_queue();
}
#[test]
fn completed_lower_stack_rule_trash_has_zero_deletion_and_physical_audit_in_clones() {
    let mut r = fixture();
    let h = stack(&mut r, &["ST1-03", "BT9-109", "BT9-062"]);
    let x = r.game.players[1].battle_area[h.index as usize].card_sources[1].handle();
    assert_eq!(
        r.game.observe_public_movement()[1]["BT9-109"].deletion_dispatches,
        0
    );
    let frozen = r.game.clone();
    for _ in 0..2 {
        r.game = frozen.clone();
        expose(&mut r, h);
        let observations = r.game.observe_public_movement();
        let w = &observations[1]["BT9-109"];
        assert_eq!(w.deletion_dispatches, 0);
        assert_eq!(w.rule_trashes.len(), 1);
        assert_eq!(w.rule_trashes[0].lower_sources[0].card_id, "ST1-03");
        assert_eq!(w.rule_trashes[0].lower_sources[0].owner, 1);
        let audit: Vec<_> = r
            .game
            .movement_audit(1)
            .into_iter()
            .filter(|a| a.kind == "rule_trash_completed")
            .collect();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].native_top, x.0 as u64);
        assert_eq!(audit[0].kind, "rule_trash_completed");
        assert!(r.game.players[1].battle_area.is_empty());
    }
}
#[test]
fn actual_deletion_is_dispatched_even_without_any_listener() {
    let mut r = fixture();
    let h = stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement();
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
    let obs = r.game.observe_public_movement();
    let w = &obs[1]["ST1-03"];
    assert_eq!(w.deletion_dispatches, 1);
    assert!(w.rule_trashes.is_empty());
    assert_eq!(
        r.game
            .movement_audit(1)
            .iter()
            .filter(|a| a.kind == "deletion_dispatch")
            .count(),
        1
    );
}
#[test]
fn evade_accept_cancels_dispatch_decline_dispatches_in_clones() {
    let mut r = fixture();
    let h = stack(&mut r, &["ST1-03"]);
    r.game
        .modifiers
        .grant_keyword(h, Keyword::Evade, Expiry::Permanent, 1);
    r.game.observe_public_movement();
    r.game.delete_permanent_with_effects(h);
    let frozen = r.game.clone();
    for accept in [true, false] {
        r.game = frozen.clone();
        let action = if accept { REPLACEMENT_ACCEPT } else { PASS };
        assert_eq!(build_action_mask(&r.game, 1)[action as usize], 1.0);
        r.game.resolve_selection(1, action).unwrap();
        r.game.drain_effect_queue();
        let obs = r.game.observe_public_movement();
        assert_eq!(
            obs[1]["ST1-03"].deletion_dispatches,
            if accept { 0 } else { 1 }
        );
        assert_eq!(
            r.game.players[1].battle_area.len(),
            if accept { 1 } else { 0 }
        );
    }
}
#[test]
fn unseen_card_never_yields_a_complete_zero_history() {
    let mut r = fixture();
    let h = stack(&mut r, &["ST1-03"]);
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
    assert!(!r.game.observe_public_movement()[1].contains_key("ST1-03"));
}
#[test]
fn duplicate_public_printing_invalidates_watch_permanently() {
    let mut r = fixture();
    stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement();
    stack(&mut r, &["ST1-03"]);
    assert!(!r.game.observe_public_movement()[1].contains_key("ST1-03"));
    r.game.players[1].battle_area.pop();
    assert!(!r.game.observe_public_movement()[1].contains_key("ST1-03"));
}
#[test]
fn rule_trash_reports_two_links_oldest_to_newest() {
    let mut r = fixture();
    let h = stack(&mut r, &["BT9-109", "BT9-062"]);
    for id in ["ST1-03", "BT9-062"] {
        let i = r.add_to_hand(1, id);
        let card = r.game.players[1].hand.remove(i);
        r.game.players[1].battle_area[h.index as usize]
            .linked_cards
            .push(card);
    }
    r.game.observe_public_movement();
    expose(&mut r, h);
    let obs = r.game.observe_public_movement();
    let links = &obs[1]["BT9-109"].rule_trashes[0].linked;
    assert_eq!(
        links.iter().map(|c| c.card_id.as_str()).collect::<Vec<_>>(),
        vec!["ST1-03", "BT9-062"]
    );
    assert!(links.iter().all(|c| c.owner == 1));
}
#[test]
fn observing_is_digest_and_mask_neutral() {
    let mut r = fixture();
    stack(&mut r, &["ST1-03"]);
    let before = r.game.verification_digest();
    let mask = build_action_mask(&r.game, 0);
    let observed = r.game.observe_public_movement();
    assert!(observed[1].contains_key("ST1-03"));
    assert_eq!(r.game.verification_digest(), before);
    assert_eq!(build_action_mask(&r.game, 0), mask);
    assert_eq!(r.game.clone().verification_digest(), before);
}

#[test]
fn replay_reset_discards_old_watch_and_uses_new_baseline_anchor() {
    let mut r = fixture();
    stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement_at(4);
    assert_eq!(r.game.movement_audit(1)[0].binding_start, 4);
    r.game.reset_for_replay();
    assert!(r.game.movement_audit(1).is_empty());
    stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement_at(9);
    assert_eq!(r.game.movement_audit(1)[0].binding_start, 9);
}
#[test]
fn batch_and_battle_dispatch_each_physical_top_once() {
    let mut r = fixture();
    let a = r.place_on_field(0, "BT9-062", None);
    let b = stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement_at(0);
    r.game.battle_digimon(a, b);
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.observe_public_movement_at(1)[1]["ST1-03"].deletion_dispatches,
        1
    );
    let mut r = fixture();
    let a = r.place_on_field(0, "BT9-062", None);
    let b = stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement_at(0);
    r.game.delete_permanents_batch(
        vec![a, b],
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    r.game.drain_effect_queue();
    let obs = r.game.observe_public_movement_at(1);
    assert_eq!(obs[0]["BT9-062"].deletion_dispatches, 1);
    assert_eq!(obs[1]["ST1-03"].deletion_dispatches, 1);
}
#[test]
fn zero_dp_rule_deletion_is_not_dp_less_rule_trash() {
    use digimon_engine::enums::ModifierType;
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = fixture();
    let h = stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement_at(0);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::ChangeDp, -20000, Expiry::Permanent, 0),
    );
    r.game.drain_effect_queue();
    let obs = r.game.observe_public_movement_at(1);
    assert_eq!(obs[1]["ST1-03"].deletion_dispatches, 1);
    assert!(obs[1]["ST1-03"].rule_trashes.is_empty());
}

#[test]
fn unobservable_deleted_stack_invalidates_only_its_bound_watch() {
    let mut r = fixture();
    let bad = stack(&mut r, &["BT9-062", "ST1-03"]);
    stack(&mut r, &["ST5-15"]);
    r.game.observe_public_movement_at(0);
    r.game.players[1].battle_area[bad.index as usize].card_sources[0].face_down = true;
    r.game.delete_permanent_with_effects(bad);
    r.game.drain_effect_queue();
    let obs = r.game.observe_public_movement_at(1);
    assert!(!obs[1].contains_key("ST1-03"));
    assert!(obs[1].contains_key("ST5-15"), "unrelated public history remains valid");
}

#[test]
fn unobservable_rule_trash_invalidates_only_its_bound_watch() {
    let mut r = fixture();
    let bad = stack(&mut r, &["ST1-03", "BT9-109", "BT9-062"]);
    r.place_on_field(0, "ST5-15", None);
    r.game.observe_public_movement_at(0);
    r.game.players[1].battle_area[bad.index as usize].card_sources[0].face_down = true;
    expose(&mut r, bad);
    let obs = r.game.observe_public_movement_at(1);
    assert!(!obs[1].contains_key("BT9-109"));
    assert!(obs[0].contains_key("ST5-15"), "other seat history remains valid");
}

#[test]
fn duplicate_batch_input_records_one_dispatch_per_physical_top() {
    let mut r = fixture();
    let h = stack(&mut r, &["ST1-03"]);
    r.game.observe_public_movement_at(0);
    r.game.delete_permanents_batch(vec![h,h], digimon_engine::replacement::ReplacementCause::OwnEffect);
    r.game.drain_effect_queue();
    assert_eq!(r.game.observe_public_movement_at(1)[1]["ST1-03"].deletion_dispatches, 1);
}

#[test]
fn parked_two_member_batch_observes_each_actual_dispatch_once_in_clones() {
    let mut r = fixture();
    let a = stack(&mut r, &["ST1-03"]);
    let b = stack(&mut r, &["BT9-062"]);
    r.game.modifiers.grant_keyword(a, Keyword::Evade, Expiry::Permanent, 1);
    r.game.observe_public_movement_at(0);
    r.game.delete_permanents_batch(vec![a,b], digimon_engine::replacement::ReplacementCause::OpponentEffect);
    let frozen = r.game.clone();
    for accept in [true,false] {
        r.game = frozen.clone();
        let action = if accept { REPLACEMENT_ACCEPT } else { PASS };
        assert_eq!(build_action_mask(&r.game, 1)[action as usize],1.0);
        r.game.resolve_selection(1,action).unwrap();
        r.game.drain_effect_queue();
        let obs = r.game.observe_public_movement_at(1);
        assert_eq!(obs[1]["ST1-03"].deletion_dispatches, if accept {0} else {1});
        assert_eq!(obs[1]["BT9-062"].deletion_dispatches,1);
        if accept { assert!(r.game.players[1].battle_area[0].is_suspended); }
    }
}
