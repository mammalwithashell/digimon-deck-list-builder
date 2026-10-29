//! Official AD1-015 observation 63ebc7ef...; image + DCGO AD1_015 checked.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::{permanent::PermanentHandle, TriggerSource};
fn runner(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card("AD1-015")
        .expect("Beowolfmon YAML");
    for (id, color, inherited) in [
        ("TEST-Y", CardColor::Yellow, true),
        ("TEST-B", CardColor::Black, true),
        ("TEST-P", CardColor::Purple, true),
        ("TEST-R", CardColor::Red, true),
        ("TEST-N", CardColor::Yellow, false),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Tamer;
        c.dp = None;
        c.level = None;
        c.colors = vec![color];
        c.play_cost = 9;
        if inherited {
            c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
        }
        b = b.add_card(c);
    }
    for (id, trait_name, level, color, name) in [
        ("TEST-H", "Hybrid", 4, CardColor::Yellow, "Hybrid"),
        ("TEST-W", "Ten Warriors", 4, CardColor::Blue, "Warrior"),
        ("TEST-O", "Other", 6, CardColor::Yellow, "Other"),
        ("TEST-K", "Other", 0, CardColor::Blue, "Koji Minamoto"),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.dp = Some(15000);
        c.colors = vec![color];
        c.traits = vec![trait_name.into()];
        if id == "TEST-K" {
            c.card_kind = CardKind::Tamer;
            c.level = None;
            c.dp = None;
        }
        b = b.add_card(c);
    }
    b.hand(0, hand).deck(0, &["TEST-O"; 20]).memory(10).start()
}
fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn pick(r: &mut DebugRunner, n: usize) {
    let a = r
        .pending_selection()
        .expect("player decision")
        .valid_action_ids[n];
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.5);
    r.execute_action(0, a).unwrap();
}
#[test]
fn ad1_015_dp_reduction_native_and_inherited_repeat_without_opt() {
    for inherited in [false, true] {
        for timing in [EffectTiming::WhenAttacking, EffectTiming::WhenDigivolving] {
            if inherited && timing == EffectTiming::WhenDigivolving {
                continue;
            }
            let mut r = runner(&[]);
            let h = if inherited {
                r.place_stack(0, &["AD1-015", "TEST-O"])
            } else {
                r.place_on_field(0, "AD1-015", Some(0))
            };
            let victim = r.place_on_field(1, "TEST-O", Some(0));
            r.place_on_field(1, "TEST-Y", Some(0));
            for expected in [11000, 7000] {
                trigger(&mut r, h, timing);
                assert_eq!(r.pending_selection().unwrap().valid_action_ids.len(), 1);
                pick(&mut r, 0);
                assert_eq!(r.dp_of(victim), Some(expected));
            }
        }
    }
}
#[test]
fn ad1_015_first_decline_still_places_and_draws_two() {
    for material in ["TEST-H", "TEST-W"] {
        let mut r = runner(&["TEST-Y", material]);
        let h = r.place_on_field(0, "AD1-015", Some(0));
        trigger(&mut r, h, EffectTiming::EndOfAttack);
        r.execute_action(0, PASS).unwrap();
        pick(&mut r, 0);
        pick(&mut r, 0);
        assert_eq!(r.hand_size(0), 3);
        assert_eq!(
            r.game.players[0].battle_area[0].digivolution_cards().len(),
            2
        );
    }
}
#[test]
fn ad1_015_declines_never_draw_or_move_cards() {
    let mut r = runner(&["TEST-Y", "TEST-H"]);
    let h = r.place_on_field(0, "AD1-015", Some(0));
    trigger(&mut r, h, EffectTiming::EndOfAttack);
    r.execute_action(0, PASS).unwrap();
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.hand_size(0), 2);
    let mut r = runner(&["TEST-H"]);
    let h = r.place_on_field(0, "AD1-015", Some(0));
    trigger(&mut r, h, EffectTiming::EndOfAttack);
    pick(&mut r, 0);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn ad1_015_free_play_filters_colors_and_inherited_text() {
    for tamer in ["TEST-Y", "TEST-B", "TEST-P"] {
        let mut r = runner(&[tamer, "TEST-R", "TEST-N", "TEST-O"]);
        let h = r.place_on_field(0, "AD1-015", Some(0));
        trigger(&mut r, h, EffectTiming::EndOfAttack);
        assert_eq!(
            r.pending_selection()
                .unwrap()
                .valid_action_ids
                .iter()
                .filter(|&&a| a != PASS)
                .count(),
            1
        );
        pick(&mut r, 0);
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[0].battle_area.len(), 2);
        assert_eq!(r.memory(), 10);
        assert_eq!(r.hand_size(0), 3);
    }
}
#[test]
fn ad1_015_native_and_named_source_requirements_use_mask_and_payment() {
    for (base, sources, legal) in [
        ("TEST-H", 0, true),
        ("TEST-W", 0, false),
        ("TEST-K", 1, false),
        ("TEST-K", 2, true),
    ] {
        let mut r = runner(&["AD1-015"]);
        let mut stack = vec!["TEST-H"; sources];
        stack.push(base);
        r.place_stack(0, &stack);
        let a = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize] > 0.5,
            legal,
            "{base}/{sources}"
        );
        if legal {
            r.game.decode_action(a, 0);
            r.auto_resolve().unwrap();
            assert_eq!(r.memory(), 7);
            assert_eq!(r.hand_size(0), 1);
        }
    }
}
#[test]
fn ad1_015_trash_play_then_placement_under_new_tamer_after_deletion() {
    let mut r = runner(&["TEST-H"]);
    r.inject_trash(0, "TEST-P");
    let h = r.place_on_field(0, "AD1-015", Some(0));
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
    pick(&mut r, 0); // free play from trash
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        1,
        "deleted source is not a destination"
    );
    pick(&mut r, 0);
    pick(&mut r, 0);
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(
        r.game.players[0].battle_area[0].digivolution_cards().len(),
        2
    );
    assert_eq!(r.memory(), 10);
}
#[test]
fn ad1_015_deleted_carrier_without_tamer_does_not_place_or_draw() {
    let mut r = runner(&["TEST-H"]);
    let h = r.place_on_field(0, "AD1-015", Some(0));
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
    assert!(r.game.players[0].battle_area.is_empty());
}
#[test]
fn ad1_015_jamming_is_face_up_only_and_no_candidate_safe() {
    use digimon_engine::enums::Keyword;
    let mut r = runner(&["TEST-O"]);
    let h = r.place_on_field(0, "AD1-015", Some(0));
    assert!(r.game.has_keyword(h, Keyword::Jamming));
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    assert!(r.pending_selection().is_none());
    let h = r.place_stack(0, &["AD1-015", "TEST-O"]);
    assert!(!r.game.has_keyword(h, Keyword::Jamming));
}

#[test]
fn ad1_015_full_field_skips_play_but_keeps_independent_placement() {
    let mut r = runner(&["TEST-Y", "TEST-H"]);
    let h = r.place_on_field(0, "AD1-015", Some(0));
    for _ in 1..14 {
        r.place_on_field(0, "TEST-O", Some(0));
    }
    trigger(&mut r, h, EffectTiming::EndOfAttack);
    // The only legal first selection is the source destination; no free-play slot.
    assert!(r.pending_selection().unwrap().prompt.contains("place"));
    pick(&mut r, 0);
    pick(&mut r, 0);
    assert_eq!(r.game.players[0].battle_area.len(), 14);
    assert_eq!(r.hand_size(0), 3);
}
#[test]
fn ad1_015_deleted_carrier_does_not_alias_surviving_digimon_slot() {
    let mut r = runner(&["TEST-H"]);
    let h = r.place_on_field(0, "AD1-015", Some(0));
    r.place_on_field(0, "TEST-O", Some(0));
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "unrelated Digimon cannot replace deleted source as a destination"
    );
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn ad1_015_jamming_survives_higher_dp_security_battle() {
    let mut high = make_test_card("TEST-SECURITY", "Security Digimon");
    high.dp = Some(15000);
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-015")
        .unwrap()
        .add_card(high)
        .security(1, &["TEST-SECURITY"; 3])
        .memory(10)
        .start();
    r.game.turn_count = 3;
    let h = r.place_on_field(0, "AD1-015", Some(0));
    let _ = r.attack_player(h, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[1].security.len(), 2);
}
