//! BT24-098 Invasion of the Titans — Option, Purple, cost 3, Titan/TS.
//!
//! [Main] <Draw 2> and trash 2 cards in your hand. Then, place this card in
//! the battle area.
//! [Your Turn] When any of your [Titan] trait Digimon are played, <Delay>.
//! ・If your opponent has 5 or more memory, you may play 1 level 5 or lower
//!   Digimon card with the [Titan] trait from your trash without paying the
//!   cost.
//! [Security] You may play 1 level 4 or lower [Titan] trait Digimon card from
//! your hand or trash without paying the cost. Then, add this card to the hand.
//!
//! DCGO: BT24/Purple/BT24_098.cs.
#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{self, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, DelayTrigger, EffectTiming, Keyword, ModifierType,
};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-098";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-098")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "PURPLE",
            "Purple Guy",
            CardColor::Purple,
            3,
            3,
            &[],
        ))
        .add_card(digimon(
            "TITAN-PLAY",
            "Titan Play",
            CardColor::Purple,
            5,
            7,
            &["Titan"],
        ))
        .add_card(digimon(
            "PLAIN-PLAY",
            "Plain Play",
            CardColor::Purple,
            5,
            7,
            &["TS"],
        ))
        .add_card(digimon(
            "TITAN4",
            "Titan Four",
            CardColor::Purple,
            4,
            5,
            &["Titan"],
        ))
        .add_card(digimon(
            "TITAN5",
            "Titan Five",
            CardColor::Purple,
            5,
            7,
            &["Titan"],
        ))
        .add_card(digimon(
            "TITAN6",
            "Titan Six",
            CardColor::Purple,
            6,
            9,
            &["Titan"],
        ))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
}

fn setup() -> DebugRunner {
    let mut r = builder().memory(10).start();
    r.set_first_player(0);
    r
}

fn delayed_state(r: &DebugRunner) -> Option<OptionState> {
    r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|p| p.option_state)
}

/// Use the Option ([Main]) from hand, answering the two trash prompts with
/// the first two FILLER cards, leaving it parked as a Delayed Option.
fn use_main(r: &mut DebugRunner) {
    r.place_on_field(0, "PURPLE", Some(0));
    r.game.enter_main_phase();
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == CARD_ID)
        .unwrap();
    let res = r.game.play_option_from_hand(0, idx);
    assert!(
        !matches!(res, OptionPlayResult::Invalid),
        "usable with a purple Digimon"
    );
    for _ in 0..2 {
        let v = r.pending_selection_view().expect("mandatory hand trash");
        assert!(!v.is_optional, "trash 2 is not optional");
        pick_hand(r, 0, "FILLER");
    }
    let _ = r.auto_resolve();
}

fn advance_to_next_own_main(r: &mut DebugRunner) {
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
}

fn play_from_hand_by_id(r: &mut DebugRunner, card_id: &str) {
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap();
    assert!(r.play(0, idx).is_some(), "{card_id} playable");
}

#[test]
fn bt24_098_structure() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.cost, Some(3));
    assert!(c.traits.contains(&"Titan".to_string()) && c.traits.contains(&"TS".to_string()));
    let main = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main]");
    assert!(!main.optional);
    assert_eq!(
        main.process.last(),
        Some(&CompiledStep::PlaceSelfAsDelayOption)
    );
    let delay = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
                Some(trigger)
            }
            _ => None,
        })
        .expect("<Delay>");
    assert_eq!(*delay, CompiledTiming::OnAllyPlayed);
    let sec = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("[Security]");
    assert_eq!(sec.scope, CompiledScope::Inherited);
    assert_eq!(sec.process.last(), Some(&CompiledStep::AddThisOptionToHand));
}

#[test]
fn bt24_098_main_draws_two_trashes_two_and_parks() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    let deck_before = r.deck_size(0);
    use_main(&mut r);
    assert_eq!(r.deck_size(0), deck_before - 2, "<Draw 2>");
    assert_eq!(r.hand_size(0), 2, "2 + 2 drawn − 2 trashed");
    assert_eq!(r.game.players[0].trash.len(), 2);
    assert!(
        matches!(
            delayed_state(&r),
            Some(OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed),
                ..
            })
        ),
        "placed in the battle area as an event-gated Delay"
    );
    assert_eq!(r.memory(), 7, "use cost 3");
}

#[test]
fn bt24_098_delay_plays_titan_from_trash_when_opponent_has_five() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "TITAN-PLAY");
    use_main(&mut r);
    push_trash(&mut r, 0, "TITAN4");
    push_trash(&mut r, 0, "TITAN5");
    push_trash(&mut r, 0, "TITAN6");
    advance_to_next_own_main(&mut r);
    r.game.memory = 2;
    play_from_hand_by_id(&mut r, "TITAN-PLAY");
    assert_eq!(r.memory(), -5, "opponent now has 5 memory");
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional(), "<Delay> is optional");
    r.execute_action(v.selecting_player, REPLACEMENT_ACCEPT)
        .unwrap();
    let v = r.pending_selection_view().expect("trash play prompt");
    assert!(v.is_optional, "'you may play'");
    assert!(trash_pickable(&r, 0, "TITAN5"));
    assert!(trash_pickable(&r, 0, "TITAN4"));
    assert!(!trash_pickable(&r, 0, "TITAN6"), "level 5 or lower");
    assert!(
        !trash_pickable(&r, 0, CARD_ID),
        "the Delay card itself is not a Digimon"
    );
    pick_trash(&mut r, 0, "TITAN5");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TITAN5".to_string()));
    assert!(
        trash_ids(&r, 0).contains(&CARD_ID.to_string()),
        "Delay trashes this card"
    );
    // The turn passes once memory sits on the opponent's side, so read the
    // gauge from player 0's perspective.
    let p0_memory = if r.game.turn_player() == 0 {
        r.memory()
    } else {
        -r.memory()
    };
    assert_eq!(p0_memory, -5, "without paying the cost");
}

#[test]
fn bt24_098_delay_play_is_optional() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "TITAN-PLAY");
    use_main(&mut r);
    push_trash(&mut r, 0, "TITAN5");
    advance_to_next_own_main(&mut r);
    r.game.memory = 2;
    play_from_hand_by_id(&mut r, "TITAN-PLAY");
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    r.execute_action(v.selecting_player, REPLACEMENT_ACCEPT)
        .unwrap();
    let v = r.pending_selection_view().expect("trash play prompt");
    assert!(v.is_optional);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"TITAN5".to_string()));
    assert!(!field_ids(&r, 0).contains(&"TITAN5".to_string()));
    assert!(
        trash_ids(&r, 0).contains(&CARD_ID.to_string()),
        "the Delay was still activated"
    );
}

#[test]
fn bt24_098_declining_delay_keeps_it_parked() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "TITAN-PLAY");
    use_main(&mut r);
    push_trash(&mut r, 0, "TITAN5");
    advance_to_next_own_main(&mut r);
    r.game.memory = 2;
    play_from_hand_by_id(&mut r, "TITAN-PLAY");
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    pass(&mut r, v.selecting_player);
    let _ = r.auto_resolve();
    assert!(
        delayed_state(&r).is_some(),
        "declined <Delay> stays in the battle area"
    );
    assert!(trash_ids(&r, 0).contains(&"TITAN5".to_string()));
}

#[test]
fn bt24_098_main_with_one_card_to_trash_trashes_one() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-098")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "PURPLE",
            "Purple Guy",
            CardColor::Purple,
            3,
            3,
            &[],
        ))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"; 4])
        .memory(10)
        .start();
    r.set_first_player(0);
    let deck = r.deck_size(0);
    push_hand(&mut r, 0, CARD_ID);
    r.place_on_field(0, "PURPLE", Some(0));
    r.game.enter_main_phase();
    let idx = r.game.players[0].hand.len() - 1;
    let res = r.game.play_option_from_hand(0, idx);
    assert!(!matches!(res, OptionPlayResult::Invalid));
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), 0);
    assert_eq!(
        r.hand_size(0),
        0,
        "every drawn card is trashed (min(2, hand))"
    );
    assert_eq!(r.game.players[0].trash.len(), deck);
    assert!(delayed_state(&r).is_some(), "still placed");
}

#[test]
fn bt24_098_delay_without_opponent_five_memory_plays_nothing() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "TITAN-PLAY");
    use_main(&mut r);
    push_trash(&mut r, 0, "TITAN5");
    advance_to_next_own_main(&mut r);
    r.game.memory = 10;
    play_from_hand_by_id(&mut r, "TITAN-PLAY");
    assert_eq!(r.memory(), 3);
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    r.execute_action(v.selecting_player, REPLACEMENT_ACCEPT)
        .unwrap();
    assert!(
        r.pending_selection_view().is_none(),
        "opponent has < 5 memory → no play prompt"
    );
    assert!(trash_ids(&r, 0).contains(&"TITAN5".to_string()));
}

#[test]
fn bt24_098_delay_ignores_non_titan_play() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "PLAIN-PLAY");
    use_main(&mut r);
    advance_to_next_own_main(&mut r);
    r.game.memory = 2;
    play_from_hand_by_id(&mut r, "PLAIN-PLAY");
    assert!(
        r.pending_selection_view().is_none(),
        "not a [Titan] Digimon"
    );
    assert!(delayed_state(&r).is_some(), "stays parked");
}

#[test]
fn bt24_098_delay_not_on_placing_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "TITAN-PLAY");
    use_main(&mut r);
    play_from_hand_by_id(&mut r, "TITAN-PLAY");
    assert!(r.pending_selection_view().is_none(), "placing turn");
    assert!(delayed_state(&r).is_some());
}

#[test]
fn bt24_098_delay_not_on_opponents_turn() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    use_main(&mut r);
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    r.game.enter_main_phase();
    // Opponent plays their own Titan Digimon on their turn.
    let idx = data_idx(&r, "TITAN-PLAY");
    let next = r.game.next_card_index();
    r.game.players[1].hand.push(CardSource::new(idx, 1, next));
    r.game.memory = 10;
    let hidx = r.game.players[1].hand.len() - 1;
    assert!(r.play(1, hidx).is_some());
    assert!(
        r.pending_selection_view().is_none(),
        "[Your Turn] + your Digimon"
    );
    assert!(delayed_state(&r).is_some());
}

#[test]
fn bt24_098_security_plays_titan_from_hand_then_adds_self() {
    let mut r = builder().security(1, &[CARD_ID]).memory(3).start();
    r.set_first_player(0);
    let idx = data_idx(&r, "TITAN4");
    let next = r.game.next_card_index();
    r.game.players[1].hand.push(CardSource::new(idx, 1, next));
    let idx5 = data_idx(&r, "TITAN5");
    let next = r.game.next_card_index();
    r.game.players[1].hand.push(CardSource::new(idx5, 1, next));
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional, "'you may play'");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "only the level 4 Titan (level 5 excluded)"
    );
    let a = non_pass(&r)[0];
    r.execute_action(v.selecting_player, a).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"TITAN4".to_string()));
    assert!(
        hand_ids(&r, 1).contains(&CARD_ID.to_string()),
        "then add this card to the hand"
    );
    assert_eq!(r.memory(), 3, "without paying the cost");
}

#[test]
fn bt24_098_security_plays_titan_from_trash() {
    let mut r = builder().security(1, &[CARD_ID]).memory(3).start();
    r.set_first_player(0);
    push_trash(&mut r, 1, "TITAN4");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let v = r.pending_selection_view().expect("security play prompt");
    assert!(v.is_optional);
    pick_trash(&mut r, 1, "TITAN4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"TITAN4".to_string()));
    assert!(hand_ids(&r, 1).contains(&CARD_ID.to_string()));
}

#[test]
fn bt24_098_security_decline_still_adds_self() {
    let mut r = builder().security(1, &[CARD_ID]).memory(3).start();
    r.set_first_player(0);
    push_trash(&mut r, 1, "TITAN4");
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    pass(&mut r, 1);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 1).contains(&"TITAN4".to_string()));
    assert!(hand_ids(&r, 1).contains(&CARD_ID.to_string()));
    assert!(!trash_ids(&r, 1).contains(&CARD_ID.to_string()));
}

// ─── Local fixtures (this set has no shared `support` module) ───────────────

fn digimon(
    id: &str,
    name: &str,
    color: CardColor,
    level: u8,
    cost: u16,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn filler(id: &str) -> CardData {
    digimon(id, id, CardColor::Red, 3, 3, &[])
}

fn data_idx(r: &DebugRunner, card_id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"))
}

fn push_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
}

fn push_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

fn pass(r: &mut DebugRunner, p: u8) {
    r.execute_action(p, PASS).expect("PASS legal");
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&a| a != PASS)
                .collect()
        })
        .unwrap_or_default()
}

fn pick_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    let action = space::PLAY_HAND_START + idx as u16;
    let v = r.pending_selection_view().expect("hand selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal hand pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

fn pick_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in trash"));
    let action = space::TRASH_EFFECT_START + idx as u16;
    let v = r.pending_selection_view().expect("trash selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal trash pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

/// `true` when `card_id` (in `p`'s trash) is a legal pick of the pending prompt.
fn trash_pickable(r: &DebugRunner, p: u8, card_id: &str) -> bool {
    let Some(idx) = r.game.players[p as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
    else {
        return false;
    };
    non_pass(r).contains(&(space::TRASH_EFFECT_START + idx as u16))
}

/// Pick a permanent on one side of the field (OwnField / OppField prompt).
fn pick_side_field(r: &mut DebugRunner, selector: u8, h: PermanentHandle) {
    let action = space::encode_attack(0, h.index as u16);
    let v = r.pending_selection_view().expect("field selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(selector, action).unwrap();
}

/// Test-only [On Play] "trash 1 card in your hand" Digimon — an own-effect
/// hand trash that fires `on_discard_hand`.
const HAND_TRASHER_YAML: &str = r#"
card: T-HANDTRASH
name: HandTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash 1 card in your hand"
    process:
      - select_hand:
          of: you
          bind_as: gone
          filter: {}
          prompt: "Trash 1 card in your hand"
      - trash_from_hand_by_index: { of: you, hand_index: gone }
"#;
