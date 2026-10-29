//! Hand-to-breeding DSL lowering through a real On Play clause.
//! CR 4.3 §§3-4-7-2/3 (one breeding permanent and explicit-zone effects),
//! 3-4-7-4/6 (breeding entry does not trigger On Play or battle-area observers),
//! 7-1-3 (card play), and 3-5-2 (hand order). Q6238/Q4663 pin the effect
//! controller, not the destination player, for play prohibitions.
//! The card-text idiom is BT20-053 Grademon; production-card fidelity is tested
//! separately. Fixed YAML constants intentionally expose the tested lowerings.
use digimon_engine::action::{
    build_action_mask,
    space::{MAX_FIELD_SLOTS, PASS, PLAY_HAND_START},
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::selection::SelectionKind;

const OWN: &str = r#"
card: BREED-SOURCE
name: Breeding Source
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          optional: true
          bind_as: chosen
          filter: { can_play_to_breeding_from_hand: true }
          prompt: Choose a physical hand card
      - if:
          condition: { binding_present: chosen }
          then:
            - play_to_breeding_from_hand: { of: you, hand_index: chosen }
"#;

const OWN_PARKED: &str = r#"
card: BREED-SOURCE
name: Breeding Source
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          optional: true
          bind_as: chosen
          filter: { can_play_to_breeding_from_hand: true }
          prompt: Choose a physical hand card
      - if:
          condition: { binding_present: chosen }
          then:
            - select_own_permanent:
                filter: { name_is: Breeding Source }
                bind_as: witness
                prompt: Continue after choosing the physical card
            - play_to_breeding_from_hand: { of: you, hand_index: chosen }
"#;

const OPPONENT: &str = r#"
card: BREED-SOURCE
name: Breeding Source
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - select_hand:
          of: opponent
          optional: true
          bind_as: chosen
          filter: { can_play_to_breeding_from_hand: true }
          prompt: Choose a physical hand card
      - if:
          condition: { binding_present: chosen }
          then:
            - play_to_breeding_from_hand: { of: opponent, hand_index: chosen }
"#;

const NOT_PLAYABLE: &str = r#"
card: BREED-SOURCE
name: Breeding Source
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          optional: true
          bind_as: chosen
          filter: { can_play_to_breeding_from_hand: false }
          prompt: Choose a physical hand card
      - if:
          condition: { binding_present: chosen }
          then:
            - play_to_breeding_from_hand: { of: you, hand_index: chosen }
"#;

fn fixture(park_after_choice: bool, opponent_hand: bool) -> DebugRunner {
    let source = if opponent_hand {
        OPPONENT
    } else if park_after_choice {
        OWN_PARKED
    } else {
        OWN
    };
    fixture_from_yaml(source)
}

fn fixture_from_yaml(source: &str) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(source)
        .expect("hand-to-breeding primitive must compile")
        .from_dsl_yaml(
            r#"
card: BREED-TARGET
name: Breeding Target
kind: digimon
color: [red]
level: 3
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - gain_memory: 7
"#,
        )
        .unwrap();
    for (id, kind) in [
        ("BREED-NO", CardKind::Option),
        ("BREED-TAMER", CardKind::Tamer),
        ("BREED-DUAL", CardKind::Dual),
        ("BREED-EGG", CardKind::DigiEgg),
    ] {
        let mut card = make_test_card(id, id);
        card.card_kind = kind;
        builder = builder.add_card(card);
    }
    let cards = [
        "BREED-NO",
        "BREED-TARGET",
        "BREED-TARGET",
        "BREED-TAMER",
        "BREED-DUAL",
        "BREED-EGG",
    ];
    let mut r = builder
        .hand(0, &cards)
        .hand(1, &cards)
        .deck(0, &["BREED-TARGET"; 12])
        .deck(1, &["BREED-TARGET"; 12])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r.add_to_hand(0, "BREED-SOURCE");
    r
}

fn start(r: &mut DebugRunner) {
    let source_index = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "BREED-SOURCE")
        .unwrap();
    let action = PLAY_HAND_START + source_index as u16;
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BREED-SOURCE"));
}

fn act(r: &mut DebugRunner, action: u16) {
    let v = r
        .pending_selection_view()
        .expect("real player-visible choice");
    assert_eq!(v.selecting_player, 0);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.execute_action(0, action).unwrap();
}

fn finish_parked(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("second DSL selection parks the chosen binding");
    assert_eq!(v.kind, SelectionKind::OwnField);
    let action = v.valid_action_ids[0];
    act(r, action);
    assert!(r.pending_selection().is_none());
}

fn breeding_handle(r: &DebugRunner, player: usize) -> Option<CardHandle> {
    r.game.players[player]
        .breeding_area
        .as_ref()
        .map(|p| p.top_card().handle())
}

fn prohibit(r: &mut DebugRunner, player: u8) {
    r.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1 - player,
        ),
    );
}

#[test]
fn breeding_dsl_accept_decline_and_cloned_optional_prompt_preserve_physical_card() {
    let mut base = fixture(false, false);
    start(&mut base);
    let view = base.pending_selection_view().unwrap();
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional);
    assert_eq!(view.valid_action_ids, vec![1, 2, 4]);
    for action in [1, 2, 4, PASS] {
        assert_eq!(build_action_mask(&base.game, 0)[action as usize], 1.0);
    }
    for action in [0, 3, 5] {
        assert_eq!(
            build_action_mask(&base.game, 0)[action],
            0.0,
            "Option/Tamer/DigiEgg excluded"
        );
    }
    for action in [1, 2, 4, PASS] {
        let mut r = DebugRunner::wrap(base.game.clone());
        let accept = action != PASS;
        let chosen = accept.then(|| r.game.players[0].hand[action as usize].handle());
        act(&mut r, action);
        assert_eq!(breeding_handle(&r, 0), chosen);
        assert_eq!(r.hand_size(0), if accept { 5 } else { 6 });
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(
            r.memory(),
            2,
            "only the source costs 1; breeding play is free and does not activate On Play +7"
        );
        assert!(r.pending_selection().is_none());
    }
    assert!(
        breeding_handle(&base, 0).is_none(),
        "forks do not mutate parked original"
    );
}

#[test]
fn breeding_dsl_dual_is_a_digimon_for_this_destination() {
    let mut r = fixture(false, false);
    let dual = r.game.players[0].hand[4].handle();
    start(&mut r);
    act(&mut r, 4);
    assert_eq!(breeding_handle(&r, 0), Some(dual));
    assert_eq!(r.memory(), 2);
}

#[test]
fn breeding_dsl_full_battle_area_does_not_block_empty_breeding() {
    let mut r = fixture(false, false);
    for _ in 1..MAX_FIELD_SLOTS {
        r.place_on_field(0, "BREED-TARGET", Some(0));
    }
    assert_eq!(r.battle_area_size(0), MAX_FIELD_SLOTS as usize - 1);
    let chosen = r.game.players[0].hand[1].handle();
    start(&mut r);
    act(&mut r, 1);
    assert_eq!(breeding_handle(&r, 0), Some(chosen));
    assert_eq!(r.battle_area_size(0), MAX_FIELD_SLOTS as usize);
}

#[test]
fn breeding_dsl_occupied_area_or_effect_prohibition_offer_no_illegal_choice() {
    for occupied in [true, false] {
        let mut r = fixture(false, false);
        if occupied {
            r.place_in_breeding(0, "BREED-TARGET");
        } else {
            prohibit(&mut r, 0);
        }
        let before = breeding_handle(&r, 0);
        start(&mut r);
        assert!(
            r.pending_selection().is_none(),
            "no selectable candidates means no prompt"
        );
        assert_eq!(breeding_handle(&r, 0), before);
        assert_eq!(r.hand_size(0), 6);
    }
}

#[test]
fn breeding_dsl_predicate_rejects_foreign_physical_owner_even_in_own_hand() {
    let mut r = fixture(false, false);
    r.game.players[0]
        .hand
        .retain(|c| c.card_id(&r.game.card_data) == "BREED-SOURCE");
    let foreign = r.game.players[1].hand.remove(1);
    r.game.players[0].hand.push(foreign);
    start(&mut r);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
    assert!(breeding_handle(&r, 0).is_none());
}

#[test]
fn breeding_dsl_chosen_instance_survives_hand_index_shift_and_parked_clone() {
    let mut base = fixture(true, false);
    let chosen = base.game.players[0].hand[1].handle();
    start(&mut base);
    act(&mut base, 1);
    for mut r in [DebugRunner::wrap(base.game.clone()), base] {
        let before = r.game.players[0].hand.remove(0);
        r.game.players[0].trash.push(before);
        finish_parked(&mut r);
        assert_eq!(
            breeding_handle(&r, 0),
            Some(chosen),
            "index shift must not choose the same-ID copy"
        );
        assert_eq!(r.memory(), 2);
    }
}

#[test]
fn breeding_dsl_removed_choice_never_substitutes_an_identical_copy() {
    let mut r = fixture(true, false);
    let chosen = r.game.players[0].hand[1].handle();
    let spare = r.game.players[0].hand[2].handle();
    start(&mut r);
    act(&mut r, 1);
    let removed = r.game.players[0].hand.remove(1);
    r.game.players[0].trash.push(removed);
    finish_parked(&mut r);
    assert!(breeding_handle(&r, 0).is_none());
    assert!(r.game.players[0].hand.iter().any(|c| c.handle() == spare));
    assert!(r.game.players[0].trash.iter().any(|c| c.handle() == chosen));
}

#[test]
fn breeding_dsl_resolution_rechecks_new_occupation_and_new_prohibition() {
    for occupied in [true, false] {
        let mut r = fixture(true, false);
        let chosen = r.game.players[0].hand[1].handle();
        start(&mut r);
        act(&mut r, 1);
        if occupied {
            r.place_in_breeding(0, "BREED-TARGET");
        } else {
            prohibit(&mut r, 0);
        }
        let before = breeding_handle(&r, 0);
        finish_parked(&mut r);
        assert_eq!(breeding_handle(&r, 0), before);
        assert!(r.game.players[0].hand.iter().any(|c| c.handle() == chosen));
    }
}

#[test]
fn breeding_dsl_q4663_controller_not_destination_owns_the_effect_play_gate() {
    for blocked_player in [0, 1] {
        let mut r = fixture(false, true);
        prohibit(&mut r, blocked_player);
        let chosen = r.game.players[1].hand[1].handle();
        start(&mut r);
        if blocked_player == 0 {
            assert!(
                r.pending_selection().is_none(),
                "prohibited effect controller has no legal candidate"
            );
            assert!(breeding_handle(&r, 1).is_none());
        } else {
            let mut clone = DebugRunner::wrap(r.game.clone());
            act(&mut clone, 1);
            act(&mut r, 1);
            assert_eq!(breeding_handle(&clone, 1), Some(chosen));
            assert_eq!(breeding_handle(&r, 1), Some(chosen));
            assert!(breeding_handle(&r, 0).is_none());
        }
    }
}

#[test]
fn breeding_dsl_false_predicate_inverts_kind_masks_and_does_not_bypass_play_legality() {
    let mut base = fixture_from_yaml(NOT_PLAYABLE);
    start(&mut base);
    let view = base
        .pending_selection_view()
        .expect("non-playable cards are selectable by the false predicate");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(view.valid_action_ids, vec![0, 3, 5]);
    assert!(view.is_optional);
    let mask = build_action_mask(&base.game, 0);
    for action in [0, 3, 5, PASS] {
        assert_eq!(mask[action as usize], 1.0);
    }
    for action in [1, 2, 4] {
        assert_eq!(
            mask[action], 0.0,
            "playable Digimon and DUAL fail the false predicate"
        );
    }
    let before: Vec<_> = base.game.players[0]
        .hand
        .iter()
        .map(|c| c.handle())
        .collect();
    for action in [0, 3, 5, PASS] {
        let mut r = DebugRunner::wrap(base.game.clone());
        act(&mut r, action);
        assert!(breeding_handle(&r, 0).is_none());
        assert_eq!(
            r.game.players[0]
                .hand
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(r.memory(), 2);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn breeding_dsl_false_predicate_sees_occupation_and_controller_prohibition() {
    for occupied in [true, false] {
        let mut r = fixture_from_yaml(NOT_PLAYABLE);
        if occupied {
            r.place_in_breeding(0, "BREED-TARGET");
        } else {
            prohibit(&mut r, 0);
        }
        let before = breeding_handle(&r, 0);
        let chosen = r.game.players[0].hand[1].handle();
        start(&mut r);
        let view = r
            .pending_selection_view()
            .expect("all hand cards now fail breeding playability");
        assert_eq!(view.valid_action_ids, vec![0, 1, 2, 3, 4, 5]);
        act(&mut r, 1);
        assert_eq!(breeding_handle(&r, 0), before);
        assert!(r.game.players[0].hand.iter().any(|c| c.handle() == chosen));
        assert_eq!(r.hand_size(0), 6);
        assert_eq!(r.memory(), 2);
        assert!(r.pending_selection().is_none());
    }
}
