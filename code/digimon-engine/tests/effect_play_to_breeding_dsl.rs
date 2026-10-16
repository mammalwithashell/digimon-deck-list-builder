//! Reusable hand-to-breeding DSL primitive, including Q6238/Q4663 controller scope.
//! Forced play-event fixtures execute real compiled DSL; no production card stubs.
use digimon_engine::action::{build_action_mask, space::{PASS, MAX_FIELD_SLOTS}};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::selection::SelectionKind;

fn fixture(park_after_choice: bool, opponent_hand: bool) -> DebugRunner {
    let destination = if opponent_hand { "opponent" } else { "you" };
    let pause = if park_after_choice {
        "            - select_own_permanent:\n                filter: { name_is: Breeding Source }\n                bind_as: witness\n                prompt: Continue after choosing the physical card\n"
    } else { "" };
    let source = format!(r#"
card: BREED-SOURCE
name: Breeding Source
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_play
    process:
      - select_hand:
          of: {destination}
          optional: true
          bind_as: chosen
          filter: {{ can_play_to_breeding_from_hand: true }}
          prompt: Choose a physical hand Digimon for breeding
      - if:
          condition: {{ binding_present: chosen }}
          then:
{pause}            - play_to_breeding_from_hand: {{ of: {destination}, hand_index: chosen }}
"#);
    let mut builder = DebugRunner::builder()
        .from_dsl_yaml(&source).expect("hand-to-breeding primitive must compile")
        .from_dsl_yaml(r#"
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
"#).unwrap();
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
    let cards = ["BREED-NO", "BREED-TARGET", "BREED-TARGET", "BREED-TAMER", "BREED-DUAL", "BREED-EGG"];
    let mut r = builder.hand(0, &cards).hand(1, &cards)
        .deck(0, &["BREED-TARGET"; 12]).deck(1, &["BREED-TARGET"; 12])
        .memory(3).start();
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "BREED-SOURCE", Some(0));
    r
}

fn start(r: &mut DebugRunner) {
    r.fire_play_event_triggers(0, 0, true, false);
}

fn act(r: &mut DebugRunner, action: u16) {
    let v = r.pending_selection_view().expect("real player-visible choice");
    assert_eq!(v.selecting_player, 0);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.execute_action(0, action).unwrap();
}

fn finish_parked(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("second DSL selection parks the chosen binding");
    assert_eq!(v.kind, SelectionKind::OwnField);
    let action = v.valid_action_ids[0];
    act(r, action);
    assert!(r.pending_selection().is_none());
}

fn breeding_handle(r: &DebugRunner, player: usize) -> Option<CardHandle> {
    r.game.players[player].breeding_area.as_ref().map(|p| p.top_card().handle())
}

fn prohibit(r: &mut DebugRunner, player: u8) {
    r.game.modifiers.add_player_modifier(player, PlayerModifierEntry::simple(
        ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1 - player,
    ));
}

#[test]
fn breeding_dsl_accept_decline_and_cloned_optional_prompt_preserve_physical_card() {
    let mut base = fixture(false, false);
    let chosen = base.game.players[0].hand[1].handle();
    start(&mut base);
    let view = base.pending_selection_view().unwrap();
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional);
    for action in [1, 2, 4, PASS] {
        assert_eq!(build_action_mask(&base.game, 0)[action as usize], 1.0);
    }
    for action in [0, 3, 5] {
        assert_eq!(build_action_mask(&base.game, 0)[action], 0.0, "Option/Tamer/DigiEgg excluded");
    }
    for accept in [true, false] {
        let mut r = DebugRunner::wrap(base.game.clone());
        act(&mut r, if accept { 1 } else { PASS });
        assert_eq!(breeding_handle(&r, 0), accept.then_some(chosen));
        assert_eq!(r.hand_size(0), if accept { 5 } else { 6 });
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.memory(), 3, "breeding play is free and does not activate On Play +7");
        assert!(r.pending_selection().is_none());
    }
    assert!(breeding_handle(&base, 0).is_none(), "forks do not mutate parked original");
}

#[test]
fn breeding_dsl_dual_is_a_digimon_for_this_destination() {
    let mut r = fixture(false, false);
    let dual = r.game.players[0].hand[4].handle();
    start(&mut r);
    act(&mut r, 4);
    assert_eq!(breeding_handle(&r, 0), Some(dual));
    assert_eq!(r.memory(), 3);
}

#[test]
fn breeding_dsl_full_battle_area_does_not_block_empty_breeding() {
    let mut r = fixture(false, false);
    for _ in 1..MAX_FIELD_SLOTS { r.place_on_field(0, "BREED-TARGET", Some(0)); }
    assert_eq!(r.battle_area_size(0), MAX_FIELD_SLOTS as usize);
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
        if occupied { r.place_in_breeding(0, "BREED-TARGET"); } else { prohibit(&mut r, 0); }
        let before = breeding_handle(&r, 0);
        start(&mut r);
        assert!(r.pending_selection().is_none(), "no selectable candidates means no prompt");
        assert_eq!(breeding_handle(&r, 0), before);
        assert_eq!(r.hand_size(0), 6);
    }
}

#[test]
fn breeding_dsl_predicate_rejects_foreign_physical_owner_even_in_own_hand() {
    let mut r = fixture(false, false);
    r.game.players[0].hand.clear();
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
        assert_eq!(breeding_handle(&r, 0), Some(chosen), "index shift must not choose the same-ID copy");
        assert_eq!(r.memory(), 3);
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
        if occupied { r.place_in_breeding(0, "BREED-TARGET"); } else { prohibit(&mut r, 0); }
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
            assert!(r.pending_selection().is_none(), "prohibited effect controller has no legal candidate");
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
