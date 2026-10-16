use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
const ID: &str = "BT20-040";
fn setup() -> (DebugRunner, PermanentHandle) {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-040.yaml"))
        .expect("BT20-040 production card")
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-042.yaml"))
        .expect("BT20-042 production Groundramon");
    for (id, name, color, text, level) in [
        ("BLUE", "Blue", CardColor::Blue, "[Dracomon]", 3),
        ("EXAMON", "Blue", CardColor::Blue, "[Examon]", 3),
        ("RED", "Red", CardColor::Red, "[Dracomon]", 3),
        ("PLAIN", "Plain", CardColor::Blue, "", 3),
        ("NATIVE", "Native", CardColor::Green, "", 3),
        ("DRACO", "Dracomon Variant", CardColor::Yellow, "", 3),
        ("GROUND", "Groundramon", CardColor::Green, "", 5),
        ("WRONG", "Groundramon X", CardColor::Green, "", 5),
        ("ILLEGAL", "Groundramon", CardColor::Yellow, "", 5),
        ("FILL", "Fill", CardColor::Red, "", 3),
    ] {
        let mut c = make_test_card(id, name);
        c.colors = vec![color];
        c.level = Some(level);
        c.effect_text = text.into();
        c.play_cost = 3;
        c.dp = Some(7000);
        if level == 5 {
            c.evo_costs = vec![EvoCost {
                card_color: if id == "ILLEGAL" { 2 } else { 3 },
                level: 4,
                memory_cost: 4,
            }];
        }
        b = b.add_card(c);
    }
    let mut tamer = make_test_card("TAMER", "Examon Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Blue];
    b = b.add_card(tamer);
    let mut r = b
        .hand(0, &["GROUND", "WRONG", "ILLEGAL"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, Some(0));
    (r, h)
}
fn fire(r: &mut DebugRunner, owner: u8, id: &str) {
    let a = r.place_on_field(owner, id, None);
    r.fire_play_event_triggers(owner, a.index as usize, true, false);
}
fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .into()
}
#[test]
fn bt20_040_blue_text_play_evolves_with_reduction_and_exact_legal_mask() {
    for text in ["BLUE", "EXAMON"] {
        let (mut r, h) = setup();
        fire(&mut r, 0, text);
        let v = r.pending_selection_view().expect("optional evolution");
        assert!(v.is_optional);
        assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
        let pick = [PLAY_HAND_START, HAND_EFFECT_START]
            .into_iter()
            .find(|a| v.valid_action_ids.contains(a))
            .expect("legal Groundramon");
        for slot in [1, 2] {
            assert!(!v.valid_action_ids.contains(&(PLAY_HAND_START + slot)));
            assert!(!v.valid_action_ids.contains(&(HAND_EFFECT_START + slot)));
        }
        let before = r.memory();
        r.execute_action(0, pick).unwrap();
        assert_eq!(top(&r, h), "GROUND");
        assert_eq!(before - r.memory(), 2);
        assert_eq!(r.effective_dp(h), Some(9000));
    }
}
#[test]
fn bt20_040_decline_and_trigger_filters() {
    let (mut r, h) = setup();
    fire(&mut r, 0, "BLUE");
    r.execute_action(0, PASS).unwrap();
    assert_eq!(top(&r, h), ID);
    fire(&mut r, 0, "EXAMON");
    assert!(
        r.pending_selection().is_some(),
        "the trigger is not once per turn"
    );
    r.execute_action(0, PASS).unwrap();
    for (owner, id) in [(0, "RED"), (0, "PLAIN"), (1, "BLUE"), (0, "TAMER")] {
        let (mut r, h) = setup();
        fire(&mut r, owner, id);
        assert!(r.pending_selection().is_none(), "{owner} {id}");
        assert_eq!(top(&r, h), ID);
    }
    let (mut r, _) = setup();
    r.end_turn();
    r.auto_resolve().unwrap();
    fire(&mut r, 0, "BLUE");
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt20_040_raid_is_face_only_and_inherited_dp_is_own_turn_only() {
    let (mut r, h) = setup();
    assert!(r.game.has_keyword(h, Keyword::Raid));
    assert_eq!(r.effective_dp(h), Some(5000));
    let carrier = r.place_stack(0, &[ID, "GROUND"]);
    assert_eq!(r.effective_dp(carrier), Some(9000));
    assert!(!r.game.has_keyword(carrier, Keyword::Raid));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(carrier), Some(7000));
}
#[test]
fn bt20_040_native_and_dracomon_digivolution_routes() {
    for (base, cost) in [("RED", 3), ("NATIVE", 3), ("DRACO", 2)] {
        let (mut r, _) = setup();
        r.game.players[0].hand.clear();
        r.add_to_hand(0, ID);
        let h = r.place_on_field(0, base, None);
        let before = r.memory();
        let action = encode_digivolve(0, h.index as u16);
        assert!(digimon_engine::action::build_action_mask(&r.game, 0)[action as usize] > 0.0);
        r.game.decode_action(action, 0);
        assert_eq!(top(&r, h), ID);
        assert_eq!(before - r.memory(), cost);
    }
}

#[test]
fn bt20_040_no_native_legal_target_cannot_evolve() {
    let (mut r, h) = setup();
    r.game.players[0].hand.clear();
    r.add_to_hand(0, "ILLEGAL");
    fire(&mut r, 0, "BLUE");
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, h), ID);
}

#[test]
fn bt20_040_real_groundramon_cost_routes_are_player_selected_and_clone_safe() {
    let (mut r, h) = setup();
    r.game.players[0].hand.clear();
    r.add_to_hand(0, "BT20-042");
    fire(&mut r, 0, "BLUE");
    let hand_pick = r.pending_selection_view().unwrap().valid_action_ids[0];
    r.execute_action(0, hand_pick).unwrap();
    let choice = r
        .pending_selection_view()
        .expect("both printed routes must be offered");
    assert_eq!(
        choice.kind,
        digimon_engine::selection::SelectionKind::EffectChoice
    );
    assert!(!choice.is_optional);
    assert_eq!(
        choice.valid_action_ids,
        vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]
    );
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(mask[PASS as usize], 0.0);
    for action in &choice.valid_action_ids {
        assert!(mask[*action as usize] > 0.5);
    }
    assert_eq!(top(&r, h), ID);
    assert_eq!(r.memory(), 10, "cost is unpaid until the route is selected");
    assert_eq!(r.game.players[0].hand.len(), 1);
    let mut branch = r.game.clone();
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    branch.decode_action(HAND_EFFECT_START + 1, 0);
    for (game, paid) in [(&r.game, 1), (&branch, 2)] {
        assert_eq!(game.memory, 10 - paid);
        assert_eq!(
            game.players[0].battle_area[h.index as usize]
                .top_card()
                .card_id(&game.card_data),
            "BT20-042"
        );
        assert_eq!(game.players[0].hand.len(), 1, "one digivolution bonus draw");
        assert!(game.pending_selection.is_none());
    }
    assert_eq!(r.effective_dp(h), Some(9000));
}

#[test]
fn bt20_040_raid_mask_offers_both_highest_unsuspended_targets() {
    use digimon_engine::action::space::encode_attack;
    let (mut r, h) = setup();
    r.game.turn_count = 1;
    r.game.enter_main_phase();
    let first = r.place_on_field(1, "FILL", Some(0));
    let second = r.place_on_field(1, "FILL", Some(0));
    let lower = r.place_on_field(1, ID, Some(0));
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    for target in [first, second] {
        assert!(mask[encode_attack(h.index as u16, target.index as u16) as usize] > 0.5);
    }
    assert_eq!(
        mask[encode_attack(h.index as u16, lower.index as u16) as usize],
        0.0
    );
    assert!(
        mask[encode_attack(h.index as u16, 14) as usize] > 0.5,
        "Raid does not force an attack on the highest-DP Digimon"
    );
}
