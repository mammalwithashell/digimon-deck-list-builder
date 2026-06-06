//! BT25-014 Meramon — Digimon, Lv.4, Red, DP 5000, Cost 4.
//! Traits: Flame, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! [Main] [Once Per Turn] By trashing 1 [Flame] or [TS] trait card from your
//! hand, delete 1 of your opponent's Digimon with 4000 DP or less. If this
//! effect didn't delete, <Draw 2> (Draw 2 cards from your deck.)
//!
//! Inherited: [When Attacking] Delete 1 of your opponent's Digimon with 4000
//! DP or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_014.cs
//!
//! - Region "Alt Digivolution" (None): two AddSelfDigivolutionRequirement-
//!   StaticEffect (Flame Lv.3 cost 2; TS Lv.3 cost 2).
//! - Region "Main" (OnDeclaration, maxCountPerTurn 1): MANDATORY trash of 1
//!   Flame/TS hand card (the cost), then if an opp Digimon DP<=4000 exists,
//!   delete it; if not deleted, Draw 2.
//! - Region "ESS - When Attacking" (OnAllyAttack, inherited, maxCountPerTurn 1):
//!   delete 1 opp Digimon with DP <= 4000.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - Main [OPT] activated effect with hand-trash cost
//! - F-deletion by DP cap (dp_lte)
//! - conditional "if didn't delete, draw" via effect_deleted_any_opponent_digimon
//! - G4 inherited When Attacking delete

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-014";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: level.saturating_sub(1),
        memory_cost: 2,
    }];
    card
}

fn make_card_with_cost(id: &str, traits: &[&str]) -> CardData {
    // A playable hand card (has a play cost), used as Flame/TS trash fodder.
    let mut card = make_digimon(id, 3, 3000, traits);
    card.play_cost = Some(3);
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-014 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_card_with_cost("FLAME-FODDER", &["Flame"]))
        .add_card(make_card_with_cost("PLAIN-FODDER", &["Beast"]))
        .add_card(make_digimon("OPP-SMALL", 4, 4000, &["Beast"])) // <=4000 → deletable
        .add_card(make_digimon("OPP-BIG", 5, 9000, &["Beast"])) // >4000 → safe
        .add_card(make_digimon("CARRIER", 5, 7000, &["Beast"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_014_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-014 present in pack");

    assert_eq!(card.name, "Meramon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    for t in ["Flame", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_014_has_main_clause_and_inherited_when_attacking() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_main = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::Main)
    ));
    assert!(has_main, "BT25-014 must have a [Main] activated clause");

    let inherited_wa = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::WhenAttacking)
    ));
    assert!(
        inherited_wa,
        "BT25-014 must have an inherited When Attacking clause"
    );
}

#[test]
fn bt25_014_main_clause_is_once_per_turn() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::Main) => Some(t),
            _ => None,
        })
        .expect("main clause present");
    assert!(main.once_per_turn, "[Main][Once Per Turn]");
}

// ─── Section 2 — Behavior: Main clause ───────────────────────────────────────

/// Positive gate: with a Flame card in hand, the [Main] effect is activatable
/// (a prompt installs when activated).
#[test]
fn bt25_014_main_installs_prompt_with_flame_in_hand() {
    let mut runner = base().hand(0, &["FLAME-FODDER"]).memory(5).start();
    let _meramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));

    // Fire the Main activated effect of Meramon (field index 0).
    let fired = runner.activate_main_effect(0, 0);
    assert!(fired, "Main effect must be activatable with a Flame card in hand");
    assert!(
        runner.pending_selection().is_some(),
        "activating Main installs the trash-cost / delete prompt"
    );
}

/// Negative gate: no Flame/TS card in hand → the [Main] effect cannot be
/// activated (the activation condition fails).
#[test]
fn bt25_014_main_not_activatable_without_flame_or_ts_card() {
    let mut runner = base().hand(0, &["PLAIN-FODDER"]).memory(5).start();
    let _meramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-SMALL", Some(0));

    let fired = runner.activate_main_effect(0, 0);
    assert!(
        !fired,
        "without a Flame or TS card in hand, the Main effect must not be activatable"
    );
}

/// Behavioral: trash a Flame card, delete a <=4000 opp Digimon. The opp Digimon
/// leaves the field and the trashed card lands in trash.
#[test]
fn bt25_014_main_trashes_card_and_deletes_small_opponent() {
    let mut runner = base().hand(0, &["FLAME-FODDER"]).memory(5).start();
    let _meramon = runner.place_on_field(0, CARD_ID, Some(0));
    let victim = runner.place_on_field(1, "OPP-SMALL", Some(0));

    let opp_before = runner.battle_area_size(1);
    let trash_before = runner.trash_size(0);

    assert!(runner.activate_main_effect(0, 0), "Main fires");
    runner.auto_resolve().expect("resolve trash + delete");

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the <=4000 DP opponent Digimon must be deleted"
    );
    assert!(
        runner.trash_size(0) > trash_before,
        "the Flame cost card must be trashed"
    );
}

/// Behavioral: when no eligible delete target exists (opponent only has a
/// >4000 DP Digimon), the effect didn't delete → Draw 2.
#[test]
fn bt25_014_main_draws_two_when_no_deletion() {
    let mut runner = base().hand(0, &["FLAME-FODDER"]).memory(5).start();
    let _meramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-BIG", Some(0)); // 9000 DP — not deletable

    let hand_before = runner.hand_size(0); // 1 (the fodder; will be trashed)

    assert!(runner.activate_main_effect(0, 0), "Main fires");
    runner.auto_resolve().expect("resolve trash + draw");

    // Cost trashes 1 (the fodder), then Draw 2 → net hand = hand_before - 1 + 2.
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 2,
        "no deletion → Draw 2 (hand: trash 1 fodder, +2 draws)"
    );
}

// ─── Section 3 — Behavior: inherited When Attacking ──────────────────────────

/// Inherited When Attacking: Meramon under a carrier; attacking deletes a
/// <=4000 DP opponent Digimon.
#[test]
fn bt25_014_inherited_when_attacking_deletes_small_opponent() {
    let mut runner = base().memory(5).start();
    // Carrier with Meramon as a digivolution source (last id = top).
    let attacker = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let small = runner.place_on_field(1, "OPP-SMALL", Some(0));
    let big = runner.place_on_field(1, "OPP-BIG", Some(0));

    let opp_before = runner.battle_area_size(1);
    // Attack the opponent player to open the When Attacking window.
    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().ok();

    assert!(
        runner.battle_area_size(1) < opp_before,
        "inherited When Attacking must delete a <=4000 DP opponent Digimon"
    );
}
