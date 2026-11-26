//! Source: `data/card_bundles/BT27-025.md` (printed text and official Q&A).
//!
//! [On Play] [When Digivolving] 1 of your Digimon can't be deleted in battle
//! until your opponent's turn ends. If played by effects, it also gains
//! <Blocker>. (Rule) Trait: Has [Aquatic] Type. Inherited: <Evade>.
//! Q&A: the protection covers battles with Digimon and Security Digimon.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;
use digimon_engine::enums::ModifierType;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-025";

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .dsl_card("BT27-002")
        .unwrap()
        .add_card(mon("DS3", "Ds", 3, 3, &["DATA SQUAD"]))
        .add_card(mon("ALLY", "Ally", 4, 5, &[]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .hand(0, &[CARD])
        .memory(10)
        .start()
}

#[test]
fn rule_trait_has_aquatic() {
    let r = runner();
    assert!(r.compiled_card(CARD).unwrap().traits.iter().any(|t| t == "Aquatic"));
}

#[test]
fn hand_play_protects_chosen_digimon_without_blocker() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("own Digimon pick");
    assert!(!v.is_optional);
    assert_eq!(candidates(&r), vec!["ALLY", CARD]);
    r.choose_prompt_card("ALLY", None).unwrap();
    no_prompt(&r);
    assert!(r.game.modifiers.has(ally, ModifierType::CannotBeDestroyedByBattle));
    assert!(!r.game.has_keyword(ally, Keyword::Blocker), "not played by effects");
}

#[test]
fn when_digivolving_protects_without_blocker() {
    let mut r = runner();
    let base = r.place_on_field(0, "DS3", Some(0));
    assert!(digivolve_hand(&mut r, CARD, base));
    if r.pending_selection().is_some() {
        r.choose_prompt_card(CARD, None).unwrap();
    }
    no_prompt(&r);
    assert!(r.game.modifiers.has(base, ModifierType::CannotBeDestroyedByBattle));
    assert!(!r.game.has_keyword(base, Keyword::Blocker));
}

#[test]
fn played_by_effects_also_grants_blocker_to_the_chosen_digimon() {
    let mut r = runner();
    // BT27-002's inherited effect plays this card from digivolution cards.
    let carrier = r.place_stack(0, &[CARD, "BT27-002", "TOP"]);
    r.game.players[0].hand.clear();
    attack_security(&mut r, carrier.index);
    r.choose_prompt_card(CARD, None).unwrap();
    assert_eq!(field_ids(&r, 0).last().map(String::as_str), Some(CARD));
    r.choose_prompt_card("TOP", None).unwrap();
    no_prompt(&r);
    assert!(r.game.modifiers.has(carrier, ModifierType::CannotBeDestroyedByBattle));
    assert!(r.game.has_keyword(carrier, Keyword::Blocker));
}

#[test]
fn inherited_grants_evade() {
    let mut r = runner();
    let h = r.place_stack(0, &[CARD, "TOP"]);
    assert!(r.game.has_keyword(h, Keyword::Evade));
}
