//! Source: `data/card_bundles/BT27-023.md` (printed text).
//!
//! ### Effect
//! [When Moving] [On Play] 1 of your Digimon with [Gammamon] in its text or the [VB] trait can't be deleted in battle for the turn. [When Attacking] [Once Per Turn] 1 of your opponent's level 5 or lower Digimon can't suspend until their turn ends.
//!
//! ### Inherited Effect
//! ＜Evade＞ (When this Digimon would be deleted, by suspending it, it isn't deleted.)
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

use super::common::*;
use digimon_engine::enums::ModifierType;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-023";

#[test]
fn on_play_protects_only_eligible_own_digimon_in_battle() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_on_field(0, "TEXT-3", Some(0));
    r.play(0, 0);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    let attacker = r.place_on_field(1, "TEXT-6", Some(0));
    let target = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 1,
    };
    r.game.suspend(target);
    assert_eq!(r.dp_of(attacker), Some(12000));
    assert!(r
        .game
        .modifiers
        .has(target, ModifierType::CannotBeDestroyedByBattle));
}

#[test]
fn attack_prevents_only_a_selected_level_five_or_lower_digimon_from_suspending() {
    let mut r = start(builder(CARD));
    let carrier = r.place_on_field(0, CARD, Some(0));
    let legal = r.place_on_field(1, "TEXT-5", Some(0));
    r.place_on_field(1, "TEXT-6", Some(0));
    r.attack_player(carrier, 1, false);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert!(r.game.modifiers.has(legal, ModifierType::CannotSuspend));
    next_turn(&mut r);
    assert!(r.game.modifiers.has(legal, ModifierType::CannotSuspend));
    next_turn(&mut r);
    assert!(!r.game.modifiers.has(legal, ModifierType::CannotSuspend));
}

#[test]
fn inherited_evade_suspends_this_digimon_to_survive_deletion() {
    let mut r = start(builder(CARD));
    let carrier = r.place_stack(0, &[CARD, "TEXT-5"]);
    let enemy = r.place_on_field(1, "TEXT-6", Some(0));
    r.battle_digimon(carrier, enemy);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    assert!(r.game.players[0].battle_area[carrier.index as usize].is_suspended);
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn moving_from_breeding_protects_one_eligible_digimon() {
    let mut r = start(builder(CARD));
    r.place_on_field(0, "TEXT-3", Some(0));
    r.set_phase(digimon_engine::enums::GamePhase::Breeding);
    r.place_in_breeding(0, CARD);
    assert!(r.move_from_breeding(0));
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    assert!(r
        .game
        .modifiers
        .has(r.perm_handle(0, 1), ModifierType::CannotBeDestroyedByBattle));
}
