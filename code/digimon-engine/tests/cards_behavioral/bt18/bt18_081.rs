use digimon_engine::action::space::{HAND_EFFECT_START, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::TriggerSource;
fn base() -> DebugRunnerBuilder {
    let mut t = make_test_card("T", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.colors = vec![CardColor::Yellow];
    t.inherited_text = "Inherited".into();
    let mut b = t.clone();
    b.card_id = "BLANK".into();
    b.inherited_text.clear();
    let mut l = make_test_card("L", "Loweemon");
    l.level = Some(4);
    l.dp = Some(10000);
    l.colors = vec![CardColor::Purple];
    l.traits = vec!["Hybrid".into()];
    let mut k = l.clone();
    k.card_id = "K".into();
    k.card_name = "KaiserLeomon".into();
    DebugRunner::builder()
        .dsl_card("BT18-081")
        .expect("Rhihimon loads")
        .add_card(t)
        .add_card(b)
        .add_card(l)
        .add_card(k)
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .memory(10)
}
fn pick(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("choice");
    let a = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[a as usize] > 0.5);
    r.execute_action(0, a).unwrap();
}
#[test]
fn bt18_081_metadata() {
    let r = base().start();
    let c = r.compiled_card("BT18-081").unwrap();
    assert_eq!(c.name, "Rhihimon");
    assert_eq!(c.dp, Some(8000));
    assert_eq!(c.level, Some(5));
    assert_eq!(c.alt_paths.len(), 2);
}
#[test]
fn bt18_081_hand_mask_needs_both_roles_and_tamer() {
    for trash in [vec![], vec!["L"], vec!["K"], vec!["L", "L"]] {
        let mut r = base().hand(0, &["BT18-081"]).start();
        for id in &trash {
            r.inject_trash(0, id)
        }
        r.place_stack(0, &["T"]);
        r.game.current_phase = digimon_engine::enums::GamePhase::Main;
        assert_eq!(
            digimon_engine::action::mask::build_action_mask(&r.game, 0)[HAND_EFFECT_START as usize],
            0.0
        );
    }
    let mut r = base().hand(0, &["BT18-081"]).start();
    for id in ["L", "K"] {
        r.inject_trash(0, id)
    }
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    assert_eq!(
        digimon_engine::action::mask::build_action_mask(&r.game, 0)[HAND_EFFECT_START as usize],
        0.0
    );
}
#[test]
fn bt18_081_hand_pays_both_materials_and_three_memory() {
    let mut r = base().hand(0, &["BT18-081", "BT18-081"]).start();
    for id in ["L", "L", "K"] {
        r.inject_trash(0, id)
    }
    r.place_stack(0, &["T"]);
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let chosen = r.game.players[0].hand[1].handle();
    let other = r.game.players[0].hand[0].handle();
    assert!(r.game.activate_hand_main(0, 1));
    pick(&mut r);
    pick(&mut r);
    assert_eq!(r.pending_action_count(), 1, "duplicate Loweemon excluded");
    pick(&mut r);
    r.auto_resolve().unwrap();
    let p = &r.game.players[0].battle_area[0];
    assert_eq!(p.top_card().card_id(&r.game.card_data), "BT18-081");
    assert_eq!(p.card_sources.len(), 4);
    assert_eq!(p.top_card().handle(), chosen);
    assert!(r.game.players[0].hand.iter().any(|c| c.handle() == other));
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[0].trash.len(), 1);
}
#[test]
fn bt18_081_free_tamer_filter_and_decline() {
    for decline in [false, true] {
        let mut r = base().start();
        for id in ["T", "BLANK", "L"] {
            r.inject_trash(0, id)
        }
        let h = r.place_stack(0, &["L", "BT18-081"]);
        r.game
            .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
        r.game.drain_effect_queue();
        assert!(r.pending_is_optional());
        assert_eq!(r.pending_action_count(), 1);
        if decline {
            r.execute_action(0, PASS).unwrap();
        } else {
            pick(&mut r);
        }
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.players[0].battle_area.len(),
            if decline { 1 } else { 2 }
        );
        assert_eq!(r.memory(), 10);
    }
}

#[test]
fn bt18_081_inherited_targets_opponent_digimon_once_per_turn() {
    let mut r = base().start();
    let host = r.place_stack(0, &["BT18-081", "L"]);
    let victim = r.place_stack(1, &["L"]);
    r.place_stack(1, &["T"]);
    r.place_stack(0, &["L"]);
    let before = r.effective_dp(victim).unwrap();
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    assert_eq!(r.pending_action_count(), 1);
    assert!(!r.pending_is_optional());
    pick(&mut r);
    r.auto_resolve().unwrap();
    // Fixture Digimon must survive the reduction to test the persistent modifier.
    assert_eq!(r.effective_dp(victim), Some(before - 4000));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.effective_dp(victim), Some(before - 4000));
}
#[test]
fn bt18_081_native_and_special_routes_mask_and_payment() {
    use digimon_engine::action::{mask::build_action_mask, space::encode_digivolve};
    use digimon_engine::enums::{GamePhase, Keyword};
    for (color, hybrid, kind, cost) in [
        (CardColor::Purple, false, CardKind::Digimon, Some(4)),
        (CardColor::Yellow, false, CardKind::Digimon, Some(4)),
        (CardColor::Yellow, true, CardKind::Digimon, Some(3)),
        (CardColor::Purple, true, CardKind::Digimon, Some(3)),
        (CardColor::Red, true, CardKind::Digimon, None),
        (CardColor::Purple, true, CardKind::Tamer, None),
    ] {
        let mut b = make_test_card("BASE", "Base");
        b.level = Some(4);
        b.colors = vec![color];
        b.card_kind = kind;
        if hybrid {
            b.traits = vec!["Hybrid".into()];
        }
        let mut r = base().add_card(b).hand(0, &["BT18-081"]).start();
        r.place_stack(0, &["BASE"]);
        r.game.current_phase = GamePhase::Main;
        let action = encode_digivolve(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.5,
            cost.is_some()
        );
        if let Some(cost) = cost {
            r.game.decode_action(action, 0);
            if let Some(v) = r.pending_selection_view() {
                if let Some(choices) = v.effect_choices {
                    let a = choices
                        .iter()
                        .find(|c| c.label == format!("Digivolve for cost {cost}"))
                        .unwrap()
                        .action_id;
                    r.execute_action(0, a).unwrap();
                }
            }
            r.auto_resolve().unwrap();
            assert_eq!(r.memory(), 10 - cost);
            let h = digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 0,
            };
            assert!(r.game.has_keyword(h, Keyword::Jamming));
        }
    }
}
#[test]
fn bt18_081_no_eligible_free_tamer_no_selection() {
    let mut r = base().start();
    r.inject_trash(0, "BLANK");
    r.inject_trash(0, "L");
    let h = r.place_stack(0, &["L", "BT18-081"]);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

#[test]
fn bt18_081_hand_tamer_color_filter_and_multiple_targets() {
    for (color, kind, legal) in [
        (CardColor::Red, CardKind::Tamer, false),
        (CardColor::Purple, CardKind::Digimon, false),
        (CardColor::Purple, CardKind::Tamer, true),
        (CardColor::Yellow, CardKind::Tamer, true),
    ] {
        let mut t = make_test_card("TARGET", "Target");
        t.card_kind = kind;
        t.colors = vec![color];
        let mut r = base().add_card(t).hand(0, &["BT18-081"]).start();
        r.inject_trash(0, "L");
        r.inject_trash(0, "K");
        r.place_stack(0, &["TARGET"]);
        r.game.current_phase = digimon_engine::enums::GamePhase::Main;
        assert_eq!(
            digimon_engine::action::mask::build_action_mask(&r.game, 0)[HAND_EFFECT_START as usize]
                > 0.5,
            legal
        );
        if legal {
            r.place_stack(0, &["T"]);
            assert!(r.game.activate_hand_main(0, 0));
            assert_eq!(r.pending_action_count(), 2, "player chooses Tamer");
        }
    }
}
#[test]
fn bt18_081_full_field_cannot_play_free_tamer() {
    let mut r = base().start();
    r.inject_trash(0, "T");
    let h = r.place_stack(0, &["L", "BT18-081"]);
    for _ in 1..14 {
        r.place_stack(0, &["PAD"]);
    }
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 14);
    assert_eq!(r.trash_size(0), 1);
}
