//! Behavioral qualification of the remaining authored EX13 Royal Knights pool.
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType, PlaySource};
use digimon_engine::selection::TriggerSource;
const IDS: &[&str] = &[
    "P-238", "BT19-023", "BT20-026", "BT21-032", "BT9-098", "ST17-13", "P-248",
];
fn digi(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(6000);
    c.play_cost = 0;
    c.colors = vec![CardColor::Blue];
    c.traits = traits.iter().map(|x| x.to_string()).collect();
    c
}
fn setup(id: &str) -> DebugRunner {
    setup_extra(id, vec![])
}
fn setup_extra(id: &str, extras: Vec<CardData>) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(id)
        .expect("production card registered");
    for (c, l, t) in [
        ("LOW", 4, vec![]),
        ("HIGH", 7, vec![]),
        ("CS", 3, vec!["CS"]),
        ("ARMOR", 4, vec!["Armor Form"]),
        ("HERO", 4, vec!["Hero"]),
        ("FREE", 3, vec!["Free"]),
        ("FILL", 3, vec![]),
    ] {
        b = b.add_card(digi(c, l, &t));
    }
    for c in extras {
        b = b.add_card(c);
    }
    b.deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(0, &["FILL"; 4])
        .security(1, &["FILL"; 4])
        .memory(10)
        .start()
}
fn pick(r: &mut DebugRunner, n: usize) {
    let p = r.pending_selection_view().expect("visible choice");
    r.execute_action(p.selecting_player, p.valid_action_ids[n])
        .unwrap();
}
#[test]
fn all_seven_production_cards_load() {
    for id in IDS {
        setup(id);
    }
}
#[test]
fn huankunmon_selects_ally_and_grants_only_battle_protection() {
    let mut r = setup("BT19-023");
    let h = r.place_on_field(0, "BT19-023", None);
    let ally = r.place_on_field(0, "LOW", None);
    r.fire_on_play(0, h.index as usize);
    pick(&mut r, 1);
    assert!(r
        .game
        .modifiers
        .has(ally, ModifierType::CannotBeDestroyedByBattle));
    assert!(!r
        .game
        .modifiers
        .has(h, ModifierType::CannotBeDestroyedByBattle));
    assert!(!r
        .game
        .modifiers
        .has(ally, ModifierType::CannotBeDestroyedByEffect));
    r.end_turn();
    assert!(r
        .game
        .modifiers
        .has(ally, ModifierType::CannotBeDestroyedByBattle));
    r.end_turn();
    assert!(!r
        .game
        .modifiers
        .has(ally, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn megaseadramon_bottoms_only_low_level_and_requires_named_source_for_lock() {
    for matching in [false, true] {
        let source = if matching { "MegaSeadramon" } else { "OTHER" };
        // Additional source is registered before constructing this fixture.
        let mut c = digi(source, 5, &[]);
        c.card_name = source.into();
        let mut r = DebugRunner::builder()
            .dsl_card("BT20-026")
            .unwrap()
            .add_card(c)
            .add_card(digi("LOW", 4, &[]))
            .add_card(digi("HIGH", 7, &[]))
            .memory(10)
            .start();
        let h = r.place_stack(0, &[source, "BT20-026"]);
        r.place_on_field(1, "LOW", None);
        r.place_on_field(1, "HIGH", None);
        r.fire_on_play(0, h.index as usize);
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids.len(),
            1
        );
        pick(&mut r, 0);
        assert_eq!(
            r.game.players[1]
                .deck
                .last()
                .unwrap()
                .card_id(&r.game.card_data),
            "LOW"
        );
        if matching {
            pick(&mut r, 0);
            assert!(r.game.modifiers.has(
                digimon_engine::permanent::PermanentHandle {
                    player: 1,
                    index: 0
                },
                ModifierType::CannotSuspend
            ));
        } else {
            assert!(r.pending_selection_view().is_none());
        }
    }
}
fn security(r: &mut DebugRunner, id: &str) {
    r.add_to_hand(0, id);
    let card = r.game.players[0].hand.pop().unwrap();
    let h = card.handle();
    r.game.pending_security = Some(digimon_engine::selection::PendingSecurity {
        defender: 0,
        card,
        played: false,
    });
    r.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::SecurityRevealed {
            defender: 0,
            card: h,
        },
    );
    r.game.drain_effect_queue();
}
#[test]
fn destruction_cannon_main_and_security_exclude_level_seven_and_place_delay() {
    for sec in [false, true] {
        let mut r = setup("P-238");
        r.place_on_field(0, "CS", Some(0));
        r.place_on_field(1, "LOW", None);
        r.place_on_field(1, "HIGH", None);
        if sec {
            security(&mut r, "P-238");
        } else {
            r.add_to_hand(0, "P-238");
            r.game.activate_hand_main(0, 0);
        }
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids.len(),
            1
        );
        pick(&mut r, 0);
        assert_eq!(r.battle_area_size(1), 1);
        assert_eq!(
            r.game.players[1].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "HIGH"
        );
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "P-238"));
    }
}
#[test]
fn destruction_cannon_delay_requires_cs_attack_and_can_be_declined() {
    for cs in [false, true] {
        let mut r = setup("P-238");
        let attacker = r.place_on_field(0, if cs { "CS" } else { "LOW" }, Some(0));
        r.add_to_hand(0, "P-238");
        r.game.activate_hand_main(0, 0);
        r.end_turn();
        r.end_turn();
        r.place_on_field(1, "LOW", None);
        r.attack_player(attacker, 1, false);
        if cs {
            assert!(r.pending_selection_view().is_some());
            r.decline_optional_trigger().unwrap();
        } else {
            assert!(r.pending_selection_view().is_none());
        }
        assert!(r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "P-238"));
    }
}
#[test]
fn promo_veemon_discard_qualifiers_and_decline_gate_both_rewards() {
    for (id, accept) in [
        ("FREE", true),
        ("ARMOR", true),
        ("TEXT", true),
        ("LOW", false),
        ("FREE", false),
    ] {
        let mut text = digi("TEXT", 3, &[]);
        text.effect_text = "Play a Veedramon.".into();
        let mut r = setup_extra("P-248", vec![text]);
        let h = r.place_on_field(0, "P-248", None);
        r.game.memory = 5;
        r.add_to_hand(0, id);
        r.game.enqueue_triggered(
            EffectTiming::StartOfYourMainPhase,
            TriggerSource::Permanent(h),
        );
        r.game.drain_effect_queue();
        if accept {
            pick(&mut r, 0);
        } else if r.pending_selection_view().is_some() {
            r.execute_action(0, digimon_engine::action::space::PASS)
                .unwrap();
        }
        assert_eq!(r.memory(), if accept { 6 } else { 5 });
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(r.game.players[0].trash.len(), usize::from(accept));
    }
}
#[test]
fn hero_veemon_reduces_only_armor_or_hero_evolution_and_inherited_dp_is_turn_scoped() {
    for (trait_name, discount) in [("Armor Form", true), ("Hero", true), ("Other", false)] {
        let mut e = digi("EVO", 4, &[trait_name]);
        e.evo_costs = vec![EvoCost {
            card_color: 1,
            level: 3,
            memory_cost: 3,
        }];
        let mut r = setup_extra("BT21-032", vec![e]);
        let h = r.place_on_field(0, "BT21-032", None);
        r.add_to_hand(0, "EVO");
        assert!(r
            .game
            .digivolve_from_hand(0, 0, h.index as usize, PlaySource::ByHand));
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), if discount { 8 } else { 7 });
        assert_eq!(r.effective_dp(h), Some(8000));
        r.end_turn();
        assert_eq!(r.effective_dp(h), Some(6000));
    }
}
#[test]
fn golden_knight_free_evolution_protects_only_from_opponent_dp_minus() {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::Expiry;
    let mut evo = digi("MAGNA", 6, &[]);
    evo.card_name = "Magnamon Test".into();
    evo.dp = Some(12000);
    let mut r = setup_extra("BT9-098", vec![evo]);
    let host = r.place_on_field(0, "ARMOR", None);
    r.add_to_hand(0, "BT9-098");
    r.add_to_hand(0, "MAGNA");
    r.game.activate_hand_main(0, 0);
    pick(&mut r, 0);
    pick(&mut r, 0);
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "MAGNA"
    );
    assert!(!r
        .game
        .modifiers
        .has(host, ModifierType::CannotBeDeDigivolved));
    let src = r.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, src, None, 1);
        ctx.add_dp_modifier(host, -3000, Expiry::EndOfTurn);
    }
    assert_eq!(r.effective_dp(host), Some(12000));
    {
        let mut ctx = EffectContext::new(&mut r.game, src, None, 0);
        ctx.add_dp_modifier(host, -2000, Expiry::EndOfTurn);
    }
    assert_eq!(r.effective_dp(host), Some(10000));
}
#[test]
fn magnamon_trashes_by_selected_targets_colors_then_bounces_sourceless() {
    let mut two = digi("TWO", 5, &[]);
    two.colors = vec![CardColor::Red, CardColor::Blue];
    let mut r = setup_extra("ST17-13", vec![two]);
    let h = r.place_on_field(0, "ST17-13", None);
    let victim = r.place_stack(1, &["LOW", "LOW", "TWO"]);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    pick(&mut r, 0);
    assert_eq!(
        r.game.players[1].battle_area[victim.index as usize]
            .card_sources
            .len(),
        1
    );
    pick(&mut r, 0);
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.hand_size(1), 1);
}
#[test]
fn magnamon_security_defers_exact_card_evolution_until_battle_end_and_clones_choice() {
    let mut r = setup("ST17-13");
    let base = r.place_on_field(0, "FREE", Some(0));
    r.add_to_hand(0, "ST17-13");
    let card = r.game.players[0].hand.pop().unwrap();
    let checked = card.handle();
    r.game.players[0].security = vec![card];
    let attacker = r.place_on_field(1, "HIGH", Some(0));
    r.end_turn();
    r.attack_player(attacker, 0, false);
    // Security De-Digivolve targets either player's Digimon; choose the own level-3 base.
    let p = r.pending_selection_view().expect("security target");
    assert_eq!(p.selecting_player, 0);
    assert_eq!(p.valid_action_ids.len(), 2);
    pick(&mut r, 0);
    let p = r
        .pending_selection_view()
        .expect("end-of-battle optional evolution");
    assert_eq!(p.selecting_player, 0);
    assert!(p.is_optional);
    assert!(
        digimon_engine::action::build_action_mask(&r.game, 0)
            [digimon_engine::action::space::PASS as usize]
            > 0.0
    );
    let action = *p
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("legal level-3 base");
    let mut fork = r.game.clone();
    fork.decode_action(action, 0);
    r.execute_action(0, action).unwrap();
    assert_eq!(
        r.game.players[0].battle_area[base.index as usize]
            .top_card()
            .handle(),
        checked
    );
    assert_eq!(
        fork.players[0].battle_area[base.index as usize]
            .top_card()
            .handle(),
        checked
    );
    assert!(r.game.pending_security.as_ref().is_none_or(|p| p.played));
}
#[test]
fn inherited_attack_target_locks_and_promo_dp_bonus_expire_on_opponent_turn() {
    for id in ["BT19-023", "BT20-026", "P-248"] {
        let mut r = setup(id);
        let h = r.place_stack(0, &[id, "HIGH"]);
        r.game.tick_declarative_effects();
        if id == "P-248" {
            assert_eq!(r.effective_dp(h), Some(8000));
        } else {
            assert!(r
                .game
                .modifiers
                .has(h, ModifierType::CannotSwitchAttackTarget));
        }
        r.end_turn();
        r.game.tick_declarative_effects();
        if id == "P-248" {
            assert_eq!(r.effective_dp(h), Some(6000));
        } else {
            assert!(!r
                .game
                .modifiers
                .has(h, ModifierType::CannotSwitchAttackTarget));
        }
    }
}
#[test]
fn printed_blocker_and_armor_purge_are_live_effects() {
    use digimon_engine::enums::Keyword;
    use digimon_engine::replacement::ReplacementCause;
    for id in ["BT19-023", "ST17-13"] {
        let mut r = setup(id);
        let h = r.place_stack(0, &["FREE", id]);
        r.game.tick_declarative_effects();
        assert!(r.game.has_keyword(h, Keyword::Blocker));
        if id == "ST17-13" {
            r.game
                .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
            r.accept_optional_trigger().unwrap();
            r.auto_resolve().unwrap();
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "FREE"
            );
        }
    }
}
#[test]
fn destruction_cannon_accepts_delay_deletes_target_and_pays_option() {
    let mut r = setup("P-238");
    let a = r.place_on_field(0, "CS", Some(0));
    r.add_to_hand(0, "P-238");
    r.game.activate_hand_main(0, 0);
    r.end_turn();
    r.end_turn();
    r.place_on_field(1, "LOW", None);
    r.attack_player(a, 1, false);
    r.accept_optional_trigger().unwrap();
    pick(&mut r, 0);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "P-238"));
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LOW"));
}
#[test]
fn golden_knight_security_recovers_any_magnamon_card_and_itself_even_without_match() {
    for matched in [false, true] {
        let mut c = digi("MAGNA", 4, &[]);
        c.card_name = "Magnamon Option".into();
        c.card_kind = CardKind::Option;
        let mut r = setup_extra("BT9-098", vec![c, digi("NOT-MAGNA", 4, &[])]);
        r.add_to_hand(0, "NOT-MAGNA");
        let decoy = r.game.players[0].hand.pop().unwrap();
        r.game.players[0].trash.push(decoy);
        if matched {
            r.add_to_hand(0, "MAGNA");
            let c = r.game.players[0].hand.pop().unwrap();
            r.game.players[0].trash.push(c);
        }
        security(&mut r, "BT9-098");
        if matched {
            assert_eq!(r.pending_selection_view().unwrap().valid_action_ids.len(), 1);
            pick(&mut r, 0);
            assert!(r.game.players[0].hand.iter().any(|c| c.card_id(&r.game.card_data) == "MAGNA"));
        }
        assert!(r.game.players[0].trash.iter().any(|c| c.card_id(&r.game.card_data) == "NOT-MAGNA"));
        assert!(!r.game.players[0].hand.iter().any(|c| c.card_id(&r.game.card_data) == "NOT-MAGNA"));
        assert_eq!(r.hand_size(0), if matched { 2 } else { 1 });
        assert!(r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "BT9-098"));
    }
}
#[test]
fn golden_knight_declined_evolution_does_not_grant_protection() {
    let mut c = digi("MAGNA", 6, &[]);
    c.card_name = "Magnamon".into();
    let mut r = setup_extra("BT9-098", vec![c]);
    let h = r.place_on_field(0, "ARMOR", None);
    r.add_to_hand(0, "BT9-098");
    r.add_to_hand(0, "MAGNA");
    r.game.activate_hand_main(0, 0);
    pick(&mut r, 0);
    r.execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert!(!r.game.modifiers.has(h, ModifierType::ImmuneFromDPMinus));
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ARMOR"
    );
}
#[test]
fn magnamon_security_decline_or_ineligible_base_trashes_checked_card_normally() {
    for legal in [true, false] {
        let mut r = setup("ST17-13");
        r.place_on_field(0, if legal { "FREE" } else { "LOW" }, Some(0));
        r.add_to_hand(0, "ST17-13");
        let card = r.game.players[0].hand.pop().unwrap();
        let checked = card.handle();
        r.game.players[0].security = vec![card];
        let a = r.place_on_field(1, "HIGH", Some(0));
        r.end_turn();
        r.attack_player(a, 0, false);
        pick(&mut r, 0);
        if legal {
            assert!(r.pending_selection_view().unwrap().is_optional);
            r.execute_action(0, digimon_engine::action::space::PASS)
                .unwrap();
        }
        assert!(r.pending_selection_view().is_none());
        assert!(r.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == checked));
    }
}

#[test]
fn destruction_cannon_use_requirement_accepts_cs_digimon_or_tamer_not_options() {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    // CR 16-42-3: specified Digimon and/or Tamers on the field, not Options.
    for kind in [CardKind::Digimon, CardKind::Tamer, CardKind::Option] {
        let mut c = digi("ENABLER", 3, &["CS"]);
        c.card_kind = kind;
        let mut r = setup_extra("P-238", vec![c]);
        r.place_on_field(0, "ENABLER", None);
        r.add_to_hand(0, "P-238");
        assert_eq!(
            build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0,
            kind != CardKind::Option,
            "enabler kind {kind:?}"
        );
    }
}
