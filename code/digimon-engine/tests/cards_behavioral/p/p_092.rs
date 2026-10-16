use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;
const ID: &str = "P-092";
fn setup(inherited: bool) -> DebugRunner {
    let yaml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/cards/p/P-092.yaml"))
        .expect("P-092 implementation");
    let mut wing = make_test_card("WING", "Wingdramon");
    wing.level = Some(5);
    wing.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: CardColor::Blue as u8,
        level: 4,
        memory_cost: 4,
    }];
    let mut wrong_wing = wing.clone();
    wrong_wing.card_id = "WRONGWING".into();
    wrong_wing.card_name = "Wingdramon Variant".into();
    let mut alias = make_test_card("GROUNDALIAS", "Different Printed Name");
    alias.also_treated_as = vec!["Groundramon".into()];
    let mut core = make_test_card("CORE", "Coredramon");
    core.level = Some(4);
    core.colors = vec![CardColor::Blue];
    let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .from_dsl_yaml("card: QUEUED\nname: Groundramon\nkind: digimon\nlevel: 5\ncolor: [green]\ncost: 7\ndp: 7000\neffects:\n  - when: on_play\n    process:\n      - draw: {of: you, count: 1}\n").unwrap()
        .add_card(wing).add_card(wrong_wing).add_card(alias).add_card(core).add_card(make_test_card("BADCORE", "Unrelated Rookie")).add_card(make_test_card("GROUND", "Groundramon"))
        .add_card(make_test_card("WRONG", "Groundramon Variant"))
        .add_card(make_test_card("FILL", "Plain"))
        .hand(0, &["WING", "WRONGWING"]).deck(0,&["FILL";20]).deck(1,&["FILL";20]).memory(8).start();
    r.skip_mulligan();
    if inherited {
        r.place_stack(0, &[ID, "CORE"]);
    } else {
        r.place_on_field(0, ID, Some(0));
    }
    r
}
#[test]
fn played_groundramon_departure_does_not_erase_pending_trigger() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        event(&mut r, 0, "QUEUED");
        assert!(matches!(
            r.pending_kind(),
            Some(digimon_engine::selection::SelectionKind::TriggerOrder)
        ));
        // A simultaneous effect can move the played Digimon before this observer resolves.
        let departed = r.game.players[0].battle_area.pop().unwrap();
        r.game.players[0].trash.extend(departed.card_sources);
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "WING",
            "event identity survives subject departure"
        );
    }
}
fn event(r: &mut DebugRunner, player: u8, id: &str) {
    let h = r.place_on_field(player, id, Some(0));
    r.fire_play_event_triggers(player, h.index as usize, true, false);
    r.game.drain_effect_queue();
}
#[test]
fn face_fixed_three_and_inherited_free_with_clone() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        event(&mut r, 0, "GROUND");
        assert!(r.pending_selection().is_some());
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids.len(),
            1
        );
        let pick = r.pending_selection_view().unwrap().valid_action_ids[0];
        let mut clone = DebugRunner::wrap(r.game.clone());
        for x in [&mut r, &mut clone] {
            x.execute_action(0, pick).unwrap();
            assert_eq!(x.memory(), if inherited { 8 } else { 5 });
            assert_eq!(
                x.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&x.game.card_data),
                "WING"
            );
        }
    }
}
#[test]
fn exact_played_name_not_existing_groundramon_and_own_turn_only() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        r.place_on_field(0, "GROUND", Some(0));
        event(&mut r, 0, "WRONG");
        assert!(r.pending_selection().is_none());
        event(&mut r, 1, "GROUND");
        assert!(r.pending_selection().is_none());
        r.game.turn_player_idx = 1;
        event(&mut r, 0, "GROUND");
        assert!(r.pending_selection().is_none());
        r.game.turn_player_idx = 0;
        event(&mut r, 0, "GROUND");
        assert!(r.pending_selection().is_some());
        r.execute_action(0, PASS).unwrap();
        assert_eq!(r.memory(), 8);
        event(&mut r, 0, "GROUND");
        assert!(r.pending_selection().is_some(), "not once per turn");
    }
}

#[test]
fn inherited_respects_q4182_and_face_ignores_level_requirement() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        if inherited {
            // Dracomon under a level 3 cannot meet Wingdramon's level 4 route.
            let top = r.game.players[0].battle_area[0].card_sources.pop().unwrap();
            r.game.players[0].trash.push(top);
            let source = r.place_on_field(0, "BADCORE", Some(0));
            let extra = r.game.players[0].battle_area.remove(source.index as usize);
            r.game.players[0].battle_area[0]
                .card_sources
                .extend(extra.card_sources);
        }
        event(&mut r, 0, "GROUND");
        if inherited {
            assert!(
                r.pending_selection().is_none(),
                "Q4182: incompatible Wingdramon is not offered"
            );
            assert_eq!(r.memory(), 8);
        } else {
            let pick = r.pending_selection_view().unwrap().valid_action_ids[0];
            assert_eq!(build_action_mask(&r.game, 0)[pick as usize], 1.0);
            r.execute_action(0, pick).unwrap();
            assert_eq!(r.memory(), 5);
        }
    }
}

#[test]
fn missing_wingdramon_finishes_without_spurious_choice() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        let hand = std::mem::take(&mut r.game.players[0].hand);
        r.game.players[0].trash.extend(hand);
        event(&mut r, 0, "GROUND");
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 8);
    }
}

#[test]
fn rule_name_alias_triggers_and_unrelated_host_is_untouched() {
    for inherited in [false, true] {
        let mut r = setup(inherited);
        let other = r.place_on_field(0, "CORE", Some(0));
        event(&mut r, 0, "GROUNDALIAS");
        let view = r
            .pending_selection_view()
            .expect("exact rule-name alias triggers");
        assert_eq!(
            view.valid_action_ids.len(),
            1,
            "substring Wingdramon is excluded"
        );
        let pick = view.valid_action_ids[0];
        assert_eq!(build_action_mask(&r.game, 0)[pick as usize], 1.0);
        r.execute_action(0, pick).unwrap();
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "WING"
        );
        assert_eq!(
            r.game.players[0].battle_area[other.index as usize]
                .top_card()
                .card_id(&r.game.card_data),
            "CORE"
        );
    }
}

#[test]
fn native_routes_and_exact_bebydomon_zero_cost_route_expose_legal_masks() {
    use digimon_engine::action::space::encode_digivolve;
    let yaml = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/cards/p/P-092.yaml"))
        .unwrap();
    for (name, color, cost) in [
        ("Blue Egg", CardColor::Blue, Some(1)),
        ("Green Egg", CardColor::Green, Some(1)),
        ("Red Egg", CardColor::Red, None),
        ("Bebydomon", CardColor::Red, Some(0)),
        ("Bebydomon Variant", CardColor::Red, None),
    ] {
        let mut egg = make_test_card("EGG", name);
        egg.card_kind = digimon_engine::enums::CardKind::DigiEgg;
        egg.level = Some(2);
        egg.colors = vec![color];
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(&yaml)
            .unwrap()
            .add_card(egg)
            .add_card(make_test_card("FILL", "Plain"))
            .hand(0, &[ID])
            .deck(0, &["FILL"; 20])
            .deck(1, &["FILL"; 20])
            .memory(8)
            .start();
        r.skip_mulligan();
        r.place_in_breeding(0, "EGG");
        let action = encode_digivolve(0, digimon_engine::action::space::BREEDING_TARGET);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.0,
            cost.is_some(),
            "{name}"
        );
        if let Some(cost) = cost {
            r.game.decode_action(action, 0);
            assert_eq!(r.memory(), 8 - cost, "{name}");
            assert_eq!(
                r.game.players[0]
                    .breeding_area
                    .as_ref()
                    .unwrap()
                    .top_card()
                    .card_id(&r.game.card_data),
                ID
            );
        }
    }
}
