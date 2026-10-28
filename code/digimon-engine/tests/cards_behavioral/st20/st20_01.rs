//! ST20-01 Koromon — Digi-Egg, Lv.2, Black. Traits: Lesser / ADVENTURE.
//!
//! Printed (official Bandai DB, data/card_bundles/ST20-01.md):
//!   Inherited Effect: [All Turns] This Digimon with the [ADVENTURE] trait
//!   gets +1000 DP.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/ST20/Black/ST20_01.cs —
//!   ChangeSelfDPStaticEffect(+1000, isInheritedEffect: true), condition
//!   `PermanentOfThisCard().TopCard.HasAdventureTraits` (the CARRIER's top
//!   card must have [ADVENTURE]); no turn gate.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "ST20-01";

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Black];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-01 YAML must be present in the embedded DSL pack")
        .add_card(digimon("ADV-HOST", &["Reptile", "ADVENTURE"]))
        .add_card(digimon("PLAIN-HOST", &["Reptile"]))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(5)
        .start()
}

#[test]
fn st20_01_is_black_lv2_egg_with_one_inherited_dp_aura() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert_eq!(card.name, "Koromon");
    assert!(card.traits.iter().any(|t| t == "ADVENTURE"));
    assert_eq!(card.effects.len(), 1);
    match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, dp_modifier, .. }) => {
            assert_eq!(*scope, CompiledScope::Inherited);
            assert_eq!(*dp_modifier, Some(1000));
        }
        other => panic!("expected an inherited aura, got {other:?}"),
    }
}

#[test]
fn st20_01_inherited_gives_adventure_carrier_plus_1000_on_both_turns() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "ADV-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(4000), "3000 + 1000 for an [ADVENTURE] carrier");

    // [All Turns]: still applies on the opponent's turn.
    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(4000), "[All Turns] — opponent's turn too");
}

#[test]
fn st20_01_inherited_does_not_apply_to_non_adventure_carrier() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "PLAIN-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(3000), "carrier without [ADVENTURE] gets nothing");
}

#[test]
fn st20_01_inherited_only_buffs_its_own_carrier() {
    let mut r = runner();
    let host = r.place_stack(0, &[CARD_ID, "ADV-HOST"]);
    let other = r.place_on_field(0, "ADV-HOST", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(4000));
    assert_eq!(r.effective_dp(other), Some(3000), "\"This Digimon\" only");
}
