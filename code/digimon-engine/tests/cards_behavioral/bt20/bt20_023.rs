//! Official observation 064fe77ff3f21af1997ea928feb09f4244c849274d76e03e4c3624ce001de64d,
//! actual BT20-023 card image, and Q4312/Q4313. Base-repo DCGO BT20_023.cs agrees.
//! Rules: general_rule.pdf p.18, 8-1-2-1/8-1-3; p.36, 16-9 (Jamming).
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
const ID: &str = "BT20-023";
fn setup() -> (DebugRunner, PermanentHandle) {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cards/bt20/BT20-023.yaml"
            ))
            .expect("production YAML"),
        )
        .expect("BT20-023 compiled")
        .dsl_card("BT20-025")
        .expect("real Wingdramon routes");
    for (id, name, color, text, level) in [
        ("GREEN", "Green", CardColor::Green, "[Dracomon]", 3),
        ("EXAMON", "Green", CardColor::Green, "[Examon]", 3),
        ("RED", "Red", CardColor::Red, "[Dracomon]", 3),
        ("BLUE", "Blue", CardColor::Blue, "", 3),
        ("PLAIN", "Plain", CardColor::Green, "", 3),
        ("DRACO", "Dracomon Variant", CardColor::Yellow, "", 3),
        ("WING", "Wingdramon", CardColor::Blue, "", 5),
        ("WRONG", "Wingdramon X", CardColor::Blue, "", 5),
        ("ILLEGAL", "Wingdramon", CardColor::Yellow, "", 5),
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
                card_color: if id == "ILLEGAL" { 2 } else { 1 },
                level: 4,
                memory_cost: 4,
            }];
        }
        b = b.add_card(c);
    }
    let mut tamer = make_test_card("TAMER", "Examon Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.colors = vec![CardColor::Green];
    b = b.add_card(tamer);
    let mut r = b
        .hand(0, &["WING", "WRONG", "ILLEGAL"])
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
fn bt20_023_green_text_play_evolves_with_reduction_and_exact_legal_mask() {
    for text in ["GREEN", "EXAMON"] {
        let (mut r, h) = setup();
        fire(&mut r, 0, text);
        let v = r.pending_selection_view().expect("optional evolution");
        assert!(v.is_optional);
        assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
        let pick = [PLAY_HAND_START, HAND_EFFECT_START]
            .into_iter()
            .find(|a| v.valid_action_ids.contains(a))
            .expect("legal Wingdramon");
        for slot in [1, 2] {
            assert!(!v.valid_action_ids.contains(&(PLAY_HAND_START + slot)));
            assert!(!v.valid_action_ids.contains(&(HAND_EFFECT_START + slot)));
        }
        let before = r.memory();
        r.execute_action(0, pick).unwrap();
        assert_eq!(top(&r, h), "WING");
        assert_eq!(before - r.memory(), 2);
        assert_eq!(r.effective_dp(h), Some(9000));
    }
}
#[test]
fn bt20_023_decline_and_trigger_filters() {
    let (mut r, h) = setup();
    fire(&mut r, 0, "GREEN");
    r.execute_action(0, PASS).unwrap();
    assert_eq!(top(&r, h), ID);
    for (owner, id) in [(0, "RED"), (0, "PLAIN"), (1, "GREEN"), (0, "TAMER")] {
        let (mut r, h) = setup();
        fire(&mut r, owner, id);
        assert!(r.pending_selection().is_none(), "{owner} {id}");
        assert_eq!(top(&r, h), ID);
    }
    let (mut r, _) = setup();
    r.end_turn();
    r.auto_resolve().unwrap();
    fire(&mut r, 0, "GREEN");
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt20_023_jamming_is_face_only_and_inherited_dp_is_own_turn_only() {
    let (mut r, h) = setup();
    assert!(r.game.has_keyword(h, Keyword::Jamming));
    assert_eq!(r.effective_dp(h), Some(5000));
    let carrier = r.place_stack(0, &[ID, "WING"]);
    assert_eq!(r.effective_dp(carrier), Some(9000));
    assert!(!r.game.has_keyword(carrier, Keyword::Jamming));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(carrier), Some(7000));
}
#[test]
fn bt20_023_native_and_dracomon_digivolution_routes() {
    for (base, cost) in [("RED", 3), ("BLUE", 3), ("DRACO", 2)] {
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
fn bt20_023_real_wingdramon_cost_routes_are_chosen_after_hand_pick_and_clone() {
    let (mut r, h) = setup();
    r.game.players[0].hand.clear();
    r.add_to_hand(0, "BT20-025");
    fire(&mut r, 0, "GREEN");
    let hand_choice = r
        .pending_selection_view()
        .expect("optional Wingdramon choice");
    assert!(hand_choice.is_optional);
    assert_eq!(hand_choice.valid_action_ids.len(), 1);
    let before = r.memory();
    let deck_before = r.game.players[0].deck.len();
    r.execute_action(0, hand_choice.valid_action_ids[0])
        .unwrap();
    let costs = r.pending_selection_view().expect("mandatory cost choice");
    assert!(!costs.is_optional);
    assert_eq!(
        costs.valid_action_ids,
        vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]
    );
    let entries = costs.effect_choices.as_ref().unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Digivolve for cost 1", "Digivolve for cost 2"]
    );
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(mask[HAND_EFFECT_START as usize], 1.0);
    assert_eq!(mask[HAND_EFFECT_START as usize + 1], 1.0);
    assert_eq!(mask[PASS as usize], 0.0);
    assert_eq!(r.memory(), before);
    assert_eq!(top(&r, h), ID);
    assert_eq!(r.hand_size(0), 1);
    let mut cloned = r.game.clone();
    r.execute_action(0, HAND_EFFECT_START).unwrap();
    cloned.decode_action(HAND_EFFECT_START + 1, 0);
    for (game, cost) in [(&r.game, 1), (&cloned, 2)] {
        assert_eq!(game.memory, before - cost);
        assert_eq!(
            game.players[0].battle_area[h.index as usize]
                .top_card()
                .card_id(&game.card_data),
            "BT20-025"
        );
        assert_eq!(
            game.players[0].hand.len(),
            1,
            "exactly one digivolution draw"
        );
        assert_eq!(game.players[0].deck.len(), deck_before - 1);
        assert!(game.pending_selection.is_none());
        assert!(!game.has_keyword(h, Keyword::Jamming));
    }
    assert_eq!(r.effective_dp(h), Some(9000));
}

#[test]
fn bt20_023_declining_one_trigger_does_not_consume_a_once_per_turn_limit() {
    let (mut r, h) = setup();
    fire(&mut r, 0, "GREEN");
    r.execute_action(0, PASS).unwrap();
    fire(&mut r, 0, "EXAMON");
    let v = r
        .pending_selection_view()
        .expect("printed effect is not once per turn");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    assert_eq!(top(&r, h), "WING");
}

#[test]
fn bt20_023_no_native_legal_target_cannot_evolve() {
    let (mut r, h) = setup();
    r.game.players[0].hand.clear();
    r.add_to_hand(0, "ILLEGAL");
    fire(&mut r, 0, "GREEN");
    assert!(r.pending_selection().is_none());
    assert_eq!(top(&r, h), ID);
}
