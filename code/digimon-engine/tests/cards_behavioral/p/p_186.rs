//! P-186 Gallantmon — Digimon, Lv.6, Red+Purple, Cost 12, DP 12000.
//! Traits: [Holy Warrior, Royal Knight]. Attribute: Virus.
//!
//! # Card text (cards.json — verbatim)
//!
//! When this card would be played, if there is a Digimon with 13000 DP or more,
//! reduce the play cost by 2 for every 5 total cards in both players' trashes.
//! ＜Rush＞ (This Digimon can attack the turn it comes into play.)
//! ＜Blocker＞ (This Digimon can block in the blocker timing.)
//! [On Play] [When Digivolving] Delete 1 Digimon with 13000 DP or more. If
//! this effect didn't delete, ＜Recovery +1 (Deck)＞ (Place the top card of
//! your deck on top of your security stack.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/P_186.cs
//!
//! # DSL YAML
//! code/digimon-engine/cards/p/P-186.yaml
//!
//! # Patterns this test covers
//! - Structural: 4 compiled clauses (cost_reduction, Rush, Blocker, on_play+when_digivolving)
//! - Cost-reduction gated on any Digimon ≥13000 DP existing (any player, battle_area)
//! - Cost-reduction formula: floor(combined_trash / 5) * 2 (BasePerDelta)
//! - Rush keyword installed on field entry
//! - Blocker keyword installed on field entry
//! - [On Play] delete-or-recover: SelectionKind::Target (select_any_permanent)
//! - [When Digivolving] same delete-or-recover via enqueue_triggered
//! - Recovery +1 (Deck) fires when no ≥13000 DP target exists (binding absent)
//! - Recovery +1 does NOT fire when a target is deleted (binding present)
//! - No selection installs when no ≥13000 DP Digimon exists at all (optional)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const YAML: &str = include_str!("../../../cards/p/P-186.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// A filler card used for deck / security padding.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon with exactly 13000 DP (threshold: just-eligible for all DP gates).
fn make_digimon_13k(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(13000);
    c
}

/// A Digimon with 14000 DP (clearly above the 13000 threshold).
fn make_digimon_14k(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(14000);
    c
}

/// A Digimon with 12000 DP (below the 13000 threshold — NOT a legal target).
fn make_digimon_12k(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(12000);
    c
}

/// A Lv.5 Red Digimon (eligible to be evolved from for the standard digivolve path).
fn make_lv5_red(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c
}

/// Push a card into a player's trash (without going through play/delete path).
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, next_idx);
    runner.game.players[player as usize].trash.push(card);
}

/// Fire the [When Digivolving] timing for a permanent on the field.
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// P-186 YAML must parse and compile without errors.
#[test]
fn p_186_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-186 YAML must parse and compile without errors");
}

/// P-186 compiles to exactly 4 clauses:
///   [0] CostReduction (before_pay_cost, when_playing_this, gated on ≥13000 DP)
///   [1] GrantKeyword Rush (declarative, FaceUp)
///   [2] GrantKeyword Blocker (declarative, FaceUp)
///   [3] Triggered [On Play][When Digivolving] delete-or-recover
#[test]
fn p_186_has_exactly_four_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-186")
        .expect("P-186 compiled card must be registered");

    assert_eq!(
        compiled.effects.len(),
        4,
        "expected 4 clauses (cost_reduction, Rush, Blocker, on_play+when_digivolving); got {}",
        compiled.effects.len()
    );
}

/// Clause 0: CostReduction declarative with `when_playing_this: true` and
/// `reduction_timing: before_pay_cost`.
#[test]
fn p_186_clause_0_is_cost_reduction_before_pay_cost_when_playing_this() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-186").expect("P-186 compiled");

    let cost_red = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            reduction_timing,
            when_playing_this,
            condition,
            amount_fn,
            ..
        }) => Some((
            reduction_timing.clone(),
            *when_playing_this,
            condition.clone(),
            amount_fn.clone(),
        )),
        _ => None,
    });

    let (reduction_timing, when_playing_this, condition, amount_fn) =
        cost_red.expect("P-186 must have a CostReduction declarative clause");

    assert_eq!(
        reduction_timing.as_deref(),
        Some("before_pay_cost"),
        "reduction_timing must be before_pay_cost"
    );
    assert!(
        when_playing_this,
        "when_playing_this must be true (only fires when P-186 itself would be played)"
    );
    assert!(
        condition.is_some(),
        "cost reduction must be gated on a condition (any Digimon ≥13000 DP)"
    );
    assert!(
        amount_fn.is_some(),
        "cost reduction must use amount_fn (BasePerDelta formula)"
    );
}

/// Clause 1 and 2: two GrantKeyword declaratives (Rush and Blocker), both FaceUp scope.
#[test]
fn p_186_has_rush_and_blocker_grant_keyword_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-186").expect("P-186 compiled");

    let keywords: Vec<&str> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if *scope == CompiledScope::FaceUp => Some(keyword.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        keywords.contains(&"Rush"),
        "P-186 must have a GrantKeyword(Rush) declarative clause; found: {keywords:?}"
    );
    assert!(
        keywords.contains(&"Blocker"),
        "P-186 must have a GrantKeyword(Blocker) declarative clause; found: {keywords:?}"
    );
    assert_eq!(
        keywords.len(),
        2,
        "P-186 must have exactly 2 GrantKeyword clauses (Rush + Blocker); found: {keywords:?}"
    );
}

/// Clause 3: triggered [On Play][When Digivolving], FaceUp scope, not optional.
#[test]
fn p_186_clause_3_is_on_play_when_digivolving_face_up_not_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-186").expect("P-186 compiled");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("P-186 must have a triggered clause");

    assert!(
        triggered.when.contains(&CompiledTiming::OnPlay),
        "triggered clause must fire on OnPlay; timings={:?}",
        triggered.when
    );
    assert!(
        triggered.when.contains(&CompiledTiming::WhenDigivolving),
        "triggered clause must fire on WhenDigivolving; timings={:?}",
        triggered.when
    );
    assert_eq!(
        triggered.scope,
        CompiledScope::FaceUp,
        "triggered clause must be FaceUp scope"
    );
    assert!(
        !triggered.optional,
        "triggered clause must not be optional (delete is mandatory; 'if no delete' branches Recovery)"
    );
}

/// P-186 is a Digimon card with cost 12 and DP 12000.
#[test]
fn p_186_metadata_is_digimon_cost_12_dp_12000() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-186").expect("P-186 compiled");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Digimon,
        "P-186 must be a Digimon"
    );
    assert_eq!(compiled.cost, Some(12), "P-186 must have play cost 12");
    assert_eq!(compiled.dp, Some(12000), "P-186 must have 12000 DP");
    assert_eq!(compiled.level, Some(6), "P-186 must be Level 6");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Keywords: Rush and Blocker
// ═══════════════════════════════════════════════════════════════════════════════

/// Placing P-186 on the field installs Rush.
#[test]
fn p_186_on_field_has_rush() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    let handle = runner.place_on_field(0, "P-186", Some(0));
    runner.game.tick_declarative_effects();

    // Check via keyword modifier or card_data path.
    let via_modifier = runner.game.has_keyword(handle, Keyword::Rush);
    let via_card_data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "P-186")
        .map_or(false, |d| {
            d.keywords.iter().any(|k| matches!(k, Keyword::Rush))
        });

    assert!(
        via_modifier || via_card_data,
        "Rush must be present on P-186 after it enters the field"
    );
}

/// Placing P-186 on the field installs Blocker.
#[test]
fn p_186_on_field_has_blocker() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    let handle = runner.place_on_field(0, "P-186", Some(0));
    runner.game.tick_declarative_effects();

    let via_modifier = runner.game.has_keyword(handle, Keyword::Blocker);
    let via_card_data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "P-186")
        .map_or(false, |d| {
            d.keywords.iter().any(|k| matches!(k, Keyword::Blocker))
        });

    assert!(
        via_modifier || via_card_data,
        "Blocker must be present on P-186 after it enters the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Cost reduction: condition gate
// ═══════════════════════════════════════════════════════════════════════════════

/// When a ≥13000 DP Digimon is on the field AND there are ≥5 combined trash
/// cards, playing P-186 costs less than its base 12. Specifically with 5 trash
/// cards the reduction is 2 (floor(5/5) * 2 = 2), so net cost is 10.
///
/// Drive: P0 hand has P-186 (cost 12). P0 field has a 13000-DP Digimon.
/// 5 trash cards total (all owned by P0). Start with memory = 12 to cover full cost.
/// After play, memory should be 12 - 10 = 2, not 12 - 12 = 0.
#[test]
fn p_186_cost_reduced_by_2_when_5_combined_trash_cards_and_high_dp_digimon_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(12)
        .start();

    // Place a 13000-DP Digimon on either player's field (gating condition).
    runner.place_on_field(1, "BIG-DIG", Some(0));

    // Add exactly 5 trash cards (all P0).
    for _ in 0..5 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    // Note: memory is clamped to 10 (rules.memory_range.1), not 12.
    let mem_before = runner.memory();

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    // Cost = 12 - floor(5/5)*2 = 12 - 2 = 10 → memory delta = -10.
    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 10,
        "with 5 combined trash and a ≥13000 DP Digimon, play cost must be 12 - 2 = 10; \
         memory before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// With 10 combined trash cards the reduction is 4 (floor(10/5)*2 = 4),
/// so the effective play cost becomes 12 - 4 = 8.
#[test]
fn p_186_cost_reduced_by_4_when_10_combined_trash_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(12)
        .start();

    runner.place_on_field(1, "BIG-DIG", Some(0));

    // 5 cards in P0's trash + 5 cards in P1's trash = 10 total.
    for _ in 0..5 {
        push_to_trash(&mut runner, 0, "FILL");
    }
    for _ in 0..5 {
        push_to_trash(&mut runner, 1, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 8,
        "with 10 combined trash, floor(10/5)*2=4 → effective cost 8; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// When there are only 3 combined trash cards (< 5 bucket threshold), no
/// reduction applies (floor(3/5)*2 = 0), so the full 12-cost is paid even
/// though a ≥13000 DP Digimon is on the field.
#[test]
fn p_186_no_cost_reduction_when_fewer_than_5_combined_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(12)
        .start();

    runner.place_on_field(1, "BIG-DIG", Some(0));

    // Only 3 combined trash cards (< 5).
    for _ in 0..3 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 12,
        "with only 3 combined trash, no reduction applies; full cost 12; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// When there is NO ≥13000 DP Digimon on either player's field, the
/// cost-reduction gate evaluates false and no reduction is applied — even
/// if trash is abundant. The card costs its full 12.
#[test]
fn p_186_no_cost_reduction_without_high_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_12k("SMALL-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(12)
        .start();

    // Only a 12000-DP Digimon on the field — below the 13000 threshold.
    runner.place_on_field(1, "SMALL-DIG", Some(0));

    // 10 combined trash cards — would yield -4 if gate were met.
    for _ in 0..10 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 12,
        "without a ≥13000 DP Digimon, gate is false and full cost 12 is paid; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

/// The cost-reduction gate counts Digimon on EITHER player's field —
/// including P-186's own player's field (not only the opponent's).
#[test]
fn p_186_cost_reduction_gate_includes_own_field_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("OWN-BIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(12)
        .start();

    // 13000-DP Digimon on P0's OWN field (not the opponent's).
    runner.place_on_field(0, "OWN-BIG", Some(0));

    // Exactly 5 combined trash cards → should reduce by 2.
    for _ in 0..5 {
        push_to_trash(&mut runner, 0, "FILL");
    }

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    let paid = mem_before - mem_after;
    assert_eq!(
        paid, 10,
        "gate counts own-field Digimon too; 5 trash → -2 reduction → paid 10; \
         before={mem_before}, after={mem_after}, paid={paid}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [On Play] delete-or-recover
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] with a ≥13000 DP Digimon on the field installs a selection prompt
/// (SelectionKind::Target, since select_any_permanent encodes via encode_attack).
#[test]
fn p_186_on_play_installs_target_selection_when_high_dp_target_exists() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    // A 13000-DP Digimon on P1's field — legal target for the delete.
    runner.place_on_field(1, "BIG-DIG", Some(0));

    runner.play(0, 0);

    assert!(
        runner.game.pending_selection.is_some(),
        "[On Play] must install a selection prompt when a ≥13000 DP Digimon exists"
    );
    let view = runner
        .pending_selection_view()
        .expect("selection view must be present");
    assert_eq!(
        view.kind,
        SelectionKind::Target,
        "select_any_permanent must install SelectionKind::Target"
    );
}

/// [On Play] with a ≥13000 DP Digimon: after selecting and resolving the
/// delete, the target Digimon must be gone from the field.
#[test]
fn p_186_on_play_deletes_selected_high_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "BIG-DIG", Some(0));
    let opp_field_before = runner.battle_area_size(1);

    runner.play(0, 0);
    // auto_resolve picks the first valid target.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "[On Play] delete must remove the targeted Digimon from P1's field"
    );
}

/// [On Play] with a ≥13000 DP Digimon: after deleting, the Recovery +1 branch
/// must NOT fire (binding present = no recovery).
#[test]
fn p_186_on_play_no_recovery_when_delete_succeeds() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(0, &["FILL", "FILL", "FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "BIG-DIG", Some(0));
    let sec_before = runner.security_count(0);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    // Security stack must be unchanged (no Recovery +1 triggered).
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "Recovery +1 must NOT fire when delete succeeded (binding present branch)"
    );
}

/// [On Play] when there is NO ≥13000 DP Digimon: no selection installs
/// (optional select_any_permanent with zero candidates), and Recovery +1 fires
/// automatically (binding_absent branch → security grows by 1).
#[test]
fn p_186_on_play_recovery_plus_1_when_no_high_dp_target() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_12k("SMALL-DIG"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(0, &["FILL"])
        .memory(15)
        .start();

    // Only a 12000-DP Digimon — no legal target for the delete.
    runner.place_on_field(1, "SMALL-DIG", Some(0));
    let sec_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    // No pending selection (optional, zero candidates — no prompt installs).
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection should be pending after [On Play] with no legal target"
    );

    // Recovery +1: top card of deck moves to top of security.
    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "[On Play] with no ≥13000 DP target must apply Recovery +1 (security grows by 1)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Recovery +1 consumes the top card of the deck"
    );
}

/// [On Play] when there is NO Digimon at all on either field: same as above —
/// no selection, and Recovery +1 fires.
#[test]
fn p_186_on_play_recovery_plus_1_when_field_completely_empty() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(0, &["FILL"])
        .memory(15)
        .start();

    let sec_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no selection when no field Digimon exists"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "Recovery +1 fires when no ≥13000 DP Digimon exists (empty field)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Recovery +1 consumes top of deck"
    );
}

/// [On Play] the optional select_any_permanent only offers ≥13000 DP Digimon.
/// A 12000-DP Digimon on the field must NOT appear as a valid target.
#[test]
fn p_186_on_play_only_high_dp_digimon_selectable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_12k("SMALL-12K"))
        .add_card(make_digimon_14k("BIG-14K"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    // One ineligible (12000 DP) and one eligible (14000 DP) Digimon.
    runner.place_on_field(1, "SMALL-12K", Some(0));
    runner.place_on_field(1, "BIG-14K", Some(0));

    runner.play(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("selection must install when a ≥13000 DP target exists");

    // Exactly 1 valid target (the 14000 DP one only).
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the ≥13000 DP Digimon must be a valid target; got {} candidates",
        view.valid_action_ids.len()
    );
}

/// [On Play] the select targets from EITHER player's field (any_permanent).
/// A ≥13000 DP Digimon on P0's OWN field is a legal target.
#[test]
fn p_186_on_play_can_target_own_field_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_13k("OWN-13K"))
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    // 13000-DP Digimon on P0's OWN field.
    runner.place_on_field(0, "OWN-13K", Some(0));
    let trash_before = runner.trash_size(0);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    // OWN-13K was the only ≥13000 DP target; auto_resolve selects and deletes it.
    // After delete: field has only P-186 (OWN-13K went to trash).
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "P-186 [On Play] can target and delete its own field Digimon (select_any_permanent); \
         trash should have grown by 1 (OWN-13K deleted into P0's trash)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [When Digivolving] delete-or-recover
// ═══════════════════════════════════════════════════════════════════════════════

/// [When Digivolving] with a ≥13000 DP Digimon: delete fires and removes the target.
#[test]
fn p_186_when_digivolving_deletes_high_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv5_red("EVOLVE-SRC"))
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    // P1 has a 13000-DP Digimon that will be the delete target.
    runner.place_on_field(1, "BIG-DIG", Some(0));
    let opp_field_before = runner.battle_area_size(1);

    // Place P-186 on top of a stack to simulate having digivolved.
    let stack = runner.place_stack(0, &["EVOLVE-SRC", "P-186"]);

    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "[When Digivolving] must delete the ≥13000 DP Digimon"
    );
}

/// [When Digivolving] with no ≥13000 DP target: Recovery +1 fires.
#[test]
fn p_186_when_digivolving_recovery_plus_1_when_no_high_dp_target() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv5_red("EVOLVE-SRC"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(0, &["FILL"])
        .memory(15)
        .start();

    let sec_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);

    let stack = runner.place_stack(0, &["EVOLVE-SRC", "P-186"]);

    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no selection when no ≥13000 DP Digimon exists during When Digivolving"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "[When Digivolving] Recovery +1 must fire when no target exists"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Recovery +1 consumes top of deck"
    );
}

/// [When Digivolving] with a target: Recovery +1 does NOT fire.
#[test]
fn p_186_when_digivolving_no_recovery_when_delete_succeeds() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv5_red("EVOLVE-SRC"))
        .add_card(make_digimon_13k("BIG-DIG"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(0, &["FILL", "FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "BIG-DIG", Some(0));
    let sec_before = runner.security_count(0);

    let stack = runner.place_stack(0, &["EVOLVE-SRC", "P-186"]);

    fire_when_digivolving(&mut runner, stack);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before,
        "Recovery +1 must NOT fire when the delete step successfully deleted a Digimon"
    );
}

/// [When Digivolving] only targets Digimon with dp_gte: 13000.
/// A 12000-DP Digimon must not appear as a valid target.
#[test]
fn p_186_when_digivolving_only_high_dp_selectable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv5_red("EVOLVE-SRC"))
        .add_card(make_digimon_12k("SMALL-12K"))
        .add_card(make_digimon_14k("BIG-14K"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(15)
        .start();

    runner.place_on_field(1, "SMALL-12K", Some(0));
    runner.place_on_field(1, "BIG-14K", Some(0));

    let stack = runner.place_stack(0, &["EVOLVE-SRC", "P-186"]);
    fire_when_digivolving(&mut runner, stack);

    let view = runner
        .pending_selection_view()
        .expect("selection must install when a ≥13000 DP target exists");

    // Only the 14000-DP Digimon is selectable.
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "[When Digivolving] only ≥13000 DP Digimon must be valid targets; got {} candidates",
        view.valid_action_ids.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Recovery +1 semantics
// ═══════════════════════════════════════════════════════════════════════════════

/// Recovery +1 places the TOP card of the deck on TOP of the security stack.
/// The security stack grows by 1, the deck shrinks by 1.
#[test]
fn p_186_recovery_plus_1_deck_top_goes_to_security_top() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .security(0, &["FILL"])
        .memory(15)
        .start();

    // No ≥13000 DP Digimon → Recovery +1 fires automatically on play.
    let deck_before = runner.deck_size(0);
    let sec_before = runner.security_count(0);

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Recovery +1 must consume exactly 1 card from the top of the deck"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "Recovery +1 must add exactly 1 card to the security stack"
    );
}

/// When the deck is empty and Recovery +1 would fire, it must not panic
/// (deck out with 0 cards to recover is a known edge case — the engine
/// silently no-ops or handles it gracefully).
#[test]
fn p_186_recovery_plus_1_with_empty_deck_does_not_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-186"])
        .deck(0, &[]) // intentionally empty
        .deck(1, &["FILL"])
        .security(0, &["FILL"])
        .memory(15)
        .start();

    // No high-DP target → Recovery +1 fires; empty deck must not panic.
    runner.play(0, 0);
    let _ = runner.auto_resolve();
    // If we reach here without panic the test passes.
}
