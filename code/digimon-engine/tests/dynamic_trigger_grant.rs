//! Duration-scoped grants: official EX1-068 Q3256 includes later entrants.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase};
use digimon_engine::selection::TriggerSource;

fn runner(dynamic: bool) -> DebugRunner {
    let yaml = include_str!("../cards/ex1/EX1-068.yaml")
        .replace("\r\n", "\n")
        .replace("          dynamic: true\n", "")
        .replace(
            "      - grant_triggered_effect:\n",
            &format!("      - grant_triggered_effect:\n          dynamic: {dynamic}\n"),
        );
    let mut dig = make_test_card("DIG", "Digimon");
    dig.card_kind = CardKind::Digimon;
    dig.colors = vec![CardColor::Blue];
    dig.dp = Some(3000);
    dig.level = Some(3);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .add_card(dig)
        .hand(0, &["EX1-068"])
        .deck(0, &["DIG"; 10])
        .deck(1, &["DIG"; 10])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "DIG", None);
    r
}
fn grant(r: &mut DebugRunner) {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    assert!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.game.pending_selection.is_none());
}
fn attack(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.set_memory(0);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn dynamic_grant_includes_later_entrants_and_repeated_attacks() {
    let mut r = runner(true);
    grant(&mut r);
    let later = r.place_on_field(1, "DIG", None);
    attack(&mut r, later);
    assert_eq!(
        r.memory(),
        2,
        "Q3256 includes opponent Digimon entering after resolution"
    );
    attack(&mut r, later);
    assert_eq!(
        r.memory(),
        2,
        "duration grant is reusable, not consumed on firing"
    );
}
#[test]
fn snapshot_grant_does_not_include_later_entrants() {
    let mut r = runner(false);
    grant(&mut r);
    let later = r.place_on_field(1, "DIG", None);
    attack(&mut r, later);
    assert_eq!(r.memory(), 0);
}
#[test]
fn dynamic_grant_expiry_and_clone_are_independent() {
    let mut r = runner(true);
    grant(&mut r);
    let mut fork = runner(true);
    fork.game = r.game.clone();
    r.game.modifiers.expire_end_of_turn(0);
    let later = r.place_on_field(1, "DIG", None);
    attack(&mut r, later);
    assert_eq!(r.memory(), 2);
    r.game.modifiers.expire_end_of_turn(1);
    attack(&mut r, later);
    assert_eq!(r.memory(), 0);
    let fork_later = fork.place_on_field(1, "DIG", None);
    attack(&mut fork, fork_later);
    assert_eq!(fork.memory(), 2);
}
#[test]
fn dynamic_grant_includes_breeding_entrant_after_option_has_left_play() {
    let mut r = runner(true);
    r.place_in_breeding(1, "DIG");
    grant(&mut r);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "EX1-068"));
    assert!(r.move_from_breeding(1));
    let h = digimon_engine::permanent::PermanentHandle {
        player: 1,
        index: 0,
    };
    attack(&mut r, h);
    assert_eq!(r.memory(), 2);
}
#[test]
fn dynamic_grant_rechecks_immunity_using_grantor_option_kind() {
    use digimon_engine::enums::{EffectSourceKind, Expiry, ModifierType};
    use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};
    let mut r = runner(true);
    let h = r.place_on_field(1, "DIG", None);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::EndOfYourTurn, 1)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(EffectSourceKind::Option),
                controller: EffectControllerFilter::OpponentOnly,
            }),
    );
    grant(&mut r);
    attack(&mut r, h);
    assert_eq!(r.memory(), 0, "immune during install and trigger");
    // Remove only the immunity; the Ice Wall duration ends on the other player's turn.
    r.game.modifiers.clear_permanent(h);
    attack(&mut r, h);
    assert_eq!(r.memory(), 2, "dynamic grant applies once immunity is gone");
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 1)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(EffectSourceKind::Digimon),
                controller: EffectControllerFilter::OpponentOnly,
            }),
    );
    attack(&mut r, h);
    assert_eq!(
        r.memory(),
        2,
        "Digimon-only immunity does not block an Option grant"
    );
}
#[test]
fn dynamic_grant_installed_during_opponent_turn_skips_current_turn_end() {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::Expiry;
    let mut r = runner(true);
    r.game.turn_player_idx = 1;
    let card = r.game.players[0].hand[0].handle();
    EffectContext::new(&mut r.game, card, None, 0).grant_dynamic_triggered_effect(
        EffectTiming::WhenAttacking,
        Expiry::EndOfOpponentsNextTurn,
        |ctx, h| h.player != ctx.player,
        |ctx| ctx.lose_memory(2),
    );
    let h = r.place_on_field(1, "DIG", None);
    assert!(r
        .game
        .modifiers
        .collect_expiring_granted_body_ids(1)
        .is_empty());
    r.game.modifiers.expire_end_of_turn(1);
    attack(&mut r, h);
    assert_eq!(
        r.memory(),
        -2,
        "current opponent turn end does not expire next-turn grant"
    );
    r.game.modifiers.expire_end_of_turn(0);
    attack(&mut r, h);
    assert_eq!(r.memory(), -2);
    let expired = r.game.modifiers.collect_expiring_granted_body_ids(1);
    assert_eq!(
        expired.len(),
        1,
        "native cleanup must identify the expiring body"
    );
    r.game.modifiers.expire_end_of_turn(1);
    attack(&mut r, h);
    assert_eq!(r.memory(), 0);
}
#[test]
fn dynamic_grant_preserves_option_attribution_when_grantor_is_absent() {
    use digimon_engine::enums::{EffectSourceKind, Expiry, ModifierType};
    use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};
    let mut r = runner(true);
    grant(&mut r);
    r.game.players[0].trash.clear();
    let h = r.place_on_field(1, "DIG", None);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 1)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(EffectSourceKind::Option),
                controller: EffectControllerFilter::OpponentOnly,
            }),
    );
    attack(&mut r, h);
    assert_eq!(
        r.memory(),
        0,
        "stored Option grantor kind survives source removal"
    );
    r.game.modifiers.clear_permanent(h);
    attack(&mut r, h);
    assert_eq!(
        r.memory(),
        2,
        "duration survives the grantor card's disappearance"
    );
}
#[test]
fn dynamic_grant_rejects_non_duration_expiry() {
    let yaml = include_str!("../cards/ex1/EX1-068.yaml")
        .replace("\r\n", "\n")
        .replace("          dynamic: true\n", "")
        .replace(
            "      - grant_triggered_effect:\n",
            "      - grant_triggered_effect:\n          dynamic: true\n",
        )
        .replace("expiry: end_of_opponents_next_turn", "expiry: permanent");
    assert!(DebugRunner::builder().from_dsl_yaml(&yaml).is_err());
}

#[test]
fn dynamic_grant_keeps_distinct_carriers_in_simultaneous_unsuspend_batch() {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::Expiry;
    use digimon_engine::selection::SelectionKind;
    let mut r = runner(true);
    let first = r.place_on_field(1, "DIG", None);
    let second = r.place_on_field(1, "DIG", None);
    r.game.players[1].battle_area[first.index as usize].is_suspended = true;
    r.game.players[1].battle_area[second.index as usize].is_suspended = true;
    let card = r.game.players[0].hand[0].handle();
    EffectContext::new(&mut r.game, card, None, 0).grant_dynamic_triggered_effect(
        EffectTiming::OnUnsuspend,
        Expiry::EndOfOpponentsNextTurn,
        |ctx, h| h.player != ctx.player,
        |ctx| ctx.gain_memory(1),
    );
    r.game.pass_turn();
    assert!(!r.game.players[1].battle_area[0].is_suspended);
    assert!(!r.game.players[1].battle_area[1].is_suspended);
    let pending = r.game.pending_selection.as_ref()
        .expect("each carrier owns a distinct granted effect, requiring an ordering choice");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    assert_eq!(pending.selecting_player, 1);
    assert_eq!(pending.valid_action_ids.len(), 2);
    let action = pending.valid_action_ids[0];
    assert_eq!(r.game.turn_player(), 1);
    assert_eq!(r.memory(), 3, "memory uses the current turn player's perspective");
    r.game.resolve_selection(1, action).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 5, "each of two carriers gains one memory exactly once");
}

#[test]
fn dynamic_grant_digest_distinguishes_active_and_expired_duration() {
    let mut r = runner(true);
    grant(&mut r);
    let active = r.game.verification_digest();
    r.game.modifiers.expire_end_of_turn(1);
    assert_ne!(active, r.game.verification_digest(),
        "duration removal changes future attack behavior and must change its verification digest");
}

#[test]
fn dynamic_grant_digest_distinguishes_remaining_next_turn_skip() {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::Expiry;
    let mut r = runner(true);
    r.game.turn_player_idx = 1;
    let card = r.game.players[0].hand[0].handle();
    EffectContext::new(&mut r.game, card, None, 0).grant_dynamic_triggered_effect(
        EffectTiming::WhenAttacking,
        Expiry::EndOfOpponentsNextTurn,
        |ctx, h| h.player != ctx.player,
        |ctx| ctx.lose_memory(2),
    );
    let before_skip = r.game.verification_digest();
    r.game.modifiers.expire_end_of_turn(1);
    assert!(r.game.modifiers.collect_expiring_granted_body_ids(1).len() == 1);
    assert_ne!(before_skip, r.game.verification_digest(),
        "otherwise identical active grants with different remaining durations must hash differently");
}
