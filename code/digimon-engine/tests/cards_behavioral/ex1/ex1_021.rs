//! EX1-021 MetalGarurumon -- Digimon, Lv.6, Blue, DP 12000, Cost 11.
//! Traits: Cyborg.
//! Evo: Lv.5 Blue / cost 4.
//!
//! # Card text (cards.json)
//!
//! [When Digivolving] Gain 1 memory for every 4 cards in your hand.
//! [When Attacking] If you have 8 or more cards in your hand and a Tamer in
//!   play, return 1 of your opponent's Digimon that has an [On Deletion]
//!   effect to the bottom of its owners deck.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX1/Blue/EX1_021.cs
//!
//! # Verdict: PARTIAL (structural-only)
//!
//! Both clauses' bodies are `process: []` pending the DSL-vocab gaps below.
//! Structural assertions (alt-path, clause registration, scope, OPT) pass
//! today; behavioral memory-gain and return-to-deck tests are `#[ignore]`'d.
//!
//! # Known DSL gaps affecting these tests
//!
//! - **G-DSL-GAIN-MEMORY-FN** [dsl-gap, filed 2026-05-03 by this card]:
//!   `gain_memory` is literal-only -- no `gain_memory_fn:` formula variant.
//!   Blocks Clause 1.
//! - **G-COUNT-GTE-NOT-EVALUATED** [dsl-gap, pre-existing]: generic
//!   `count_gte` aggregate predicates compile but are not evaluated at
//!   runtime. Blocks the chained-`if` workaround for Clause 1, and the
//!   hand-size gate for Clause 2.
//! - **G-DSL-HAS-ON-DELETION-EFFECT** [dsl-gap, filed 2026-05-03 by this
//!   card]: no predicate to filter permanents by whether they carry an
//!   `OnDeletion` triggered effect. Blocks Clause 2's candidate filter.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/ex1/EX1-021.yaml");

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn make_filler(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.dp = Some(3000);
    c.level = Some(3);
    c
}

fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.dp = Some(dp);
    c.level = Some(5);
    c.card_kind = CardKind::Digimon;
    c
}

fn make_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c
}

/// Standard fixture: EX1-021 from YAML + a generic Tamer + a generic
/// opponent Digimon + hand-filler cards. Memory pre-set high so attacks
/// can be initiated freely.
fn metal_garurumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX1-021 YAML loads")
        .add_card(make_filler("HAND-FILLER", "HandFiller"))
        .add_card(make_tamer("OWN-TAMER", "OwnTamer"))
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigimon", 5000))
        .memory(12)
        .build()
}

/// Push N filler cards into player `p`'s hand using the registered
/// "HAND-FILLER" card.
fn push_n_to_hand(r: &mut DebugRunner, p: PlayerId, n: usize) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "HAND-FILLER")
        .expect("HAND-FILLER must be registered via add_card");
    for _ in 0..n {
        let next_idx = r.game.next_card_index();
        let card = CardSource::new(data_idx, p, next_idx);
        r.game.players[p as usize].hand.push(card);
    }
}

/// Set player `p`'s hand to exactly `target` cards (clears + repushes).
fn set_hand_size(r: &mut DebugRunner, p: PlayerId, target: usize) {
    r.game.players[p as usize].hand.clear();
    push_n_to_hand(r, p, target);
}

fn compiled_ex1_021() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("EX1-021.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("EX1-021.yaml compiles");
    registry
        .lookup("EX1-021")
        .expect("EX1-021 in registry")
        .clone()
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

#[test]
fn ex1_021_compiles_from_yaml() {
    let _r = metal_garurumon_runner();
}

#[test]
fn ex1_021_has_lv5_blue_cost4_digivolve_alt_path() {
    let compiled = compiled_ex1_021();
    let dv = compiled.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && matches!(p.cost, Some(CompiledCost::Literal(4)))
    });
    assert!(
        dv.is_some(),
        "Must have Lv.5 Blue / Cost 4 digivolve alt-path"
    );
}

#[test]
fn ex1_021_has_when_digivolving_clause() {
    let compiled = compiled_ex1_021();
    let clause = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::WhenDigivolving) {
                Some(t)
            } else {
                None
            }
        } else {
            None
        }
    });
    assert!(clause.is_some(), "Must have [When Digivolving] clause");
    let c = clause.unwrap();
    assert!(!c.optional, "[When Digivolving] is mandatory");
    assert!(!c.once_per_turn, "[When Digivolving] has no OPT");
    assert_eq!(c.scope, CompiledScope::FaceUp);
}

#[test]
fn ex1_021_has_when_attacking_clause() {
    let compiled = compiled_ex1_021();
    let clause = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::WhenAttacking) {
                Some(t)
            } else {
                None
            }
        } else {
            None
        }
    });
    assert!(clause.is_some(), "Must have [When Attacking] clause");
    let c = clause.unwrap();
    assert!(
        !c.optional,
        "[When Attacking] is mandatory (per DCGO canNoSelect: false)"
    );
    assert!(!c.once_per_turn, "[When Attacking] has no printed OPT");
    assert_eq!(c.scope, CompiledScope::FaceUp);
}

#[test]
fn ex1_021_total_triggered_clause_count_is_two() {
    let compiled = compiled_ex1_021();
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .collect();
    assert_eq!(triggered.len(), 2, "Must have exactly 2 triggered clauses");
}

/// Card-level metadata must match the printed card exactly. Changes to the YAML
/// header that accidentally alter level, dp, cost, color, or kind are caught here.
#[test]
fn ex1_021_card_metadata() {
    let compiled = compiled_ex1_021();
    assert_eq!(compiled.kind, CompiledCardKind::Digimon, "kind must be Digimon");
    assert_eq!(compiled.level, Some(6), "level must be 6");
    assert_eq!(compiled.dp, Some(12000), "DP must be 12000");
    assert_eq!(compiled.cost, Some(11), "play cost must be 11");
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue],
        "color must be [blue]"
    );
    assert!(
        compiled.traits.iter().any(|t| t == "Cyborg"),
        "traits must include Cyborg; got {:?}",
        compiled.traits
    );
}

// ─── SECTION 2 — [When Digivolving] memory-gain behavioral (BLOCKED) ─────────
//
// Both gating gaps (G-DSL-GAIN-MEMORY-FN and G-COUNT-GTE-NOT-EVALUATED)
// must close before this clause can be expressed faithfully. Tests are
// `#[ignore]`'d until then; the YAML body is `process: []` so the tests
// would currently see no memory change at all.

#[test]
#[ignore = "BLOCKED: G-DSL-GAIN-MEMORY-FN / G-COUNT-GTE-NOT-EVALUATED -- gain_memory_fn formula variant missing AND count_gte not evaluated; process: [] pending"]
fn ex1_021_when_digivolving_hand_lt_4_gains_no_memory() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));

    set_hand_size(&mut r, 0, 3);

    let mem_before = r.memory();
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        mem_before,
        "hand=3 -> floor(3/4)=0 -> no memory gain"
    );
}

#[test]
#[ignore = "BLOCKED: G-DSL-GAIN-MEMORY-FN / G-COUNT-GTE-NOT-EVALUATED -- process: [] pending"]
fn ex1_021_when_digivolving_hand_eq_4_gains_one_memory() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));

    set_hand_size(&mut r, 0, 4);

    let mem_before = r.memory();
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert_eq!(
        (r.memory() - mem_before).abs(),
        1,
        "hand=4 -> floor(4/4)=1 -> +1 memory"
    );
}

#[test]
#[ignore = "BLOCKED: G-DSL-GAIN-MEMORY-FN / G-COUNT-GTE-NOT-EVALUATED -- process: [] pending"]
fn ex1_021_when_digivolving_hand_eq_8_gains_two_memory() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));

    set_hand_size(&mut r, 0, 8);

    let mem_before = r.memory();
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert_eq!(
        (r.memory() - mem_before).abs(),
        2,
        "hand=8 -> floor(8/4)=2 -> +2 memory"
    );
}

#[test]
#[ignore = "BLOCKED: G-DSL-GAIN-MEMORY-FN / G-COUNT-GTE-NOT-EVALUATED -- process: [] pending"]
fn ex1_021_when_digivolving_hand_eq_7_gains_one_memory() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));

    set_hand_size(&mut r, 0, 7);

    let mem_before = r.memory();
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert_eq!(
        (r.memory() - mem_before).abs(),
        1,
        "hand=7 -> floor(7/4)=1 -> +1 memory"
    );
}

// Negative behavioral check that DOES pass today: with `process: []` the
// trigger fires but no memory changes. This locks in the current "no-op
// faithful-to-process-[]" behavior so a regression that accidentally adds
// memory gain (e.g. a future implementer wires the chained-`if` workaround
// without realizing G-COUNT-GTE-NOT-EVALUATED breaks it) is caught.
#[test]
fn ex1_021_when_digivolving_currently_does_not_change_memory() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));

    set_hand_size(&mut r, 0, 8);

    let mem_before = r.memory();
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        mem_before,
        "Body is process: [] pending G-DSL-GAIN-MEMORY-FN; no memory change should occur"
    );
}

// ─── SECTION 3 — [When Attacking] body behavioral (BLOCKED) ──────────────────
//
// Body is `process: []` so the trigger no-ops today. Once
// G-DSL-HAS-ON-DELETION-EFFECT and G-COUNT-GTE-NOT-EVALUATED close, the
// `#[ignore]`'d tests become live.

#[test]
fn ex1_021_when_attacking_currently_installs_no_selection() {
    // Pre-condition: with process: [], trigger fires but body installs
    // nothing -- regression guard against accidental over-selection.
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));
    let _tamer = r.place_on_field(0, "OWN-TAMER", Some(0));
    let _opp = r.place_on_field(1, "OPP-DIGI", None);

    set_hand_size(&mut r, 0, 8);

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "Body is process: [] pending G-DSL-HAS-ON-DELETION-EFFECT; no selection should install"
    );
}

#[test]
#[ignore = "BLOCKED: G-DSL-HAS-ON-DELETION-EFFECT / G-COUNT-GTE-NOT-EVALUATED -- process: [] pending; selection install requires both gaps closed"]
fn ex1_021_when_attacking_installs_selection_when_all_conditions_met() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));
    let _tamer = r.place_on_field(0, "OWN-TAMER", Some(0));
    let _opp = r.place_on_field(1, "OPP-DIGI", None);

    set_hand_size(&mut r, 0, 8);

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_some(),
        "All conditions met -> selection of opp [On Deletion] Digimon must install"
    );
}

#[test]
#[ignore = "BLOCKED: G-DSL-HAS-ON-DELETION-EFFECT / G-COUNT-GTE-NOT-EVALUATED -- process: [] pending"]
fn ex1_021_when_attacking_blocks_when_hand_lt_8() {
    // Once gaps close, this test must verify the hand-size gate prevents
    // the selection from installing when hand < 8.
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));
    let _tamer = r.place_on_field(0, "OWN-TAMER", Some(0));
    let _opp = r.place_on_field(1, "OPP-DIGI", None);

    set_hand_size(&mut r, 0, 5); // hand < 8

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "[When Attacking] must not install selection when hand<8"
    );
}

/// Regression guard: hand < 8 (tamer present). Process is `[]` so no
/// selection installs today. Once G-COUNT-GTE-NOT-EVALUATED closes, this
/// test is superseded by the ignored `ex1_021_when_attacking_blocks_when_hand_lt_8`.
#[test]
fn ex1_021_when_attacking_hand_lt_8_currently_installs_no_selection() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));
    let _tamer = r.place_on_field(0, "OWN-TAMER", Some(0));
    let _opp = r.place_on_field(1, "OPP-DIGI", None);

    set_hand_size(&mut r, 0, 5); // hand < 8

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "Body is process: [] -- no selection installs regardless of hand size"
    );
}

/// Regression guard: hand >= 8 but NO tamer on field. Process is `[]` so no
/// selection installs today. Once both gaps close the tamer condition must
/// gate the effect; this test documents the intended semantics.
#[test]
fn ex1_021_when_attacking_no_tamer_currently_installs_no_selection() {
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));
    // Intentionally omit OWN-TAMER to exercise the no-tamer gate.
    let _opp = r.place_on_field(1, "OPP-DIGI", None);

    set_hand_size(&mut r, 0, 8);

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "Body is process: [] -- no selection installs regardless of board state"
    );
}

#[test]
#[ignore = "BLOCKED: G-DSL-HAS-ON-DELETION-EFFECT / G-COUNT-GTE-NOT-EVALUATED -- process: [] pending; once gaps close, no-tamer must block the effect"]
fn ex1_021_when_attacking_blocks_when_no_tamer_in_play() {
    // Once both gaps close, this test must verify that the Tamer condition
    // prevents the selection from installing when no Tamer is in play.
    let mut r = metal_garurumon_runner();
    let perm = r.place_on_field(0, "EX1-021", Some(0));
    // No tamer placed.
    let _opp = r.place_on_field(1, "OPP-DIGI", None);

    set_hand_size(&mut r, 0, 8);

    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "[When Attacking] must not install selection when no Tamer in play"
    );
}

#[test]
#[ignore = "BLOCKED: G-DSL-HAS-ON-DELETION-EFFECT / G-COUNT-GTE-NOT-EVALUATED -- process: [] pending"]
fn ex1_021_when_attacking_returns_selected_digimon_to_deck_bottom() {
    let mut r = metal_garurumon_runner();
    let attacker = r.place_on_field(0, "EX1-021", Some(0));
    let _tamer = r.place_on_field(0, "OWN-TAMER", Some(0));
    let target = r.place_on_field(1, "OPP-DIGI", None);

    set_hand_size(&mut r, 0, 8);

    let opp_deck_before = r.deck_size(1);
    let opp_field_before = r.battle_area_size(1);

    r.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(attacker),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        opp_field_before - 1,
        "Opp Digimon must leave the battle area"
    );
    assert_eq!(
        r.deck_size(1),
        opp_deck_before + 1,
        "Returned Digimon's top card must land on opp deck (bottom)"
    );
    let _ = target;
}
