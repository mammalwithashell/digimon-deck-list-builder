//! Official observation d56c63ca7e328fce784da58990735fe50d591f111325169cb8035b006c26978b,
//! card image and Q2342; base DCGO BT13/Blue/BT13_096.cs.
use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
const ID: &str = "BT13-096";
fn base() -> DebugRunnerBuilder {
    let yaml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/cards/bt13/BT13-096.yaml"
    ))
    .expect("BT13-096 production YAML required");
    let mut b = DebugRunner::builder().from_dsl_yaml(&yaml).unwrap();
    for (id, level, blue, kind) in [
        ("BLUE3", 3, true, CardKind::Digimon),
        ("BLUE4", 4, true, CardKind::Digimon),
        ("BLUE5", 5, true, CardKind::Digimon),
        ("RED3", 3, false, CardKind::Digimon),
        ("TAMER", 3, true, CardKind::Tamer),
        ("HOST", 6, false, CardKind::Digimon),
    ] {
        let mut c = make_test_card(id, id);
        c.level = Some(level);
        c.colors = vec![if blue {
            CardColor::Blue
        } else {
            CardColor::Red
        }];
        c.card_kind = kind;
        b = b.add_card(c);
    }
    b.deck(0, &["HOST"; 20]).deck(1, &["HOST"; 20]).memory(10)
}
fn setup() -> DebugRunner {
    let mut r = base().start();
    r.skip_mulligan();
    r.place_on_field(0, ID, Some(0));
    r
}
fn choose(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("expected player choice");
    let a = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[a as usize],
        1.0
    );
    r.execute_action(v.selecting_player, a).unwrap();
}
fn event(r: &mut DebugRunner, p: u8, id: &str) {
    let h = r.place_on_field(p, id, Some(0));
    r.fire_play_event_triggers(p, h.index as usize, true, false);
    r.game.drain_effect_queue();
}
#[test]
fn on_play_source_filter_across_digimon_hosts_and_decline() {
    for accept in [false, true] {
        let mut r = setup();
        r.place_stack(0, &["BLUE3", "RED3", "BLUE4", "HOST"]);
        r.place_stack(0, &["BLUE3", "HOST"]);
        r.place_stack(0, &["BLUE3", "TAMER"]);
        r.place_stack(1, &["BLUE3", "HOST"]);
        r.fire_on_play(0, 0);
        assert_eq!(
            r.pending_selection_view()
                .unwrap()
                .valid_action_ids
                .iter()
                .filter(|&&a| a != PASS)
                .count(),
            2,
            "only blue Lv3 sources under own Digimon"
        );
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
        if accept {
            choose(&mut r);
            if r.pending_selection().is_some() {
                r.execute_action(0, PASS).unwrap();
            }
            assert_eq!(r.game.players[0].battle_area.len(), 5);
            assert_eq!(r.memory(), 10);
        } else {
            r.execute_action(0, PASS).unwrap();
            assert_eq!(r.game.players[0].battle_area.len(), 4);
        }
    }
}
#[test]
fn both_turns_tuck_exact_played_subject_and_hand_filters_clone() {
    for turn in [0, 1] {
        let mut r = setup();
        r.game.turn_player_idx = turn;
        r.place_on_field(0, "BLUE3", Some(0));
        for c in ["BLUE3", "BLUE4", "BLUE5", "RED3", "TAMER"] {
            r.add_to_hand(0, c);
        }
        event(&mut r, 0, "BLUE3");
        assert!(r.pending_selection().is_some());
        choose(&mut r);
        let v = r.pending_selection_view().unwrap();
        assert_eq!(
            v.valid_action_ids,
            vec![PLAY_HAND_START, PLAY_HAND_START + 1]
        );
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
        assert!(r.game.players[0].battle_area[0].is_suspended);
        let mut clone = DebugRunner::wrap(r.game.clone());
        for g in [&mut r, &mut clone] {
            g.execute_action(0, PLAY_HAND_START + 1).unwrap();
            assert_eq!(g.game.players[0].battle_area[1].card_sources.len(), 1);
            assert_eq!(g.game.players[0].battle_area[2].card_sources.len(), 2);
            assert_eq!(
                g.game.players[0].battle_area[2].card_sources[0].card_id(&g.game.card_data),
                "BLUE4"
            );
        }
    }
}
#[test]
fn declining_activation_or_tuck_and_already_suspended() {
    let mut r = setup();
    r.add_to_hand(0, "BLUE3");
    event(&mut r, 0, "BLUE3");
    r.execute_action(0, PASS).unwrap();
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    event(&mut r, 0, "BLUE3");
    choose(&mut r);
    r.execute_action(0, PASS).unwrap();
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.hand_size(0), 1);
    event(&mut r, 0, "BLUE3");
    assert!(r.pending_selection().is_none());
}
#[test]
fn opponent_nonblue_tamer_and_inherited_homer_do_not_trigger() {
    let mut r = setup();
    r.add_to_hand(0, "BLUE3");
    for (p, c) in [(1, "BLUE3"), (0, "RED3"), (0, "TAMER")] {
        event(&mut r, p, c);
        assert!(r.pending_selection().is_none());
    }
    let mut r = base().start();
    r.skip_mulligan();
    r.place_stack(0, &[ID, "HOST"]);
    r.add_to_hand(0, "BLUE3");
    event(&mut r, 0, "BLUE3");
    assert!(r.pending_selection().is_none());
}
#[test]
fn vanished_event_subject_does_not_attach_to_shifted_replacement_slot() {
    let mut r = setup();
    r.add_to_hand(0, "BLUE3");
    event(&mut r, 0, "BLUE3");
    r.place_on_field(0, "BLUE4", Some(0));
    // Remove the played instance while the already-triggered observer is parked.
    let departed = r.game.players[0].battle_area.remove(1);
    r.game.players[0].trash.extend(departed.card_sources);
    choose(&mut r);
    if r.pending_selection().is_some() {
        choose(&mut r);
    }
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "Q2342 still permits paying suspension"
    );
    assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 1);
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn vanished_subject_does_not_retarget_another_copy_of_same_card() {
    let mut r = setup();
    r.add_to_hand(0, "BLUE4");
    event(&mut r, 0, "BLUE3");
    r.place_on_field(0, "BLUE3", Some(0));
    assert_ne!(
        r.game.players[0].battle_area[1].top_card().handle(),
        r.game.players[0].battle_area[2].top_card().handle()
    );
    let departed = r.game.players[0].battle_area.remove(1);
    r.game.players[0].trash.extend(departed.card_sources);
    choose(&mut r);
    if r.pending_selection().is_some() {
        choose(&mut r);
    }
    assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 1);
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn live_played_subject_survives_lower_slot_removal_and_evolution() {
    for evolve in [false, true] {
        let mut r = setup();
        r.place_on_field(0, "HOST", Some(0));
        r.add_to_hand(0, "BLUE4");
        event(&mut r, 0, "BLUE3");
        // Another simultaneous effect removes a lower slot or digivolves the same permanent.
        if evolve {
            r.add_to_hand(0, "BLUE5");
            let evolved = r.game.players[0].hand.pop().unwrap();
            r.game.players[0].battle_area[2].card_sources.push(evolved);
        } else {
            let gone = r.game.players[0].battle_area.remove(1);
            r.game.players[0].trash.extend(gone.card_sources);
        }
        choose(&mut r);
        choose(&mut r);
        let target = if evolve { 2 } else { 1 };
        assert_eq!(
            r.game.players[0].battle_area[target].card_sources[0].card_id(&r.game.card_data),
            "BLUE4",
            "that Digimon retains its physical identity"
        );
        assert_eq!(r.hand_size(0), 0);
    }
}
#[test]
fn q2342_immediate_zero_dp_deletion_still_triggers_suspend_cost() {
    let aura="card: AURA\nname: Aura\nkind: tamer\ncolor: [yellow]\ncost: 0\neffects:\n  - kind: aura\n    target: { owner: opponent, zone: [battle_area], kind: digimon }\n    dp_modifier: -20000\n";
    let mut r = base()
        .from_dsl_yaml(aura)
        .unwrap()
        .hand(0, &["BLUE3", "BLUE4"])
        .start();
    r.skip_mulligan();
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "AURA", Some(0));
    r.game.tick_declarative_effects();
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "blue Digimon rule-deleted at zero DP"
    );
    assert!(
        r.pending_selection().is_some(),
        "Q2342 says Homer still triggers"
    );
    choose(&mut r);
    if r.pending_selection().is_some() {
        r.execute_action(0, PASS).unwrap();
    }
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.hand_size(0), 1);
}
#[test]
fn security_plays_homer_free() {
    let mut r = base().security(0, &[ID]).start();
    r.skip_mulligan();
    let attacker = r.place_on_field(1, "HOST", Some(0));
    r.game.turn_player_idx = 1;
    r.attack_player(attacker, 0, false);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == ID));
    assert_eq!(r.memory(), 10);
}
#[test]
fn cannot_suspend_cost_and_breeding_sources_excluded() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = base().start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, Some(0));
    r.add_to_hand(0, "BLUE3");
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );
    event(&mut r, 0, "BLUE3");
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.hand_size(0), 1);
    let mut r = setup();
    r.place_stack(0, &["BLUE3", "HOST"]);
    let breeding = r.game.players[0].battle_area.pop().unwrap();
    r.game.players[0].breeding_area = Some(breeding);
    r.fire_on_play(0, 0);
    assert!(r.pending_selection().is_none());
}

#[test]
fn on_play_source_candidates_exclude_controller_prohibited_effect_plays() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    for restricted_player in [0, 1] {
        let mut r = setup();
        r.place_stack(0, &["BLUE3", "HOST"]);
        r.game.modifiers.add_player_modifier(
            restricted_player,
            PlayerModifierEntry::simple(
                ModifierType::CannotPlayDigimonByEffect,
                0,
                Expiry::Permanent,
                None,
                0,
            ),
        );
        r.fire_on_play(0, 0);
        if restricted_player == 0 {
            assert!(r.pending_selection().is_none(),
                "DCGO CanSelectCardCondition checks CanPlayAsNewPermanent; prohibited source plays must not be selectable, even if execution would later no-op");
            assert_eq!(r.game.players[0].battle_area.len(), 2);
            assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 2);
        } else {
            let view = r
                .pending_selection_view()
                .expect("opponent prohibition cannot block our source play");
            assert_eq!(
                view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
                1
            );
            choose(&mut r);
            if r.pending_selection().is_some() {
                r.execute_action(0, PASS).unwrap();
            }
            assert_eq!(r.game.players[0].battle_area.len(), 3);
            assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 1);
        }
    }
}

#[test]
fn may_pay_suspend_without_eligible_hand_and_reuse_if_unsuspended_same_turn() {
    let mut r = setup();
    event(&mut r, 0, "BLUE3");
    choose(&mut r);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert!(r.pending_selection().is_none());
    // This card has no once-per-turn restriction: a separate unsuspend effect
    // would make its cost payable on another played blue Digimon this turn.
    r.game.players[0].battle_area[0].is_suspended = false;
    r.add_to_hand(0, "BLUE4");
    event(&mut r, 0, "BLUE3");
    choose(&mut r);
    choose(&mut r);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.game.players[0].battle_area[2].card_sources.len(), 2);
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn blue_digimon_token_play_triggers_homer_but_cannot_gain_sources() {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::token_registry::TokenDef;
    // No built-in token is blue. Register a test token through the real token
    // catalog and play pipeline; do not put an ordinary card on the field and
    // pretend that it was a token play.
    let token = TokenDef {
        name: "homer_blue_token".into(),
        card_id: "TOKEN_HOMER_BLUE".into(),
        card_name: "Blue Digimon Token".into(),
        colors: vec![CardColor::Blue],
        dp: Some(3000),
        level: None,
        traits: vec![],
        keywords: vec![],
    };
    let mut r = base().add_card(token.to_card_data()).start();
    r.game.token_registry.insert(token);
    r.skip_mulligan();
    let homer = r.place_on_field(0, ID, Some(0));
    r.add_to_hand(0, "BLUE4");
    let source = r.top_card(homer);
    let played = {
        let mut ctx = EffectContext::new(&mut r.game, source, Some(homer), 0);
        ctx.play_token(0, "homer_blue_token")
            .expect("registered token play")
    };
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_some(),
        "a blue Digimon token triggers Homer"
    );
    let mut cloned = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut cloned] {
        choose(game);
        assert!(game.game.players[0].battle_area[0].is_suspended,
            "optional suspension remains payable for blue token play");
        assert!(game.pending_selection().is_none(),
            "token destination must not expose an impossible hand-placement choice");
        assert_eq!(game.game.players[0].battle_area[played.index as usize].card_sources.len(), 1);
        assert_eq!(game.hand_size(0), 1, "hand card never selected or moved");
    }
}

#[test]
fn on_play_source_candidates_require_a_free_field_slot() {
    for full in [false, true] {
        let mut r = setup();
        r.place_stack(0, &["BLUE3", "HOST"]);
        let target_size = r.game.rules.field_slots as usize - usize::from(!full);
        while r.game.players[0].battle_area.len() < target_size {
            r.place_on_field(0, "HOST", Some(0));
        }
        r.fire_on_play(0, 0);
        if full {
            assert!(
                r.pending_selection().is_none(),
                "a source cannot be chosen for play onto a full field"
            );
            assert_eq!(r.game.players[0].battle_area[1].card_sources.len(), 2);
        } else {
            let view = r
                .pending_selection_view()
                .expect("last free slot remains playable");
            assert_eq!(
                view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
                1
            );
            choose(&mut r);
            if r.pending_selection().is_some() {
                r.execute_action(0, PASS).unwrap();
            }
            assert_eq!(
                r.game.players[0].battle_area.len(),
                r.game.rules.field_slots as usize
            );
        }
    }
}

#[test]
fn source_playability_predicate_false_and_missing_subject_are_explicit() {
    use digimon_dsl::compiled::CompiledPredicate;
    use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
    use digimon_engine::effect_context::EffectReadContext;
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    for blocked in [false, true] {
        let mut r = setup();
        r.add_to_hand(0, "BLUE3");
        if blocked {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotPlayDigimonByEffect,
                    0,
                    Expiry::Permanent,
                    None,
                    0,
                ),
            );
        }
        let card = r.game.players[0].hand[0].handle();
        let ctx = EffectReadContext::new(&r.game, card, None, 0);
        for wanted in [false, true] {
            let pred = CompiledPredicate {
                can_play_from_effect_without_cost: Some(wanted),
                ..Default::default()
            };
            assert_eq!(
                eval_predicate(&pred, &ctx, PredicateSubject::Card(card)),
                wanted != blocked
            );
            assert!(
                !eval_predicate(&pred, &ctx, PredicateSubject::None),
                "missing candidate never satisfies a card playability predicate"
            );
        }
    }
}

#[test]
fn ordinary_blue_digimon_still_exposes_masked_source_choice_after_homer_cost() {
    let mut r=setup();
    r.add_to_hand(0,"BLUE4");
    event(&mut r,0,"BLUE3");
    let mut cloned=DebugRunner::wrap(r.game.clone());
    for game in [&mut r,&mut cloned] {
        choose(game);
        assert!(game.game.players[0].battle_area[0].is_suspended);
        let view=game.pending_selection_view().expect("ordinary destination retains hand selection");
        assert_eq!(view.valid_action_ids.iter().filter(|&&a|a!=PASS).count(),1);
        assert_eq!(build_action_mask(&game.game,view.selecting_player)[PASS as usize],1.0);
        choose(game);
        assert_eq!(game.hand_size(0),0);
        assert_eq!(game.game.players[0].battle_area[1].card_sources.len(),2);
        assert!(game.pending_selection().is_none());
    }
}
