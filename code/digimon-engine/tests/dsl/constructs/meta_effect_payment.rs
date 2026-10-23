//! CR 4.3 §§4-2-2, 8-1-2-6, 8-1-3-1/3: the digivolving player
//! pays toward their opponent, chooses a distinct payable route, and keeps
//! the revealed card in its original zone when the payment cannot complete.

use digimon_engine::action::{
    build_action_mask,
    space::{HAND_EFFECT_START, PASS},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardSourceRef, CostDelta, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const REDUCER: &str = r#"
card: PAYMENT-REDUCER
name: Payment reducer
kind: tamer
color: [red]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    optional: true
    when_any_ally_digivolves_into: {card_number_is: PAYMENT-RESULT}
    amount: 1
    pay_cost:
      - suspend: {target: source}
"#;

const SECURITY_DRAW_WITNESS: &str = r#"
card: PAYMENT-RESULT
name: Security draw witness
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
alt_paths:
  - kind: digivolve
    from: {level_eq: 3, color_is: red}
    cost: 3
effects:
  - when: on_digivolve
    process:
      - select_hand:
          of: you
          filter: {card_number_is: PAYMENT-HOST}
          optional: true
          bind_as: drawn
          prompt: The digivolution draw is available during When Digivolving
      - trash_from_hand_by_index: {of: you, hand_index: drawn}
"#;

#[derive(Clone, Copy, Debug)]
enum SourceZone {
    Hand,
    Trash,
    Security,
}

fn fixture(
    owner: u8,
    turn_player: u8,
    memory: i16,
    costs: &[u16],
) -> (DebugRunner, PermanentHandle) {
    let mut host = make_test_card("PAYMENT-HOST", "Payment host");
    host.level = Some(3);
    let mut result = make_test_card("PAYMENT-RESULT", "Payment result");
    result.level = Some(4);
    result.evo_costs = costs
        .iter()
        .map(|&cost| EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: cost,
        })
        .collect();
    let mut r = DebugRunner::builder()
        .add_card(host)
        .add_card(result)
        .from_dsl_yaml(REDUCER)
        .unwrap()
        .deck(0, &["PAYMENT-HOST"; 12])
        .deck(1, &["PAYMENT-HOST"; 12])
        .start();
    r.skip_mulligan();
    r.set_first_player(turn_player);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(memory);
    let h = r.place_on_field(owner, "PAYMENT-HOST", None);
    (r, h)
}

fn source(r: &mut DebugRunner, owner: u8, zone: SourceZone) -> CardSourceRef {
    let index = r.add_to_hand(owner, "PAYMENT-RESULT");
    match zone {
        SourceZone::Hand => CardSourceRef::Hand(owner, index),
        SourceZone::Trash => {
            let card = r.game.players[owner as usize].hand.remove(index);
            let i = r.game.players[owner as usize].trash.len();
            r.game.players[owner as usize].trash.push(card);
            CardSourceRef::Trash(owner, i)
        }
        SourceZone::Security => {
            let card = r.game.players[owner as usize].hand.remove(index);
            let i = r.game.players[owner as usize].security.len();
            // Rejected security payment must also preserve face-up information.
            r.game.players[owner as usize]
                .face_up_security
                .insert(card.card_index);
            r.game.players[owner as usize].security.push(card);
            CardSourceRef::Security(owner, i)
        }
    }
}

fn evolve(r: &mut DebugRunner, owner: u8, from: CardSourceRef, h: PermanentHandle) -> bool {
    r.game.effect_initiated_digivolve_from_source(
        owner,
        from,
        h,
        CostDelta::Reduce(0),
        false,
        PlaySource::ByEffect,
    )
}

fn top(r: &DebugRunner, h: PermanentHandle) -> &str {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
}

fn commit_case(zone: SourceZone) {
    for owner in [0, 1] {
        for turn_player in [0, 1] {
            let (mut r, h) = fixture(owner, turn_player, 5, &[3]);
            let from = source(&mut r, owner, zone);
            let owner_deck = r.game.players[owner as usize].deck.len();
            let other_deck = r.game.players[(1 - owner) as usize].deck.len();
            assert!(
                evolve(&mut r, owner, from, h),
                "{zone:?}, owner={owner}, turn={turn_player}"
            );
            assert_eq!(top(&r, h), "PAYMENT-RESULT");
            assert_eq!(
                r.memory(),
                if owner == turn_player { 2 } else { 8 },
                "{zone:?}, owner={owner}, turn={turn_player}"
            );
            assert_eq!(r.game.players[owner as usize].deck.len(), owner_deck - 1);
            assert_eq!(r.game.players[(1 - owner) as usize].deck.len(), other_deck);
            assert!(r.game.pending_selection.is_none());
        }
    }
}

#[test]
fn effect_digivolve_hand_payment_uses_owner_on_either_turn() {
    commit_case(SourceZone::Hand);
}

#[test]
fn effect_digivolve_trash_payment_uses_owner_on_either_turn() {
    commit_case(SourceZone::Trash);
}

#[test]
fn effect_digivolve_security_payment_uses_owner_on_either_turn() {
    commit_case(SourceZone::Security);
}

#[test]
fn unaffordable_effect_payment_preserves_each_source_and_security_visibility() {
    for owner in [0, 1] {
        for turn_player in [0, 1] {
            for zone in [SourceZone::Hand, SourceZone::Trash, SourceZone::Security] {
                let memory = if owner == turn_player { -8 } else { 8 };
                let (mut r, h) = fixture(owner, turn_player, memory, &[3]);
                let from = source(&mut r, owner, zone);
                let original = r.game.clone();
                assert!(
                    !evolve(&mut r, owner, from, h),
                    "{zone:?}, owner={owner}, turn={turn_player}"
                );
                assert_eq!(top(&r, h), "PAYMENT-HOST");
                assert_eq!(r.memory(), memory);
                let before = &original.players[owner as usize];
                let after = &r.game.players[owner as usize];
                for (a, b) in [
                    (&after.hand, &before.hand),
                    (&after.trash, &before.trash),
                    (&after.security, &before.security),
                ] {
                    assert_eq!(
                        a.iter().map(|c| c.handle()).collect::<Vec<_>>(),
                        b.iter().map(|c| c.handle()).collect::<Vec<_>>()
                    );
                }
                assert_eq!(after.face_up_security, before.face_up_security);
                assert_eq!(after.deck.len(), before.deck.len());
                assert!(r.game.pending_selection.is_none());
            }
        }
    }
}

#[test]
fn effect_payment_can_reach_both_memory_boundaries_exactly() {
    for owner in [0, 1] {
        for turn_player in [0, 1] {
            for zone in [SourceZone::Hand, SourceZone::Trash, SourceZone::Security] {
                let active = owner == turn_player;
                let (mut r, h) = fixture(owner, turn_player, if active { -7 } else { 7 }, &[3]);
                let from = source(&mut r, owner, zone);
                assert!(evolve(&mut r, owner, from, h));
                assert_eq!(r.memory(), if active { -10 } else { 10 });
                assert_eq!(top(&r, h), "PAYMENT-RESULT");
            }
        }
    }
}

fn route_choices(r: &DebugRunner, owner: u8) -> Vec<u16> {
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("distinct payable costs remain a choice");
    assert_eq!(pending.kind, SelectionKind::EffectChoice);
    assert_eq!(pending.selecting_player, owner);
    let expected = vec![HAND_EFFECT_START, HAND_EFFECT_START + 1];
    assert_eq!(pending.valid_action_ids, expected);
    let mask = build_action_mask(&r.game, owner);
    assert_eq!(
        mask.iter()
            .enumerate()
            .filter_map(|(i, &v)| (v != 0.0).then_some(i as u16))
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(mask[PASS as usize], 0.0);
    assert!(build_action_mask(&r.game, 1 - owner)
        .iter()
        .all(|&v| v == 0.0));
    expected
}

#[test]
fn inactive_owner_selects_either_cost_and_cloned_prompt_preserves_payer() {
    for owner in [0, 1] {
        let (mut r, h) = fixture(owner, 1 - owner, 5, &[2, 4]);
        let from = source(&mut r, owner, SourceZone::Hand);
        assert!(!evolve(&mut r, owner, from, h));
        let choices = route_choices(&r, owner);
        assert_eq!(r.memory(), 5, "route search is speculative");
        let saved = r.game.clone();
        for (action, cost) in choices.into_iter().zip([2, 4]) {
            r.game = saved.clone();
            r.execute_action(owner, action).unwrap();
            assert_eq!(r.memory(), 5 + cost);
            assert_eq!(top(&r, h), "PAYMENT-RESULT");
            assert!(r.game.pending_selection.is_none());
        }
    }
}

#[test]
fn route_affordability_filters_against_the_payers_end_of_the_gauge() {
    for owner in [0, 1] {
        for turn_player in [0, 1] {
            let active = owner == turn_player;
            for headroom in [1, 2] {
                let memory = if active {
                    -10 + headroom
                } else {
                    10 - headroom
                };
                let (mut r, h) = fixture(owner, turn_player, memory, &[2, 4]);
                let from = source(&mut r, owner, SourceZone::Hand);
                let succeeded = evolve(&mut r, owner, from, h);
                assert!(
                    r.game.pending_selection.is_none(),
                    "unpayable routes must not be offered"
                );
                assert_eq!(succeeded, headroom == 2);
                assert_eq!(
                    top(&r, h),
                    if succeeded {
                        "PAYMENT-RESULT"
                    } else {
                        "PAYMENT-HOST"
                    }
                );
                assert_eq!(
                    r.memory(),
                    if succeeded {
                        if active {
                            -10
                        } else {
                            10
                        }
                    } else {
                        memory
                    }
                );
            }
        }
    }
}

#[test]
fn parked_route_rechecks_inactive_payment_before_taking_the_card() {
    for owner in [0, 1] {
        let (mut r, h) = fixture(owner, 1 - owner, 5, &[2, 4]);
        let from = source(&mut r, owner, SourceZone::Hand);
        assert!(!evolve(&mut r, owner, from, h));
        let action = route_choices(&r, owner)[0];
        let hand = r.game.players[owner as usize]
            .hand
            .iter()
            .map(|c| c.handle())
            .collect::<Vec<_>>();
        r.game.set_memory(9);
        r.execute_action(owner, action).unwrap();
        assert_eq!(r.memory(), 9);
        assert_eq!(top(&r, h), "PAYMENT-HOST");
        assert_eq!(
            r.game.players[owner as usize]
                .hand
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            hand
        );
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn inactive_route_probe_retains_payable_reduction_without_spending_it() {
    for owner in [0, 1] {
        let (mut r, h) = fixture(owner, 1 - owner, 7, &[2, 4]);
        let reducer = r.place_on_field(owner, "PAYMENT-REDUCER", None);
        let from = source(&mut r, owner, SourceZone::Hand);
        assert!(!evolve(&mut r, owner, from, h));
        let action = route_choices(&r, owner)[1];
        assert!(!r.game.players[owner as usize].battle_area[reducer.index as usize].is_suspended);
        assert_eq!(r.memory(), 7);
        r.execute_action(owner, action).unwrap();
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("optional reducer payment");
        assert_eq!(pending.selecting_player, owner);
        assert!(pending.is_optional);
        assert_eq!(pending.valid_action_ids, vec![HAND_EFFECT_START]);
        let mask = build_action_mask(&r.game, owner);
        assert_eq!(mask[HAND_EFFECT_START as usize], 1.0);
        assert_eq!(mask[PASS as usize], 1.0);
        let saved = r.game.clone();
        r.execute_action(owner, PASS).unwrap();
        assert_eq!(
            r.memory(),
            7,
            "declining leaves printed cost four unaffordable"
        );
        assert_eq!(top(&r, h), "PAYMENT-HOST");
        r.game = saved;
        r.execute_action(owner, HAND_EFFECT_START).unwrap();
        assert_eq!(r.memory(), 10, "owner pays four reduced to three");
        assert_eq!(top(&r, h), "PAYMENT-RESULT");
        assert!(r.game.players[owner as usize].battle_area[reducer.index as usize].is_suspended);
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn security_digivolution_draw_precedes_when_digivolving_and_empty_deck_still_evolves() {
    use digimon_engine::action::space::PLAY_HAND_START;
    for owner in [0, 1] {
        for has_deck in [true, false] {
            let mut r = DebugRunner::builder()
                .add_card(make_test_card("PAYMENT-HOST", "Payment host"))
                .from_dsl_yaml(SECURITY_DRAW_WITNESS)
                .unwrap()
                .deck(owner, if has_deck { &["PAYMENT-HOST"][..] } else { &[] })
                .deck(1 - owner, &["PAYMENT-HOST"; 12])
                .start();
            r.skip_mulligan();
            r.set_first_player(1 - owner);
            r.set_phase(GamePhase::Main);
            r.game.set_memory(5);
            let h = r.place_on_field(owner, "PAYMENT-HOST", None);
            let from = source(&mut r, owner, SourceZone::Security);
            assert!(evolve(&mut r, owner, from, h));
            assert_eq!(top(&r, h), "PAYMENT-RESULT");
            assert_eq!(r.memory(), 8);
            assert_eq!(r.game.players[owner as usize].deck.len(), 0);
            if has_deck {
                let pending = r
                    .game
                    .pending_selection
                    .as_ref()
                    .expect("WD sees the drawn card");
                assert_eq!(pending.selecting_player, owner);
                assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START]);
                assert_eq!(
                    pending.prompt,
                    "The digivolution draw is available during When Digivolving"
                );
                let mask = build_action_mask(&r.game, owner);
                assert_eq!(mask[PLAY_HAND_START as usize], 1.0);
                assert_eq!(mask[PASS as usize], 1.0);
                r.execute_action(owner, PLAY_HAND_START).unwrap();
                assert!(r.game.players[owner as usize].hand.is_empty());
                assert_eq!(r.game.players[owner as usize].trash.len(), 1);
                assert_eq!(
                    r.game.players[owner as usize].trash[0].card_id(&r.game.card_data),
                    "PAYMENT-HOST"
                );
            } else {
                assert!(r.game.pending_selection.is_none());
                assert!(r.game.players[owner as usize].hand.is_empty());
            }
        }
    }
}
