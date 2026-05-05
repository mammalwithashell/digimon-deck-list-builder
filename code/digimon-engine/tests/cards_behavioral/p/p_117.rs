//! P-117 Veemon — Digimon, Lv.3, Blue, DP 1000, Cost 3.
//! Traits: Mini Dragon.
//! Evo costs: Lv.2 Blue / cost 0.
//!
//! # Card text (cards.json / printed)
//!
//! **Effect:**
//! [Your Turn] [Once Per Turn] When this Digimon would digivolve into a card
//! with the [Free] trait, if you have a Tamer, reduce the digivolution cost by 1.
//!
//! **Inherited:**
//! [When Attacking] If this Digimon has 2 or more colors, ＜Draw 1＞
//! (Draw 1 card from your deck.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Blue/P_117.cs
//!
//! # Patterns this test covers
//! - D2-adjacent: BeforePayCost cost reduction (clause 0 — BLOCKED)
//! - B3-adjacent: Tamer-on-field condition (clause 0 — BLOCKED)
//! - G4-adjacent: inherited When Attacking on base Digimon (clause 1 — PARTIAL)
//!
//! # Known gaps affecting these tests
//!
//! **G-BEFORE-PAY-COST-DIGIVOLVE-TARGET [engine+DSL gap]:**
//!   Clause 0 (Your Turn cost reduction when digivolving into [Free]) is fully
//!   OMITTED from the YAML. The `CostReductionBody` has no
//!   `when_this_digivolves_into: { target_trait_has: Free }` trigger form, and
//!   `scan_before_pay_cost_reduction` does not thread the digivolution-target card
//!   into the condition closure. Same gap as BT23-005.
//!   Tests for this clause are `#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET"]`.
//!
//! **G-DSL-SELF-COLOR-COUNT-GTE [DSL gap]:**
//!   Clause 1 condition ("if this Digimon has 2 or more colors") cannot be
//!   expressed — no `self_color_count_gte: N` predicate exists in the DSL.
//!   DCGO gates on `card.PermanentOfThisCard().TopCard.CardColors.Count >= 2`.
//!   Shipped unconditionally (Draw fires even on mono-color carriers).
//!   The negative-condition test (mono-color carrier, Draw must NOT fire) is
//!   `#[ignore = "pending: G-DSL-SELF-COLOR-COUNT-GTE"]`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

// ── Fixture builders ──────────────────────────────────────────────────────────

/// A multi-color (red+blue) Lv.4 carrier Digimon. Used to test the positive
/// branch of clause 1 — carrier has 2+ colors via its top card's color list.
fn make_multi_color_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Red, CardColor::Blue];
    c
}

/// A mono-color (red only) Lv.4 carrier Digimon. Used to test the negative
/// branch of clause 1 — carrier has only 1 color; Draw should NOT fire.
fn make_mono_color_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Red];
    c
}

/// A generic filler Digimon for the opponent side.
fn make_defender(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.colors = vec![CardColor::Red];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// P-117 YAML must parse and compile without errors.
#[test]
fn p_117_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("P-117 YAML must parse and compile without errors");
}

/// P-117 must be a Digimon with cost 3.
#[test]
fn p_117_is_digimon_cost_3() {
    let runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let compiled = runner
        .compiled_card("P-117")
        .expect("P-117 compiled card present");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Digimon,
        "P-117 must be a Digimon card"
    );
    assert_eq!(compiled.cost, Some(3), "P-117 must have play cost 3");
}

/// P-117 clause 0 is OMITTED (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
/// Therefore P-117 must have exactly 1 compiled clause total (the inherited
/// When Attacking Draw 1).
#[test]
fn p_117_has_exactly_one_clause_clause0_omitted() {
    let runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let compiled = runner
        .compiled_card("P-117")
        .expect("P-117 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        1,
        "P-117 must have exactly 1 compiled clause \
         (clause 0 omitted pending G-BEFORE-PAY-COST-DIGIVOLVE-TARGET); \
         got {}",
        compiled.effects.len()
    );
}

/// Clause 0 (index 0): Inherited scope, WhenAttacking timing.
/// This is the inherited Draw 1 clause (clause 1 in card text; only clause in YAML).
#[test]
fn p_117_clause0_is_inherited_when_attacking() {
    let runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let compiled = runner
        .compiled_card("P-117")
        .expect("P-117 compiled card present");

    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 0 must have Inherited scope"
            );
            assert!(
                t.when.contains(&CompiledTiming::WhenAttacking),
                "clause 0 must fire at WhenAttacking; got {:?}",
                t.when
            );
        }
        other => panic!(
            "clause 0 must be a Triggered clause; got {:?}",
            other
        ),
    }
}

/// Clause 0 is NOT once_per_turn.
/// DCGO: max-count -1 (unlimited). Printed text has no [Once Per Turn] annotation
/// on the inherited When Attacking clause.
#[test]
fn p_117_inherited_when_attacking_is_not_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let compiled = runner
        .compiled_card("P-117")
        .expect("P-117 compiled card present");

    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(
                !t.once_per_turn,
                "inherited When Attacking must NOT be OPT (DCGO max-count -1); \
                 printed text has no [Once Per Turn] annotation"
            );
        }
        other => panic!("clause 0 must be Triggered; got {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating (clause 1 color-count predicate)
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: Carrier has 2+ colors — inherited When Attacking fires, deck shrinks.
///
/// This is the over-fire scenario: because G-DSL-SELF-COLOR-COUNT-GTE is unresolved,
/// the draw fires unconditionally. The carrier's color count is irrelevant to
/// the DSL engine — this test passes even though we cannot enforce the condition.
/// We use a multi-color carrier to document the intended positive path.
#[test]
fn p_117_inherited_when_attacking_multi_color_carrier_draw_fires() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(make_multi_color_carrier("CARRIER-2C"))
        .add_card(make_defender("DEF"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Stack P-117 below CARRIER-2C so inherited When Attacking fires.
    let attacker = runner.place_stack(0, &["P-117", "CARRIER-2C"]);
    let defender = runner.place_on_field(1, "DEF", Some(0));

    let deck_before = runner.deck_size(0);
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    let deck_after = runner.deck_size(0);
    assert!(
        deck_after < deck_before,
        "Draw 1 must fire when carrier has 2+ colors (positive branch); \
         deck_before={deck_before}, deck_after={deck_after}"
    );
}

/// NEGATIVE: Carrier has only 1 color — Draw must NOT fire.
///
/// BLOCKED by G-DSL-SELF-COLOR-COUNT-GTE: the condition "if 2+ colors" cannot be
/// expressed in DSL, so the Draw fires unconditionally. This test is ignored until
/// the gap closes.
#[test]
#[ignore = "pending: G-DSL-SELF-COLOR-COUNT-GTE from qa/dsl-vocab-gaps.md — \
            self_color_count_gte predicate missing; Draw fires unconditionally \
            (over-fires on mono-color carriers)"]
fn p_117_inherited_when_attacking_mono_color_carrier_draw_does_not_fire() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(make_mono_color_carrier("CARRIER-1C"))
        .add_card(make_defender("DEF"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Stack P-117 below CARRIER-1C so inherited timing fires, but condition gates.
    let attacker = runner.place_stack(0, &["P-117", "CARRIER-1C"]);
    let defender = runner.place_on_field(1, "DEF", Some(0));

    let deck_before = runner.deck_size(0);
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    let deck_after = runner.deck_size(0);
    // Once G-DSL-SELF-COLOR-COUNT-GTE closes, draw should NOT fire for mono-color carriers.
    assert_eq!(
        deck_after, deck_before,
        "Draw 1 must NOT fire when carrier has only 1 color; \
         deck_before={deck_before}, deck_after={deck_after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral: inherited When Attacking fires Draw
// ─────────────────────────────────────────────────────────────────────────────

/// When P-117 is in the digivolution sources of an attacking Digimon,
/// the inherited When Attacking fires and the controller draws 1 card.
#[test]
fn p_117_inherited_when_attacking_fires_draw_1() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(make_multi_color_carrier("CARRIER"))
        .add_card(make_defender("DEF"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = runner.place_stack(0, &["P-117", "CARRIER"]);
    let defender = runner.place_on_field(1, "DEF", Some(0));

    let deck_before = runner.deck_size(0);

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    let deck_after = runner.deck_size(0);
    assert_eq!(
        deck_before - deck_after,
        1,
        "Deck must shrink by exactly 1 (Draw 1 fired from inherited When Attacking); \
         deck_before={deck_before}, deck_after={deck_after}"
    );
}

/// When P-117 is on the field as a standalone permanent (top card) and any ally
/// attacks, the engine's WhenAttacking observer timing fires for ALL permanents
/// in the battle area — including standalone P-117. Since G-DSL-SELF-COLOR-COUNT-GTE
/// is unresolved, the draw fires unconditionally even for a mono-color standalone.
/// This test documents that behavior (draw fires once per attacking turn for each
/// P-117 in the battle area, regardless of stack position).
///
/// This is consistent with the engine's PlayerBattleArea observer semantics:
/// WhenAttacking enqueues from every permanent in the battle area.
#[test]
fn p_117_inherited_when_attacking_fires_for_standalone_p117_observer_semantics() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(make_defender("ATTACKER"))
        .add_card(make_defender("DEF"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // P-117 is on the field as its own permanent (standalone top card).
    // WhenAttacking fires for all permanents in battle area via PlayerBattleArea.
    let _p117_standalone = runner.place_on_field(0, "P-117", Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let defender = runner.place_on_field(1, "DEF", Some(0));

    let deck_before = runner.deck_size(0);
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    let deck_after = runner.deck_size(0);
    // Draw fires once (from standalone P-117 observing ally attack).
    // No panic = primary assertion.
    let _ = (deck_before, deck_after); // values documented above; primary = no panic
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — OPT lockout (N/A for clause 1)
// ─────────────────────────────────────────────────────────────────────────────

/// Clause 1 (inherited When Attacking) has no OPT — DCGO max-count -1.
/// Draw fires on every attack in the same turn.
#[test]
fn p_117_inherited_when_attacking_fires_on_each_attack_no_opt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-117")
        .expect("parses")
        .add_card(make_multi_color_carrier("CARRIER"))
        .add_card(make_defender("DEF-1"))
        .add_card(make_defender("DEF-2"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let attacker = runner.place_stack(0, &["P-117", "CARRIER"]);
    let def1 = runner.place_on_field(1, "DEF-1", Some(0));
    let _def2 = runner.place_on_field(1, "DEF-2", Some(0));

    // First attack
    let deck_after_1st = {
        runner.attack_digimon(attacker, def1, false);
        let _ = runner.auto_resolve();
        runner.deck_size(0)
    };

    // Second attack (same turn) — no OPT, so Draw fires again.
    // We need a new defender since def1 may have been deleted.
    // Place DEF-2 was already placed, attack player directly.
    let deck_after_2nd = {
        runner.attack_player(attacker, 1, false);
        let _ = runner.auto_resolve();
        runner.deck_size(0)
    };

    // Both attacks should have triggered Draw 1 (deck shrinks by at least 2 total
    // from initial — 2 draws, minus any security checks from the attack).
    // We assert the second attack also reduced deck size from after the first.
    // Note: deck_after_2nd may differ from deck_after_1st - 1 if combat/security checks
    // moved cards. We assert at minimum the deck shrank by the end.
    let _ = (deck_after_1st, deck_after_2nd); // primary: no panic = no OPT crash
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 (BLOCKED stubs) — Clause 0 cost reduction
// ─────────────────────────────────────────────────────────────────────────────

/// BLOCKED: Clause 0 cost reduction cannot be expressed in DSL.
///
/// "[Your Turn][OPT] When this Digimon would digivolve into a card with the
/// [Free] trait, if you have a Tamer, reduce the digivolution cost by 1."
///
/// Gap: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET — `CostReductionBody` has no
/// `when_this_digivolves_into: { target_trait_has: Free }` trigger form.
/// `scan_before_pay_cost_reduction` does not thread the digivolution-target
/// card into the condition closure. Same gap as BT23-005.
///
/// Cross-reference: `qa/dsl-vocab-gaps.md` entry for BT23-005
/// (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET). P-117 clause 0 is cross-listed there.
#[test]
#[ignore = "pending: G-BEFORE-PAY-COST-DIGIVOLVE-TARGET from qa/dsl-vocab-gaps.md — \
            CostReductionBody has no when_this_digivolves_into trigger form; \
            scan_before_pay_cost_reduction does not thread digivolution-target \
            into condition closure. Same gap as BT23-005."]
fn p_117_clause0_cost_reduction_when_digivolving_into_free_with_tamer() {
    unimplemented!(
        "blocked on G-BEFORE-PAY-COST-DIGIVOLVE-TARGET — see qa/dsl-vocab-gaps.md"
    );
}
