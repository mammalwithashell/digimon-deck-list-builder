//! BT12-028 Paildramon — Digimon, Lv.5, Blue/Green, DP 8000, Cost 8.
//! Traits: Dragonkin.
//! Evo: Lv4 Blue / Cost 4 (printed evo_costs).
//! DNA: Lv4 Blue + Lv4 Green / Cost 0.
//!
//! # Card text (cards.json)
//!
//! ```text
//! Effect:
//! [When Digivolving] Trash the top 3 digivolution cards of all of your
//! opponent's Digimon. Then, if DNA digivolving, 2 of your opponent's
//! Digimon with no digivolution cards can't attack until the end of your
//! opponent's turn.
//!
//! Inherited Effect:
//! [End of Attack] If this Digimon has [Imperialdramon] in its name or
//! [Free] trait, gain 1 memory.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_028.cs
//!
//! # Clause coverage
//! - Clause 0a ([When Digivolving] trash top 3 from all opp Digimon):
//!     BLOCKED G-DSL-TRASH-TOP-N-DIGI-CARDS (qa/dsl-vocab-gaps.md)
//! - Clause 0b ([When Digivolving] DNA → CannotAttack 2 targets):
//!     BLOCKED G-DSL-IS-DNA-DIGIVOLVING (qa/dsl-vocab-gaps.md)
//! - Clause 1  ([Inherited][End of Attack] gain memory if carrier name/trait):
//!     BLOCKED G-DSL-SOURCE-NAME-CONTAINS (qa/dsl-vocab-gaps.md)
//!
//! # Patterns covered
//! - G2: DNA digivolve alt-path (dna_digivolve with two materials)
//! - for_each over opponent Digimon (intended; blocked on G-DSL-TRASH-TOP-N-DIGI-CARDS)
//! - Inherited End of Attack conditional memory gain (blocked on G-DSL-SOURCE-NAME-CONTAINS)

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

// ─── Runner factory ──────────────────────────────────────────────────────────

fn paildramon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-028")
        .expect("BT12-028 in embedded DSL pack")
        .memory(8)
        .start()
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

#[test]
fn bt12_028_has_dna_digivolve_alt_path() {
    let runner = paildramon();
    let compiled = runner
        .compiled_card("BT12-028")
        .expect("BT12-028 compiled");

    let dna = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve);
    assert!(
        dna.is_some(),
        "BT12-028 must declare a DnaDigivolve alt-path (Lv4-Blue + Lv4-Green)"
    );

    let dna = dna.unwrap();
    assert_eq!(
        dna.materials.len(),
        2,
        "DNA alt-path must list exactly 2 material predicates"
    );
}

#[test]
fn bt12_028_has_no_effects_due_to_all_clauses_blocked() {
    // All three clauses are blocked on DSL gaps; the YAML authors no effect
    // entries. The compiled card should have zero triggered/declarative clauses.
    let runner = paildramon();
    let compiled = runner
        .compiled_card("BT12-028")
        .expect("BT12-028 compiled");

    assert_eq!(
        compiled.effects.len(),
        0,
        "BT12-028 has no effect entries — all clauses blocked on DSL gaps"
    );
}

// ─── §2 Behavioral — Clause 0a: trash top 3 divi-cards (BLOCKED) ──────────

/// BLOCKED on G-DSL-TRASH-TOP-N-DIGI-CARDS.
///
/// No DSL step trashes N source cards from *below* the top card of a target
/// permanent. The intended primitive would be `trash_top_n_digivolution_cards`
/// which removes up to N cards from `card_sources[..len-1]` (below the top),
/// keeping the active Digimon card on the field.
///
/// `trash_top_source` removes `card_sources.last()` (the top card itself),
/// de-activating the Digimon. `de_digivolve` similarly pops the top card.
/// Both are wrong for "Trash the top 3 digivolution cards" semantics.
#[test]
#[ignore = "pending: G-DSL-TRASH-TOP-N-DIGI-CARDS from qa/dsl-vocab-gaps.md — no DSL verb trashes N source cards from below the top card while keeping the Digimon on the field"]
fn bt12_028_when_digivolving_trashes_top_3_source_cards_from_each_opponent_digimon() {
    // Intended test body (once gap closes):
    //
    // 1. Place an opponent Digimon with a 4-card stack
    //    (top card + 3 digivolution sources).
    // 2. Digivolve BT12-028 onto P0's field to fire the WhenDigivolving batch.
    // 3. Assert: opponent Digimon's stack shrinks from 4 to 1 (only top card remains).
    // 4. Assert: opponent Digimon is still on the field (not deleted).
    // 5. Place a second opponent Digimon with only its top card (no divi cards).
    // 6. Assert: second Digimon's stack is unchanged (no crash on empty source set).
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-TRASH-TOP-N-DIGI-CARDS from qa/dsl-vocab-gaps.md — fires OnDigivolutionCardTrashed per source removed"]
fn bt12_028_when_digivolving_fires_on_digivolution_card_trashed_per_source() {
    // Intended: each of the 3 trashed sources fires OnDigivolutionCardTrashed.
    // For an opponent Digimon with 3+ divi cards, exactly 3 events must fire.
    // For an opponent with 1 divi card, exactly 1 event fires (stops at top card).
    todo!()
}

// ─── §3 Behavioral — Clause 0b: DNA sub-condition (BLOCKED) ─────────────────

/// BLOCKED on G-DSL-IS-DNA-DIGIVOLVING.
///
/// DSL has no `is_dna_digivolving` predicate leaf and the engine's
/// TriggerSource::Digivolved carries no `via_dna` flag. The DNA sub-clause
/// that applies CannotAttack modifiers to 2 opponent Digimon with no divi cards
/// cannot be expressed faithfully.
#[test]
#[ignore = "pending: G-DSL-IS-DNA-DIGIVOLVING from qa/dsl-vocab-gaps.md — engine TriggerSource::Digivolved lacks via_dna flag; DSL has no is_dna_digivolving predicate"]
fn bt12_028_dna_digivolve_applies_cannot_attack_to_two_digimon_with_no_sources() {
    // Intended: when BT12-028 enters via DNA digivolve, a multi-select
    // (up to 2) prompt appears for opponent Digimon with stack_size == 1.
    // Selected Digimon receive CannotAttack expiring end_of_opponents_turn.
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-IS-DNA-DIGIVOLVING from qa/dsl-vocab-gaps.md — standard digivolve must not trigger DNA sub-condition"]
fn bt12_028_standard_digivolve_does_not_trigger_dna_sub_condition() {
    // Negative: standard (non-DNA) digivolving must NOT apply CannotAttack
    // even if opponent Digimon with no divi cards are present.
    todo!()
}

// ─── §4 Behavioral — Clause 1: Inherited [End of Attack] (BLOCKED) ──────────

/// BLOCKED on G-DSL-SOURCE-NAME-CONTAINS.
///
/// `source_name_contains` and `source_permanent_trait_has` are defined in
/// PredicateSpec and compile cleanly, but have no runtime evaluation branch in
/// predicate.rs. The condition check silently no-ops (formula_eval.rs marks
/// them as needing formula context but the evaluator branch is absent).
///
/// The inherited clause is omitted from the YAML; all behavioral tests here
/// are #[ignore]d until the gap closes.
#[test]
#[ignore = "pending: G-DSL-SOURCE-NAME-CONTAINS from qa/dsl-vocab-gaps.md — source_name_contains not evaluated at runtime in predicate.rs"]
fn bt12_028_inherited_end_of_attack_gains_memory_when_carrier_has_imperialdramon_in_name() {
    // Intended: carrier top card name contains "Imperialdramon" → gain 1 memory.
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-SOURCE-NAME-CONTAINS from qa/dsl-vocab-gaps.md — source_permanent_trait_has not evaluated at runtime"]
fn bt12_028_inherited_end_of_attack_gains_memory_when_carrier_has_free_trait() {
    // Intended: carrier has no Imperialdramon name but has [Free] trait → gain 1 memory.
    todo!()
}

#[test]
#[ignore = "pending: G-DSL-SOURCE-NAME-CONTAINS from qa/dsl-vocab-gaps.md — condition must not fire for unrelated carrier"]
fn bt12_028_inherited_end_of_attack_does_not_fire_for_unrelated_carrier() {
    // Negative: carrier has neither "Imperialdramon" in name nor [Free] trait.
    // The inherited End of Attack must not gain memory.
    todo!()
}

// ─── §5 Inherited scope structural check (BLOCKED) ───────────────────────────

/// When all inherited-clause gaps close, this test verifies the structural
/// shape: scope=Inherited, when=EndOfAttack, not optional, no OPT flag.
#[test]
#[ignore = "pending: G-DSL-SOURCE-NAME-CONTAINS from qa/dsl-vocab-gaps.md — inherited clause not yet authored in YAML"]
fn bt12_028_inherited_clause_is_end_of_attack_non_optional() {
    use digimon_dsl::compiled::CompiledClause;

    let runner = paildramon();
    let compiled = runner
        .compiled_card("BT12-028")
        .expect("BT12-028 compiled");

    let inherited: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(inherited.len(), 1, "BT12-028 has exactly one inherited clause");
    let clause = &inherited[0];
    assert!(
        clause.when.contains(&CompiledTiming::EndOfAttack),
        "inherited clause fires at EndOfAttack"
    );
    assert!(!clause.optional, "inherited clause is mandatory (no 'you may')");
    assert!(!clause.once_per_turn, "inherited clause has no OPT restriction");
}
