//! ST1-07 Greymon — Digimon, Lv.4, Red, Cost 5, DP 4000.
//! Traits: Dinosaur.
//!
//! # Card text (cards.json)
//!
//! ```text
//! Inherited Effect <Security A. +1>
//! (This Digimon checks 1 additional security card.)
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_07.cs
//!   CardEffectFactory.ChangeSelfSAttackStaticEffect(changeValue: 1, isInheritedEffect: true, ...)
//!
//! # Patterns (RUST_DSL_TEST_API.md §4.3)
//! - H4 Security A. +N declarative keyword grant (inherited scope)
//! - D4 Declarative aura / keyword grant (inherited)
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | Inherited SecurityAttackPlus(1) — compiled | none | PASS |
//! | Inherited SecurityAttackPlus(1) — runtime installation | G-DECLARATIVE-KEYWORD | #[ignore] |
//! | Inherited SecurityAttackPlus(1) — stack-walk keyword grant | none (Batch 7 fix landed) | PASS |

#![allow(unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

// ─── Helper ───────────────────────────────────────────────────────────────────

/// Build a DebugRunner with ST1-07 loaded from the embedded DSL pack.
fn greymon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST1-07")
        .expect("ST1-07 must be in embedded DSL pack")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// ST1-07 compiles: it is present in the embedded DSL pack.
#[test]
fn st1_07_compiles_and_is_in_pack() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("ST1-07");
    assert!(
        compiled.is_some(),
        "ST1-07 must be present in the embedded DSL pack"
    );
}

/// ST1-07 has exactly one declarative clause and no triggered clauses.
/// The sole clause is the inherited <Security A. +1> grant_keyword.
#[test]
fn st1_07_has_one_declarative_and_no_triggered() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("ST1-07").expect("ST1-07 in pack");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 0,
        "ST1-07 has no triggered clauses — the single effect is a declarative grant_keyword"
    );

    let declarative_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(
        declarative_count, 1,
        "ST1-07 has exactly 1 declarative clause (inherited SecurityAttackPlus)"
    );
}

/// The declarative clause is a GrantKeyword for SecurityAttackPlus
/// at scope Inherited (not FaceUp).
///
/// DCGO: isInheritedEffect: true maps to scope: inherited in DSL.
#[test]
fn st1_07_declarative_clause_is_inherited_security_attack_plus() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("ST1-07").expect("ST1-07 in pack");

    let inherited_sap = compiled.effects.iter().find(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword == "SecurityAttackPlus"
        )
    });

    assert!(
        inherited_sap.is_some(),
        "ST1-07 must have an inherited-scope GrantKeyword SecurityAttackPlus clause; \
         found effects: {:?}",
        compiled
            .effects
            .iter()
            .map(|c| format!("{c:?}"))
            .collect::<Vec<_>>()
    );
}

/// The SecurityAttackPlus keyword value must be 1 (matches card text "+1").
///
/// The `value` field on the compiled GrantKeyword clause encodes the integer
/// argument. SecurityAttackPlus(1) means one additional security check.
#[test]
fn st1_07_security_attack_plus_value_is_1() {
    let runner = greymon_runner();
    let compiled = runner.compiled_card("ST1-07").expect("ST1-07 in pack");

    // Locate the GrantKeyword clause and confirm its value encodes 1.
    // In the compiled form, `keyword` is a string ("SecurityAttackPlus") and
    // `value` carries the numeric argument when present.
    let grant_kw = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            value,
            ..
        }) if keyword == "SecurityAttackPlus" => Some(value),
        _ => None,
    });

    let value = grant_kw.expect("SecurityAttackPlus clause must exist");
    assert_eq!(
        *value,
        Some(1i32),
        "SecurityAttackPlus value must be 1 (card text says '+1')"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Inherited Security A. +1: stack-walk behavioral test
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: when ST1-07 is a digivolution source under a carrier Digimon,
/// the carrier must have SecurityAttackPlus(1) via the stack-walk (has_keyword).
///
/// This exercises the Batch 7 fix: game.has_keyword() scans card_sources for
/// inherited grant_keyword effects (same path as BT24-011 inherited Raid).
#[test]
fn st1_07_inherited_security_attack_plus_grants_to_carrier_via_stack() {
    let lv5_card = make_test_card("PLAIN-LV5", "PlainLv5");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-07")
        .expect("ST1-07 in pack")
        .add_card(lv5_card)
        .memory(10)
        .start();

    // Place ST1-07 at slot 0 first.
    let _greymon = runner.place_on_field(0, "ST1-07", Some(0));

    // Grab ST1-07's card source data before overwriting the slot.
    let greymon_source = {
        let game = runner.game_mut();
        game.players[0].battle_area[0].top_card().clone()
    };

    // Place PLAIN-LV5 at slot 0 (replaces ST1-07).
    let carrier = runner.place_on_field(0, "PLAIN-LV5", Some(0));

    // Push ST1-07's card source as a digivolution source under the carrier
    // (simulates a real digivolve stack).
    {
        let game = runner.game_mut();
        game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .insert(0, greymon_source);
    }

    // The carrier must inherit SecurityAttackPlus(1) from ST1-07 in its stack.
    assert!(
        runner
            .game
            .has_keyword(carrier, Keyword::SecurityAttackPlus(1)),
        "PLAIN-LV5 with ST1-07 as a digivolution source must inherit SecurityAttackPlus(1)"
    );
}

/// Negative: a carrier Digimon without ST1-07 in its stack does NOT have
/// SecurityAttackPlus(1).
#[test]
fn st1_07_inherited_security_attack_plus_absent_without_source() {
    let lv5_card = make_test_card("PLAIN-LV5", "PlainLv5");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-07")
        .expect("ST1-07 in pack")
        .add_card(lv5_card)
        .memory(10)
        .start();

    // Place ONLY the carrier — no ST1-07 in its sources.
    let carrier = runner.place_on_field(0, "PLAIN-LV5", Some(0));

    assert!(
        !runner
            .game
            .has_keyword(carrier, Keyword::SecurityAttackPlus(1)),
        "A carrier with no ST1-07 in its digivolution stack must NOT have SecurityAttackPlus(1)"
    );
}

#[test]
fn st1_07_top_card_inherited_security_attack_plus_does_not_apply_to_itself() {
    let mut runner = greymon_runner();
    let handle = runner.place_on_field(0, "ST1-07", Some(0));

    assert!(
        !runner
            .game
            .has_keyword(handle, Keyword::SecurityAttackPlus(1)),
        "ST1-07's inherited SecurityAttackPlus clause must not apply while ST1-07 is the top card"
    );
}

/// Regression: the inherited `<Security A. +1>` must grant the carrier a NET
/// security-attack bonus of exactly **+1** — not +2.
///
/// The embedded DSL pack leaves `card_data.inherited_text` EMPTY, but the real
/// game loads it from `data/cards.json`, where ST1-07's text lives in
/// `inherited_effect_description_eng` = "＜Security A. +1＞ (...)". With the
/// printed inherited text present, the keyword was double-counted:
///   1. `security_attack_keyword_bonus` reads the buried source's
///      `inherited_keywords(card_data)` text-parse → +1, AND
///   2. `tick_declarative_effects` materializes the `scope: inherited`
///      `grant_keyword` onto the carrier → +1.
/// => +2. The de-dup in `tick_declarative_effects` only ran for face-up
/// (`!inherited_source`) grants; inherited grants escaped it.
///
/// This test mirrors production by injecting the printed inherited text, so it
/// exercises the path the embedded-pack tests above cannot.
#[test]
fn st1_07_inherited_security_attack_plus_not_double_counted_with_printed_text() {
    let lv5_card = make_test_card("PLAIN-LV5", "PlainLv5");

    let mut runner = DebugRunner::builder()
        .dsl_card("ST1-07")
        .expect("ST1-07 in pack")
        .add_card(lv5_card)
        .memory(10)
        .start();

    // Mirror production cards.json: ST1-07's inherited text carries the
    // printed <Security A. +1> keyword (fullwidth brackets, as ingested).
    {
        let game = runner.game_mut();
        for cd in game.card_data.iter_mut() {
            if cd.card_id == "ST1-07" {
                cd.inherited_text =
                    "＜Security A. +1＞ (This Digimon checks 1 additional security card.)"
                        .to_string();
            }
        }
    }

    // Build a digivolve stack: ST1-07 buried under PLAIN-LV5 (the carrier).
    let _greymon = runner.place_on_field(0, "ST1-07", Some(0));
    let greymon_source = {
        let game = runner.game_mut();
        game.players[0].battle_area[0].top_card().clone()
    };
    let carrier = runner.place_on_field(0, "PLAIN-LV5", Some(0));
    {
        let game = runner.game_mut();
        game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .insert(0, greymon_source);
    }

    // Install declarative grants (materializes the inherited grant onto the carrier).
    runner.game_mut().tick_declarative_effects();

    let bonus = runner.game.security_attack_keyword_bonus(carrier);
    assert_eq!(
        bonus, 1,
        "carrier with ST1-07 buried must check exactly +1 additional security card \
         (got {bonus}); the inherited <Security A. +1> must not be double-counted by \
         both the printed-text path and the materialized grant_keyword"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Runtime modifier installation (pending G-DECLARATIVE-KEYWORD)
// ═══════════════════════════════════════════════════════════════════════════════

/// Behavioral: when ST1-07 is placed face-up on the field, its OWN permanent
/// should have SecurityAttackPlus(1) installed via the declarative grant_keyword path.
///
/// Skipped pending G-DECLARATIVE-KEYWORD: DSL declarative `grant_keyword` clauses
/// compile to Effect::declarative(...) with EffectTiming::Declarative, but
/// EffectTiming::Declarative is never enqueued or fired by the engine.
/// The modifier is therefore not installed at runtime for face-up own-scope effects.
///
/// For the inherited case (which is what this card actually uses), runtime
/// installation is handled via has_keyword stack-walk (see Section 2 above) —
/// that path works today. This test targets the face-up runtime modifier path only.
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — EffectTiming::Declarative not yet fired by engine; \
            grant_keyword modifier is compiled but not installed at runtime for face-up scope"]
fn st1_07_security_attack_plus_installed_on_field_via_modifier() {
    let mut runner = greymon_runner();
    let handle = runner.place_on_field(0, "ST1-07", Some(0));
    runner.fire_on_play(0, handle.index as usize);

    let installed = runner
        .modifiers()
        .has_keyword(handle, Keyword::SecurityAttackPlus(1));

    assert!(
        installed,
        "ST1-07 must have SecurityAttackPlus(1) installed via modifier when face-up on field"
    );
}
