//! BT18-088 official observation 9caf5deb...; image and base DCGO checked.
use digimon_engine::{
    action::{build_action_mask, space::PASS},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardKind, EffectTiming},
    permanent::PermanentHandle,
    TriggerSource,
};
fn runner() -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card("BT18-088")
        .expect("Takuya and Koji YAML");
    for (id, name, tr) in [
        ("A", "Agunimon", "Hybrid"),
        ("B", "Agunimon", "Hybrid"),
        ("C", "Lobomon", "Hybrid"),
        ("D", "Beetlemon", "Hybrid"),
        ("E", "Kazemon", "Hybrid"),
        ("F", "Kumamon", "Hybrid"),
        ("W", "Warrior", "Ten Warriors"),
        ("N", "Normal", "Other"),
        ("T", "Tamer", "Other"),
    ] {
        let mut c = make_test_card(id, name);
        c.traits = vec![tr.into()];
        c.dp = Some(10000);
        if id == "T" {
            c.card_kind = CardKind::Tamer;
            c.level = None;
            c.dp = None;
        }
        b = b.add_card(c);
    }
    for i in 0..20 {
        let mut c = make_test_card(&format!("H{i}"), &format!("Hybrid {i}"));
        c.traits = vec!["Hybrid".into()];
        b = b.add_card(c);
    }
    b.deck(0, &["N"; 20])
        .deck(1, &["N"; 20])
        .security(1, &["T"; 5])
        .memory(10)
        .start()
}
fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn pick(r: &mut DebugRunner, n: usize) {
    let p = r.pending_selection().expect("choice");
    let a = p
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .nth(n)
        .unwrap();
    assert!(build_action_mask(&r.game, p.selecting_player)[a as usize] > 0.5);
    r.execute_action(p.selecting_player, a).unwrap();
}
#[test]
fn bt18_088_memory_setter_and_aliases() {
    for (before, after) in [(0, 3), (2, 3), (4, 4)] {
        let mut r = runner();
        r.game.memory = before;
        let h = r.place_on_field(0, "BT18-088", Some(0));
        trigger(&mut r, h, EffectTiming::StartOfYourTurn);
        assert_eq!(r.memory(), after);
        let c = r.compiled_card("BT18-088").unwrap();
        assert_eq!(c.also_treated_as, ["Takuya Kanbara", "Koji Minamoto"]);
    }
}
#[test]
fn bt18_088_trash_different_names_and_other_tamer_scaling() {
    for (others, want) in [(0, 1), (1, 3), (2, 5)] {
        let mut r = runner();
        let h = r.place_on_field(0, "BT18-088", Some(0));
        for _ in 0..others {
            r.place_on_field(0, "T", Some(0));
        }
        r.place_on_field(0, "N", Some(0));
        r.place_on_field(1, "T", Some(0));
        for id in ["A", "B", "C", "D", "E", "F", "N", "W"] {
            r.inject_trash(0, id);
        }
        r.add_to_hand(0, "F");
        trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
        for i in 0..want {
            let p = r.pending_selection().unwrap();
            assert_eq!(
                p.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
                if i == 0 { 6 } else { 5 - i }
            );
            pick(&mut r, 0);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(
            r.game.players[0].battle_area[0].card_sources.len(),
            want + 1
        );
        assert_eq!(r.hand_size(0), 1);
    }
}
#[test]
fn bt18_088_zero_partial_and_empty_choices() {
    for count in [0, 1] {
        let mut r = runner();
        let h = r.place_on_field(0, "BT18-088", Some(0));
        r.place_on_field(0, "T", Some(0));
        for id in ["A", "C", "D"] {
            r.inject_trash(0, id);
        }
        trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
        if count == 1 {
            pick(&mut r, 0);
        }
        assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
        r.execute_action(0, PASS).unwrap();
        assert_eq!(
            r.game.players[0].battle_area[0].card_sources.len(),
            count + 1
        );
        assert_eq!(r.game.players[0].trash.len(), 3 - count);
    }
    let mut r = runner();
    let h = r.place_on_field(0, "BT18-088", Some(0));
    r.inject_trash(0, "W");
    trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt18_088_inherited_attack_filters_decline_and_opt() {
    for top in ["A", "W", "N"] {
        let mut r = runner();
        r.game.turn_count = 3;
        let h = r.place_stack(0, &["BT18-088", top]);
        trigger(&mut r, h, EffectTiming::EndOfYourTurn);
        if top == "N" {
            assert!(r.pending_selection().is_none());
            continue;
        }
        assert!(r.pending_selection().is_some());
        r.execute_action(0, PASS).unwrap();
        assert_eq!(r.game.players[1].security.len(), 5);
    }
}
#[test]
fn bt18_088_inherited_attacks_player_and_is_once_per_turn() {
    for top in ["A", "W"] {
        let mut r = runner();
        r.game.turn_count = 3;
        let h = r.place_stack(0, &["BT18-088", top]);
        trigger(&mut r, h, EffectTiming::EndOfYourTurn);
        pick(&mut r, 0); // accept activation
        pick(&mut r, 0); // choose the optional player attack
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[1].security.len(), 4);
        r.game.players[0].battle_area[0].is_suspended = false;
        trigger(&mut r, h, EffectTiming::EndOfYourTurn);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt18_088_security_plays_without_cost() {
    let attacker = make_test_card("ATK", "Attacker");
    let mut r = DebugRunner::builder()
        .dsl_card("BT18-088")
        .unwrap()
        .add_card(attacker)
        .security(0, &["BT18-088"])
        .memory(10)
        .start();
    r.game.turn_count = 3;
    r.game.turn_player_idx = 1;
    let h = r.place_on_field(1, "ATK", Some(0));
    let _ = r.attack_player(h, 0, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT18-088"
    );
    assert_eq!(r.memory(), 10);
}
#[test]
fn bt18_088_can_place_thirteen_sources_without_an_arbitrary_cap() {
    let mut r = runner();
    let h = r.place_on_field(0, "BT18-088", Some(0));
    for _ in 0..6 {
        r.place_on_field(0, "T", Some(0));
    }
    for i in 0..15 {
        r.inject_trash(0, &format!("H{i}"));
    }
    trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
    for _ in 0..13 {
        pick(&mut r, 0);
    }
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 14);
    assert_eq!(r.game.players[0].trash.len(), 2);
}
#[test]
fn bt18_088_suspended_carrier_can_activate_without_attacking_and_spends_opt() {
    let mut r = runner();
    r.game.turn_count = 3;
    let h = r.place_stack(0, &["BT18-088", "A"]);
    r.game.players[0].battle_area[0].is_suspended = true;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_some(),
        "even an unable carrier may activate"
    );
    pick(&mut r, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].security.len(), 5);
    r.game.players[0].battle_area[0].is_suspended = false;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_none(),
        "activation spent the use despite no attack"
    );
}
// CR 4.3 15-14-1-4 counts activation; 16-16-3 makes attacking optional processing.
#[test]
fn bt18_088_activation_decline_preserves_opt_but_attack_decline_spends_it() {
    let mut r = runner();
    r.game.turn_count = 3;
    let h = r.place_stack(0, &["BT18-088", "A"]);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    r.execute_action(0, PASS).unwrap();
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_some(),
        "declined activation must remain available"
    );
    pick(&mut r, 0); // activate the inherited effect
    assert!(
        r.pending_selection().is_some(),
        "attacking is a second optional processing choice"
    );
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.game.players[1].security.len(), 5);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_none(),
        "accepted activation spends OPT even when attack declined"
    );
}
#[test]
fn bt18_088_q3046_can_activate_during_an_attack_but_cannot_declare_nested_attack() {
    use digimon_engine::{
        enums::GamePhase,
        selection::{AttackState, AttackTarget, PendingAttack},
    };
    let mut r = runner();
    r.game.turn_count = 3;
    let h = r.place_stack(0, &["BT18-088", "A"]);
    let attacking = r.place_on_field(0, "N", Some(0));
    r.game.pending_attack = Some(PendingAttack {
        attacker: attacking,
        original_target: AttackTarget::Player(1),
        effective_target: AttackTarget::Player(1),
        effective_target_removed: false,
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        battle_defender_deleted: false,
        effect_battle_piercing_pending: false,
        security_check_started: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_some(),
        "Q3046 permits activation during an existing attack"
    );
    assert_eq!(
        r.pending_selection().unwrap().kind,
        digimon_engine::selection::SelectionKind::Replacement,
        "Q3046 offers activation, not attack target selection"
    );
    pick(&mut r, 0);
    assert!(
        r.pending_selection().is_none(),
        "no nested attack target choice is legal"
    );
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "inherited carrier never declared a nested attack"
    );
    assert_eq!(r.game.digimon_attacks_this_turn[0], 0);
    r.game.pending_attack = None;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_none(),
        "no-op activation still consumed OPT"
    );
}
