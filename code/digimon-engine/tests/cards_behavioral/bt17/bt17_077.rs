//! BT17-077 Imperialdramon: Paladin Mode — Digimon, Lv.7, Blue, DP 16000, Cost 9.
//! Traits: Free, Mythical Dragon.
//! Evo: Lv.6 Blue / Cost 6 (standard). Also: Lv.6 [Imperialdramon]-named / Cost 5 (DCGO alt).
//!
//! # Card text (cards.json — printed)
//!
//! [Hand] [Counter] <Blast Digivolve>
//!   (Your Digimon may digivolve into this card without paying the cost.)
//!
//! [On Play] [When Digivolving]
//!   Trash all digivolution cards of all of your opponent's Digimon. Then,
//!   return all cards from your or your opponent's trash to the bottom of
//!   the deck. If this effect returned a white level 7 card, gain 3 memory.
//!
//! [When Attacking]
//!   By returning 1 of your opponent's Digimon with no digivolution cards to
//!   the bottom of the deck, unsuspend this Digimon.
//!
//! Inherited: Ace Overflow <-5>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/White/BT17_077.cs
//!
//! # Source priority
//! Per CLAUDE.md: printed text > docs/RULES_CONTEXT.md > fandom > DCGO.
//!
//! # Known gaps that block coverage of Clause 1
//!
//! - **G-ASL-07** (qa/archetype-qa/dsl/alter-s-ladder-cross-archetype-gaps-2026-05-03.md):
//!   Mass source cleanup and bottom-deck returns from trash. Three sub-clauses
//!   of Clause 1 still need card-local wiring around the landed Track E verbs:
//!     1a. "Trash all digivolution cards of all opponent Digimon" — needs
//!         an all-sources variant rather than bounded top-N.
//!     1b. "Return all cards from chosen trash to deck bottom" — the
//!         `return_all_trash_to_deck_bottom` verb exists, but the player-choice
//!         binding over whose trash remains open.
//!     1c. "If returned white Lv7 card, gain 3 memory" — needs
//!         `any_returned_card` predicate on moved-card result.
//!
//! # Patterns exercised
//! - H12 Blast Digivolve (single-card hand-counter, `burst_digivolve` alt-path)
//! - H13 ACE Overflow -5 (`ace_overflow: -5` top-level field)
//! - G4  Named ancestor digivolve bonus (Lv.6 Imperialdramon / cost 5)
//! - [When Attacking] by-cost-return opp Digimon (no digi cards) → unsuspend self
//! - Condition gate: at least 1 opp Digimon with no digi cards required

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledDpConstraint, CompiledPredicate, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

const YAML: &str = include_str!("../../../cards/bt17/BT17-077.yaml");

/// Compile the YAML directly (bypassing the embedded pack lookup).
fn compiled_bt17_077() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("BT17-077.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("BT17-077.yaml compiles");
    registry
        .lookup("BT17-077")
        .expect("BT17-077 in registry")
        .clone()
}

/// Standard fixture: BT17-077 registered + room to play it.
fn paladin_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT17-077 YAML loads")
        .memory(20)
        .build()
}

/// Build a Digimon CardData for use in tests.
fn make_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c.card_kind = CardKind::Digimon;
    c
}

/// Recursively walk a CompiledPredicate tree looking for `f`.
/// Also descends into existential predicates (any_permanent, no_permanent, etc.).
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
    // Walk into existential predicate bodies.
    if let Some(ref e) = p.any_permanent {
        if pred_any(&e.predicate, f) {
            return true;
        }
    }
    if let Some(ref e) = p.no_permanent {
        if pred_any(&e.predicate, f) {
            return true;
        }
    }
    if let Some(ref e) = p.any_field_permanent {
        if pred_any(&e.predicate, f) {
            return true;
        }
    }
    if let Some(ref e) = p.all_permanents {
        if pred_any(&e.predicate, f) {
            return true;
        }
    }
    false
}

/// Fire WhenAttacking for the given permanent handle by enqueueing and draining.
fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Check if a permanent is suspended by inspecting the game state directly.
fn is_suspended(runner: &DebugRunner, handle: PermanentHandle) -> bool {
    runner.game.players[handle.player as usize]
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.is_suspended)
        .unwrap_or(false)
}

// ─── SECTION 1 — Structural assertions on CompiledCard ──────────────────────

/// The YAML must parse and compile without errors.
#[test]
fn bt17_077_compiles() {
    let _c = compiled_bt17_077();
}

/// Card-level metadata: level 7, DP 16000, cost 9.
#[test]
fn bt17_077_card_metadata_matches_print() {
    let c = compiled_bt17_077();
    assert_eq!(c.level, Some(7), "Imperialdramon: Paladin Mode is Lv.7");
    assert_eq!(c.dp, Some(16000), "DP is 16000");
    assert_eq!(c.cost, Some(9), "Play cost is 9");
}

/// ACE Overflow: top-level `ace_overflow: -5` per Group 8 closure.
#[test]
fn bt17_077_ace_overflow_is_minus_5() {
    let c = compiled_bt17_077();
    assert_eq!(
        c.ace_overflow,
        Some(-5),
        "Inherited Ace Overflow <-5> must be recorded via ace_overflow: -5"
    );
}

/// Standard digivolve alt-path: Lv.6 Blue / Cost 6 (cards.json evo_costs).
#[test]
fn bt17_077_has_standard_digivolve_alt_path_lv6_blue_cost_6() {
    let c = compiled_bt17_077();
    let has_std = c.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && !p.ignore_requirements
            && matches!(p.cost, Some(CompiledCost::Literal(6)))
            && p.from
                .as_ref()
                .map_or(false, |f| pred_any(f, |q| q.level_eq == Some(6)))
    });
    assert!(
        has_std,
        "Must have a standard Digivolve alt-path at Lv.6 / Cost 6 \
         (per cards.json evo_costs)"
    );
}

/// Named-ancestor alt-path: Lv.6 [Imperialdramon]-named / Cost 5.
/// DCGO BT17_077.cs: AddSelfDigivolutionRequirementStaticEffect at None timing,
/// Level==6 && ContainsCardName("Imperialdramon"), cost 5.
#[test]
fn bt17_077_has_named_ancestor_digivolve_alt_path_imperialdramon_lv6_cost_5() {
    let c = compiled_bt17_077();
    let has_named = c.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(5)))
            && p.from.as_ref().map_or(false, |f| {
                pred_any(f, |q| q.level_eq == Some(6))
                    && pred_any(f, |q| q.name_contains.as_deref() == Some("Imperialdramon"))
            })
    });
    assert!(
        has_named,
        "Must have a named-ancestor Digivolve alt-path: Lv.6 [Imperialdramon]-named / \
         Cost 5 (DCGO AddSelfDigivolutionRequirementStaticEffect)"
    );
}

/// Blast Digivolve alt-path: BurstDigivolve kind, marker: true, cost 0.
/// Canonical reference: BT17-018 (Gallantmon: Crimson Mode).
#[test]
fn bt17_077_has_burst_digivolve_alt_path_marker_true_cost_0() {
    let c = compiled_bt17_077();
    let has_burst = c.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::BurstDigivolve
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.marker
    });
    assert!(
        has_burst,
        "Must have a BurstDigivolve alt-path with marker: true and cost 0 \
         (DCGO BlastDigivolveEffect at OnCounterTiming; canonical: BT17-018)"
    );
}

/// Face-up BlastDigivolve grant_keyword clause must be present.
#[test]
fn bt17_077_has_face_up_blast_digivolve_grant_keyword() {
    let c = compiled_bt17_077();
    let has_blast = c.effects.iter().any(|cl| {
        matches!(
            cl,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "BlastDigivolve"
        )
    });
    assert!(
        has_blast,
        "BT17-077 must declare a face-up GrantKeyword(BlastDigivolve) clause \
         (DCGO BlastDigivolveEffect at OnCounterTiming)"
    );
}

/// [When Attacking] triggered clause must be present.
#[test]
fn bt17_077_has_when_attacking_triggered_clause() {
    let c = compiled_bt17_077();
    let has_wa = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| t.when.contains(&CompiledTiming::WhenAttacking));
    assert!(
        has_wa,
        "BT17-077 must have a WhenAttacking triggered clause for the \
         'By returning ... unsuspend self' effect"
    );
}

/// The [When Attacking] clause must be optional (DCGO canNoSelect: true).
#[test]
fn bt17_077_when_attacking_clause_is_optional() {
    let c = compiled_bt17_077();
    let wa = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("WhenAttacking clause must exist");
    assert!(
        wa.optional,
        "WhenAttacking clause must be optional: true \
         (DCGO: SelectPermanentEffect.SetUp(canNoSelect: true) — player may skip)"
    );
}

/// The [When Attacking] clause must have a condition gating on opp Digimon
/// with no digivolution cards (stack_size_lte: 1).
#[test]
fn bt17_077_when_attacking_clause_condition_gates_on_opp_digimon_no_digi_cards() {
    let c = compiled_bt17_077();
    let wa = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("WhenAttacking clause must exist");

    let condition = wa
        .condition
        .as_ref()
        .expect("WhenAttacking clause must have a condition");

    assert!(
        pred_any(condition, |q| q.stack_size_lte
            == Some(CompiledDpConstraint::Literal(1))),
        "WhenAttacking condition must include stack_size_lte: 1 \
         (models 'no digivolution cards'; DCGO: DigivolutionCards.Count == 0)"
    );
}

/// BLOCKED structural assertion — [On Play][When Digivolving] Clause 1.
/// When G-ASL-07 closes, the YAML will gain a triggered clause covering
/// OnPlay + WhenDigivolving. Shape asserted here so unblocking is mechanical.
#[test]
#[ignore = "pending: G-ASL-07 (qa/archetype-qa/dsl/alter-s-ladder-cross-archetype-gaps-2026-05-03.md) \
            -- Clause 1 (mass trash sources + return trash to deck + conditional memory) \
            is a commented stub in BT17-077.yaml until the remaining choice/result gaps close."]
fn bt17_077_has_on_play_when_digivolving_clause() {
    let c = compiled_bt17_077();
    let has_clause = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        });
    assert!(
        has_clause,
        "Clause 1 must fire on both OnPlay and WhenDigivolving"
    );
}

// ─── SECTION 2 — Behavioral: [When Attacking] positive path ──────────────────

/// [When Attacking] positive path: opp Digimon with no digi cards is returned
/// to the bottom of the deck; self is unsuspended.
///
/// Setup:
///   - Opp has one Digimon with no digivolution cards (stack_size == 1).
///   - Paladin Mode is suspended (simulating having attacked).
///   - Fire WhenAttacking timing.
/// Expected:
///   - A pending selection is installed (OppField target pick).
///   - Resolve by auto-picking the first valid action.
///   - Opp battle area shrinks by 1; opp deck grows by 1.
///   - Paladin Mode is unsuspended.
#[test]
fn bt17_077_when_attacking_returns_opp_digimon_and_unsuspends_self() {
    let mut runner = paladin_runner();

    // Spawn an opponent Digimon with no digivolution cards (stack_size == 1).
    runner
        .game
        .card_data
        .push(make_digimon("OPP-NO-SRC", "OppNoSrc", 5, 5000));
    let opp_no_src = runner.place_on_field(1, "OPP-NO-SRC", None);

    // Spawn Imperialdramon: Paladin Mode on player 0's side.
    let paladin = runner.place_on_field(0, "BT17-077", None);

    // Suspend Paladin Mode to simulate the attacked state.
    runner.game.players[0].battle_area[paladin.index as usize].is_suspended = true;

    let opp_deck_before = runner.deck_size(1);
    let opp_battle_before = runner.battle_area_size(1);

    // Fire the WhenAttacking trigger.
    fire_when_attacking(&mut runner, paladin);

    // A pending selection must be installed offering the opp Digimon as a target.
    let view = runner
        .pending_selection_view()
        .expect("WhenAttacking must install a pending selection when eligible target exists");

    // Resolve by taking the first valid action (picks the opp Digimon).
    let action_id = view.valid_action_ids[0];
    let sel_player = view.selecting_player;
    runner
        .execute_action(sel_player, action_id)
        .expect("selection resolves");

    // Drain any remaining effects.
    runner.game.drain_effect_queue();

    // opp Digimon must have left the battle area.
    assert_eq!(
        runner.battle_area_size(1),
        opp_battle_before - 1,
        "Opponent Digimon with no digi cards must leave the battle area"
    );
    // opp deck gains one card (bottom-decked).
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 1,
        "Returned Digimon must land at the bottom of the opponent's deck"
    );
    // Paladin Mode must be unsuspended.
    assert!(
        !is_suspended(&runner, paladin),
        "Imperialdramon: Paladin Mode must be unsuspended after the return"
    );
}

/// [When Attacking] negative path: no opp Digimon with no digi cards →
/// condition fails → no pending selection installed.
#[test]
fn bt17_077_when_attacking_skips_when_all_opp_digimon_have_digi_cards() {
    let mut runner = paladin_runner();

    // Spawn an opponent Digimon and push a source under it (stack_size == 2).
    runner
        .game
        .card_data
        .push(make_digimon("OPP-SRC", "OppWithSrc", 5, 5000));
    runner
        .game
        .card_data
        .push(make_digimon("OPP-UNDER", "OppUnder", 4, 3000));
    let opp = runner.place_on_field(1, "OPP-SRC", None);
    runner.push_source(opp, "OPP-UNDER");

    let paladin = runner.place_on_field(0, "BT17-077", None);

    // Fire the WhenAttacking trigger.
    fire_when_attacking(&mut runner, paladin);

    // No selection should be installed — condition fails (all opp Digimon have digi cards).
    assert!(
        runner.pending_selection().is_none(),
        "WhenAttacking must NOT install a pending selection when all opp \
         Digimon have digivolution cards (stack_size > 1)"
    );
}

/// [When Attacking] negative path: no opp Digimon at all → condition fails →
/// no pending selection installed.
#[test]
fn bt17_077_when_attacking_skips_when_no_opp_digimon() {
    let mut runner = paladin_runner();

    // No opponent Digimon on the field.
    let paladin = runner.place_on_field(0, "BT17-077", None);

    fire_when_attacking(&mut runner, paladin);

    assert!(
        runner.pending_selection().is_none(),
        "WhenAttacking must NOT install a pending selection when opponent \
         has no Digimon"
    );
}

// ─── SECTION 3 — Behavioral: [On Play][When Digivolving] Clause 1 ────────────
//
// All tests for Clause 1 are #[ignore]'d because the YAML clause body is a
// commented stub pending G-ASL-07.

/// BLOCKED: On Play — trashes all opp digi sources (Clause 1a).
#[test]
#[ignore = "pending: G-ASL-07 -- Clause 1a still needs an all-sources variant; bounded Track E trash_top_n_digivolution_cards_of_each is landed"]
fn bt17_077_on_play_trashes_all_opp_digimon_sources() {
    let mut runner = paladin_runner();

    // Opponent has 2 Digimon, each with digivolution sources.
    runner
        .game
        .card_data
        .push(make_digimon("OPP-A", "OppA", 5, 5000));
    runner
        .game
        .card_data
        .push(make_digimon("OPP-A-SRC", "OppASrc", 4, 3000));
    runner
        .game
        .card_data
        .push(make_digimon("OPP-B", "OppB", 6, 8000));
    runner
        .game
        .card_data
        .push(make_digimon("OPP-B-SRC", "OppBSrc", 5, 4000));

    let opp_a = runner.place_on_field(1, "OPP-A", None);
    runner.push_source(opp_a, "OPP-A-SRC");
    let opp_b = runner.place_on_field(1, "OPP-B", None);
    runner.push_source(opp_b, "OPP-B-SRC");

    let opp_trash_before = runner.trash_size(1);

    let paladin = runner.place_on_field(0, "BT17-077", None);
    runner.fire_on_play(0, paladin.index as usize);

    // All sources should be in trash now (2 sources moved).
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 2,
        "Clause 1a: all opponent Digimon sources must be trashed"
    );
    // The Digimon top cards must still be on the field.
    assert_eq!(
        runner.battle_area_size(1),
        2,
        "Opponent Digimon (top cards) must remain on the field after source trash"
    );
}

/// BLOCKED: On Play — player chooses whose trash to return (Clause 1b).
#[test]
#[ignore = "pending: G-ASL-07 -- Clause 1b has return_all_trash_to_deck_bottom, but still needs player choice prompt between your/opponent's trash"]
fn bt17_077_on_play_prompts_player_to_choose_whose_trash_to_return() {
    let mut runner = paladin_runner();

    let paladin = runner.place_on_field(0, "BT17-077", None);
    runner.fire_on_play(0, paladin.index as usize);

    // After Clause 1a, Clause 1b installs a choice prompt.
    let pending = runner
        .pending_selection()
        .expect("Clause 1b must install a player choice selection");
    assert_eq!(
        pending.kind,
        SelectionKind::EffectChoice,
        "Clause 1b selection must be an EffectChoice (your trash vs opponent's trash)"
    );
}

/// BLOCKED: On Play — returning white Lv7 card triggers +3 memory (Clause 1c).
#[test]
#[ignore = "pending: G-ASL-07 -- Clause 1c requires `any_returned_card` predicate; \
            no DSL support for conditional based on moved-card properties"]
fn bt17_077_on_play_gains_3_memory_when_returned_white_lv7_card() {
    let mut runner = paladin_runner();

    let memory_before = runner.game.memory;
    let paladin = runner.place_on_field(0, "BT17-077", None);
    runner.fire_on_play(0, paladin.index as usize);
    // Clause 1b: auto-resolve choice (e.g. pick opponent's trash containing a white Lv7).
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.memory,
        memory_before + 3,
        "Clause 1c: returning a white Lv.7 card must gain 3 memory"
    );
}

// ─── SECTION 4 — OPT enforcement ─────────────────────────────────────────────
//
// No clause on BT17-077 is [Once Per Turn]. Not applicable.
//
// ─── SECTION 5 — Ace Overflow runtime ────────────────────────────────────────
//
// Ace Overflow is structural metadata (not a runtime trigger), verified in
// Section 1 (bt17_077_ace_overflow_is_minus_5). Runtime memory-loss enforcement
// requires the engine's `AceOverflow` modifier system and is covered by the
// shared ace_overflow test suite (tests/engine/ace_overflow.rs).
