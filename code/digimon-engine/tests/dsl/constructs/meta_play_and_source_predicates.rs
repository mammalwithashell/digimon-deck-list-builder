//! Reusable eligibility/identity leaves used by BT20-051, BT20-095 and EX5-070.
//! Free-play eligibility separates the physical destination from the causing
//! controller (Q4663/Q6238). `is_source_permanent` identifies the whole carrier,
//! including an inherited host, rather than its card name or a same-index foe.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, MAX_FIELD_SLOTS, PASS, REPLACEMENT_ACCEPT, TRASH_EFFECT_START},
    },
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardKind, Expiry, GamePhase, ModifierType},
    modifiers::PlayerModifierEntry,
    permanent::PermanentHandle,
    replacement::ReplacementCause,
    selection::SelectionKind,
};

const FREE_HAND: &str = r#"
card: ELIGIBILITY-SOURCE
name: Eligibility source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: chosen
          optional: true
          filter:
            all_of:
              - any_of: [{kind: digimon}, {kind: tamer}]
              - can_play_from_effect_without_cost: true
          prompt: Choose an eligible free play
      - play_from_hand_free: {of: you, hand_index: chosen}
"#;

const FREE_UNION: &str = r#"
card: ELIGIBILITY-SOURCE
name: Eligibility source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: chosen
          optional: true
          cost: false
          filter:
            all_of:
              - any_of: [{kind: digimon}, {kind: tamer}]
              - can_play_from_effect_without_cost: true
          prompt: Choose an eligible free play from hand or trash
      - if:
          condition: {binding_present: chosen}
          then:
            - play_union_bound_free: {binding: chosen}
"#;

const SOURCE_PICK: &str = r#"
card: IDENTITY-SOURCE
name: Identical carrier name
kind: digimon
color: [red]
level: 3
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - select_any_permanent:
          bind_as: selected
          filter: {is_source_permanent: true}
          prompt: Choose the effect carrier
      - suspend: {target: selected}
"#;

const INHERITED_LOCAL: &str = r#"
card: IDENTITY-INHERITED
name: Local inherited guard
kind: digimon
color: [red]
level: 3
cost: 0
dp: 2000
effects:
  - scope: inherited
    kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when: {kind: digimon, is_source_permanent: true}
    process:
      - cancel_replacement: {}
"#;

fn fixture(yaml: &str, owner: u8) -> DebugRunner {
    let mut builder = DebugRunner::builder().from_dsl_yaml(yaml).unwrap();
    for (id, kind) in [
        ("ELIGIBLE-D", CardKind::Digimon),
        ("ELIGIBLE-T", CardKind::Tamer),
        ("EXCLUDED-O", CardKind::Option),
    ] {
        let mut card = make_test_card(id, id);
        card.card_kind = kind;
        card.play_cost = 8;
        if kind == CardKind::Digimon {
            card.level = Some(3);
            card.dp = Some(2000);
        } else {
            card.level = None;
            card.dp = None;
        }
        builder = builder.add_card(card);
    }
    let mut r = builder
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["ELIGIBLE-D"; 20])
        .deck(1, &["ELIGIBLE-D"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.set_phase(GamePhase::Main);
    r
}

fn forbid(r: &mut DebugRunner, player: u8, kind: ModifierType) {
    r.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(kind, 0, Expiry::Permanent, None, 1 - player),
    );
}

fn mask(r: &DebugRunner, actor: u8, expected: &[u16], optional: bool) {
    let pending = r.game.pending_selection.as_ref().expect("real DSL choice");
    assert_eq!(pending.selecting_player, actor);
    assert_eq!(pending.is_optional, optional);
    assert_eq!(pending.valid_action_ids, expected);
    for (action, enabled) in build_action_mask(&r.game, actor).iter().enumerate() {
        assert_eq!(
            *enabled,
            if expected.contains(&(action as u16)) || (optional && action == PASS as usize) {
                1.0
            } else {
                0.0
            },
            "action {action}"
        );
    }
    assert!(build_action_mask(&r.game, 1 - actor)
        .iter()
        .all(|v| *v == 0.0));
}

fn choose(r: &mut DebugRunner, owner: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.execute_action(owner, action).unwrap();
}

fn free_fixture(yaml: &str, owner: u8, destination: u8) -> (DebugRunner, PermanentHandle) {
    let mut r = fixture(yaml, owner);
    for id in ["ELIGIBLE-D", "ELIGIBLE-D", "ELIGIBLE-T", "EXCLUDED-O"] {
        r.add_to_hand(destination, id);
    }
    let source = r.place_on_field(owner, "ELIGIBILITY-SOURCE", None);
    (r, source)
}

fn start(r: &mut DebugRunner, source: PermanentHandle) {
    r.fire_on_play(source.player, source.index as usize);
}
fn contains(r: &DebugRunner, owner: u8, card: CardHandle) -> bool {
    r.game.players[owner as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().handle() == card)
}
fn fill(r: &mut DebugRunner, owner: u8) {
    while r.battle_area_size(owner) < MAX_FIELD_SLOTS as usize {
        r.place_on_field(owner, "ELIGIBLE-D", None);
    }
}

#[test]
fn free_play_hand_mask_accepts_both_kinds_and_distinct_physical_duplicates_for_both_owners() {
    for owner in [0, 1] {
        let (mut base, source) = free_fixture(FREE_HAND, owner, owner);
        let cards: Vec<_> = base.game.players[owner as usize]
            .hand
            .iter()
            .map(|c| c.handle())
            .collect();
        start(&mut base, source);
        assert_eq!(
            base.game.pending_selection.as_ref().unwrap().kind,
            SelectionKind::Hand
        );
        mask(&base, owner, &[0, 1, 2], true);
        for action in [0, 1, 2, PASS] {
            let mut r = DebugRunner::wrap(base.game.clone());
            let memory = r.memory();
            choose(&mut r, owner, action);
            for (i, card) in cards.iter().enumerate() {
                assert_eq!(contains(&r, owner, *card), action == i as u16);
            }
            assert_eq!(r.hand_size(owner), if action == PASS { 4 } else { 3 });
            assert_eq!(r.memory(), memory, "printed cost 8 is not paid");
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn free_play_prohibitions_discriminate_digimon_and_tamer_for_both_owners() {
    for owner in [0, 1] {
        for (kind, expected) in [
            (ModifierType::CannotPlayDigimonByEffect, vec![2]),
            (ModifierType::CannotPlayTamerByEffect, vec![0, 1]),
        ] {
            let (mut r, source) = free_fixture(FREE_HAND, owner, owner);
            forbid(&mut r, owner, kind);
            start(&mut r, source);
            mask(&r, owner, &expected, true);
            let action = expected[0];
            let selected = r.game.players[owner as usize].hand[action as usize].handle();
            choose(&mut r, owner, action);
            assert!(contains(&r, owner, selected));
        }
    }
}

#[test]
fn free_play_cross_owner_uses_causing_controller_for_prohibitions() {
    let yaml = FREE_HAND.replace("of: you", "of: opponent");
    for owner in [0, 1] {
        for prohibited in [owner, 1 - owner] {
            let (mut r, source) = free_fixture(&yaml, owner, 1 - owner);
            forbid(&mut r, prohibited, ModifierType::CannotPlayDigimonByEffect);
            forbid(&mut r, prohibited, ModifierType::CannotPlayTamerByEffect);
            let selected = r.game.players[(1 - owner) as usize].hand[1].handle();
            start(&mut r, source);
            if prohibited == owner {
                assert!(r.pending_selection().is_none());
                assert_eq!(r.hand_size(1 - owner), 4);
            } else {
                mask(&r, owner, &[0, 1, 2], true);
                choose(&mut r, owner, 1);
                assert!(contains(&r, 1 - owner, selected));
                assert!(!contains(&r, owner, selected));
            }
        }
    }
}

#[test]
fn free_play_capacity_checks_destination_board_not_effect_controllers_board() {
    let yaml = FREE_HAND.replace("of: you", "of: opponent");
    for owner in [0, 1] {
        for full in [owner, 1 - owner] {
            let (mut r, source) = free_fixture(&yaml, owner, 1 - owner);
            fill(&mut r, full);
            let selected = r.game.players[(1 - owner) as usize].hand[0].handle();
            start(&mut r, source);
            if full == 1 - owner {
                assert!(r.pending_selection().is_none());
                assert_eq!(r.hand_size(1 - owner), 4);
            } else {
                mask(&r, owner, &[0, 1, 2], true);
                choose(&mut r, owner, 0);
                assert!(contains(&r, 1 - owner, selected));
            }
        }
    }
}

#[test]
fn free_play_union_filters_hand_and_trash_and_preserves_the_chosen_instance() {
    for owner in [0, 1] {
        let (mut base, source) = free_fixture(FREE_UNION, owner, owner);
        base.inject_trash(owner, "ELIGIBLE-D");
        base.inject_trash(owner, "ELIGIBLE-T");
        base.inject_trash(owner, "EXCLUDED-O");
        start(&mut base, source);
        let expected = [0, 1, 2, TRASH_EFFECT_START, TRASH_EFFECT_START + 1];
        mask(&base, owner, &expected, true);
        for action in expected {
            let mut r = DebugRunner::wrap(base.game.clone());
            let selected = if action < TRASH_EFFECT_START {
                r.game.players[owner as usize].hand[action as usize].handle()
            } else {
                r.game.players[owner as usize].trash[(action - TRASH_EFFECT_START) as usize]
                    .handle()
            };
            let memory = r.memory();
            choose(&mut r, owner, action);
            assert!(contains(&r, owner, selected));
            assert!(!r.game.players[owner as usize]
                .hand
                .iter()
                .chain(r.game.players[owner as usize].trash.iter())
                .any(|c| c.handle() == selected));
            assert_eq!(r.memory(), memory);
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn free_play_false_polarity_selects_blocked_cards_without_bypassing_commit_legality() {
    let yaml = FREE_HAND.replace(
        "can_play_from_effect_without_cost: true",
        "can_play_from_effect_without_cost: false",
    );
    for owner in [0, 1] {
        let (mut r, source) = free_fixture(&yaml, owner, owner);
        forbid(&mut r, owner, ModifierType::CannotPlayDigimonByEffect);
        let selected = r.game.players[owner as usize].hand[1].handle();
        start(&mut r, source);
        mask(&r, owner, &[0, 1], true);
        choose(&mut r, owner, 1);
        assert!(!contains(&r, owner, selected));
        assert_eq!(r.hand_size(owner), 4);
        assert!(r.game.players[owner as usize]
            .hand
            .iter()
            .any(|c| c.handle() == selected));
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn source_permanent_true_and_false_masks_compare_owner_and_slot_not_card_name() {
    for owner in [0, 1] {
        for want in [true, false] {
            let yaml = SOURCE_PICK.replace(
                "is_source_permanent: true",
                &format!("is_source_permanent: {want}"),
            );
            let mut base = fixture(&yaml, owner);
            for player in [0, 1] {
                for _ in 0..2 {
                    base.place_on_field(player, "IDENTITY-SOURCE", None);
                }
            }
            let source = PermanentHandle {
                player: owner,
                index: 1,
            };
            start(&mut base, source);
            let expected: Vec<_> = (0..2)
                .flat_map(|player| (0..2).map(move |index| (player, index)))
                .filter(|&(player, index)| ((player, index) == (owner, 1)) == want)
                .map(|(player, index)| encode_attack(player as u16, index as u16))
                .collect();
            mask(&base, owner, &expected, false);
            assert_eq!(
                base.game.pending_selection.as_ref().unwrap().kind,
                SelectionKind::AnyField
            );
            for action in expected {
                let mut r = DebugRunner::wrap(base.game.clone());
                choose(&mut r, owner, action);
                for player in [0, 1] {
                    for index in 0..2 {
                        assert_eq!(
                            r.game.players[player].battle_area[index].is_suspended,
                            action == encode_attack(player as u16, index as u16)
                        );
                    }
                }
                assert!(r.pending_selection().is_none());
            }
        }
    }
}

#[test]
fn inherited_source_permanent_guard_matches_only_its_carrier_and_exposes_accept_decline() {
    for owner in [0, 1] {
        for victim in [0, 1, 2] {
            let mut base = fixture(INHERITED_LOCAL, owner);
            let host = base.place_stack(owner, &["IDENTITY-INHERITED", "ELIGIBLE-D"]);
            let sibling = base.place_on_field(owner, "ELIGIBLE-D", None);
            let foe = base.place_on_field(1 - owner, "ELIGIBLE-D", None);
            let target = [host, sibling, foe][victim];
            let physical = base.game.players[target.player as usize].battle_area
                [target.index as usize]
                .top_card()
                .handle();
            base.game
                .delete_permanent_with_cause(target, ReplacementCause::OpponentEffect);
            if victim != 0 {
                assert!(
                    base.pending_selection().is_none(),
                    "another same-name/index permanent is not the inherited host"
                );
                assert!(!contains(&base, target.player, physical));
                assert_eq!(
                    base.game.players[owner as usize].battle_area[0]
                        .card_sources
                        .len(),
                    2
                );
                continue;
            }
            assert_eq!(
                base.game.pending_selection.as_ref().unwrap().kind,
                SelectionKind::Replacement
            );
            mask(&base, owner, &[REPLACEMENT_ACCEPT], true);
            for action in [REPLACEMENT_ACCEPT, PASS] {
                let mut r = DebugRunner::wrap(base.game.clone());
                choose(&mut r, owner, action);
                assert_eq!(contains(&r, owner, physical), action == REPLACEMENT_ACCEPT);
                assert!(r.pending_selection().is_none());
            }
        }
    }
}
