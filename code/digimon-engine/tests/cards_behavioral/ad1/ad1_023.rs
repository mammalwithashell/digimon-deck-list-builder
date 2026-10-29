//! AD1-023 official observation add28324...; Q6113 and Q6114.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{PASS, TRASH_EFFECT_START},
    },
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, EffectTiming},
    replacement::ReplacementCause,
    TriggerSource,
};
const ID: &str = "AD1-023";
fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("AD1-023 YAML");
    for (id, colors, tr) in [
        ("R", vec![CardColor::Red], "Hybrid"),
        ("B", vec![CardColor::Blue], "Hybrid"),
        ("RB", vec![CardColor::Red, CardColor::Blue], "Hybrid"),
        ("W", vec![CardColor::Yellow], "Ten Warriors"),
        ("N", vec![CardColor::Red], "Other"),
    ] {
        let mut c = make_test_card(id, id);
        c.colors = colors;
        c.traits = vec![tr.into()];
        c.dp = Some(9000);
        b = b.add_card(c);
    }
    b.hand(0, hand)
        .deck(0, &["N"; 20])
        .security(0, &["R", "B"])
        .memory(5)
        .start()
}
fn trigger(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn pick(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection().unwrap().selecting_player;
    assert!(build_action_mask(&r.game, p)[a as usize] > 0.5);
    r.execute_action(p, a).unwrap();
}
fn accept(r: &mut DebugRunner) {
    let a = *r
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    pick(r, a);
}
#[test]
fn ad1_023_mixed_colors_draw_once_both_timings() {
    for t in [EffectTiming::OnPlay, EffectTiming::StartOfYourMainPhase] {
        let mut r = setup(&["R", "N"]);
        r.inject_trash(0, "R");
        r.inject_trash(0, "B");
        let h = r.place_on_field(0, ID, None);
        trigger(&mut r, h, t);
        assert_eq!(
            r.pending_selection()
                .unwrap()
                .valid_action_ids
                .iter()
                .filter(|&&a| a != PASS)
                .count(),
            3
        );
        pick(&mut r, 0);
        assert!(!r
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .contains(&TRASH_EFFECT_START));
        pick(&mut r, TRASH_EFFECT_START + 1);
        assert_eq!(
            r.game.players[0].battle_area[0].digivolution_cards().len(),
            3
        );
        assert_eq!(r.hand_size(0), 2);
        assert_eq!(r.memory(), 5);
    }
}
#[test]
fn ad1_023_dual_color_matching_allows_two_same_printings() {
    let mut r = setup(&["RB", "RB"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    pick(&mut r, 1);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(
        r.game.players[0].battle_area[0].digivolution_cards().len(),
        3
    );
}
#[test]
fn ad1_023_threshold_independent_of_placement() {
    for n in [3, 4, 8] {
        let mut r = setup(&["B"]);
        let mut stack = vec!["R"; n];
        stack.push(ID);
        let h = r.place_stack(0, &stack);
        trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
        pick(&mut r, PASS);
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(r.memory(), if n >= 4 { 7 } else { 5 });
    }
}
#[test]
fn ad1_023_partial_one_draw_then_threshold() {
    let mut r = setup(&["B"]);
    let h = r.place_stack(0, &["R", "R", "R", ID]);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), 7);
}
#[test]
fn ad1_023_no_candidates_no_draw_nonhybrid_sources_dont_count() {
    let mut r = setup(&["N"]);
    let h = r.place_stack(0, &["R", "R", "R", "W", "N", ID]);
    trigger(&mut r, h, EffectTiming::OnPlay);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), 5);
}
#[test]
fn ad1_023_protection_all_causes_both_traits_opt() {
    for cause in [
        ReplacementCause::OpponentEffect,
        ReplacementCause::OwnEffect,
        ReplacementCause::Battle,
    ] {
        for host in ["R", "W"] {
            let mut r = setup(&[]);
            let h = r.place_stack(0, &[ID, host]);
            r.game.delete_permanent_with_cause(h, cause);
            accept(&mut r);
            assert_eq!(r.game.players[0].battle_area.len(), 1);
            assert_eq!(r.hand_size(0), 1);
            assert_eq!(r.game.players[0].security.len(), 1);
            r.game.delete_permanent_with_cause(h, cause);
            assert!(r.pending_selection().is_none());
            assert!(r.game.players[0].battle_area.is_empty());
        }
    }
}
#[test]
fn ad1_023_protection_decline_wrong_trait_empty_security() {
    for mode in 0..3 {
        let mut r = setup(&[]);
        if mode == 2 {
            r.game.players[0].security.clear();
        }
        let h = r.place_stack(0, &[ID, if mode == 1 { "N" } else { "R" }]);
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OwnEffect);
        if mode == 0 {
            pick(&mut r, PASS);
        } else {
            assert!(r.pending_selection().is_none());
        }
        assert!(r.game.players[0].battle_area.is_empty());
        assert_eq!(r.hand_size(0), 0);
    }
}

#[test]
fn ad1_023_rule_names_in_hand_and_sources() {
    let mut r = setup(&[ID]);
    for name in ["J.P. Shibayama", "Koji Minamoto", "Koichi Kimura"] {
        assert!(r.game.players[0].hand[0].contains_card_name(name, &r.game.card_data));
    }
    r.place_stack(0, &[ID, "R"]);
    for name in ["J.P. Shibayama", "Koji Minamoto", "Koichi Kimura"] {
        assert!(r.game.players[0].battle_area[0].card_sources[0]
            .contains_card_name(name, &r.game.card_data));
    }
}
#[test]
fn ad1_023_security_dispatch_plays_self_free() {
    let mut r = setup(&[]);
    r.game.players[0].security.clear();
    r.add_to_hand(0, ID);
    let c = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].security.push(c);
    let attacker = r.place_on_field(1, "N", Some(0));
    r.game.turn_player_idx = 1;
    r.game.turn_count = 3;
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let memory = r.memory();
    let _ = r.attack_player(attacker, 0, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        ID
    );
    assert_eq!(r.memory(), memory);
}
