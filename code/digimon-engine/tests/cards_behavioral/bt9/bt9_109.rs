//! BT9-109: card image, official reviewed text, DCGO BT9_109.cs;
//! general_rule 8-1, 15-3, 15-8-2 (persistent prohibition).
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
const ID: &str = "BT9-109";
fn yaml() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/cards/bt9/BT9-109.yaml"
    ))
    .expect("BT9-109 production YAML must exist")
}
fn base() -> DebugRunnerBuilder {
    let mut top = make_test_card_with_level("TOP", "Host", 3);
    top.colors = vec![CardColor::Blue];
    top.dp = Some(10000);
    let mut evo = make_test_card_with_level("EVO", "Candidate", 4);
    evo.colors = vec![CardColor::Blue];
    // The attack resumes after evolution; survive the 2000-DP security fixture.
    evo.dp = Some(6000);
    evo.traits = vec!["X Antibody".into()];
    evo.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: 2,
    }];
    let mut wrong = evo.clone();
    wrong.card_id = "WRONG".into();
    wrong.traits.clear();
    let mut illegal = evo.clone();
    illegal.card_id = "ILLEGAL".into();
    illegal.evo_costs[0].level = 5;
    let mut tamer = make_test_card("TAMER", "Blue Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Blue];
    let mut near = make_test_card("NEAR", "X Antibody Proto Form");
    near.card_kind = CardKind::Option;
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .unwrap()
        .add_card(top)
        .add_card(evo)
        .add_card(wrong)
        .add_card(illegal)
        .add_card(tamer)
        .add_card(near)
        .add_card(make_test_card("PLAIN", "Plain"))
        .deck(0, &["PLAIN"; 20])
        .deck(1, &["PLAIN"; 20])
        .memory(5)
}
fn main(r: &mut DebugRunner) {
    r.skip_mulligan();
    r.game.enter_main_phase();
}
fn choose(r: &mut DebugRunner, action: u16) {
    let player = r
        .game
        .pending_selection
        .as_ref()
        .expect("selection")
        .selecting_player;
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.execute_action(player, action).unwrap();
}
fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_owned())
        .collect()
}
#[test]
fn color_bypass_requires_own_battle_digimon() {
    for state in 0..4 {
        let mut r = base().hand(0, &[ID]).start();
        main(&mut r);
        match state {
            0 => {}
            1 => {
                r.place_on_field(0, "TAMER", Some(0));
            }
            2 => {
                r.place_on_field(1, "TOP", Some(0));
            }
            _ => {
                r.place_on_field(0, "TOP", Some(0));
            }
        }
        assert_eq!(
            build_action_mask(&r.game, 0)[0],
            if state == 3 { 1.0 } else { 0.0 },
            "state={state}"
        );
    }
}
#[test]
fn main_places_bottom_and_excludes_existing_x_antibody_sources_and_tamers() {
    let mut r = base().hand(0, &[ID]).start();
    main(&mut r);
    r.place_stack(0, &[ID, "TOP"]);
    let eligible = r.place_stack(0, &["NEAR", "TOP"]);
    r.place_on_field(0, "TAMER", Some(0));
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
    r.game.decode_action(0, 0);
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("mandatory host choice");
    assert!(!p.is_optional);
    assert_eq!(p.valid_action_ids.len(), 1);
    let action = p.valid_action_ids[0];
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        choose(game, action);
        assert_eq!(stack_ids(game, eligible), vec![ID, "NEAR", "TOP"]);
        assert_eq!(game.hand_size(0), 0);
        assert_eq!(game.memory(), 5);
        assert!(game.game.players[0].trash.is_empty());
    }
}
#[test]
fn main_with_only_ineligible_host_trashes_option_without_duplicating_source() {
    let mut r = base().hand(0, &[ID]).start();
    main(&mut r);
    let h = r.place_stack(0, &[ID, "TOP"]);
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
    r.game.decode_action(0, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(stack_ids(&r, h), vec![ID, "TOP"]);
    assert_eq!(r.game.players[0].trash.len(), 1);
}
fn attack_setup() -> DebugRunner {
    let mut r = base()
        .hand(0, &["EVO", "WRONG", "ILLEGAL"])
        .security(1, &["PLAIN"; 3])
        .start();
    main(&mut r);
    r.place_stack(0, &[ID, "TOP"]);
    r.game.players[0].battle_area[0].turn_played = 0;
    r.game.turn_count = 2;
    let action = encode_attack(0, 14);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    r
}
#[test]
fn inherited_attack_choice_filters_trait_and_legality_pays_printed_cost_after_clone() {
    let mut r = attack_setup();
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional evolution");
    assert!(p.is_optional);
    let actions: Vec<_> = p
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(actions.len(), 1, "wrong trait and illegal route excluded");
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        choose(game, actions[0]);
        assert_eq!(
            game.game.players[0].battle_area[0]
                .top_card()
                .card_id(&game.game.card_data),
            "EVO"
        );
        assert_eq!(game.memory(), 3);
        assert_eq!(game.hand_size(0), 3, "evolution draws one");
    }
}
#[test]
fn inherited_attack_may_decline_without_spending_memory_or_card() {
    let mut r = attack_setup();
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        choose(game, PASS);
        assert_eq!(
            game.game.players[0].battle_area[0]
                .top_card()
                .card_id(&game.game.card_data),
            "TOP"
        );
        assert_eq!(game.memory(), 5);
        assert_eq!(game.hand_size(0), 3);
    }
}
#[test]
fn security_gains_memory_and_adds_exact_option_to_hand() {
    let mut r = base().security(1, &[ID]).start();
    main(&mut r);
    r.place_on_field(0, "TOP", Some(0));
    r.game.players[0].battle_area[0].turn_played = 0;
    r.game.turn_count = 2;
    let action = encode_attack(0, 14);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 4);
    assert_eq!(r.hand_size(1), 1);
    assert_eq!(r.game.players[1].hand[0].card_id(&r.game.card_data), ID);
    assert!(r.game.players[1].trash.is_empty());
}
#[test]
fn both_players_source_trash_prohibition_preserves_exact_name_and_not_other_sources() {
    for player in [0, 1] {
        let mut r = base().start();
        let h = r.place_stack(0, &[ID, "NEAR", "TOP"]);
        r.game.tick_declarative_effects();
        let cards = r.game.players[0].battle_area[0].card_sources.clone();
        let mut clone = DebugRunner::wrap(r.game.clone());
        for game in [&mut r, &mut clone] {
            let mut ctx = EffectContext::new(&mut game.game, cards[2].handle(), Some(h), player);
            assert!(!ctx.trash_card_source(h, cards[0].handle()));
            assert!(ctx.trash_card_source(h, cards[1].handle()));
            assert_eq!(stack_ids(game, h), vec![ID, "TOP"]);
        }
    }
}
#[test]
fn protected_bottom_does_not_replace_fixed_positional_target_with_next_source() {
    let mut r = base().start();
    let h = r.place_stack(0, &[ID, "PLAIN", "PLAIN", "TOP"]);
    r.game.tick_declarative_effects();
    let card = r.game.players[0].battle_area[0].top_card().handle();
    assert_eq!(
        EffectContext::new(&mut r.game, card, Some(h), 1).trash_bottom_sources(h, 2),
        1
    );
    assert_eq!(stack_ids(&r, h), vec![ID, "PLAIN", "TOP"]);
}
#[test]
fn whole_stack_departure_still_trashes_sources() {
    let mut r = base().start();
    let h = r.place_stack(0, &[ID, "PLAIN", "TOP"]);
    r.game.tick_declarative_effects();
    r.game.return_to_hand(h);
    assert_eq!(r.game.players[0].trash.len(), 2);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn cloned_source_cost_rechecks_protection_added_after_first_pick() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .unwrap()
        .from_dsl_yaml(include_str!("../../../cards/st4/ST4-13.yaml"))
        .unwrap()
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .memory(5)
        .start();
    let h = r.place_stack(0, &[ID, "PLAIN", "PLAIN", "ST4-13"]);
    r.place_on_field(1, "PLAIN", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();
    let first_card = r.game.players[0].battle_area[0].card_sources[1].handle();
    let action = digimon_engine::action::FIELD_EFFECT_START
        + digimon_engine::action::FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    let first = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, first).unwrap();
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::CannotTrash, 0, Expiry::Permanent, 0)
                .with_payload(ModifierPayload::ProtectedSourceCards(vec![first_card])),
        );
        let second = game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        game.resolve_selection(0, second).unwrap();
        assert!(!game.players[1].battle_area[0].is_suspended);
        assert_eq!(
            game.players[0].trash.len(),
            1,
            "only the unprotected selected card paid"
        );
        assert!(
            game.pending_selection.is_none(),
            "no benefit target selection after partial payment"
        );
    }
}
#[test]
fn digi_burst_excludes_protection_and_clone_completes_paid_effect() {
    use digimon_engine::action::{
        build_action_mask, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
    };
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .unwrap()
        .from_dsl_yaml(include_str!("../../../cards/st4/ST4-13.yaml"))
        .unwrap()
        .add_card(make_test_card_with_level("PLAIN", "Plain", 4))
        .memory(5)
        .start();
    r.place_stack(0, &[ID, "PLAIN", "PLAIN", "ST4-13"]);
    r.place_on_field(1, "PLAIN", Some(0));
    r.game.enter_main_phase();
    r.game.tick_declarative_effects();
    let action = (FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN) as usize;
    assert_eq!(build_action_mask(&r.game, 0)[action], 1.0);
    let action = digimon_engine::action::FIELD_EFFECT_START
        + digimon_engine::action::FIELD_EFFECT_SLOT_FOR_MAIN;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    assert_eq!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .len(),
        2
    );
    let mut cloned = r.game.clone();
    for game in [&mut r.game, &mut cloned] {
        for _ in 0..4 {
            let Some(pending) = game.pending_selection.as_ref() else {
                break;
            };
            let player = pending.selecting_player;
            let action = pending.valid_action_ids[0];
            game.resolve_selection(player, action).unwrap();
        }
        assert!(game.players[1].battle_area[0].is_suspended);
        assert_eq!(game.players[0].trash.len(), 2);
        assert_eq!(
            game.players[0].battle_area[0].card_sources[0].card_id(&game.card_data),
            ID
        );
    }
}

#[test]
fn breeding_only_digimon_does_not_satisfy_color_bypass() {
    let mut r = base().hand(0, &[ID]).start();
    main(&mut r);
    r.place_in_breeding(0, "TOP");
    assert_eq!(build_action_mask(&r.game, 0)[0], 0.0);
}
#[test]
fn inherited_evolution_respects_cannot_digivolve_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = base().hand(0, &["EVO"]).security(1, &["PLAIN"; 3]).start();
    main(&mut r);
    let h = r.place_stack(0, &[ID, "TOP"]);
    r.game.players[0].battle_area[0].turn_played = 0;
    r.game.turn_count = 2;
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 1),
    );
    let a = encode_attack(0, 14);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    assert!(
        r.game.pending_selection.is_none(),
        "no legal evolution must not offer an illegal card"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TOP"
    );
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), 5);
}

#[test]
fn main_can_choose_token_but_cannot_stack_card_with_it() {
    // general_rule 4-21-3; DCGO BT9_109 selects the token then skips placement.
    use digimon_engine::token_registry::TokenDef;
    let token = TokenDef {
        name: "x_antibody_blue_token".into(),
        card_id: "TOKEN_X_ANTIBODY_BLUE".into(),
        card_name: "Blue Digimon Token".into(),
        colors: vec![CardColor::Blue],
        dp: Some(3000),
        level: None,
        traits: vec![],
        keywords: vec![],
    };
    let mut r = base().add_card(token.to_card_data()).hand(0, &[ID]).start();
    main(&mut r);
    r.game.token_registry.insert(token);
    let source = r.game.players[0].hand[0].handle();
    EffectContext::new(&mut r.game, source, None, 0)
        .play_token(0, "x_antibody_blue_token")
        .expect("registered token play");
    r.game.drain_effect_queue();
    assert_eq!(
        build_action_mask(&r.game, 0)[0],
        1.0,
        "own blue token enables color bypass"
    );
    r.game.decode_action(0, 0);
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("token is selectable");
    assert!(!pending.is_optional);
    assert_eq!(pending.valid_action_ids.len(), 1);
    let action = pending.valid_action_ids[0];
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        choose(game, action);
        assert_eq!(
            game.game.players[0].battle_area[0].card_sources.len(),
            1,
            "tokens cannot gain source cards"
        );
        assert_eq!(game.game.players[0].trash.len(), 1);
        assert_eq!(
            game.game.players[0].trash[0].card_id(&game.game.card_data),
            ID
        );
        assert_eq!(game.hand_size(0), 0);
    }
}

#[test]
fn inherited_evolution_rechecks_late_prohibition_on_clone() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = attack_setup();
    let action = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action != PASS)
        .unwrap();
    let mut clone = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut clone] {
        game.game.modifiers.add(
            PermanentHandle {
                player: 0,
                index: 0,
            },
            ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 1),
        );
        choose(game, action);
        assert_eq!(
            game.game.players[0].battle_area[0]
                .top_card()
                .card_id(&game.game.card_data),
            "TOP"
        );
        assert_eq!(
            game.memory(),
            5,
            "a late prohibition cannot pay for an illegal evolution"
        );
        assert_eq!(
            game.hand_size(0),
            3,
            "candidate remains in hand and no evolution draw occurs"
        );
    }
}
