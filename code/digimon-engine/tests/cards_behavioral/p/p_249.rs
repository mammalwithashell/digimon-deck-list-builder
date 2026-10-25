//! P-249: checked P-249.webp, official source candidate and DCGO P/Yellow/P_249.cs.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::{permanent::PermanentHandle, TriggerSource};
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card("P-249")
        .expect("P-249 YAML")
        .dsl_card("BT18-037")
        .unwrap();
    for (id, kind, inherited, hybrid) in [
        ("TAMER", CardKind::Tamer, true, false),
        ("BLANK", CardKind::Tamer, false, false),
        ("OTHER", CardKind::Digimon, true, false),
        ("MATERIAL", CardKind::Digimon, false, true),
        ("FILL", CardKind::Digimon, false, false),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.colors = vec![CardColor::Yellow];
        if kind == CardKind::Tamer {
            c.level = None;
            c.dp = None;
        }
        if inherited {
            c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
        }
        if hybrid {
            c.traits = vec!["Hybrid".into()];
        }
        b = b.add_card(c);
    }
    b.deck(0, &["FILL"; 20]).deck(1, &["FILL"; 20]).memory(10)
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn pick(r: &mut DebugRunner, n: usize) {
    let p = r.pending_selection().expect("player-visible choice");
    let acts: Vec<_> = p
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    let mask = build_action_mask(&r.game, p.selecting_player);
    for &a in &p.valid_action_ids {
        assert!(mask[a as usize] > 0.5);
    }
    r.execute_action(p.selecting_player, acts[n]).unwrap();
}
fn decline(r: &mut DebugRunner) {
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    r.execute_action(0, PASS).unwrap();
}
#[test]
fn p_249_placing_hand_or_trash_pays_then_evolves_exact_host_for_one_memory() {
    for trash in [false, true] {
        for tamer in [false, true] {
            let hand = if trash {
                vec!["BT18-037"]
            } else {
                vec!["MATERIAL", "BT18-037"]
            };
            let mut r = base().hand(0, &hand).start();
            if trash {
                r.inject_trash(0, "MATERIAL");
            }
            let h = r.place_on_field(0, "P-249", Some(0));
            if tamer {
                r.place_on_field(0, "TAMER", Some(1));
            }
            fire(&mut r, h, EffectTiming::StartOfYourMainPhase);
            // Both the payment Hybrid and evolution Hybrid are selectable from hand.
            pick(&mut r, if trash { 1 } else { 0 });
            pick(&mut r, if tamer { 1 } else { 0 });
            let host = if tamer { 1 } else { 0 };
            assert_eq!(
                r.game.players[0].battle_area[host].card_sources[0].card_id(&r.game.card_data),
                "MATERIAL"
            );
            assert_eq!(r.memory(), 10);
            pick(&mut r, 0);
            assert_eq!(
                r.game.players[0].battle_area[host]
                    .top_card()
                    .card_id(&r.game.card_data),
                "BT18-037"
            );
            assert_eq!(r.memory(), 9);
            // Lobomon's separate security-search effect is optional; no security candidates here.
            r.auto_resolve().unwrap();
        }
    }
}
#[test]
fn p_249_payment_or_evolution_may_be_declined_without_refunding_paid_material() {
    let mut r = base().hand(0, &["MATERIAL", "BT18-037"]).start();
    let h = r.place_on_field(0, "P-249", Some(0));
    fire(&mut r, h, EffectTiming::StartOfYourMainPhase);
    decline(&mut r);
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.memory(), 10);
    fire(&mut r, h, EffectTiming::StartOfYourMainPhase);
    pick(&mut r, 0);
    pick(&mut r, 0);
    decline(&mut r);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), 10);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
}
#[test]
fn p_249_recipient_filter_excludes_other_digimon_blank_and_opponent_tamers() {
    let mut r = base().hand(0, &["MATERIAL"]).start();
    let h = r.place_on_field(0, "P-249", Some(0));
    for id in ["TAMER", "BLANK", "OTHER"] {
        r.place_stack(0, &[id]);
    }
    r.place_stack(1, &["TAMER"]);
    fire(&mut r, h, EffectTiming::StartOfYourMainPhase);
    pick(&mut r, 0);
    assert_eq!(
        r.pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        2
    );
    pick(&mut r, 1);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 2);
}
#[test]
fn p_249_no_hybrid_means_no_payment_or_evolution() {
    let mut r = base().hand(0, &["TAMER", "OTHER"]).start();
    let h = r.place_on_field(0, "P-249", Some(0));
    fire(&mut r, h, EffectTiming::StartOfYourMainPhase);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 2);
}
#[test]
fn p_249_inherited_deletion_free_play_filters_and_decline() {
    for play in [false, true] {
        let mut r = base().hand(0, &["TAMER", "BLANK", "OTHER"]).start();
        let h = r.place_stack(0, &["P-249", "OTHER"]);
        r.game.delete_permanent_with_effects(h);
        r.game.drain_effect_queue();
        assert_eq!(
            r.pending_selection()
                .unwrap()
                .valid_action_ids
                .iter()
                .filter(|&&a| a != PASS)
                .count(),
            1
        );
        if play {
            pick(&mut r, 0);
            assert_eq!(r.game.players[0].battle_area.len(), 1);
            assert_eq!(r.hand_size(0), 2);
        } else {
            decline(&mut r);
            assert!(r.game.players[0].battle_area.is_empty());
            assert_eq!(r.hand_size(0), 3);
        }
        assert_eq!(r.memory(), 10);
    }
}
#[test]
fn p_249_on_deletion_is_inherited_only() {
    let mut r = base().hand(0, &["TAMER"]).start();
    let h = r.place_stack(0, &["P-249"]);
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn p_249_host_selection_can_be_declined_before_payment() {
    let mut r = base().hand(0, &["MATERIAL", "BT18-037"]).start();
    let h = r.place_stack(0, &["P-249"]);
    fire(&mut r, h, EffectTiming::StartOfYourMainPhase);
    pick(&mut r, 0);
    decline(&mut r);
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert!(r.pending_selection().is_none());
}
#[test]
fn p_249_evolution_requires_both_hybrid_trait_and_normal_requirements() {
    use digimon_engine::card_data::EvoCost;
    let mut non_hybrid = make_test_card("NON-HYBRID-EVO", "Non Hybrid");
    non_hybrid.level = Some(4);
    non_hybrid.colors = vec![CardColor::Yellow];
    non_hybrid.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 1,
    }];
    let mut illegal = non_hybrid.clone();
    illegal.card_id = "ILLEGAL-HYBRID".into();
    illegal.traits = vec!["Hybrid".into()];
    illegal.evo_costs[0].level = 6;
    let mut r = base()
        .add_card(non_hybrid)
        .add_card(illegal)
        .hand(
            0,
            &["MATERIAL", "BT18-037", "NON-HYBRID-EVO", "ILLEGAL-HYBRID"],
        )
        .start();
    let h = r.place_stack(0, &["P-249"]);
    fire(&mut r, h, EffectTiming::StartOfYourMainPhase);
    pick(&mut r, 0);
    pick(&mut r, 0);
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
    assert_eq!(r.memory(), 9);
}
#[test]
fn p_249_native_blue_and_yellow_egg_routes_cost_zero() {
    use digimon_engine::action::space::encode_digivolve;
    for color in [CardColor::Yellow, CardColor::Blue] {
        let mut egg = make_test_card("EGG", "Egg");
        egg.card_kind = CardKind::DigiEgg;
        egg.level = Some(2);
        egg.colors = vec![color];
        egg.dp = None;
        let mut r = base().add_card(egg).hand(0, &["P-249"]).start();
        r.place_stack(0, &["EGG"]);
        let action = encode_digivolve(0, 0);
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
        r.game.decode_action(action, 0);
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 10);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "P-249"
        );
    }
}
