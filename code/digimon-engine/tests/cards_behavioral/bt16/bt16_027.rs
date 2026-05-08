//! BT16-027 Imperialdramon: Fighter Mode — Digimon, Lv.6, Blue/Green, DP 13000, Cost 8.
//! Traits: Ancient Dragonkin.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Hand] [Counter] <Blast Digivolve>
//!   (Your Digimon may digivolve into this card without paying the cost.)
//!
//! [On Play] [When Digivolving]
//!   Return 1 of your opponent's Digimon with as many or fewer digivolution
//!   cards as this Digimon to the bottom of the deck.
//!
//! [End of Attack] [Once Per Turn]
//!   Unsuspend this Digimon. Then, if [Imperialdramon: Dragon Mode] is in this
//!   Digimon's digivolution cards, return 1 of your opponent's suspended Digimon
//!   to the bottom of the deck.
//! ```
//!
//! Inherited: `Ace Overflow <-4>` — ACE keyword with -4 memory penalty.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_027.cs
//!
//! # Clause analysis
//!
//! - Clause 0 (Blast Digivolve): Supported via `kind: burst_digivolve` alt-path
//!   + `kind: grant_keyword, keyword: BlastDigivolve`. The activated digivolve from
//!   [Imperialdramon: Dragon Mode] is modelled as `kind: activated_digivolve`.
//!
//! - Clause 1 ([On Play][When Digivolving] return-to-bottom):
//!   BLOCKED — G-PRED-STACK-SIZE-LTE-SOURCE. The `stack_size_lte` DSL predicate
//!   takes a literal u8; there is no `stack_size_lte_source` form that compares
//!   the target's stack size against the source permanent's stack size at runtime.
//!   See qa/dsl-vocab-gaps.md.
//!
//! - Clause 2 ([End of Attack][OPT] unsuspend + conditional return):
//!   BLOCKED — G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME. The `if` condition requires
//!   checking whether "Imperialdramon: Dragon Mode" is in the source permanent's own
//!   digivolution stack. No DSL predicate supports this. See qa/dsl-vocab-gaps.md.
//!
//! - Inherited (Ace Overflow -4): Fully implemented via top-level `ace_overflow: -4`.
//!
//! # Patterns
//! - H12: Blast Digivolve / burst_digivolve alt-path + BlastDigivolve keyword grant
//! - H13: ACE Overflow -4 via `ace_overflow` top-level field
//! - G-PRED-STACK-SIZE-LTE-SOURCE: dynamic stack-size comparison predicate gap
//! - G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME: self-digivolution stack name-check gap

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
};
use digimon_engine::debug_runner::DebugRunner;

// ─── Fixture ─────────────────────────────────────────────────────────────────

fn fighter_mode_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT16-027")
        .expect("BT16-027 YAML loads")
}

// ─── Structural tests ────────────────────────────────────────────────────────

/// Verify basic card metadata matches cards.json.
#[test]
fn bt16_027_metadata_level_cost_dp_color() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    assert_eq!(compiled.name, "Imperialdramon: Fighter Mode");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.cost, Some(8));
    assert_eq!(compiled.dp, Some(13000));
    assert!(
        compiled.color.contains(&CompiledColor::Blue),
        "must be blue; color={:?}",
        compiled.color
    );
    assert!(
        compiled.color.contains(&CompiledColor::Green),
        "must be green; color={:?}",
        compiled.color
    );
}

/// Verify Ace Overflow -4 is recorded on the compiled card.
#[test]
fn bt16_027_ace_overflow_is_minus_4() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let ace = compiled.ace_overflow.expect("ace_overflow must be present");
    assert_eq!(ace, -4, "Ace Overflow must be -4 per printed text");
}

/// Verify the standard digivolve path (Lv.5 Blue / Cost 5) is present.
#[test]
fn bt16_027_has_standard_lv5_blue_digivolve_path() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_standard = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(5))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(5) && f.color_is == Some(CompiledColor::Blue))
    });
    assert!(
        has_standard,
        "must have Lv.5 Blue cost-5 digivolve path; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Verify the activated digivolve path (from Dragon Mode on own field / Cost 2).
#[test]
fn bt16_027_has_activated_digivolve_from_dragon_mode() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_activated = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::ActivatedDigivolve
            && p.cost == Some(CompiledCost::Literal(2))
    });
    assert!(
        has_activated,
        "must have ActivatedDigivolve path with cost 2 for Imperialdramon: Dragon Mode; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Verify the burst digivolve (Blast Digivolve) alt-path is present.
#[test]
fn bt16_027_has_burst_digivolve_alt_path() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_burst = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::BurstDigivolve && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        has_burst,
        "must have BurstDigivolve path with cost 0; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Verify the BlastDigivolve keyword grant declarative clause is present.
#[test]
fn bt16_027_blast_digivolve_keyword_grant_is_declared() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_blast_keyword = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "BlastDigivolve"
        )
    });
    assert!(
        has_blast_keyword,
        "must have a GrantKeyword clause for BlastDigivolve"
    );
}

/// Confirm Clause 2 (on_play/when_digivolving return) is not authored in the
/// current YAML while G-PRED-STACK-SIZE-LTE-SOURCE is open — it must be absent
/// rather than present as an over-permissive approximation.
#[test]
fn bt16_027_on_play_clause_is_absent_while_stack_size_predicate_gap_is_open() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let on_play_triggered = compiled.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(t) => {
            use digimon_dsl::compiled::CompiledTiming;
            t.when.contains(&CompiledTiming::OnPlay)
                || t.when.contains(&CompiledTiming::WhenDigivolving)
        }
        _ => false,
    });
    assert!(
        !on_play_triggered,
        "on_play / when_digivolving clause must not be authored while \
         G-PRED-STACK-SIZE-LTE-SOURCE is open (would be over-permissive)"
    );
}

/// Confirm Clause 3 (end_of_attack unsuspend + conditional) is not authored in
/// the current YAML while G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME is open.
#[test]
fn bt16_027_end_of_attack_clause_is_absent_while_digi_stack_name_gap_is_open() {
    let runner = fighter_mode_runner().start();
    let compiled = runner.compiled_card("BT16-027").expect("BT16-027 compiles");

    let has_end_of_attack = compiled.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(t) => {
            use digimon_dsl::compiled::CompiledTiming;
            t.when.contains(&CompiledTiming::EndOfAttack)
        }
        _ => false,
    });
    assert!(
        !has_end_of_attack,
        "end_of_attack clause must not be authored while \
         G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME is open (conditional arm would be missing)"
    );
}

// ─── Gap-blocked behavioural tests ───────────────────────────────────────────

/// [On Play][When Digivolving]: return 1 opp Digimon with ≤ digi-cards as this.
///
/// Blocked pending G-PRED-STACK-SIZE-LTE-SOURCE in qa/dsl-vocab-gaps.md.
/// The `stack_size_lte` predicate takes a literal u8; no `stack_size_lte_source`
/// variant exists to compare target's stack size against the source permanent's
/// card_sources count dynamically.
#[test]
#[ignore = "pending: G-PRED-STACK-SIZE-LTE-SOURCE from qa/dsl-vocab-gaps.md — stack_size_lte_source predicate not in DSL"]
fn bt16_027_on_play_returns_opp_digimon_with_lte_digi_cards() {
    // When G-PRED-STACK-SIZE-LTE-SOURCE closes:
    // 1. Place this card and an opponent Digimon with ≤ sources on field.
    // 2. Play BT16-027 from hand.
    // 3. Assert SelectionKind::OppField filters only the opponent Digimon with
    //    card_sources.len() <= source.card_sources.len().
    // 4. Execute selection; assert opponent Digimon moves to the bottom of deck.
    // Also: opponent Digimon with MORE digi-cards must be excluded from selection.
}

/// [On Play][When Digivolving]: opponent Digimon with MORE digi-cards is excluded.
///
/// Blocked pending G-PRED-STACK-SIZE-LTE-SOURCE from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-PRED-STACK-SIZE-LTE-SOURCE from qa/dsl-vocab-gaps.md — stack_size_lte_source predicate not in DSL"]
fn bt16_027_on_play_excludes_opp_digimon_with_more_digi_cards() {
    // When G-PRED-STACK-SIZE-LTE-SOURCE closes:
    // Place an opponent Digimon whose card_sources.len() > source.card_sources.len().
    // Play BT16-027 — that opponent Digimon must not appear in the selection.
}

/// [End of Attack][OPT] unsuspend this Digimon.
///
/// The unsuspend arm is not blocked by any gap (unsuspend: { target: source } is valid).
/// The entire clause is BLOCKED because the conditional "Then, if [Dragon Mode] in
/// digi-cards" arm cannot be expressed, and partial implementation would silently drop
/// the conditional return arm — violating no-approximations.
///
/// Blocked pending G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md — self digivolution contains name predicate not in DSL"]
fn bt16_027_end_of_attack_unsuspends_self() {
    // When G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME closes:
    // 1. Place BT16-027 on field (suspended after attacking).
    // 2. Trigger end_of_attack.
    // 3. Assert BT16-027 is unsuspended.
    // 4. If Dragon Mode is NOT in digi-cards: no additional selection.
    // 5. If Dragon Mode IS in digi-cards: assert OppField selection for suspended Digimon.
}

/// [End of Attack][OPT][Once Per Turn] — conditional return fires when Dragon Mode is in digi-cards.
///
/// Blocked pending G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md — self digivolution contains name predicate not in DSL"]
fn bt16_027_end_of_attack_returns_opp_suspended_when_dragon_mode_in_digi_cards() {
    // When G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME closes:
    // 1. Place a stack: [Imperialdramon: Dragon Mode] → BT16-027 on field.
    // 2. Trigger end_of_attack.
    // 3. Self unsuspends.
    // 4. A suspended opponent Digimon must appear in the OppField selection.
    // 5. Execute selection; assert opponent Digimon moves to the bottom of deck.
}

/// [End of Attack][OPT][Once Per Turn] — conditional return does NOT fire when Dragon Mode is absent.
///
/// Blocked pending G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md — self digivolution contains name predicate not in DSL"]
fn bt16_027_end_of_attack_no_selection_when_dragon_mode_absent_from_digi_cards() {
    // When G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME closes:
    // 1. Place BT16-027 on field WITHOUT Dragon Mode in digi-cards.
    // 2. Trigger end_of_attack.
    // 3. Self unsuspends.
    // 4. No selection should appear (conditional return must not fire).
}

/// [End of Attack][OPT][Once Per Turn] OPT enforcement.
///
/// Blocked pending G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md.
#[test]
#[ignore = "pending: G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME from qa/dsl-vocab-gaps.md — self digivolution contains name predicate not in DSL"]
fn bt16_027_end_of_attack_is_once_per_turn() {
    // When G-DSL-SELF-DIGIVOLUTION-CONTAINS-NAME closes:
    // 1. Trigger end_of_attack; assert effect fires (or no-pending-selection if unsuspend is only move).
    // 2. Trigger end_of_attack a second time in the same turn.
    // 3. Assert the OPT lockout prevents a second unsuspend + selection.
}
