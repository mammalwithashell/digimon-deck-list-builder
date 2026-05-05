//! ST9-06 Imperialdramon Dragon Mode — Digimon, Lv.6, Blue+Green, DP 12000, Cost 13.
//! Traits: Ancient Dragon.
//!
//! # Card text (cards.json — verbatim)
//!
//! [When Digivolving] You may play 1 level 4 or lower blue Digimon card and
//! 1 level 4 or lower green Digimon card from this Digimon's digivolution
//! cards without paying their memory costs.
//!
//! Inherited / Security: (none)
//!
//! Standard digivolve: Lv.5 Blue / Cost 4.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST9/Blue/ST9_06.cs
//!
//! # Patterns this test covers
//!
//! - Single standard digivolve alt-path: Lv.5 Blue / Cost 4.
//! - [When Digivolving] optional two-pick from own digivolution stack
//!   (blue Lv<=4 + green Lv<=4) played without cost — entirely BLOCKED on
//!   G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (see YAML header).
//!
//! # Known engine/DSL gaps blocking parts of this card
//!
//! - **G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES** (existing, first filed for
//!   EX4-060 / BT22-015 sibling, confirmed by EX9-021). No DSL bind step
//!   `select_self_digivolution_source` and no DSL play step
//!   `play_from_own_digivolution_free` exist. The entire [When Digivolving]
//!   clause (both the blue pick and the green pick) is OMITTED from `effects:`
//!   per the no-approximations rule. All behavioral tests for the clause are
//!   `#[ignore]`'d under this gap tag.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, PlayerId};

/// Production YAML for ST9-06, loaded at compile time.
const YAML: &str = include_str!("../../../cards/st9/ST9-06.yaml");

// ─── Compile helpers ──────────────────────────────────────────────────────────

fn compiled_st9_06() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("ST9-06.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("ST9-06.yaml compiles");
    registry
        .lookup("ST9-06")
        .expect("ST9-06 in registry")
        .clone()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// The YAML must parse and compile without errors.
#[test]
fn st9_06_compiles() {
    let _ = compiled_st9_06();
}

/// Card metadata must match the printed card (Lv.6, DP 12000, Cost 13).
#[test]
fn st9_06_card_metadata_matches_print() {
    let compiled = compiled_st9_06();
    assert_eq!(compiled.level, Some(6), "Imperialdramon Dragon Mode is Lv.6");
    assert_eq!(compiled.dp, Some(12000), "DP is 12000");
    assert_eq!(compiled.cost, Some(13), "play cost is 13");
}

/// Standard digivolve alt-path: Lv.5 Blue / Cost 4 (from evo_costs in cards.json).
#[test]
fn st9_06_has_standard_digivolve_lv5_blue_cost4() {
    let compiled = compiled_st9_06();
    let path = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && matches!(p.cost, Some(CompiledCost::Literal(4)))
                && p.from
                    .as_ref()
                    .and_then(|f| f.level_eq)
                    .map_or(false, |l| l == 5)
        })
        .expect("ST9-06 must have a Lv.5 digivolve path with cost 4");
    assert!(!path.ignore_requirements);
}

/// Exactly one alt-path (the standard digivolve; no DNA / DigiXros paths).
#[test]
fn st9_06_has_exactly_one_alt_path() {
    let compiled = compiled_st9_06();
    assert_eq!(
        compiled.alt_paths.len(),
        1,
        "ST9-06 has exactly 1 alt-path (standard digivolve Lv.5 Blue / Cost 4)"
    );
}

/// No face-up triggered clauses: the [When Digivolving] clause is entirely
/// BLOCKED on G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES and omitted from effects:.
#[test]
fn st9_06_has_no_face_up_triggered_clauses() {
    let compiled = compiled_st9_06();
    let face_up_triggered = compiled
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Triggered(t)
                    if matches!(t.scope, CompiledScope::FaceUp)
            )
        })
        .count();
    assert_eq!(
        face_up_triggered, 0,
        "ST9-06 [When Digivolving] clause is BLOCKED on \
         G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES and must be absent from effects:"
    );
}

// ─── Section 2: BLOCKED test stubs (#[ignore]'d under tracked gap tags) ─────

/// BLOCKED — G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES.
///
/// Printed: "[When Digivolving] You may play 1 level 4 or lower blue Digimon
/// card and 1 level 4 or lower green Digimon card from this Digimon's
/// digivolution cards without paying their memory costs."
///
/// Positive path (outer optional accepted, both eligible cards exist):
/// Setup would require stacking at least one Lv<=4 blue Digimon and one Lv<=4
/// green Digimon under ST9-06 on the field, triggering [When Digivolving],
/// accepting the outer optional prompt, picking the blue candidate, picking the
/// green candidate, and observing both played onto own battle area without
/// paying their memory costs.
#[test]
#[ignore = "pending: G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES — no DSL select_self_digivolution_source or play_from_own_digivolution_free step; see EX4-060 / EX9-021 for prior art"]
fn st9_06_when_digivolving_plays_blue_and_green_from_stack_free() {
    unreachable!(
        "unblock: add select_self_digivolution_source + play_from_own_digivolution_free \
         to DSL and EffectContext"
    );
}

/// BLOCKED — G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (negative: no eligible blue).
///
/// When the digivolution stack contains no Lv<=4 blue Digimon, the blue
/// pick step has no legal candidates. Per DCGO CanActivateCondition, the
/// outer optional should not present if no eligible cards exist in either
/// color filter. The effect must silently no-op.
#[test]
#[ignore = "pending: G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES — cannot verify activation gate without the selection step"]
fn st9_06_when_digivolving_no_blue_candidate_does_not_prompt() {
    unreachable!("unblock: same gap as above");
}

/// BLOCKED — G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (negative: no eligible green).
///
/// When the digivolution stack contains no Lv<=4 green Digimon (but has a
/// blue one), DCGO's CanActivateCondition checks for at least one card
/// satisfying EITHER filter (blue OR green) before enabling the optional.
/// If a blue candidate exists but no green candidate does, the outer optional
/// fires and the blue pick proceeds, but the green pick step has no legal
/// targets. The engine must handle this gracefully (no-op or auto-skip the
/// second pick).
#[test]
#[ignore = "pending: G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES — cannot test partial candidate scenarios without selection step"]
fn st9_06_when_digivolving_no_green_candidate_skips_green_pick() {
    unreachable!("unblock: same gap as above");
}

/// BLOCKED — G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES (decline the outer optional).
///
/// "You may" on the outer clause means the player can decline the entire
/// effect. When the player declines, no cards are played and the digivolution
/// stack is unchanged.
#[test]
#[ignore = "pending: G-PLAY-FROM-OWN-DIGIVOLUTION-SOURCES — optional outer clause cannot be presented without selection step"]
fn st9_06_when_digivolving_declining_does_nothing() {
    unreachable!("unblock: same gap as above");
}
