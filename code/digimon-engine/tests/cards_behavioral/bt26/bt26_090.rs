//! BT26-090 Kanan Yuki — Tamer, Green, ADAMAS/TS.
//!
//! [Start of Your Main Phase] If you have 4 or less memory, gain 1 memory.
//! [End of Your Turn] By suspending this Tamer, you may use 1 Option card with
//! the [TS] trait from your hand. For each memory your opponent has, reduce
//! this effect's paid cost by 1.
//! [Security] Play this card without paying the cost.
//!
//! DCGO: BT26/Green/BT26_090.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

const CARD_ID: &str = "BT26-090";

fn option(id: &str, cost: u16, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut c = digimon_engine::debug_runner::make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Green];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn setup(memory: i16) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-090")
        .add_card(filler("FILLER"))
        .add_card(digimon("ATK", "Atk", CardColor::Red, 5, 7, &[]))
        .add_card(option("TS-OPT", 5, &["TS"]))
        .add_card(option("PLAIN-OPT", 1, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(memory)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_090_start_of_main_gains_memory_at_four_or_less() {
    let mut r = setup(4);
    let t = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    assert_eq!(r.memory(), 5);
    // 5 memory ⇒ no gain.
    fire(&mut r, EffectTiming::StartOfYourMainPhase, t);
    assert_eq!(r.memory(), 5);
}

#[test]
fn bt26_090_end_of_turn_suspend_to_use_ts_option_reduced_by_opponent_memory() {
    // My gauge at -2 ⇒ my opponent has 2 memory.
    let mut r = setup(-2);
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS-OPT");
    push_hand(&mut r, 0, "PLAIN-OPT");
    fire(&mut r, EffectTiming::EndOfYourTurn, t);
    assert!(r.pending_is_optional(), "may-activate");
    r.accept_optional_trigger().expect("accept");
    assert!(
        r.game.players[0].battle_area[t.index as usize].is_suspended,
        "suspended as the cost"
    );
    // Only the [TS] Option is offered.
    let v = r.pending_selection_view().expect("option pick");
    assert!(v.is_optional, "you may use");
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        1
    );
    pick_hand(&mut r, 0, "TS-OPT");
    let _ = r.auto_resolve();
    assert!(!hand_ids(&r, 0).contains(&"TS-OPT".to_string()), "used");
    let p0_mem = if r.game.turn_player() == 0 {
        r.memory()
    } else {
        -r.memory()
    };
    assert_eq!(p0_mem, -2 - 3, "use cost 5 reduced by 2");
}

/// G-DSL-USE-OPTION-ONLY: Kanan prints "you may USE 1 Option card", so a DUAL
/// card (BT26-033 Jupitermon, [TS] on its Option face) may only be USED — the
/// engine must not offer "Play as Digimon" and must never put it on the field.
#[test]
fn bt26_090_dual_card_is_used_as_option_never_played() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-090")
        .dsl_card("BT26-033")
        .expect("BT26-033")
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(-3)
        .start();
    r.set_first_player(0);
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "BT26-033");
    fire(&mut r, EffectTiming::EndOfYourTurn, t);
    r.accept_optional_trigger().expect("accept");
    pick_hand(&mut r, 0, "BT26-033");
    // No "Play as Digimon / Use as Option" face choice may surface.
    if let Some(v) = r.pending_selection_view() {
        assert!(
            !v.prompt.contains("Play as Digimon"),
            "use-only effect offered the Digimon face: {}",
            v.prompt
        );
    }
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&"BT26-033".to_string()),
        "the DUAL card must not enter the battle area"
    );
    assert!(
        !hand_ids(&r, 0).contains(&"BT26-033".to_string()),
        "the DUAL card was used"
    );
}

#[test]
fn bt26_090_end_of_turn_is_optional() {
    let mut r = setup(-2);
    let t = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "TS-OPT");
    r.end_turn();
    assert!(r.pending_is_optional(), "may-activate at end of turn");
    r.decline_optional_trigger().expect("decline");
    assert!(
        r.game.pending_selection.is_none(),
        "declined: {:?}",
        r.pending_selection_view().map(|v| v.prompt)
    );
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert!(hand_ids(&r, 0).contains(&"TS-OPT".to_string()));
}

#[test]
fn bt26_090_security_plays_itself() {
    let mut r = setup(10);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let next = r.game.next_card_index();
    r.game.players[1]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 1, next));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&CARD_ID.to_string()));
}

// ─── Use-candidate legality (G-DSL-PLAY-OR-USE-CANDIDATE-LEGALITY +
// G-DSL-AFFORDABLE-WITH-COST) ─────────────────────────────────────────────────
// DCGO BT26_090.cs:54-62 CanSelectOptionCardCondition: `IsOption && HasTSTraits
// && !CanNotPlayThisOption && Owner.MaxMemoryCost >= GetCostItself -
// Enemy.MemoryForPlayer`. CR 4.3 1-3-2 (an impossible action is not carried
// out), 4-22 (an Option's colour requirement must be met to use it; Q1500 /
// Q2785), 7-1-2-3 (memory doesn't move when the cost can't be paid; the use is
// paid, so the card must be payable down to the memory floor).

/// Kanan Yuki (green) on the field; my gauge at `memory` (opponent has
/// `-memory`); [TS] Options of several costs / colours available.
fn legality_runner(memory: i16, hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut red = option("TS-RED", 1, &["TS"]);
    red.colors = vec![CardColor::Red];
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-090")
        .add_card(filler("FILLER"))
        .add_card(option("TS-OPT", 5, &["TS"]))
        .add_card(option("TS-10", 10, &["TS"]))
        .add_card(option("TS-11", 11, &["TS"]))
        .add_card(red)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(memory)
        .start();
    r.set_first_player(0);
    r.game.players[0].hand.clear();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    (r, t)
}

/// Sorted card ids of the non-PASS candidates of the pending hand pick.
fn offered(r: &DebugRunner) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    let mut ids: Vec<String> = if pending.card_choice.is_some() {
        physical_menu::commits(&r.game)
            .into_iter()
            .filter_map(|row| row.card_id)
            .collect()
    } else {
        let p = pending.selecting_player as usize;
        r.pending_selection_view()
            .map(|v| v.valid_action_ids)
            .unwrap_or_default()
            .into_iter()
            .filter(|&a| a != PASS)
            .filter_map(|a| a.checked_sub(PLAY_HAND_START))
            .filter_map(|i| r.game.players[p].hand.get(i as usize))
            .map(|c| c.card_id(&r.game.card_data).to_string())
            .collect()
    };
    ids.sort();
    ids
}

fn activate(r: &mut DebugRunner, t: digimon_engine::permanent::PermanentHandle) {
    fire(r, EffectTiming::EndOfYourTurn, t);
    r.accept_optional_trigger().expect("accept");
}

/// Opponent has 2 memory (reduce 2), my gauge at -2, floor -10: a cost-10 [TS]
/// Option pays 8 (→ -10, payable), a cost-11 one pays 9 (→ -11, not payable)
/// and is not offered.
#[test]
fn bt26_090_unaffordable_ts_option_not_offered() {
    let (mut r, t) = legality_runner(-2, &["TS-OPT", "TS-10", "TS-11"]);
    activate(&mut r, t);
    assert_eq!(offered(&r), vec!["TS-10".to_string(), "TS-OPT".to_string()]);
}

/// A red [TS] Option with only green Kanan Yuki on the field: its colour
/// requirement is unmet, so it can't be used and is not offered.
#[test]
fn bt26_090_ts_option_with_unmet_colour_requirement_not_offered() {
    let (mut r, t) = legality_runner(-2, &["TS-OPT", "TS-RED"]);
    activate(&mut r, t);
    assert_eq!(offered(&r), vec!["TS-OPT".to_string()]);
}

/// "You can't use Option cards": no [TS] Option is offered.
#[test]
fn bt26_090_option_lock_offers_nothing() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut r, t) = legality_runner(-2, &["TS-OPT"]);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
    );
    activate(&mut r, t);
    assert!(offered(&r).is_empty(), "offered {:?}", offered(&r));
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["TS-OPT".to_string()]);
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
