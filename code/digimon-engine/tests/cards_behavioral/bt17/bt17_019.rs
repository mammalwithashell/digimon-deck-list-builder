//! BT17-019 Gabumon — Digimon, Lv.3, Blue, DP 1000, Cost 3.
//! Traits: Reptile.
//!
//! # Card text (cards.json)
//!
//! [Start of Your Main Phase] If you have a Tamer with [Matt Ishida] in its
//! name, ＜Draw 1＞ (Draw 1 card from your deck.)
//!
//! Inherited Effect [End of Your Turn] This Digimon and any of your other
//! Digimon may DNA digivolve into a Digimon card in the hand.
//!
//! Digivolve: Lv.2 Blue / cost 0.
//! (No Tsunomon name-bonus printed on this card — sister card BT17-007
//! has the Koromon-name digivolve; BT15-020 has the Tsunomon-name
//! digivolve. BT17-019's printed card has only the generic Lv.2 Blue
//! cost 0 digivolve plus the inherited end-of-turn DNA registration.)
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT17/Blue/BT17_019.cs`
//!
//! Notable DCGO contracts:
//!   - Start-of-Main-Phase clause: `SetUpActivateClass(..., isOptional: false, ...)`
//!     plus `CanActivateCondition` requiring (a) own Matt-Ishida tamer on
//!     the field AND (b) deck size >= 1. The "Draw 1" itself takes no
//!     player choice — it is a deterministic process step, not a
//!     selection. There is no "may decline draw" prompt; once gating is
//!     met, the draw fires.
//!   - End-of-Your-Turn inherited clause: `SetUpActivateClass(..., isOptional: true, ...)`
//!     consistent with the printed "may DNA digivolve". Implemented as an
//!     `alt_path_registration` declarative (scope: inherited,
//!     trigger: end_of_your_turn, registers: dna_digivolve) per the
//!     RESOLVED-2026-05-02 vocab note in qa/dsl-vocab-gaps.md.
//!   - Self-digivolution-requirement static effect: there is an
//!     `AddSelfDigivolutionRequirementStaticEffect` block at
//!     `EffectTiming.None` for "Tsunomon" — but that block has
//!     `digivolutionCost: 0` AND `ignoreDigivolutionRequirement: false`.
//!     This is REDUNDANT with the standard Lv.2 Blue / cost 0 digivolve
//!     path printed on the card (Tsunomon IS a Lv.2 Blue Digimon). The
//!     YAML therefore authors only the standard digivolve alt-path; no
//!     separate name-anchored alt-path is needed because the standard
//!     path already covers Tsunomon at the same cost. Verified by
//!     checking BT17-007 (Agumon) — DCGO has the same redundant block
//!     for Koromon and the BT17-007 YAML similarly authors a single
//!     digivolve alt-path.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//!   - B1 Start-of-main tamer (draw branch — simpler than BT15-020
//!     because there is no Blocker grant, just the conditional Draw)
//!   - G2-adjacent: inherited end-of-turn DNA digivolve registration
//!     via the resolved `alt_path_registration` declarative
//!   - Condition gating: Matt-Ishida-tamer presence as a positive/
//!     negative pair (no tamer / wrong-name tamer / Matt tamer)
//!
//! # Test layout
//!
//! Section 1 — Structural assertions on the compiled card
//! Section 2 — Condition gating ([Start of Your Main Phase])
//! Section 3 — Behavioral: draw exactly 1 when Matt Ishida present
//! Section 4 — (no event-log assertions: `draw` is a deterministic state
//!   delta, not a selection or replacement; behavior is fully captured by
//!   hand-size assertions in Section 3.)
//! Section 5 — OPT enforcement: out of scope (clause has no [OPT]).
//!
//! # Sister card
//!
//! BT17-007 (Agumon, Tai Kamiya) is the Red sister card with the same
//! shape — start-of-main-phase tamer-conditional plus end-of-turn DNA
//! digivolve registration. BT17-007 uses `select_trash` instead of
//! `draw` for its main-phase payload.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "BT17-019";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

fn make_matt_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Matt Ishida");
    c.card_kind = CardKind::Tamer;
    c.traits = vec![];
    c
}

fn make_other_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.traits = vec![];
    c
}

// ─── Common runner builder ───────────────────────────────────────────────────

/// Base runner with BT17-019 registered from the embedded DSL pack and a
/// modest filler deck for both players (so begin_turn draws don't deck-out).
fn gabumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT17-019")
        .expect("BT17-019 in embedded DSL pack")
        .add_card(make_filler("DECK-PAD"))
        .add_card(make_matt_tamer("MATT"))
        .add_card(make_other_tamer("TAI", "Tai Kamiya"))
        .add_card(make_other_tamer("OTHER-TAMER", "Marcus Damon"))
        .deck(
            0,
            &[
                "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD",
                "DECK-PAD",
            ],
        )
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(5)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions on the compiled card
// ═══════════════════════════════════════════════════════════════════════════════

/// BT17-019 must be present in the embedded pack (YAML compiles cleanly)
/// with the printed card metadata.
#[test]
fn bt17_019_yaml_compiles_and_metadata_matches_printed() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT17-019 in embedded DSL pack");
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.level, Some(3), "Lv.3");
    assert_eq!(card.cost, Some(3), "Cost 3");
    assert_eq!(card.dp, Some(1000), "DP 1000");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Reptile")),
        "BT17-019 must carry the Reptile trait; got traits={:?}",
        card.traits
    );
}

/// Exactly one Lv.2 Blue / cost-0 digivolve alt-path is printed.
#[test]
fn bt17_019_has_lv2_blue_cost0_digivolve_alt_path() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT17-019 in embedded DSL pack");
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        1,
        "BT17-019 must expose exactly one digivolve alt-path; got {}",
        digivolve_paths.len()
    );
    assert_eq!(
        digivolve_paths[0].cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(0)),
        "Digivolve cost must be 0"
    );
}

/// Exactly one triggered clause exists, and it fires at
/// `StartOfYourMainPhase` with FaceUp scope and no [OPT].
#[test]
fn bt17_019_has_start_of_main_phase_triggered_clause() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT17-019 in embedded DSL pack");
    let clause = card.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::StartOfYourMainPhase) {
                return Some(t);
            }
        }
        None
    });
    assert!(
        clause.is_some(),
        "BT17-019 must have a triggered clause at StartOfYourMainPhase"
    );
    let clause = clause.unwrap();
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Start-of-main-phase clause is face-up only"
    );
    assert!(
        !clause.once_per_turn,
        "Start-of-main-phase clause has no [Once Per Turn]"
    );
}

/// G-DSL-EOT-DNA-INLINE (2026-05-24): inherited end-of-your-turn DNA
/// digivolve is now authored via the `may_dna_digivolve_now` step verb
/// inside a `Triggered` clause whose `when: end_of_your_turn, scope:
/// inherited, optional: true`. The body is a single `MayDnaDigivolveNow`
/// step that orchestrates the partner + target selections at trigger fire.
///
/// Predecessor authoring (`alt_path_registration { kind: dna_digivolve,
/// scope: inherited, trigger: end_of_your_turn }`) registered a next-turn
/// alt-path action and did not satisfy the printed "AT end of turn"
/// surface — see proposal `may-dna-digivolve-now-dsl-step`.
#[test]
fn bt17_019_has_inherited_dna_digivolve_may_step() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT17-019 in embedded DSL pack");
    let eot_dna_clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::EndOfYourTurn)
                && t.optional =>
        {
            Some(t)
        }
        _ => None,
    });
    let t = eot_dna_clause
        .expect("BT17-019 must carry an inherited optional EoT triggered clause for DNA digivolve");
    assert!(
        t.process.iter().any(|step| matches!(
            step,
            CompiledStep::MayDnaDigivolveNow {
                ignore_requirements: true,
                cost: 0,
                ..
            }
        )),
        "BT17-019 EoT inherited body must contain a `MayDnaDigivolveNow` step \
         with cost=0 and ignore_requirements=true"
    );
}

/// G-DSL-EOT-DNA-INLINE (2026-05-24): card now prints two triggered
/// clauses — (1) the [Start of Your Main Phase] Matt-Ishida-conditional
/// draw trigger and (2) the inherited [End of Your Turn] DNA digivolve
/// trigger (was previously an alt_path_registration declarative — see
/// proposal `may-dna-digivolve-now-dsl-step`). No more alt_path_registration
/// clauses on this card.
#[test]
fn bt17_019_clause_count_matches_card_text() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT17-019 in embedded DSL pack");
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let alt_path_regs = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration { .. })
            )
        })
        .count();
    assert_eq!(triggered, 2, "Exactly two triggered clauses expected");
    assert_eq!(
        alt_path_regs, 0,
        "Zero alt_path_registration clauses expected after EoT DNA migration"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating ([Start of Your Main Phase])
// ═══════════════════════════════════════════════════════════════════════════════

/// NEGATIVE: Gabumon on field with NO tamer present → no draw, no
/// pending selection. The trigger may fire its outer dispatch, but the
/// inner condition gates and nothing observable happens.
#[test]
fn bt17_019_start_of_main_no_draw_when_no_tamer() {
    let mut runner = gabumon_runner();
    runner.place_on_field(0, CARD_ID, None);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    let hand_after = runner.hand_size(0);

    assert_eq!(
        hand_after, hand_before,
        "no Tamer present → no draw must occur; hand before={hand_before} after={hand_after}"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no selection must be left hanging when condition fails"
    );
}

/// NEGATIVE: Gabumon on field with a Tai-Kamiya tamer (not Matt Ishida)
/// → no draw, no pending selection.
#[test]
fn bt17_019_start_of_main_no_draw_with_non_matt_ishida_tamer() {
    let mut runner = gabumon_runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAI", None);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    let hand_after = runner.hand_size(0);

    assert_eq!(
        hand_after, hand_before,
        "Tamer name does not contain 'Matt Ishida' → no draw; \
         hand before={hand_before} after={hand_after}"
    );
}

/// NEGATIVE: Other-named tamer (e.g. Marcus Damon) → no draw.
/// Separate test from the Tai-Kamiya negative so a regression in tamer-
/// name matching surfaces as a granular failure.
#[test]
fn bt17_019_start_of_main_no_draw_with_marcus_damon_tamer() {
    let mut runner = gabumon_runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "OTHER-TAMER", None);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    let hand_after = runner.hand_size(0);

    assert_eq!(
        hand_after, hand_before,
        "Marcus Damon tamer must not trigger Matt Ishida draw; \
         hand before={hand_before} after={hand_after}"
    );
}

/// NEGATIVE: Gabumon NOT on field → no trigger source, no draw even
/// with a Matt Ishida tamer present.
#[test]
fn bt17_019_no_draw_when_gabumon_not_on_field() {
    let mut runner = gabumon_runner();
    runner.place_on_field(0, "MATT", None);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    let hand_after = runner.hand_size(0);

    assert_eq!(
        hand_after, hand_before,
        "Gabumon not on field → no draw even with Matt Ishida tamer; \
         hand before={hand_before} after={hand_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: draw exactly 1 when Matt Ishida present
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: Gabumon on field + Matt Ishida tamer → drawing exactly 1
/// card on enter-main-phase. This is the canonical "happy path".
#[test]
fn bt17_019_start_of_main_draws_one_when_matt_ishida_present() {
    let mut runner = gabumon_runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "MATT", None);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    let hand_after = runner.hand_size(0);

    assert_eq!(
        hand_after,
        hand_before + 1,
        "must draw exactly 1 when a Matt Ishida tamer is on field; \
         hand before={hand_before} after={hand_after}"
    );
}

/// POSITIVE: The Draw 1 step takes no player choice. Once the Matt-
/// Ishida gating clears, no `pending_selection` is installed — the
/// draw is deterministic and self-resolves via the trigger drainer.
///
/// Faithfulness pin: printed text is "Draw 1" (no "you may"); DCGO
/// authors the clause with `isOptional: false` and the inner step has
/// no SelectCard prompt. Therefore the controller must NOT see a
/// "decline draw" choice.
#[test]
fn bt17_019_draw_does_not_install_a_selection() {
    let mut runner = gabumon_runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "MATT", None);

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "Draw 1 must not install a pending selection — printed text + DCGO \
         say the draw is mandatory and choiceless once the tamer gating clears"
    );
}

/// NEGATIVE: With a Matt Ishida tamer on the OPPONENT's field (not
/// the controller's), no draw must occur. Tests the `of: you` scope
/// of the condition predicate.
#[test]
fn bt17_019_no_draw_with_matt_ishida_on_opponent_side() {
    let mut runner = gabumon_runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(1, "MATT", None);

    let hand_before = runner.hand_size(0);
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    let hand_after = runner.hand_size(0);

    assert_eq!(
        hand_after, hand_before,
        "Matt Ishida tamer on OPPONENT's field must not trigger controller's draw; \
         hand before={hand_before} after={hand_after}"
    );
}
