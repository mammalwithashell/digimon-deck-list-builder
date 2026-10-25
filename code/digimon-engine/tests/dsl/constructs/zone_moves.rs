//! Zone movement contracts: CR 4.3 3-1-3-3/4/8, 4-7-5/9/10, 15-5-2, 15-7-3/4.
//! Card-text idioms: BT26-081 / BT26-102, ST23-15 / ST24-15.
mod grouped {
    //! Contract coverage for explicit simultaneous return; synthetic effects isolate reusable semantics.
    use digimon_engine::{
        action::{
            build_action_mask,
            space::{encode_attack, PLAY_HAND_START},
        },
        card_source::CardHandle,
        debug_runner::{make_test_card, DebugRunner},
        enums::GamePhase,
    };
    fn act(r: &mut DebugRunner, a: u16) {
        let p = r
            .game
            .pending_selection
            .as_ref()
            .expect("required public choice")
            .selecting_player;
        assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
        r.execute_action(p, a).unwrap();
    }
    fn fixture(position: &str, include_sources: bool) -> DebugRunner {
        let yaml = format!(
            r#"
card: GROUP-CONTRACT
name: Group contract fixture
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - return_all_to_deck:
          over: {{of: opponent, zone: [battle_area], kind: digimon, level_eq: 4}}
          position: {position}
          include_sources: {include_sources}
"#
        );
        let mut b = DebugRunner::builder().from_dsl_yaml(&yaml).unwrap();
        for id in ["TOP-A", "TOP-B", "TOP-C", "SOURCE-A", "SOURCE-B", "FILL"] {
            let mut c = make_test_card(id, id);
            c.level = Some(if id.starts_with("TOP-") { 4 } else { 3 });
            c.dp = Some(4000);
            b = b.add_card(c);
        }
        let mut r = b
            .hand(0, &["GROUP-CONTRACT"])
            .deck(0, &["FILL"; 20])
            .deck(1, &["FILL"; 20])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.game.current_phase = GamePhase::Main;
        r
    }
    fn begin(r: &mut DebugRunner) {
        assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
        r.game.decode_action(PLAY_HAND_START, 0);
    }
    fn choose(r: &mut DebugRunner, index: usize, remaining: usize) {
        let v = r.pending_selection_view().expect("public ordering prompt");
        assert_eq!(
            v.selecting_player, 0,
            "causing player orders the public cards"
        );
        assert_eq!(v.valid_action_ids.len(), remaining);
        assert!(!r.game.pending_selection.as_ref().unwrap().is_optional);
        let expected = v.valid_action_ids.clone();
        let mask = build_action_mask(&r.game, 0);
        for (id, value) in mask.iter().enumerate() {
            assert_eq!(
                *value,
                if expected.contains(&(id as u16)) {
                    1.0
                } else {
                    0.0
                }
            );
        }
        act(r, encode_attack(1, index as u16));
    }
    #[test]
    fn top_three_members_expose_all_six_permutations_and_original_clone_agree() {
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let mut r = fixture("top", false);
            let mut cards = Vec::<CardHandle>::new();
            for id in ["TOP-A", "TOP-B", "TOP-C"] {
                let h = r.place_on_field(1, id, None);
                cards.push(
                    r.game.players[1].battle_area[h.index as usize]
                        .top_card()
                        .handle(),
                );
            }
            begin(&mut r);
            let mut clone = r.game.clone();
            choose(&mut r, order[0], 3);
            choose(&mut r, order[1], 2);
            assert!(r.game.pending_selection.is_none());
            assert!(r.game.players[1].battle_area.is_empty());
            let top = r.game.players[1]
                .deck
                .iter()
                .rev()
                .take(3)
                .map(|c| c.handle())
                .collect::<Vec<_>>();
            assert_eq!(top, order.iter().map(|i| cards[*i]).collect::<Vec<_>>());
            let expected = r.game.verification_digest();
            std::mem::swap(&mut r.game, &mut clone);
            choose(&mut r, order[0], 3);
            choose(&mut r, order[1], 2);
            assert_eq!(r.game.verification_digest(), expected);
        }
    }
    #[test]
    fn included_sources_stay_in_each_selected_stack_for_both_public_orders() {
        for position in ["top", "bottom"] {
            for first in [0usize, 1] {
                let mut r = fixture(position, true);
                r.place_stack(1, &["SOURCE-A", "TOP-A"]);
                r.place_stack(1, &["SOURCE-B", "TOP-B"]);
                let stacks = r.game.players[1]
                    .battle_area
                    .iter()
                    .map(|p| {
                        p.card_sources
                            .iter()
                            .map(|c| c.handle())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                begin(&mut r);
                choose(&mut r, first, 2);
                assert!(r.game.pending_selection.is_none());
                assert!(r.game.players[1].battle_area.is_empty());
                assert!(
                    r.game.players[1].trash.is_empty(),
                    "included source cards move to deck, not trash"
                );
                let raw = r.game.players[1]
                    .deck
                    .iter()
                    .map(|c| c.handle())
                    .collect::<Vec<_>>();
                if position == "top" {
                    let expected = stacks[first]
                        .iter()
                        .rev()
                        .chain(stacks[1 - first].iter().rev())
                        .copied()
                        .collect::<Vec<_>>();
                    assert_eq!(
                        raw.iter().rev().take(4).copied().collect::<Vec<_>>(),
                        expected
                    );
                } else {
                    let expected = stacks[first]
                        .iter()
                        .chain(stacks[1 - first].iter())
                        .copied()
                        .collect::<Vec<_>>();
                    assert_eq!(raw[..4], expected);
                }
            }
        }
    }
    #[test]
    fn included_sources_are_routed_to_physical_owners_without_changing_controller_order() {
        let mut r = fixture("bottom", true);
        r.place_stack(1, &["SOURCE-A", "TOP-A"]);
        r.place_stack(1, &["SOURCE-B", "TOP-B"]);
        r.game.players[1].battle_area[0].card_sources[0].owner = 0;
        let foreign = r.game.players[1].battle_area[0].card_sources[0].handle();
        let top = r.game.players[1].battle_area[0].top_card().handle();
        begin(&mut r);
        choose(&mut r, 0, 2);
        assert!(r.game.players[0].deck.iter().any(|c| c.handle() == foreign));
        assert!(!r.game.players[1].deck.iter().any(|c| c.handle() == foreign));
        assert_eq!(r.game.players[1].deck[0].handle(), top);
        assert!(r.game.players.iter().all(|p| p.trash.is_empty()));
    }

    const INHERITED_LOOP: &str = r#"
card: INHERITED-LOOP
name: Inherited continuation witness
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - scope: inherited
    when: when_attacking
    process:
      - for_each:
          over: {of: you, zone: [battle_area], kind: digimon, level_eq: 4}
          bind_as: member
          body:
            - select_effect_choice:
                labels: ["Return selected member", "Return selected member also"]
                prompt: "Park each actual member"
                bind_as: branch
            - return_to_hand: {target: member}
      - suspend: {target: source}
      - draw: {of: you, count: 1}
"#;
    fn inherited_fixture(departed: bool) -> DebugRunner {
        let mut b = DebugRunner::builder()
            .from_dsl_yaml(INHERITED_LOOP)
            .unwrap();
        for (id, level) in [
            ("LOOP-FIRST", 4),
            ("LOOP-HOST", if departed { 4 } else { 5 }),
            ("LOOP-LAST", 4),
            ("LOOP-OTHER", 5),
        ] {
            let mut c = make_test_card(id, id);
            c.level = Some(level);
            c.dp = Some(5000);
            b = b.add_card(c);
        }
        let mut r = b
            .deck(0, &["LOOP-OTHER"; 20])
            .deck(1, &["LOOP-OTHER"; 20])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.game.current_phase = GamePhase::Main;
        if !departed {
            r.place_on_field(1, "LOOP-FIRST", None);
        }
        let host = r.place_stack(1, &["INHERITED-LOOP", "LOOP-HOST"]);
        r.place_on_field(1, "LOOP-LAST", None);
        r.place_on_field(1, "LOOP-OTHER", None);
        r.game.enqueue_triggered(
            digimon_engine::enums::EffectTiming::WhenAttacking,
            digimon_engine::selection::TriggerSource::Permanent(host),
        );
        r.game.drain_effect_queue();
        r
    }
    #[test]
    fn inherited_loop_continuation_re_resolves_compacted_carrier_and_keeps_actor() {
        let mut r = inherited_fixture(false);
        let before = r.game.players[1].hand.len();
        let mut cloned = r.game.clone();
        let mut expected = None;
        for run in 0..2 {
            if run == 1 {
                std::mem::swap(&mut r.game, &mut cloned);
            }
            for remaining in [2, 1] {
                let v = r
                    .pending_selection_view()
                    .expect("each iteration retains its player choice");
                assert_eq!(v.selecting_player, 1);
                assert_eq!(v.valid_action_ids.len(), 2);
                let a = v.valid_action_ids[0];
                act(&mut r, a);
                if remaining == 2 {
                    assert_eq!(r.game.players[1].hand.len(), before + 1);
                }
            }
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.game.players[1].battle_area.len(), 2);
            assert_eq!(
                r.game.players[1].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "LOOP-HOST"
            );
            assert!(r.game.players[1].battle_area[0].is_suspended);
            assert!(!r.game.players[1].battle_area[1].is_suspended);
            assert_eq!(
                r.game.players[1].hand.len(),
                before + 3,
                "two returns and exactly one actor-relative draw"
            );
            let d = r.game.verification_digest();
            if let Some(prior) = expected {
                assert_eq!(d, prior);
            } else {
                expected = Some(d);
            }
        }
    }
    #[test]
    fn departed_inherited_source_keeps_controller_tail_without_retargeting_new_slot() {
        let mut r = inherited_fixture(true);
        let before = r.game.players[1].hand.len();
        let mut cloned = r.game.clone();
        let mut expected = None;
        for run in 0..2 {
            if run == 1 {
                std::mem::swap(&mut r.game, &mut cloned);
            }
            for _ in 0..2 {
                let v = r
                    .pending_selection_view()
                    .expect("later member still resolves after source departure");
                assert_eq!(v.selecting_player, 1);
                let a = v.valid_action_ids[0];
                act(&mut r, a);
            }
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.game.players[1].battle_area.len(), 1);
            assert_eq!(
                r.game.players[1].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "LOOP-OTHER"
            );
            assert!(
                !r.game.players[1].battle_area[0].is_suspended,
                "departed source binding never points at compacted unrelated permanent"
            );
            assert_eq!(r.game.players[1].hand.len(), before + 3);
            assert_eq!(
                r.game.players[1]
                    .trash
                    .iter()
                    .filter(|c| c.card_id(&r.game.card_data) == "INHERITED-LOOP")
                    .count(),
                1
            );
            let d = r.game.verification_digest();
            if let Some(prior) = expected {
                assert_eq!(d, prior);
            } else {
                expected = Some(d);
            }
        }
    }

    #[test]
    fn captured_return_does_not_follow_same_physical_card_into_fresh_incarnation() {
        let mut r = fixture("bottom", false);
        r.place_on_field(1, "TOP-A", None);
        r.place_on_field(1, "TOP-B", None);
        let first = r.game.players[1].battle_area[0].top_card().handle();
        let second = r.game.players[1].battle_area[1].top_card().handle();
        begin(&mut r);
        // A direct state transition isolates captured-incarnation validation; no
        // claim that a native Main action can be taken during an ordering prompt.
        let departed = r.game.players[1].battle_area.remove(1);
        let card = departed.card_sources.into_iter().last().unwrap();
        r.game.players[1]
            .battle_area
            .push(digimon_engine::permanent::Permanent::new(
                card,
                r.game.turn_count,
            ));
        let mut clone = r.game.clone();
        let mut expected = None;
        for run in 0..2 {
            if run == 1 {
                std::mem::swap(&mut r.game, &mut clone);
            }
            let v = r.pending_selection_view().unwrap();
            assert_eq!(v.selecting_player, 0);
            act(&mut r, encode_attack(1, 0));
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.game.players[1].battle_area.len(), 1);
            assert_eq!(r.game.players[1].battle_area[0].top_card().handle(), second);
            assert!(r.game.players[1].deck.iter().any(|c| c.handle() == first));
            assert!(!r.game.players[1].deck.iter().any(|c| c.handle() == second));
            let d = r.game.verification_digest();
            if let Some(prior) = expected {
                assert_eq!(d, prior);
            } else {
                expected = Some(d);
            }
        }
    }
    #[test]
    fn own_group_hand_return_removes_token_and_routes_real_mother_to_egg_deck() {
        let yaml = r#"
card: GROUP-HAND-CONTRACT
name: Group hand routing fixture
kind: digimon
color: [blue]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - return_all_to_hand:
          over:
            of: you
            zone: [battle_area]
            any_of:
              - name_is: Mother D-Reaper
              - name_is: Routing token
"#;
        let mut token = make_test_card("ROUTING-TOKEN", "Routing token");
        token.card_kind = digimon_engine::enums::CardKind::Token;
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(yaml)
            .unwrap()
            .dsl_card("EX2-007")
            .unwrap()
            .add_card(token)
            .hand(0, &["GROUP-HAND-CONTRACT"])
            .deck(0, &["GROUP-HAND-CONTRACT"; 20])
            .deck(1, &["GROUP-HAND-CONTRACT"; 20])
            .memory(10)
            .start();
        r.skip_mulligan();
        r.game.current_phase = GamePhase::Main;
        let m = r.place_on_field(0, "EX2-007", None);
        let mother = r.game.players[0].battle_area[m.index as usize]
            .top_card()
            .handle();
        let t = r.place_on_field(0, "ROUTING-TOKEN", None);
        let token = r.game.players[0].battle_area[t.index as usize]
            .top_card()
            .handle();
        begin(&mut r);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        assert!(r.game.players[0]
            .digitama_deck
            .iter()
            .any(|c| c.handle() == mother));
        assert!(!r.game.players[0]
            .hand
            .iter()
            .any(|c| c.handle() == mother || c.handle() == token));
        assert!(!r.game.players[0].trash.iter().any(|c| c.handle() == token));
        assert!(!r.game.players[0].deck.iter().any(|c| c.handle() == token));
    }
}
use digimon_engine::enums::ModifierType;
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, TRASH_EFFECT_START},
    },
    debug_runner::{make_test_card, DebugRunner},
    permanent::{OptionState, PermanentHandle},
    CardKind, EffectTiming, Expiry, TriggerSource,
};
fn runner(y: &str) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(y)
        .unwrap()
        .from_dsl_yaml(DELETE_OBSERVER)
        .unwrap()
        .from_dsl_yaml(RETURN_OBSERVER)
        .unwrap();
    for (id, name, cost, level, kind) in [
        ("P2", "Probe", 2, 3, CardKind::Digimon),
        ("P3", "Probe", 3, 3, CardKind::Digimon),
        ("P4", "Probe", 4, 4, CardKind::Digimon),
        ("L4B", "Other four", 4, 4, CardKind::Digimon),
        ("Q1", "Other", 1, 5, CardKind::Digimon),
        ("TAM", "Tamer", 3, 0, CardKind::Tamer),
        ("TAM2", "Other Tamer", 3, 0, CardKind::Tamer),
    ] {
        let mut c = make_test_card(id, name);
        c.play_cost = cost;
        c.level = if level == 0 { None } else { Some(level) };
        c.card_kind = kind;
        c.dp = if level == 0 { None } else { Some(4000) };
        b = b.add_card(c);
    }
    let mut r = b
        .deck(0, &["Q1"; 20])
        .deck(1, &["Q1"; 20])
        .memory(0)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn mask(r: &DebugRunner, ids: &[u16], optional: bool) {
    let s = r
        .game
        .pending_selection
        .as_ref()
        .expect("required public choice");
    assert_eq!(s.selecting_player, 0);
    assert_eq!(s.is_optional, optional);
    let ids = ids.iter().map(|&a| crate::union_fixture_action(&r.game, a)).collect::<Vec<_>>();
    let m = build_action_mask(&r.game, 0);
    for (i, v) in m.iter().enumerate() {
        assert_eq!(
            *v,
            if ids.contains(&(i as u16)) { 1.0 } else { 0.0 },
            "action {i}"
        );
    }
}
fn pick(r: &mut DebugRunner, a: u16) {
    let a = crate::union_fixture_action(&r.game, a);
    let p = r.game.pending_selection.as_ref().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}
fn id(r: &DebugRunner, c: &digimon_engine::card_source::CardSource) -> String {
    c.card_id(&r.game.card_data).to_string()
}
fn immune(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 1),
    );
}
const BUDGET: &str = r#"
card: Z-BUDGET
name: Budget probe
kind: digimon
color: [purple]
level: 4
cost: 0
dp: 4000
effects:
  - when: when_digivolving
    process:
      - select_zone_cards:
          zones: [hand, trash]
          filter: {name_is: Probe}
          play_cost_budget: 5
          bind_as: picked
          prompt: Pick up to five play cost
      - if:
          condition: {binding_present: picked}
          then:
            - gain_memory: 1
"#;
const COUNT: &str = r#"
card: Z-COUNT
name: Count probe
kind: digimon
color: [purple]
level: 4
cost: 0
dp: 4000
effects:
  - when: when_digivolving
    process:
      - select_zone_cards:
          zones: [trash]
          filter: {name_is: Probe}
          min: 2
          max: 2
          optional_zero: true
          bind_as: picked
          prompt: Pick two or none
      - return_trash_list_to_deck_bottom: {of: you, cards: picked}
"#;
#[test]
fn budget_recomputes_legal_choices_and_pass_leaves_binding_unbound() {
    for decline in [false, true] {
        let mut r = runner(BUDGET);
        r.game.players[0].hand.clear();
        r.add_to_hand(0, "P2");
        r.add_to_hand(0, "Q1");
        r.inject_trash(0, "P3");
        r.inject_trash(0, "P4");
        let h = r.place_on_field(0, "Z-BUDGET", None);
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        mask(
            &r,
            &[0, TRASH_EFFECT_START, TRASH_EFFECT_START + 1, PASS],
            true,
        );
        assert!(matches!(
            r.game.pending_selection.as_ref().unwrap().kind,
            digimon_engine::selection::SelectionKind::PlayCostBudget {
                remaining_play_cost: 5,
                picked: 0
            }
        ));
        if decline {
            pick(&mut r, PASS);
            assert_eq!(r.memory(), 0);
        } else {
            pick(&mut r, TRASH_EFFECT_START);
            mask(&r, &[0, PASS], true);
            assert!(matches!(
                r.game.pending_selection.as_ref().unwrap().kind,
                digimon_engine::selection::SelectionKind::PlayCostBudget {
                    remaining_play_cost: 2,
                    picked: 1
                }
            ));
            pick(&mut r, 0);
            assert_eq!(r.memory(), 1);
        }
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[0].hand.len(), 2);
        assert_eq!(r.game.players[0].trash.len(), 2);
    }
}
#[test]
fn exact_count_forbids_partial_payment_and_preserves_pick_order() {
    let mut r = runner(COUNT);
    for c in ["P2", "P3", "P4"] {
        r.inject_trash(0, c);
    }
    let before = r.game.players[0].deck.len();
    let h = r.place_on_field(0, "Z-COUNT", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(
        &r,
        &[
            TRASH_EFFECT_START,
            TRASH_EFFECT_START + 1,
            TRASH_EFFECT_START + 2,
            PASS,
        ],
        true,
    );
    assert_eq!(r.game.pending_selection.as_ref().unwrap().zone_owner, None);
    assert!(matches!(
        r.game.pending_selection.as_ref().unwrap().kind,
        digimon_engine::selection::SelectionKind::CountCappedMultiSelect {
            min: 2,
            max: 2,
            picked: 0,
            ..
        }
    ));
    pick(&mut r, TRASH_EFFECT_START + 1);
    mask(&r, &[TRASH_EFFECT_START, TRASH_EFFECT_START + 2], false);
    assert!(matches!(
        r.game.pending_selection.as_ref().unwrap().kind,
        digimon_engine::selection::SelectionKind::CountCappedMultiSelect {
            min: 2,
            max: 2,
            picked: 1,
            ..
        }
    ));
    assert_eq!(r.game.players[0].trash.len(), 3);
    pick(&mut r, TRASH_EFFECT_START);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.game.players[0].deck.len(), before + 2);
    assert_eq!(id(&r, &r.game.players[0].deck[0]), "P3");
    assert_eq!(id(&r, &r.game.players[0].deck[1]), "P2");
}
#[test]
fn count_below_minimum_and_optional_zero_do_not_move_cards() {
    for n in [1, 3] {
        let mut r = runner(COUNT);
        for _ in 0..n {
            r.inject_trash(0, "P2");
        }
        let before = r.game.players[0].deck.len();
        let h = r.place_on_field(0, "Z-COUNT", None);
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        if n == 3 {
            mask(
                &r,
                &[
                    TRASH_EFFECT_START,
                    TRASH_EFFECT_START + 1,
                    TRASH_EFFECT_START + 2,
                    PASS,
                ],
                true,
            );
            pick(&mut r, PASS);
        }
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[0].trash.len(), n);
        assert_eq!(r.game.players[0].deck.len(), before);
    }
}
#[test]
fn mandatory_one_candidate_still_requires_a_choice() {
    let y = COUNT
        .replace("min: 2", "min: 1")
        .replace("max: 2", "max: 1")
        .replace("optional_zero: true", "optional_zero: false");
    let mut r = runner(&y);
    r.inject_trash(0, "P2");
    let h = r.place_on_field(0, "Z-COUNT", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(&r, &[TRASH_EFFECT_START], false);
    assert_eq!(r.game.players[0].trash.len(), 1);
    pick(&mut r, TRASH_EFFECT_START);
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.players[0].trash.is_empty());
}
const UNION: &str = r#"
card: Z-UNION
name: Union return probe
kind: digimon
color: [blue]
level: 4
cost: 0
dp: 4000
effects:
  - when: when_digivolving
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: picked
          optional: true
          filter: {name_is: Probe}
          prompt: Return a Probe
      - return_union_bound_to_deck: {binding: picked, position: top}
      - if:
          condition: {effect_returned_any_card: true}
          then:
            - gain_memory: 1
  - when: on_discard_hand
    process:
      - gain_memory: 10
"#;
#[test]
fn union_hand_top_trash_bottom_and_pass_receipts_are_not_discards() {
    for choice in [0, TRASH_EFFECT_START, PASS] {
        let y = if choice == TRASH_EFFECT_START {
            UNION.replace("position: top", "position: bottom")
        } else {
            UNION.to_string()
        };
        let mut r = runner(&y);
        r.game.players[0].hand.clear();
        r.add_to_hand(0, "P2");
        r.add_to_hand(0, "Q1");
        r.inject_trash(0, "P3");
        let h = r.place_on_field(0, "Z-UNION", None);
        let before = r.game.players[0].deck.len();
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        mask(&r, &[0, TRASH_EFFECT_START, PASS], true);
        pick(&mut r, choice);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.memory(), if choice == PASS { 0 } else { 1 });
        assert_eq!(
            r.game.players[0].deck.len(),
            before + usize::from(choice != PASS)
        );
        assert_eq!(
            r.game.players[0].hand.len(),
            if choice == 0 { 1 } else { 2 }
        );
        assert_eq!(
            r.game.players[0].trash.len(),
            if choice == TRASH_EFFECT_START { 0 } else { 1 }
        );
        if choice == 0 {
            assert_eq!(id(&r, r.game.players[0].deck.last().unwrap()), "P2");
        }
        if choice == TRASH_EFFECT_START {
            assert_eq!(id(&r, &r.game.players[0].deck[0]), "P3");
        }
    }
}
#[test]
fn sequential_union_picks_define_top_order() {
    let pair="      - select_union_zone: {of: you, zones: [hand, trash], bind_as: second, filter: {name_is: Probe}, prompt: Return another Probe}\n      - return_union_bound_to_deck: {binding: second, position: top}\n";
    let y = UNION.replace("      - if:", &format!("{pair}      - if:"));
    let mut r = runner(&y);
    r.game.players[0].hand.clear();
    r.add_to_hand(0, "P2");
    r.inject_trash(0, "P3");
    let h = r.place_on_field(0, "Z-UNION", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(&r, &[0, TRASH_EFFECT_START, PASS], true);
    pick(&mut r, 0);
    mask(&r, &[TRASH_EFFECT_START], false);
    pick(&mut r, TRASH_EFFECT_START);
    assert!(r.game.pending_selection.is_none());
    let d = &r.game.players[0].deck;
    assert_eq!(id(&r, &d[d.len() - 1]), "P3");
    assert_eq!(id(&r, &d[d.len() - 2]), "P2");
    assert_eq!(r.memory(), 1);
}
const OPTION: &str = r#"
card: Z-OPTION
name: Field Option probe
kind: option
color: [white]
cost: 0
effects:
  - when: start_of_your_main_phase
    process:
      - select_own_permanent:
          bind_as: host
          optional: true
          filter: {kind: tamer}
          prompt: Place this Option under a Tamer
      - if:
          condition: {binding_present: host}
          then:
            - move_self_option_under_permanent: {target: host, face_down: true}
            - gain_memory: 1
"#;
fn option_case(first: bool, face_down: bool, decline: bool, field: bool, third: bool) {
    let y = OPTION.replace(
        "face_down: true",
        if face_down {
            "face_down: true"
        } else {
            "face_down: false"
        },
    );
    let mut r = runner(&y);
    let (o, t) = if first {
        let o = r.place_on_field(0, "Z-OPTION", None);
        (o, r.place_on_field(0, "TAM", None))
    } else {
        let t = r.place_on_field(0, "TAM", None);
        (r.place_on_field(0, "Z-OPTION", None), t)
    };
    if third {
        r.place_on_field(0, "TAM2", None);
    }
    r.game.players[0].battle_area[o.index as usize].option_state = OptionState::OrdinaryFieldOption;
    fire(&mut r, o, EffectTiming::StartOfYourMainPhase);
    let mut ids = vec![encode_attack(0, t.index as u16), PASS];
    if third {
        ids.push(encode_attack(0, 2));
    }
    mask(&r, &ids, true);
    // Change only lifecycle state after installing the prompt so the step,
    // rather than timing eligibility, proves the non-field no-op.
    if !field {
        r.game.players[0].battle_area[o.index as usize].option_state =
            OptionState::Linked { host: t };
    }
    pick(
        &mut r,
        if decline {
            PASS
        } else {
            encode_attack(0, t.index as u16)
        },
    );
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.players[0].trash.is_empty());
    let moved = field && !decline;
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2 + usize::from(third) - usize::from(moved)
    );
    let host = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| id(&r, p.top_card()) == "TAM")
        .unwrap();
    assert_eq!(host.card_sources.len(), if moved { 2 } else { 1 });
    if moved {
        assert_eq!(id(&r, &host.card_sources[0]), "Z-OPTION");
        assert_eq!(host.card_sources[0].face_down, face_down);
    }
    if third {
        assert_eq!(
            r.game.players[0]
                .battle_area
                .iter()
                .find(|p| id(&r, p.top_card()) == "TAM2")
                .unwrap()
                .card_sources
                .len(),
            1
        );
    }
    assert_eq!(r.memory(), if decline { 0 } else { 1 });
}
#[test]
fn field_option_moves_facedown_and_faceup() {
    for down in [true, false] {
        option_case(false, down, false, true, true);
    }
}
#[test]
fn field_option_before_host_preserves_identity_after_compaction() {
    for third in [false, true] {
        option_case(true, true, false, true, third);
    }
}
#[test]
fn field_option_pass_and_nonfield_source_do_not_move() {
    option_case(false, true, true, true, true);
    option_case(false, true, false, false, true);
}
const DELETE: &str = r#"
card: Z-DELETE
name: Batch deletion probe
kind: digimon
color: [purple]
level: 5
cost: 0
dp: 5000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent: {bind_as: a, filter: {kind: digimon, level_eq: 3}, prompt: Pick a level three}
      - select_opponent_permanent: {bind_as: b, filter: {kind: digimon, level_eq: 4}, prompt: Pick a level four}
      - delete_permanents: {targets: [a, b]}
"#;
const DELETE_OBSERVER: &str = r#"
card: Z-DELETE-OBS
name: Deletion witness
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_any_deletion
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_kind: digimon
    process:
      - gain_memory: 1
"#;
#[test]
fn delete_batch_waits_for_both_picks_and_fires_one_event_vs_two_steps() {
    for sequential in [false, true] {
        let y = if sequential {
            DELETE.replace(
                "      - delete_permanents: {targets: [a, b]}",
                "      - delete_permanent: {target: a}\n      - delete_permanent: {target: b}",
            )
        } else {
            DELETE.to_string()
        };
        let mut r = runner(&y);
        // Keep the observer before victims: isolate event multiplicity from
        // the separate queued-observer compaction contract.
        r.place_on_field(1, "Z-DELETE-OBS", None);
        r.place_on_field(1, "P2", None);
        r.place_on_field(1, "P3", None);
        r.place_on_field(1, "P4", None);
        r.place_on_field(1, "L4B", None);
        let h = r.place_on_field(0, "Z-DELETE", None);
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        mask(&r, &[encode_attack(0, 1), encode_attack(0, 2)], false);
        pick(&mut r, encode_attack(0, 1));
        assert_eq!(r.game.players[1].battle_area.len(), 5);
        assert!(r.game.players[1].trash.is_empty());
        mask(&r, &[encode_attack(0, 3), encode_attack(0, 4)], false);
        pick(&mut r, encode_attack(0, 3));
        if sequential {
            let pending = r
                .game
                .pending_selection
                .as_ref()
                .expect("two separate deletion events need trigger ordering");
            assert!(matches!(
                pending.kind,
                digimon_engine::selection::SelectionKind::TriggerOrder
            ));
            assert_eq!(pending.selecting_player, 1);
            assert_eq!(pending.valid_action_ids.len(), 2);
            assert!(!pending.is_optional);
            let action = pending.valid_action_ids[0];
            pick(&mut r, action);
        }
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[1].trash.len(), 2);
        assert_eq!(r.game.players[1].battle_area.len(), 3);
        assert_eq!(p0_memory(&r), if sequential { -2 } else { -1 });
    }
}
#[test]
fn delete_batch_ignores_unset_binding() {
    let mut r = runner(DELETE);
    r.place_on_field(1, "P2", None);
    let h = r.place_on_field(0, "Z-DELETE", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(&r, &[encode_attack(0, 0)], false);
    pick(&mut r, encode_attack(0, 0));
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[1].trash.len(), 1);
    assert!(r.game.players[1].battle_area.is_empty());
}
#[test]
fn delete_batch_collapses_duplicates_with_both_bindings_present() {
    let y = DELETE.replace("targets: [a, b]", "targets: [a, a, b]");
    let mut r = runner(&y);
    r.place_on_field(1, "P2", None);
    r.place_on_field(1, "P3", None);
    r.place_on_field(1, "P4", None);
    r.place_on_field(1, "L4B", None);
    let h = r.place_on_field(0, "Z-DELETE", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(&r, &[encode_attack(0, 0), encode_attack(0, 1)], false);
    pick(&mut r, encode_attack(0, 0));
    mask(&r, &[encode_attack(0, 2), encode_attack(0, 3)], false);
    pick(&mut r, encode_attack(0, 2));
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[1].trash.len(), 2);
    assert_eq!(r.game.players[1].battle_area.len(), 2);
}
#[test]
fn delete_batch_skips_a_bound_target_that_became_immune() {
    let mut r = runner(DELETE);
    let a = r.place_on_field(1, "P2", None);
    r.place_on_field(1, "P3", None);
    r.place_on_field(1, "P4", None);
    r.place_on_field(1, "L4B", None);
    r.place_on_field(1, "Z-DELETE-OBS", None);
    let h = r.place_on_field(0, "Z-DELETE", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(&r, &[encode_attack(0, 0), encode_attack(0, 1)], false);
    pick(&mut r, encode_attack(0, 0));
    mask(&r, &[encode_attack(0, 2), encode_attack(0, 3)], false);
    immune(&mut r, a);
    pick(&mut r, encode_attack(0, 2));
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[1].trash.len(), 1);
    assert_eq!(id(&r, &r.game.players[1].trash[0]), "P4");
    assert_eq!(id(&r, r.game.players[1].battle_area[0].top_card()), "P2");
    assert_eq!(p0_memory(&r), -1);
}
const RETURN: &str = r#"
card: Z-RETURN
name: Group return probe
kind: digimon
color: [blue]
level: 5
cost: 0
dp: 5000
effects:
  - when: when_digivolving
    process:
      - return_all_to_deck:
          over: {of: opponent, zone: [battle_area], kind: digimon, level_eq: 3}
          position: top
          include_sources: false
"#;
#[test]
fn group_return_filters_owner_kind_level_and_immunity_and_single_needs_no_order() {
    for matched in [false, true] {
        let mut r = runner(RETURN);
        r.place_on_field(0, "P2", None);
        r.place_on_field(1, "Q1", None);
        r.place_on_field(1, "TAM", None);
        let immune_h = r.place_on_field(1, "P3", None);
        immune(&mut r, immune_h);
        if matched {
            r.place_stack(1, &["Q1", "P2"]);
        }
        let before = r.game.players[1].deck.len();
        let h = r.place_on_field(0, "Z-RETURN", None);
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[1].battle_area.len(), 3);
        assert_eq!(r.game.players[0].battle_area.len(), 2);
        assert_eq!(r.game.players[1].deck.len(), before + usize::from(matched));
        assert_eq!(r.game.players[1].trash.len(), usize::from(matched));
        if matched {
            assert_eq!(id(&r, &r.game.players[1].trash[0]), "Q1");
        }
        if matched {
            assert_eq!(id(&r, r.game.players[1].deck.last().unwrap()), "P2");
        }
    }
}
#[test]
fn two_member_group_requires_mandatory_controller_order_at_either_end() {
    for position in ["top", "bottom"] {
        let y = RETURN.replace("position: top", &format!("position: {position}"));
        let mut r = runner(&y);
        r.place_on_field(1, "P2", None);
        r.place_on_field(1, "P3", None);
        let h = r.place_on_field(0, "Z-RETURN", None);
        fire(&mut r, h, EffectTiming::WhenDigivolving);
        mask(&r, &[encode_attack(1, 0), encode_attack(1, 1)], false);
        assert_eq!(r.game.players[1].battle_area.len(), 2);
        pick(&mut r, encode_attack(1, 1));
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.players[1].battle_area.is_empty());
        let d = &r.game.players[1].deck;
        if position == "top" {
            assert_eq!(id(&r, d.last().unwrap()), "P3");
            assert_eq!(id(&r, &d[d.len() - 2]), "P2");
        } else {
            assert_eq!(id(&r, &d[0]), "P3");
            assert_eq!(id(&r, &d[1]), "P2");
        }
    }
}
const RETURN_OBSERVER: &str = r#"
card: Z-RETURN-OBS
name: Return witness
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_leave_field
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_kind: digimon
    process:
      - gain_memory: 1
"#;
#[test]
fn simultaneous_return_observer_fires_once_for_two_departures() {
    let mut r = runner(RETURN);
    r.place_on_field(1, "P2", None);
    r.place_on_field(1, "P3", None);
    r.place_on_field(1, "Z-RETURN-OBS", None);
    let h = r.place_on_field(0, "Z-RETURN", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(&r, &[encode_attack(1, 0), encode_attack(1, 1)], false);
    pick(&mut r, encode_attack(1, 0));
    while r.game.pending_selection.is_some() {
        let s = r.game.pending_selection.as_ref().unwrap();
        assert!(!s.is_optional);
        assert!(matches!(
            s.kind,
            digimon_engine::selection::SelectionKind::TriggerOrder
        ));
        let a = s.valid_action_ids[0];
        pick(&mut r, a);
    }
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(
        p0_memory(&r),
        -1,
        "CR 4.3 15-5-2: one simultaneous return event"
    );
}

fn p0_memory(r: &DebugRunner) -> i16 {
    if r.game.memory_pair.0 == 0 {
        r.memory()
    } else {
        -r.memory()
    }
}
const RETURN_SEQUENCE: &str = r#"
card: Z-RETURN-SEQ
name: Sequential return probe
kind: digimon
color: [blue]
level: 5
cost: 0
dp: 5000
effects:
  - when: when_digivolving
    process:
      - return_all_to_deck:
          over: {of: opponent, zone: [battle_area], kind: digimon, level_eq: 3}
          position: top
      - return_all_to_deck:
          over: {of: opponent, zone: [battle_area], kind: digimon, level_eq: 4}
          position: top
"#;
#[test]
fn separate_return_steps_remain_two_observer_events() {
    let mut r = runner(RETURN_SEQUENCE);
    r.place_on_field(1, "Z-RETURN-OBS", None);
    r.place_on_field(1, "P2", None);
    r.place_on_field(1, "P4", None);
    let h = r.place_on_field(0, "Z-RETURN-SEQ", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    while let Some(s) = r.game.pending_selection.as_ref() {
        assert!(!s.is_optional);
        assert!(matches!(
            s.kind,
            digimon_engine::selection::SelectionKind::TriggerOrder
        ));
        let a = s.valid_action_ids[0];
        pick(&mut r, a);
    }
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(p0_memory(&r), -2);
}
#[test]
fn count_top_return_keeps_first_pick_topmost() {
    let y = COUNT.replace(
        "of: you, cards: picked",
        "of: you, cards: picked, destination: top",
    );
    let mut r = runner(&y);
    for c in ["P2", "P3", "P4"] {
        r.inject_trash(0, c);
    }
    let h = r.place_on_field(0, "Z-COUNT", None);
    fire(&mut r, h, EffectTiming::WhenDigivolving);
    mask(
        &r,
        &[
            TRASH_EFFECT_START,
            TRASH_EFFECT_START + 1,
            TRASH_EFFECT_START + 2,
            PASS,
        ],
        true,
    );
    pick(&mut r, TRASH_EFFECT_START + 1);
    mask(&r, &[TRASH_EFFECT_START, TRASH_EFFECT_START + 2], false);
    pick(&mut r, TRASH_EFFECT_START);
    assert!(r.game.pending_selection.is_none());
    let d = &r.game.players[0].deck;
    assert_eq!(id(&r, d.last().unwrap()), "P3");
    assert_eq!(id(&r, &d[d.len() - 2]), "P2");
    assert_eq!(r.game.players[0].trash.len(), 1);
}
