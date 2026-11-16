//! EX13-005 Bebydomon — Digi-Egg, Lv.2, Green. Traits: Baby Dragon.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-005.md`)
//!
//! Inherited Effect: [When Attacking] [Once Per Turn] You may play or use 1
//! card with [Dracomon] or [Examon] in its text from your hand with the cost
//! reduced by 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Green/EX13_005.cs
//! - `OnAllyAttack`, inherited, OPT, skippable.
//! - Candidate: `HasText("Dracomon") || HasText("Examon")` and playable/usable
//!   at cost − 1. Optional hand pick (`canNoSelect: true`); Options are used
//!   (`PlayOptionCards`), Digimon/Tamers played (`PlayPermanentCards`), both
//!   paying cost − 1.
//!
//! # Patterns
//! - G4: inherited [When Attacking] on a DigiEgg source.
//! - A3: play-or-use from hand with a cost reduction.
//! - E2: optional + OPT.

#![allow(dead_code, unused_imports)]

use super::orphan_support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX13-005";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-005 YAML loads")
        .add_card(named("CARRIER", "Monodramon", CardColor::Green, 3, 3000))
        .add_card(with_cost(
            named("DRACO", "Dracomon X", CardColor::Green, 4, 5000),
            4,
        ))
        .add_card(with_cost(
            with_text(
                named("EXA-TEXT", "Breakdramon", CardColor::Green, 5, 7000),
                "Treated as an [Examon] material.",
            ),
            5,
        ))
        .add_card({
            let mut o = option("EXA-OPT", CardColor::Green, 2);
            o.card_name = "Examon Strike".into();
            o
        })
        .add_card(with_cost(named("PLAIN", "Greymon", CardColor::Green, 4, 5000), 4))
        .add_card(named("FILL", "Filler", CardColor::Green, 3, 3000))
        .add_card(named("SEC", "SecCard", CardColor::Green, 3, 1000))
}

fn setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["SEC", "SEC", "SEC"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(6)
        .start();
    let h = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    (r, h)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_005_has_one_inherited_when_attacking_opt_clause() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Bebydomon");
    assert_eq!(card.effects.len(), 1);
    let CompiledClause::Triggered(t) = &card.effects[0] else {
        panic!("triggered clause expected");
    };
    assert_eq!(t.scope, CompiledScope::Inherited);
    assert_eq!(t.when, vec![CompiledTiming::WhenAttacking]);
    assert!(t.once_per_turn);
}

// ─── Section 2/3 — behavior ──────────────────────────────────────────────────

#[test]
fn ex13_005_offers_only_dracomon_or_examon_text_cards() {
    let (mut r, h) = setup(&["DRACO", "EXA-TEXT", "EXA-OPT", "PLAIN"]);
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional(), "'You may play or use'");
    assert_eq!(
        offered_hand_ids(&r),
        vec![
            "DRACO".to_string(),
            "EXA-OPT".to_string(),
            "EXA-TEXT".to_string()
        ],
        "[Dracomon] in name, [Examon] in effect text, [Examon] in an Option's name"
    );
}

#[test]
fn ex13_005_plays_a_digimon_with_cost_reduced_by_one() {
    let (mut r, h) = setup(&["DRACO"]);
    let mem0 = r.memory();
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "DRACO");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DRACO".to_string()), "Dracomon X played");
    assert_eq!(mem0 - r.memory(), 3, "play cost 4 reduced by 1");
}

#[test]
fn ex13_005_uses_an_option_with_cost_reduced_by_one() {
    let (mut r, h) = setup(&["EXA-OPT"]);
    let mem0 = r.memory();
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "EXA-OPT");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty(), "Option left the hand");
    assert!(trash_ids(&r, 0).contains(&"EXA-OPT".to_string()), "used Option is trashed");
    assert_eq!(mem0 - r.memory(), 1, "use cost 2 reduced by 1");
}

#[test]
fn ex13_005_declining_plays_nothing() {
    let (mut r, h) = setup(&["DRACO"]);
    let mem0 = r.memory();
    r.attack_player(h, 1, false);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DRACO".to_string()]);
    assert_eq!(r.memory(), mem0);
}

#[test]
fn ex13_005_no_eligible_card_no_prompt() {
    let (mut r, h) = setup(&["PLAIN"]);
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none(), "no [Dracomon]/[Examon] card in hand");
}

// ─── Section 5 — OPT ─────────────────────────────────────────────────────────

#[test]
fn ex13_005_once_per_turn_second_attack_gets_no_prompt() {
    let (mut r, h) = setup(&["DRACO", "EXA-TEXT"]);
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "DRACO");
    let _ = r.auto_resolve();
    let h = handle_of(&r, 0, "CARRIER");
    set_suspended(&mut r, h, false);
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none(), "OPT spent this turn");
}

// ─── Section 6 — Candidate legality ("play or use" what can be played/used) ──
//
// DCGO EX13_005.cs:28-30 offers `HasText(...) && CanPlayOrUse(cardSource,
// activateClass, fixedCost: max(0, cost − 1))`, which is
// (PlayEffects.cs:286-294) `IsOption && !CanNotPlayThisOption && MaxMemoryCost
// >= fixedCost` || `HasPlayCost && CanPlayAsNewPermanent(payCost, fixedCost)`
// (an empty frame, no play prohibition, cost ≤ MaxMemoryCost). So a Digi-Egg
// (CR 4.3 7-1-1: never played from the hand), an Option whose colour
// requirement is unmet (CR 4.3 4-22-1..3; `CanNotPlayThisOption` includes
// `!MatchColorRequirement`) or that "can't be used", a prohibited or frameless
// play, and a card the player can't pay for at cost − 1 are never offered. With
// memory 6 the player pays at most 16 (CR 4.3 memory floor −10).

fn legality_builder() -> DebugRunnerBuilder {
    builder()
        .add_card({
            let mut egg = named("DRACO-EGG", "Dracomon Egg", CardColor::Green, 2, 0);
            egg.card_kind = digimon_engine::enums::CardKind::DigiEgg;
            egg.play_cost = 0;
            egg.dp = None;
            egg
        })
        .add_card({
            let mut o = option("EXA-PURPLE", CardColor::Purple, 2);
            o.card_name = "Examon Purple Strike".into();
            o
        })
        .add_card(with_cost(named("DRACO-17", "Dracomon Seventeen", CardColor::Green, 6, 9000), 17))
        .add_card(with_cost(named("DRACO-18", "Dracomon Eighteen", CardColor::Green, 6, 9000), 18))
}

fn legality_offers(hand: &[&str], prepare: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut r = legality_builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["SEC", "SEC", "SEC"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(6)
        .start();
    let h = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    prepare(&mut r);
    r.attack_player(h, 1, false);
    if r
        .pending_selection_view()
        .is_some_and(|v| v.kind == digimon_engine::selection::SelectionKind::Hand)
    {
        offered_hand_ids(&r)
    } else {
        Vec::new()
    }
}

/// CR 4.3 7-1-1: a [Dracomon]-text Digi-Egg is never offered.
#[test]
fn ex13_005_excludes_a_dracomon_digi_egg() {
    assert_eq!(
        legality_offers(&["DRACO-EGG", "DRACO"], |_| {}),
        vec!["DRACO".to_string()]
    );
}

/// A purple Option with only green cards in play can't be used (CR 4.3 4-22).
#[test]
fn ex13_005_excludes_an_option_whose_colour_requirement_is_unmet() {
    assert_eq!(
        legality_offers(&["EXA-PURPLE", "EXA-OPT"], |_| {}),
        vec!["EXA-OPT".to_string()]
    );
}

/// "You can't use Option cards": the Option is not offered, the Digimon is.
#[test]
fn ex13_005_excludes_options_when_options_cannot_be_used() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = legality_offers(&["EXA-OPT", "DRACO"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["DRACO".to_string()]);
}

/// "You can't play Digimon by effects": the Digimon is not offered, the Option is.
#[test]
fn ex13_005_excludes_digimon_under_effect_play_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let offered = legality_offers(&["EXA-OPT", "DRACO"], |r| {
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
        );
    });
    assert_eq!(offered, vec!["EXA-OPT".to_string()]);
}

/// A full battle area: the Digimon is not offered, the Option is.
#[test]
fn ex13_005_excludes_digimon_when_battle_area_full() {
    let offered = legality_offers(&["EXA-OPT", "DRACO"], |r| {
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert_eq!(offered, vec!["EXA-OPT".to_string()]);
}

/// DRACO-17 costs 16 after the −1 (payable); DRACO-18 costs 17 (not).
#[test]
fn ex13_005_offers_only_cards_affordable_after_the_reduction() {
    assert_eq!(
        legality_offers(&["DRACO-17", "DRACO-18"], |_| {}),
        vec!["DRACO-17".to_string()]
    );
}
