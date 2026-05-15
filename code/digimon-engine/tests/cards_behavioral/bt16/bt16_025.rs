//! BT16-025 Paildramon — Digimon, Lv.5, Blue/Green, DP 8000, Cost 8.
//! Traits: Dragonkin
//!
//! # Card text (cards.json — printed)
//!
//! ```text
//! ＜Partition (Blue Lv.4 & Green Lv.4)＞
//! [When Digivolving] Suspend all of your opponent's Digimon with as many
//!   or fewer digivolution cards as this Digimon. Then, if DNA digivolving,
//!   none of their Digimon can unsuspend until the end of their turn.
//! [When Attacking] [Once Per Turn] Suspend 1 of your opponent's unsuspended
//!   Digimon. If this effect didn't suspend, unsuspend this Digimon.
//!
//! Inherited:
//! ＜Partition (Blue Lv.4 & Green Lv.4)＞
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_025.cs
//!
//! # Source priority
//! Per CLAUDE.md: printed text > RULES_CONTEXT.md (§16-28 Partition) > fandom > DCGO.
//!
//! # Patterns this test covers
//! - Partition declarative (face-up scope) with level + color source predicates
//! - Partition declarative (inherited scope) — same sources
//! - DNA digivolve alt-path (Blue Lv.4 + Green Lv.4, cost 0)
//! - [When Digivolving] triggered clause:
//!     - Primary suspend-by-stack-size-comparison: BLOCKED G-DSL-STACK-SIZE-LTE-SOURCE
//!     - No WhenDigivolving clause is shipped in the YAML (body is fully blocked)
//! - [On DNA Digivolve] CannotUnsuspend all opp Digimon: IMPLEMENTED
//!     - Modeled as `on_dna_digivolve` timing rather than a conditional inside
//!       `when_digivolving`, because the `dna_origin` predicate in process-step
//!       `if` conditions is not evaluated by the engine (G-ENGINE-DNA-ORIGIN-PRED gap).
//! - [When Attacking][OPT] triggered clause — PARTIAL:
//!     - Suspend 1 opp unsuspended Digimon: IMPLEMENTED
//!     - Unsuspend-self fallback "if this effect didn't suspend": BLOCKED
//!       G-DSL-EFFECT-SUSPENDED-RESULT
//!
//! # Engine gaps that block full clause implementation
//!
//! ## G-DSL-STACK-SIZE-LTE-SOURCE
//! The "suspend all opp Digimon with as many or fewer digivolution cards as this
//! Digimon" step requires a `stack_size_lte` predicate that evaluates against the
//! SOURCE permanent's current stack size at runtime. The DSL's `stack_size_lte` takes
//! only a literal `u8`, not a source-relative reference.
//! A new predicate leaf `stack_size_lte_source: bool` would resolve to
//! `candidate.card_sources.len() <= ctx.source_permanent().card_sources.len()`.
//! Tracked in: qa/dsl-vocab-gaps.md
//!
//! ## G-DSL-EFFECT-SUSPENDED-RESULT
//! "If this effect didn't suspend, unsuspend this Digimon" requires branching on
//! whether the current effect actually suspended an opponent/any Digimon. The DSL
//! now has `binding_is_none` / `binding_absent`, but using selection absence would
//! approximate the printed result check and miss cases where a selected target was
//! not suspended. The current result-log predicate family includes
//! `effect_suspended_any_own_digimon`; BT16-025 needs an opponent/any-Digimon
//! suspend-result predicate before this fallback can be authored faithfully.
//! Cross-reference: qa/dsl-vocab-gaps.md still carries the older
//! G-DSL-IF-NO-TARGET wording for this card.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Build a minimal Digimon card data with specified level and DP.
fn make_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

/// Standard fixture: Paildramon registered in the embedded DSL pack.
fn paildramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .memory(20)
        .start()
}

/// Walk all_of/any_of/none_of/not branches looking for any leaf satisfying `f`.
fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    if f(p) {
        return true;
    }
    if p.all_of.iter().any(|q| pred_any(q, f)) {
        return true;
    }
    if p.any_of.iter().any(|q| pred_any(q, f)) {
        return true;
    }
    if p.none_of.iter().any(|q| pred_any(q, f)) {
        return true;
    }
    if let Some(ref n) = p.not {
        if pred_any(n, f) {
            return true;
        }
    }
    false
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt16_025_compiles_in_embedded_pack() {
    let runner = paildramon_runner();
    assert!(
        runner.compiled_card("BT16-025").is_some(),
        "BT16-025 must be present in embedded DSL pack"
    );
}

#[test]
fn bt16_025_card_metadata_matches_print() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    assert_eq!(card.name, "Paildramon");
    assert_eq!(card.level, Some(5), "Paildramon is Lv.5");
    assert_eq!(card.dp, Some(8000), "Paildramon has DP 8000");
    assert_eq!(card.cost, Some(8), "Paildramon costs 8 to play");
    assert!(
        card.color.contains(&CompiledColor::Blue),
        "Paildramon must be Blue"
    );
    assert!(
        card.color.contains(&CompiledColor::Green),
        "Paildramon must be Green"
    );
    assert!(
        card.traits.iter().any(|t| t == "Dragonkin"),
        "Paildramon has Dragonkin trait"
    );
}

/// Face-up Partition clause with Blue Lv.4 + Green Lv.4 sources must exist.
#[test]
fn bt16_025_has_face_up_partition_with_blue_lv4_and_green_lv4() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    let partition = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
            scope: CompiledScope::FaceUp,
            sources,
            ..
        }) => Some(sources),
        _ => None,
    });
    let sources = partition.expect("BT16-025 must declare a face-up Partition clause");

    assert_eq!(
        sources.len(),
        2,
        "Partition must list exactly 2 source predicates (Blue Lv.4 + Green Lv.4)"
    );

    let has_blue_lv4 = sources.iter().any(|p| {
        pred_any(p, |q| {
            q.level_eq == Some(4) && q.color_is == Some(CompiledColor::Blue)
        })
    });
    let has_green_lv4 = sources.iter().any(|p| {
        pred_any(p, |q| {
            q.level_eq == Some(4) && q.color_is == Some(CompiledColor::Green)
        })
    });

    assert!(
        has_blue_lv4,
        "Partition sources must include Blue Lv.4 (printed text)"
    );
    assert!(
        has_green_lv4,
        "Partition sources must include Green Lv.4 (printed text)"
    );
}

/// Inherited Partition clause with Blue Lv.4 + Green Lv.4 sources must exist.
#[test]
fn bt16_025_has_inherited_partition_with_blue_lv4_and_green_lv4() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    let partition = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
            scope: CompiledScope::Inherited,
            sources,
            ..
        }) => Some(sources),
        _ => None,
    });
    let sources = partition.expect("BT16-025 must declare an inherited Partition clause");

    assert_eq!(
        sources.len(),
        2,
        "Inherited Partition must list exactly 2 source predicates (Blue Lv.4 + Green Lv.4)"
    );

    let has_blue_lv4 = sources.iter().any(|p| {
        pred_any(p, |q| {
            q.level_eq == Some(4) && q.color_is == Some(CompiledColor::Blue)
        })
    });
    let has_green_lv4 = sources.iter().any(|p| {
        pred_any(p, |q| {
            q.level_eq == Some(4) && q.color_is == Some(CompiledColor::Green)
        })
    });

    assert!(has_blue_lv4, "Inherited Partition must include Blue Lv.4");
    assert!(has_green_lv4, "Inherited Partition must include Green Lv.4");
}

/// Both Partition clauses must exclude `battle` and `own_effect` per DCGO and
/// RULES_CONTEXT.md §16-28 ("not by your own effects or by battle").
#[test]
fn bt16_025_partition_clauses_exclude_battle_and_own_effect() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    let partitions: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
                exclude_cause,
                ..
            }) => Some(exclude_cause),
            _ => None,
        })
        .collect();

    assert_eq!(
        partitions.len(),
        2,
        "BT16-025 should have exactly 2 Partition clauses (face-up + inherited)"
    );

    for (i, xs) in partitions.iter().enumerate() {
        assert!(
            xs.iter().any(|s| s == "battle"),
            "Partition clause {i} must exclude `battle` per Rules Manual §16-28"
        );
        assert!(
            xs.iter().any(|s| s == "own_effect"),
            "Partition clause {i} must exclude `own_effect` per printed text \
             'other than by your own effects or by battle'"
        );
    }
}

/// Standard digivolve alt-path (Lv.4, cost 4) must be present.
#[test]
fn bt16_025_has_standard_digivolve_lv4_cost4() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    let standard = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .find(|p| {
            p.cost == Some(CompiledCost::Literal(4))
                && p.from.as_ref().is_some_and(|f| f.level_eq == Some(4))
        });
    assert!(
        standard.is_some(),
        "BT16-025 must have a standard digivolve path from Lv.4 costing 4"
    );
}

/// DNA digivolve alt-path (Blue Lv.4 + Green Lv.4, cost 0) must be present.
#[test]
fn bt16_025_has_dna_digivolve_blue_lv4_green_lv4_cost0() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    let dna_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();

    assert_eq!(
        dna_paths.len(),
        1,
        "BT16-025 must have exactly 1 DNA digivolve alt-path"
    );
    let path = dna_paths[0];
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "DNA digivolve cost is 0 (printed text: Lv.4 Blue + Lv.4 Green)"
    );
    assert_eq!(
        path.materials.len(),
        2,
        "DNA digivolve requires exactly 2 materials"
    );

    let has_blue_lv4 = path.materials.iter().any(|m| {
        pred_any(&m.filter, |q| {
            q.level_eq == Some(4) && q.color_is == Some(CompiledColor::Blue)
        })
    });
    let has_green_lv4 = path.materials.iter().any(|m| {
        pred_any(&m.filter, |q| {
            q.level_eq == Some(4) && q.color_is == Some(CompiledColor::Green)
        })
    });

    assert!(
        has_blue_lv4,
        "DNA materials must include Blue Lv.4 (printed text)"
    );
    assert!(
        has_green_lv4,
        "DNA materials must include Green Lv.4 (printed text)"
    );
}

/// [On DNA Digivolve] triggered clause must exist, implementing the "if DNA
/// digivolving, none of their Digimon can unsuspend" branch of the printed text.
/// Modeled as `on_dna_digivolve` timing rather than a `when_digivolving` clause
/// with a `dna_origin` process-`if` condition, because that predicate is not
/// evaluated by the engine's process-step `if` evaluator.
#[test]
fn bt16_025_has_on_dna_digivolve_cannot_unsuspend_clause() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    let dna_clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnDnaDigivolve));

    assert!(
        dna_clause.is_some(),
        "BT16-025 must have an [On DNA Digivolve] triggered clause for CannotUnsuspend"
    );
    let clause = dna_clause.unwrap();
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "[On DNA Digivolve] clause must have FaceUp scope"
    );
    assert!(
        !clause.optional,
        "[On DNA Digivolve] CannotUnsuspend is mandatory when DNA digivolving (no 'you may')"
    );
}

/// [When Attacking][Once Per Turn] triggered clause must exist and be OPT.
#[test]
fn bt16_025_has_when_attacking_once_per_turn_optional_clause() {
    let runner = paildramon_runner();
    let card = runner.compiled_card("BT16-025").expect("BT16-025 compiles");

    let wa = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking));

    assert!(
        wa.is_some(),
        "BT16-025 must have a [When Attacking] triggered clause"
    );
    let wa = wa.unwrap();
    assert_eq!(
        wa.scope,
        CompiledScope::FaceUp,
        "[When Attacking] clause must have FaceUp scope"
    );
    assert!(
        wa.once_per_turn,
        "[When Attacking] clause is [Once Per Turn] per printed text"
    );
    assert!(
        wa.optional,
        "[When Attacking] clause is [Once Per Turn] (implied optional — \
         the player may decline to use it this attack)"
    );
}

// ─── SECTION 2 — Behavioral: [On DNA Digivolve] CannotUnsuspend sub-block ────

/// DNA path: when Paildramon digivolves via DNA, all opp Digimon gain
/// CannotUnsuspend until end of their turn.
///
/// Uses `EffectContext::effect_initiated_dna_digivolve` with `ignore_requirements=true`
/// to trigger the DNA path. The `on_dna_digivolve` timing fires automatically as
/// part of the DNA merge procedure and applies CannotUnsuspend to all opp battle-area
/// Digimon with `expiry: end_of_opponents_turn`.
#[test]
fn bt16_025_on_dna_digivolve_applies_cannot_unsuspend_to_all_opp_digimon() {
    use digimon_engine::enums::ModifierType;

    let mat_a = make_digimon("MAT-A", "MaterialA", 4, 4000);
    let mat_b = make_digimon("MAT-B", "MaterialB", 4, 4000);
    let opp_a = make_digimon("OPP-A", "OppA", 4, 4000);
    let opp_b = make_digimon("OPP-B", "OppB", 5, 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(mat_a)
        .add_card(mat_b)
        .add_card(opp_a)
        .add_card(opp_b)
        .hand(0, &["BT16-025"])
        .memory(20)
        .start();

    let material_a = runner.place_on_field(0, "MAT-A", None);
    let material_b = runner.place_on_field(0, "MAT-B", None);
    let opp_perm_a = runner.place_on_field(1, "OPP-A", None);
    let opp_perm_b = runner.place_on_field(1, "OPP-B", None);

    // Retrieve hand card handle before consuming the borrow.
    let hand_card = runner.game.player(0).hand[0].handle();

    // Trigger DNA digivolve using EffectContext (ignore_requirements=true
    // bypasses the DNA cost validity checks since CardData.dna_costs is empty
    // for DSL cards; the actual effect is still fired with dna_origin=true and
    // the on_dna_digivolve timing fires automatically).
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true);
    }

    assert!(
        runner
            .game
            .modifiers
            .has(opp_perm_a, ModifierType::CannotUnsuspend),
        "opp_perm_a must have CannotUnsuspend after DNA digivolve"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(opp_perm_b, ModifierType::CannotUnsuspend),
        "opp_perm_b must have CannotUnsuspend after DNA digivolve"
    );
}

/// Non-DNA path: when Paildramon digivolves normally (not DNA), opp Digimon
/// must NOT receive CannotUnsuspend — the `on_dna_digivolve` timing does not fire
/// for standard digivolves.
#[test]
fn bt16_025_non_dna_digivolve_does_not_apply_cannot_unsuspend() {
    use digimon_engine::enums::ModifierType;

    let opp_d = make_digimon("OPP-D", "OppDigimon", 5, 6000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(opp_d)
        .memory(20)
        .start();

    let paildramon = runner.place_on_field(0, "BT16-025", Some(0));
    let opp = runner.place_on_field(1, "OPP-D", Some(0));

    // Fire WhenDigivolving without DNA context — on_dna_digivolve should NOT fire.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(paildramon),
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("resolve non-DNA WhenDigivolving");

    assert!(
        !runner
            .game
            .modifiers
            .has(opp, ModifierType::CannotUnsuspend),
        "non-DNA digivolve must NOT grant CannotUnsuspend to opp Digimon \
         (on_dna_digivolve timing does not fire for standard digivolves)"
    );
}

/// CannotUnsuspend granted by DNA digivolve must expire at end of the
/// opponent's turn (not before, not after).
#[test]
fn bt16_025_cannot_unsuspend_expires_at_end_of_opponents_turn() {
    use digimon_engine::enums::ModifierType;

    let mat_a = make_digimon("MAT-C", "MaterialC", 4, 4000);
    let mat_b = make_digimon("MAT-D", "MaterialD", 4, 4000);
    let opp_d = make_digimon("OPP-E", "OppExpiry", 5, 5000);
    let filler = make_test_card("FILL", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(mat_a)
        .add_card(mat_b)
        .add_card(opp_d)
        .add_card(filler)
        .hand(0, &["BT16-025"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(20)
        .start();

    let material_a = runner.place_on_field(0, "MAT-C", None);
    let material_b = runner.place_on_field(0, "MAT-D", None);
    let opp = runner.place_on_field(1, "OPP-E", None);

    let hand_card = runner.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true);
    }

    assert!(
        runner
            .game
            .modifiers
            .has(opp, ModifierType::CannotUnsuspend),
        "CannotUnsuspend must be active immediately after DNA digivolve"
    );

    // Player 0's turn ends → becomes Player 1's turn.
    runner.end_turn();
    assert!(
        runner
            .game
            .modifiers
            .has(opp, ModifierType::CannotUnsuspend),
        "CannotUnsuspend must persist during the opponent's turn"
    );

    // Player 1's turn ends → Player 0's turn; modifier must now be gone.
    runner.end_turn();
    assert!(
        !runner
            .game
            .modifiers
            .has(opp, ModifierType::CannotUnsuspend),
        "CannotUnsuspend must expire after the opponent's turn ends"
    );
}

// ─── SECTION 3 — Behavioral: [When Attacking][OPT] suspend ───────────────────

/// Positive path: when opponent has an unsuspended Digimon, [When Attacking][OPT]
/// installs an OppField selection prompt.
#[test]
fn bt16_025_when_attacking_installs_opp_field_selection_when_targets_exist() {
    let opp_d = make_digimon("OPP-ACT", "OppActive", 4, 4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(opp_d)
        .memory(20)
        .start();

    let paildramon = runner.place_on_field(0, "BT16-025", Some(0));
    let _opp = runner.place_on_field(1, "OPP-ACT", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(paildramon),
    );
    runner.game.drain_effect_queue();

    let kind = runner.pending_kind();
    assert_eq!(
        kind,
        Some(SelectionKind::OppField),
        "[When Attacking] must offer an OppField selection targeting unsuspended opp Digimon"
    );
    assert!(
        runner.pending_is_optional(),
        "[Once Per Turn] (OPT) implies the player may decline the activation"
    );
}

/// Negative path: when all opponent Digimon are already suspended, the selection
/// has no eligible targets. The DSL's `optional: true` on the select step must
/// allow skipping without crashing.
///
/// Note: the self-unsuspend fallback ("If this effect didn't suspend, unsuspend
/// this Digimon") is BLOCKED (G-DSL-EFFECT-SUSPENDED-RESULT). `binding_is_none`
/// can identify this no-target path, but using that alone would approximate the
/// printed result check. Once an opponent/any suspend-result predicate closes and
/// the fallback is wired, this test should additionally assert that Paildramon
/// becomes unsuspended after the skipped selection.
#[test]
fn bt16_025_when_attacking_no_eligible_targets_selection_skipped() {
    let opp_d = make_digimon("OPP-SUS", "OppSuspended", 4, 4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(opp_d)
        .memory(20)
        .start();

    let paildramon = runner.place_on_field(0, "BT16-025", Some(0));
    let opp = runner.place_on_field(1, "OPP-SUS", Some(0));

    // Suspend the opponent's Digimon so no unsuspended targets exist.
    runner.game.players[1].battle_area[opp.index as usize].is_suspended = true;

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(paildramon),
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("resolve with no eligible targets");

    // No pending selection should remain; the step was skipped cleanly.
    assert!(
        runner.pending_selection().is_none(),
        "when no unsuspended targets exist, WhenAttacking must complete without \
         leaving a pending selection"
    );

    // The suspended opp Digimon should remain suspended (we didn't unsuspend it).
    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "opp Digimon must remain suspended — we did not select or suspend it"
    );
}

/// Behavioral suspend: executing the selection suspends the targeted opp Digimon.
#[test]
fn bt16_025_when_attacking_suspends_selected_opp_digimon() {
    let opp_d = make_digimon("OPP-TGT", "OppTarget", 4, 4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(opp_d)
        .memory(20)
        .start();

    let paildramon = runner.place_on_field(0, "BT16-025", Some(0));
    let opp = runner.place_on_field(1, "OPP-TGT", Some(0));

    assert!(
        !runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "precondition: opp Digimon is unsuspended before WhenAttacking fires"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(paildramon),
    );
    runner.game.drain_effect_queue();

    // Drive the OppField selection to target the one available opp Digimon.
    let view = runner
        .pending_selection_view()
        .expect("selection must install");
    assert_eq!(view.kind, SelectionKind::OppField);
    let action_id = view.valid_action_ids[0];
    runner.execute_action(0, action_id).expect("select target");
    runner.auto_resolve().expect("finish effect");

    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "targeted opp Digimon must be suspended after WhenAttacking effect resolves"
    );
}

/// OPT enforcement: after firing [When Attacking] once, a second WhenAttacking
/// trigger in the same turn must not install a new selection.
#[test]
fn bt16_025_when_attacking_opt_blocks_second_activation_same_turn() {
    let opp_d = make_digimon("OPP-OPT1", "OppOpt1", 4, 4000);
    let opp_e = make_digimon("OPP-OPT2", "OppOpt2", 4, 4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(opp_d)
        .add_card(opp_e)
        .memory(20)
        .start();

    let paildramon = runner.place_on_field(0, "BT16-025", Some(0));
    let _opp_a = runner.place_on_field(1, "OPP-OPT1", Some(0));
    let _opp_b = runner.place_on_field(1, "OPP-OPT2", Some(0));

    // First activation — fire WhenAttacking and resolve.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(paildramon),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("first activation resolves");

    // Second activation same turn — OPT must block.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(paildramon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must block the second WhenAttacking activation in the same turn"
    );
}

// ─── SECTION 4 — Gap-blocked tests ───────────────────────────────────────────

/// BLOCKED: the [When Digivolving] step "Suspend all opp Digimon with as many
/// or fewer digivolution cards as this Digimon" requires a `stack_size_lte_source`
/// DSL predicate that dynamically compares against the SOURCE permanent's stack
/// size at runtime. The DSL only supports literal `stack_size_lte: <u8>` today.
#[test]
#[ignore = "pending: G-DSL-STACK-SIZE-LTE-SOURCE — no DSL predicate for \
            stack_size_lte compared against source permanent's stack size at runtime. \
            Tracked in qa/dsl-vocab-gaps.md."]
fn bt16_025_when_digivolving_suspends_all_opp_digimon_with_lte_digivolution_cards() {
    let mat_a = make_digimon("MAT-E", "MaterialE", 4, 4000);
    let mat_b = make_digimon("MAT-F", "MaterialF", 4, 4000);
    let opp_0_sources = make_digimon("OPP-X0", "Opp0Sources", 3, 3000);
    let opp_1_source = make_digimon("OPP-X1", "Opp1Source", 4, 4000);
    let opp_2_sources = make_digimon("OPP-X2", "Opp2Sources", 5, 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .add_card(mat_a)
        .add_card(mat_b)
        .add_card(opp_0_sources)
        .add_card(opp_1_source)
        .add_card(opp_2_sources)
        .hand(0, &["BT16-025"])
        .memory(20)
        .start();

    let material_a = runner.place_on_field(0, "MAT-E", None);
    let material_b = runner.place_on_field(0, "MAT-F", None);

    // Paildramon will have 2 digivolution cards (mat_a + mat_b merged as its
    // digivolution stack). For this test:
    //   opp_a (0 digi-cards ≤ 2) → should be suspended
    //   opp_b (1 digi-card  ≤ 2) → should be suspended
    //   opp_c (3 digi-cards > 2) → should NOT be suspended
    let opp_a = runner.place_on_field(1, "OPP-X0", None);
    let opp_b = runner.place_on_field(1, "OPP-X1", None);
    runner.push_source(opp_b, "OPP-X0"); // opp_b has 1 digi-card under it
    let opp_c = runner.place_on_field(1, "OPP-X2", None);
    runner.push_source(opp_c, "OPP-X0");
    runner.push_source(opp_c, "OPP-X0");
    runner.push_source(opp_c, "OPP-X0"); // opp_c has 3 digi-cards under it

    let hand_card = runner.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(material_a, material_b, hand_card, 0, true);
    }

    assert!(
        runner.game.players[1].battle_area[opp_a.index as usize].is_suspended,
        "opp_a (0 digi-cards ≤ 2) must be suspended"
    );
    assert!(
        runner.game.players[1].battle_area[opp_b.index as usize].is_suspended,
        "opp_b (1 digi-card ≤ 2) must be suspended"
    );
    assert!(
        !runner.game.players[1].battle_area[opp_c.index as usize].is_suspended,
        "opp_c (3 digi-cards > 2) must NOT be suspended"
    );
}

/// BLOCKED: the [When Attacking] self-unsuspend fallback "If this effect didn't
/// suspend, unsuspend this Digimon" requires an `if` condition that branches on
/// whether this effect actually suspended an opponent/any Digimon. The DSL now
/// has `binding_is_none`, but that would only detect selection absence. The
/// available result-log predicate `effect_suspended_any_own_digimon` explicitly
/// ignores opponent suspends, so BT16-025 still needs an opponent/any suspend
/// result predicate before this can be implemented without approximation.
#[test]
#[ignore = "pending: G-DSL-EFFECT-SUSPENDED-RESULT — no DSL predicate for branching \
            on whether this effect suspended an opponent/any Digimon. qa/dsl-vocab-gaps.md \
            still has older G-DSL-IF-NO-TARGET wording for this card."]
fn bt16_025_when_attacking_unsuspends_self_when_effect_did_not_suspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-025")
        .expect("BT16-025 must be in embedded DSL pack")
        .memory(20)
        .start();

    let paildramon = runner.place_on_field(0, "BT16-025", Some(0));
    // Suspend Paildramon itself (simulating an attack state).
    runner.game.players[0].battle_area[paildramon.index as usize].is_suspended = true;

    // No opp Digimon → selection skipped → self-unsuspend should fire.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(paildramon),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve with no targets");

    assert!(
        !runner.game.players[0].battle_area[paildramon.index as usize].is_suspended,
        "Paildramon must unsuspend itself when the suspend effect finds no target"
    );
}
