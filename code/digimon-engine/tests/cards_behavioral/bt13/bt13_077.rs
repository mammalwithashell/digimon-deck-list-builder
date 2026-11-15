//! Official BT13-077 Q2316–Q2320; CR 4.3 15-15-5. Player-directed attack.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, SECURITY_TARGET},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectSourceKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
const ID: &str = "BT13-077";
fn setup(owner: u8) -> DebugRunner {
    let mut pad = make_test_card("PAD", "Padding");
    pad.level = Some(3);
    pad.dp = Some(1000);
    pad.colors = vec![CardColor::White];
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("Craniamon implemented")
        .add_card(pad)
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .security(0, &["PAD"; 5])
        .security(1, &["PAD"; 5])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(1 - owner);
    r.set_phase(GamePhase::Main);
    r.place_on_field(owner, ID, Some(0));
    // Suspend Craniamon so its Blocker does not interrupt the forced attack witnesses.
    r.game.players[owner as usize].battle_area[0].is_suspended = true;
    r
}
fn choose(r: &mut DebugRunner, player: u8, action: u16) {
    assert_eq!(r.pending_selection().unwrap().selecting_player, player);
    assert!(build_action_mask(&r.game, player)[action as usize] > 0.5);
    r.execute_action(player, action).unwrap();
}
fn first(r: &mut DebugRunner, player: u8) {
    let a = *r
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    choose(r, player, a);
}
#[test]
fn bt13_077_q2320_immune_attacker_is_mandatory_for_opponent_and_clones() {
    for owner in [0, 1] {
        let mut r = setup(owner);
        let h = r.place_on_field(1 - owner, "PAD", Some(0));
        r.game.modifiers.add(
            h,
            ModifierEntry::cannot_be_affected_by_opponents_source_kind(
                EffectSourceKind::Digimon,
                Expiry::Permanent,
                1 - owner,
            ),
        );
        r.end_turn();
        assert!(r.pending_selection().unwrap().is_optional);
        let mut decline = DebugRunner::wrap(r.game.clone());
        choose(&mut decline, owner, PASS);
        assert_eq!(decline.game.players[owner as usize].security.len(), 5);
        first(&mut r, owner);
        assert!(
            !r.pending_selection()
                .expect("Q2320: immune attacker must attack")
                .is_optional
        );
        let mut clone = DebugRunner::wrap(r.game.clone());
        for game in [&mut r, &mut clone] {
            assert_eq!(build_action_mask(&game.game, 1 - owner)[PASS as usize], 0.0);
            choose(game, 1 - owner, encode_attack(0, SECURITY_TARGET));
            assert_eq!(game.game.players[owner as usize].security.len(), 4);
        }
    }
}
#[test]
fn bt13_077_q2318_unable_attackers_are_selectable_but_do_not_attack() {
    for case in [
        "suspended",
        "sick",
        "cannot_attack",
        "cannot_suspend",
        "cannot_attack_player",
    ] {
        let mut r = setup(0);
        let h = r.place_on_field(1, "PAD", if case == "sick" { None } else { Some(0) });
        if case == "suspended" {
            r.game.players[1].battle_area[0].is_suspended = true;
        }
        if case == "cannot_attack_player" {
            r.game.players[0].battle_area[0].is_suspended = false;
        }
        let m = match case {
            "cannot_attack" => Some(ModifierType::CannotAttack),
            "cannot_suspend" => Some(ModifierType::CannotSuspend),
            "cannot_attack_player" => Some(ModifierType::CannotAttackPlayer),
            _ => None,
        };
        if let Some(m) = m {
            r.game
                .modifiers
                .add(h, ModifierEntry::simple(m, 0, Expiry::Permanent, 1));
        }
        r.end_turn();
        first(&mut r, 0);
        assert!(r.pending_selection().is_none(), "{case}");
        assert_eq!(r.game.players[0].security.len(), 5, "{case}");
    }
}
#[test]
fn bt13_077_no_target_and_own_turn_do_not_force_attack() {
    let mut r = setup(0);
    r.end_turn();
    assert!(r.pending_selection().is_none());
    let mut r = setup(0);
    r.place_on_field(1, "PAD", Some(0));
    r.set_first_player(0);
    r.end_turn();
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt13_077_on_play_immunity_is_opponent_digimon_only_and_expires() {
    let mut r = setup(0);
    r.set_first_player(0);
    r.fire_on_play(0, 0);
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon));
    for (player, kind) in [
        (0, EffectSourceKind::Digimon),
        (1, EffectSourceKind::Tamer),
        (1, EffectSourceKind::Option),
    ] {
        assert!(!r.game.permanent_is_unaffected_by_effect(h, player, kind));
    }
    r.end_turn();
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon));
    r.end_turn();
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon));
}

#[test]
fn bt13_077_when_digivolving_grants_immunity_and_blocker_is_live() {
    use digimon_engine::{
        enums::{EffectTiming, Keyword},
        permanent::PermanentHandle,
        selection::TriggerSource,
    };
    let mut r = setup(0);
    let h = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon));
    let other = r.place_on_field(0, "PAD", Some(0));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(other, 1, EffectSourceKind::Digimon));
}

#[test]
fn bt13_077_opponent_chooses_between_player_and_legal_digimon_targets() {
    let mut r = setup(0);
    r.place_on_field(1, "PAD", Some(0));
    r.end_turn();
    first(&mut r, 0);
    let mask = build_action_mask(&r.game, 1);
    assert!(mask[encode_attack(0, SECURITY_TARGET) as usize] > 0.5);
    assert!(mask[encode_attack(0, 0) as usize] > 0.5); // suspended Craniamon
    choose(&mut r, 1, encode_attack(0, 0));
    assert_eq!(r.game.players[0].security.len(), 5);
    assert!(r.game.players[1].battle_area.is_empty()); // elected losing battle
}

#[test]
fn bt13_077_q2319_existing_attack_prevents_a_second_declaration() {
    use digimon_engine::{
        enums::EffectTiming, permanent::PermanentHandle, selection::TriggerSource,
    };
    let observer = r#"
card: TEST-CRANIAMON-ATTACK
name: Attack observer
kind: digimon
level: 3
color: [white]
cost: 3
dp: 6000
effects:
  - when: when_attacking
    optional: true
    process:
      - gain_memory: 1
"#;
    let pad = make_test_card("PAD", "Padding");
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .unwrap()
        .from_dsl_yaml(observer)
        .unwrap()
        .add_card(pad)
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .security(0, &["PAD"; 5])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(1);
    r.set_phase(GamePhase::Main);
    let source = r.place_on_field(0, ID, Some(0));
    r.game.players[0].battle_area[0].is_suspended = true;
    let attacker = r.place_on_field(1, "TEST-CRANIAMON-ATTACK", Some(0));
    r.place_on_field(1, "PAD", Some(0));
    r.attack_player(attacker, 0, false);
    assert!(r.game.pending_attack.is_some());
    // Qualify the trigger while an earlier attack is paused at When Attacking.
    r.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(source),
    );
    first(&mut r, 1);
    assert_eq!(r.game.pending_attack.as_ref().unwrap().attacker, attacker);
    let choice = encode_attack(0, 1); // choose the other, unsuspended Digimon
    choose(&mut r, 0, choice);
    assert!(r.pending_selection().is_none());
    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.game.players[0].security.len(), 4);
    assert!(!r.game.players[1].battle_area[1].is_suspended);
    assert_eq!(
        attacker,
        PermanentHandle {
            player: 1,
            index: 0
        }
    );
}

#[test]
fn bt13_077_q2315_immune_attacker_can_still_be_blocked() {
    let mut r = setup(0);
    r.set_first_player(0);
    r.game.players[0].battle_area[0].is_suspended = false;
    r.fire_on_play(0, 0);
    let attacker = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    r.place_on_field(1, ID, Some(0));
    r.attack_player(attacker, 1, false);
    first(&mut r, 1); // opponent's ordinary Blocker decision
    assert_eq!(r.game.players[1].security.len(), 5);
    assert!(r.game.players[0].battle_area.is_empty());
    assert!(r.game.players[1].battle_area.is_empty()); // equal DP battle
}
