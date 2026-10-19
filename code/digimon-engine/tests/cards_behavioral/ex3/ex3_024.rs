//! Authority: official observation ece42c6d13eec9504c73ba88ccba471f93b28a897583789dcfd02ee7d7baa42c,
//! errata card image and Q3394-3400; real phase entry, not a synthetic timing trigger.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, encode_digivolve, PASS, SECURITY_TARGET},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, EffectSourceKind, Expiry, GamePhase, ModifierType,
};
use digimon_engine::modifiers::ModifierEntry;
const ID: &str = "EX3-024";
fn setup() -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX3-024 must be implemented")
        .from_dsl_yaml(
            r#"
card: TEST-EX3-024-ATTACK
name: Attack Priority Observer
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 6000
effects:
  - when: when_attacking
    optional: true
    process:
      - gain_memory: 1
"#,
        )
        .expect("attack priority observer");
    for (id, name, level, color) in [
        ("W", "Wingdramon", 5, CardColor::Blue),
        ("G", "Groundramon", 5, CardColor::Green),
        ("B", "Other", 5, CardColor::Blue),
        ("R", "Other", 5, CardColor::Red),
        ("E", "Examon", 7, CardColor::Green),
        ("D", "TestDramon", 3, CardColor::Blue),
        ("PAD", "Padding", 3, CardColor::White),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.colors = vec![color];
        c.dp = Some(if id == "PAD" { 1000 } else { 6000 });
        b = b.add_card(c);
    }
    let mut t = make_test_card("T", "TamerDramon");
    t.card_kind = CardKind::Tamer;
    let mut r = b
        .add_card(t)
        .hand(0, &[ID])
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .security(0, &["PAD"; 5])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(1);
    r.set_phase(GamePhase::Breeding);
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
fn ex3_024_face_and_inherited_phase_entry_force_opponents_choice() {
    for inherited in [false, true] {
        let mut r = setup();
        if inherited {
            r.place_stack(0, &[ID, "E"]);
        } else {
            r.place_on_field(0, ID, Some(0));
        }
        r.place_on_field(1, "D", Some(0));
        assert!(r.pending_selection().is_none());
        r.game.enter_main_phase();
        assert!(r.pending_selection().unwrap().is_optional);
        first(&mut r, 0); // choose own suspension cost
        assert_eq!(r.pending_selection().unwrap().selecting_player, 1);
        assert!(!r.pending_selection().unwrap().is_optional);
        assert!(
            r.game.players[0].battle_area[0].is_suspended,
            "Q3401: suspension trigger waits for the forced attack declaration"
        );
        first(&mut r, 1); // opponent chooses attacking Digimon
        assert_eq!(r.pending_selection().unwrap().selecting_player, 1);
        assert!(!r.pending_selection().unwrap().is_optional);
        choose(&mut r, 1, encode_attack(0, SECURITY_TARGET)); // mandatory attack target
        assert!(r.game.players[1].battle_area[0].is_suspended);
        assert_eq!(r.game.players[0].security.len(), 4);
    }
}
#[test]
fn ex3_024_optional_suspend_choice_clones_and_declines_independently() {
    let mut r = setup();
    r.place_stack(0, &[ID, "E"]);
    r.game.enter_main_phase();
    let mut c = DebugRunner::wrap(r.game.clone());
    choose(&mut c, 0, PASS);
    first(&mut r, 0);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert!(!c.game.players[0].battle_area[0].is_suspended);
    assert!(r.pending_selection().is_none());
    assert!(c.pending_selection().is_none());
}
#[test]
fn ex3_024_own_main_and_breeding_are_not_observers() {
    for breeding in [false, true] {
        let mut r = setup();
        if breeding {
            r.place_in_breeding(0, ID);
        } else {
            r.place_on_field(0, ID, Some(0));
            r.set_first_player(0);
        }
        r.game.enter_main_phase();
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn ex3_024_cost_filter_excludes_tamer_wrong_name_suspended_and_prohibited() {
    let mut r = setup();
    r.place_stack(0, &[ID, "PAD"]);
    r.place_on_field(0, "T", Some(0));
    r.place_on_field(0, "PAD", Some(0));
    let suspended = r.place_on_field(0, "W", Some(0));
    r.game.players[0].battle_area[suspended.index as usize].is_suspended = true;
    let prohibited = r.place_on_field(0, "G", Some(0));
    r.game.modifiers.add(
        prohibited,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 1),
    );
    r.place_on_field(0, "D", Some(0));
    r.game.enter_main_phase();
    assert_eq!(
        r.pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        1
    );
    first(&mut r, 0);
    assert!(r.game.players[0].battle_area[5].is_suspended);
}
#[test]
fn ex3_024_no_payable_cost_finishes_without_opponent_choice() {
    let mut r = setup();
    r.place_stack(0, &[ID, "PAD"]);
    r.place_on_field(1, "D", Some(0));
    r.game.enter_main_phase();
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[1].battle_area[0].is_suspended);
}
#[test]
fn ex3_024_opponent_may_choose_a_digimon_that_cannot_attack() {
    let mut r = setup();
    r.place_stack(0, &[ID, "E"]);
    let h = r.place_on_field(1, "D", Some(0));
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotAttack, 0, Expiry::Permanent, 0),
    );
    r.game.enter_main_phase();
    first(&mut r, 0);
    first(&mut r, 1);
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[1].battle_area[0].is_suspended);
    assert_eq!(r.game.players[0].security.len(), 5);
}
#[test]
fn ex3_024_self_unsuspend_is_mandatory_once_per_turn_and_face_only() {
    let mut r = setup();
    r.set_phase(GamePhase::Main);
    let h = r.place_on_field(0, ID, Some(0));
    r.game.suspend(h);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    r.game.suspend(h);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    let other = r.place_stack(0, &[ID, "E"]);
    r.game.suspend(other);
    assert!(r.game.players[0].battle_area[1].is_suspended);
}
#[test]
fn ex3_024_normal_and_named_evolution_routes() {
    for (base, cost) in [("W", Some(3)), ("G", Some(3)), ("B", Some(4)), ("R", None)] {
        let mut r = setup();
        r.set_first_player(0);
        r.set_phase(GamePhase::Main);
        r.place_on_field(0, base, Some(0));
        let a = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize] > 0.5,
            cost.is_some()
        );
        if let Some(cost) = cost {
            let before = r.memory();
            r.game.decode_action(a, 0);
            if base == "W" || base == "G" {
                // Named cost-3 and standard cost-4 are both player choices.
                assert_eq!(r.pending_selection().unwrap().valid_action_ids.len(), 2);
                let mut standard = DebugRunner::wrap(r.game.clone());
                let routes = r.pending_selection().unwrap().valid_action_ids.clone();
                choose(&mut r, 0, routes[0]);
                choose(&mut standard, 0, routes[1]);
                assert_eq!(standard.memory(), before - 4);
            }
            assert_eq!(r.memory(), before - cost);
        }
    }
}

#[test]
fn ex3_024_opponent_attacker_selection_clones_with_distinct_choices() {
    let mut r = setup();
    r.place_stack(0, &[ID, "E"]);
    let unable = r.place_on_field(1, "D", Some(0));
    r.place_on_field(1, "W", Some(0));
    r.game.modifiers.add(
        unable,
        ModifierEntry::simple(ModifierType::CannotAttack, 0, Expiry::Permanent, 0),
    );
    r.game.enter_main_phase();
    first(&mut r, 0);
    let actions = r.pending_selection().unwrap().valid_action_ids.clone();
    assert_eq!(actions.len(), 2);
    let mut c = DebugRunner::wrap(r.game.clone());
    choose(&mut r, 1, actions[0]);
    choose(&mut c, 1, actions[1]);
    assert!(r.pending_selection().is_none());
    choose(&mut c, 1, encode_attack(1, SECURITY_TARGET));
    assert_eq!(r.game.players[0].security.len(), 5);
    assert_eq!(c.game.players[0].security.len(), 4);
}
#[test]
fn ex3_024_once_per_turn_resets_and_cannot_unsuspend_is_respected() {
    let mut r = setup();
    r.set_phase(GamePhase::Main);
    let h = r.place_on_field(0, ID, Some(0));
    r.game.suspend(h);
    r.game.suspend(h);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    r.end_turn();
    r.auto_resolve().unwrap();
    r.game.unsuspend(h);
    r.game.suspend(h);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    let h2 = r.place_on_field(0, ID, Some(0));
    r.game.modifiers.add(
        h2,
        ModifierEntry::simple(ModifierType::CannotUnsuspend, 0, Expiry::Permanent, 1),
    );
    r.game.suspend(h2);
    assert!(r.game.players[0].battle_area[h2.index as usize].is_suspended);
}

// Official general_rule.pdf 15-15-6-1: an unaffected card can be selected,
// but the effect's processing does not affect it. DCGO EX3_024 also checks
// CanNotBeAffected after opponent selection and before SelectAttackEffect.
fn immune_attacker_case(inherited: bool) {
    let mut r = setup();
    if inherited {
        r.place_stack(0, &[ID, "E"]);
    } else {
        r.place_on_field(0, ID, Some(0));
    }
    let immune = r.place_on_field(1, "D", Some(0));
    r.place_on_field(1, "W", Some(0));
    r.game.modifiers.add(
        immune,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::Permanent,
            1,
        ),
    );
    r.game.enter_main_phase();
    first(&mut r, 0);
    assert!(r
        .game
        .permanent_is_unaffected_by_effect(immune, 0, EffectSourceKind::Digimon));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(immune, 1, EffectSourceKind::Digimon));
    let choices = r.pending_selection().expect("opponent chooses a Digimon");
    assert_eq!(choices.selecting_player, 1);
    assert!(!choices.is_optional);
    assert_eq!(
        choices.valid_action_ids.len(),
        2,
        "immune Digimon remains selectable"
    );
    let actions = choices.valid_action_ids.clone();
    let mut same = DebugRunner::wrap(r.game.clone());
    let mut nonimmune = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut same] {
        choose(game, 1, actions[0]);
        assert!(
            game.pending_selection().is_none(),
            "opposing effect must not force an immune Digimon to attack"
        );
        assert!(game.game.pending_attack.is_none());
        assert!(!game.game.players[1].battle_area[0].is_suspended);
        assert_eq!(game.game.players[0].security.len(), 5);
    }
    choose(&mut nonimmune, 1, actions[1]);
    choose(&mut nonimmune, 1, encode_attack(1, SECURITY_TARGET));
    assert_eq!(nonimmune.game.players[0].security.len(), 4);
    assert_eq!(r.game.players[0].security.len(), 5);
}
#[test]
fn ex3_024_face_immune_attacker_remains_selectable_but_cannot_be_forced_after_clone() {
    immune_attacker_case(false);
}
#[test]
fn ex3_024_inherited_immune_attacker_remains_selectable_but_cannot_be_forced_after_clone() {
    immune_attacker_case(true);
}

#[test]
fn ex3_024_two_observers_keep_first_attack_and_turn_player_trigger_priority() {
    let mut r = setup();
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, ID, Some(0));
    let attacker = r.place_on_field(1, "TEST-EX3-024-ATTACK", Some(0));
    r.game.enter_main_phase();
    first(&mut r, 0); // choose one of the simultaneous main-phase effects
    first(&mut r, 0); // suspend the first Slayerdramon
    first(&mut r, 1); // choose the opponent's attacker
    choose(&mut r, 1, encode_attack(0, SECURITY_TARGET));
    // Q3399: the turn player's When Attacking precedes the non-turn
    // player's pending On Suspend unsuspend, even though suspension was earlier.
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.pending_selection().unwrap().selecting_player, 1);
    first(&mut r, 1); // accept the attack observer
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    let pending = r
        .game
        .pending_attack
        .as_ref()
        .expect("first attack still in progress");
    assert_eq!(pending.attacker, attacker);
    assert_eq!(r.pending_selection().unwrap().selecting_player, 0);
    assert!(r.pending_selection().unwrap().is_optional);
    first(&mut r, 0); // second Slayer's effect can still pay its suspension cost
    assert!(
        r.pending_selection().is_none(),
        "Q3400: no second opponent attack choice"
    );
    assert!(r.game.pending_attack.is_none());
    assert_eq!(
        r.game.players[0].security.len(),
        4,
        "only the first attack performs a check"
    );
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "second cost was paid despite no second attack"
    );
}
