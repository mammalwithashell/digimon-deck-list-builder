//! ST20-01 Koromon — Digi-Egg, Lv.2, Black, no DP, play cost 0.
//! Traits: Lesser, ADVENTURE. Form: In-Training.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-01.md)
//!
//! ```text
//! Inherited Effect
//! [All Turns] This Digimon with the [ADVENTURE] trait gets +1000 DP.
//! ```
//!
//! No face-up text and no digivolution cost circle (Digi-Egg).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Black/ST20_01.cs — one
//! `EffectTiming.None` `ChangeSelfDPStaticEffect(+1000, isInheritedEffect)`
//! gated on `card.PermanentOfThisCard().TopCard.HasAdventureTraits`. The gate
//! reads the CARRIER permanent's TOP card (the Digimon whose digivolution
//! stack holds the egg), and there is no turn gate ([All Turns]).
//!
//! # Patterns (docs/RUST_DSL_TEST_API.md §4.3 / §14.2)
//! - D4 inherited declarative DP aura (BT16-002 DemiVeemon sibling)
//! - Conditional gate on the CARRIER's top-card trait (`source_permanent_trait_has`)
//!   — one positive and one negative test, never combined
//! - [All Turns]: the bonus survives a turn change
//!
//! There is no cost, no selection and no once-per-turn clause on this card, so
//! there are no cost-firing, selection-mask or OPT-lockout tests to write.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "ST20-01";

/// A Lv.3 carrier with 3000 base DP and the given trait line.
fn carrier(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.dp = Some(3000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// ST20-01 on the field with `top` digivolved onto it, so the egg sits in
/// `top`'s digivolution stack and its inherited effect applies to `top`'s
/// permanent (BT16-002 `stack_with_top` mold).
fn stack_with_top(top: CardData) -> (DebugRunner, PermanentHandle) {
    stack_with_top_and(top, Vec::new())
}

/// [`stack_with_top`] plus extra registered cards (bystanders for leak tests).
fn stack_with_top_and(top: CardData, extras: Vec<CardData>) -> (DebugRunner, PermanentHandle) {
    let top_id = top.card_id.clone();
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-01 YAML parses and compiles")
        .add_card(top);
    for extra in extras {
        builder = builder.add_card(extra);
    }
    let mut runner = builder.memory(10).start();
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == top_id)
        .expect("top card registered");
    let next_idx = runner.game.next_card_index();
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[handle.index as usize]
        .digivolve(CardSource::new(data_idx, 0, next_idx), turn);
    runner.game.tick_declarative_effects();
    (runner, handle)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════

/// Identity: Lv.2 black Digi-Egg, no DP, cost 0, Lesser / ADVENTURE.
#[test]
fn st20_01_identity_matches_printed_card() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-01 compiles")
        .start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.name, "Koromon");
    assert_eq!(compiled.kind, CompiledCardKind::DigiEgg);
    assert_eq!(compiled.level, Some(2));
    assert_eq!(compiled.cost, Some(0));
    assert!(compiled.dp.is_none(), "a Digi-Egg prints no DP");
    let traits: Vec<String> = compiled.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(
        traits.contains(&"lesser".to_string()),
        "got {:?}",
        compiled.traits
    );
    assert!(
        traits.contains(&"adventure".to_string()),
        "got {:?}",
        compiled.traits
    );
    assert!(
        compiled.alt_paths.is_empty(),
        "a Digi-Egg has no digivolution cost circle"
    );
}

/// The card has exactly one clause: an inherited-scope DP aura worth +1000.
#[test]
fn st20_01_has_single_inherited_plus_1000_dp_aura() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-01 compiles")
        .start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(
        compiled.effects.len(),
        1,
        "ST20-01 prints only the inherited [All Turns] +1000 DP effect"
    );
    let (scope, dp) = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                ..
            }) => Some((*scope, *dp_modifier)),
            _ => None,
        })
        .expect("ST20-01 must compile to a declarative aura");
    assert_eq!(scope, CompiledScope::Inherited, "inherited effect");
    assert_eq!(dp, Some(1000), "+1000 DP");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating on the carrier's [ADVENTURE] trait
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: a carrier whose top card has [ADVENTURE] gets +1000 DP.
#[test]
fn st20_01_adventure_carrier_gets_plus_1000_on_your_turn() {
    let (runner, handle) = stack_with_top(carrier("ADV-CARRIER", &["Bird", "ADVENTURE"]));
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(runner.effective_dp(handle), Some(4000));
}

/// NEGATIVE: a carrier without the [ADVENTURE] trait gets nothing — the egg's
/// own [ADVENTURE] trait is irrelevant, only the carrier's top card counts.
#[test]
fn st20_01_non_adventure_carrier_gets_no_bonus() {
    let (runner, handle) = stack_with_top(carrier("PLAIN-CARRIER", &["Bird"]));
    assert_eq!(
        runner.effective_dp(handle),
        Some(3000),
        "a carrier without [ADVENTURE] must not be boosted by the egg beneath it"
    );
}

/// NEGATIVE: the same [ADVENTURE] Digimon with NO egg in its stack is not
/// boosted — the bonus belongs to the inherited effect, not the trait.
#[test]
fn st20_01_adventure_digimon_without_the_egg_gets_no_bonus() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-01 compiles")
        .add_card(carrier("ADV-ALONE", &["ADVENTURE"]))
        .memory(10)
        .start();
    let handle = runner.place_on_field(0, "ADV-ALONE", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(handle), Some(3000));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns]: no turn gate
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: [All Turns] — the +1000 is still applied on the opponent's turn.
#[test]
fn st20_01_bonus_persists_on_opponents_turn() {
    let (mut runner, handle) = stack_with_top(carrier("ADV-CARRIER", &["Bird", "ADVENTURE"]));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "now the opponent's turn");
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(handle),
        Some(4000),
        "[All Turns] must not be gated to your turn"
    );
}

/// NEGATIVE: the aura targets only the carrier hosting the egg — a second
/// [ADVENTURE] Digimon on the same side with no egg beneath it is not boosted.
#[test]
fn st20_01_bonus_does_not_leak_to_another_adventure_digimon() {
    let (mut runner, handle) = stack_with_top_and(
        carrier("ADV-CARRIER", &["ADVENTURE"]),
        vec![carrier("ADV-BYSTANDER", &["ADVENTURE"])],
    );
    let other = runner.place_on_field(0, "ADV-BYSTANDER", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(handle), Some(4000), "carrier boosted");
    assert_eq!(
        runner.effective_dp(other),
        Some(3000),
        "a Digimon without the egg in its stack is not boosted"
    );
}
