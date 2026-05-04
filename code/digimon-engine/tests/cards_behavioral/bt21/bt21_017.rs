//! BT21-017 Dimetromon — Digimon, Lv.4, Red, DP 4000, Cost 4.
//! Traits: Reptile, LIBERATOR
//! Evo: Lv3 / 0 memory
//!
//! # Card text (cards.json)
//!
//! **[When Digivolving]** If you have 1 or fewer Tamers, you may play 1
//! [Owen Dreadnought] from your hand without paying the cost.
//!
//! **Inherited: [Your Turn] [Once Per Turn]** When your opponent's security
//! stack is removed from, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_017.cs
//!
//! # Patterns this test covers
//! - Clause (a): [When Digivolving] optional conditional play tamer from hand free
//!   (count_lte Tamer condition + play_from_hand_free for named Tamer card)
//! - Clause (b): Inherited [Your Turn] OPT on_opponent_security_removed → gain 1 memory
//!   (same shape as BT21-008 / BT24-008)
//!
//! # Known gaps
//! - **G-INHERITED-DISPATCH**: inherited `on_opponent_security_removed` firing
//!   not wired up for digivolution-stack cards; behavioral tests for clause (b)
//!   are `#[ignore = "pending: G-INHERITED-DISPATCH"]`.
//! - **G-OPT-TRIGGERED**: OPT lockout not enforced for triggered effects;
//!   OPT lockout test is `#[ignore = "pending: G-OPT-TRIGGERED"]`.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const DIMETROMON_YAML: &str = include_str!("../../../cards/bt21/BT21-017.yaml");

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c
}

fn make_owen(id: &str) -> CardData {
    let mut c = make_test_card(id, "Owen Dreadnought");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c
}

/// Build a base runner for structural / compilation tests.
fn dimetromon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .memory(10)
        .start()
}

/// Fire WhenDigivolving for the given permanent handle using the direct
/// enqueue pattern (same as BT21-029 / BT24-017 tests).
fn fire_when_digivolving(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_017_compiles_with_two_clauses() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("BT21-017")
        .expect("BT21-017 in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        2,
        "BT21-017 must have exactly 2 clauses: (a) when_digivolving + (b) inherited OPT"
    );
}

#[test]
fn bt21_017_has_when_digivolving_triggered_clause() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("BT21-017")
        .expect("BT21-017 in compiled_cards");

    let has_wd = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .any(|t| t.when.contains(&CompiledTiming::WhenDigivolving));

    assert!(
        has_wd,
        "BT21-017 must have a triggered clause with when_digivolving"
    );
}

#[test]
fn bt21_017_when_digivolving_clause_is_optional() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("BT21-017")
        .expect("BT21-017 in compiled_cards");

    let wd_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WhenDigivolving clause must exist");

    assert!(
        wd_clause.optional,
        "WhenDigivolving clause must be optional ('you may')"
    );
}

#[test]
fn bt21_017_when_digivolving_clause_is_not_once_per_turn() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("BT21-017")
        .expect("BT21-017 in compiled_cards");

    let wd_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WhenDigivolving clause must exist");

    assert!(
        !wd_clause.once_per_turn,
        "WhenDigivolving clause must NOT be once_per_turn (no OPT in printed text for this clause)"
    );
}

#[test]
fn bt21_017_has_inherited_opt_on_opponent_security_removed() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("BT21-017")
        .expect("BT21-017 in compiled_cards");

    let inherited = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("Inherited clause must exist");

    assert!(
        inherited
            .when
            .contains(&CompiledTiming::OnOpponentSecurityRemoved),
        "Inherited clause must fire on OnOpponentSecurityRemoved"
    );
    assert!(
        inherited.once_per_turn,
        "Inherited clause must be once_per_turn"
    );
    assert!(
        !inherited.optional,
        "Inherited clause gain memory is not player-optional"
    );
}

#[test]
fn bt21_017_when_digivolving_clause_is_face_up_scope() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("BT21-017")
        .expect("BT21-017 in compiled_cards");

    let wd_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WhenDigivolving clause must exist");

    assert_eq!(
        wd_clause.scope,
        CompiledScope::FaceUp,
        "WhenDigivolving clause must have FaceUp (default own) scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (WhenDigivolving clause)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: P0 has 0 Tamers and Owen Dreadnought in hand → WhenDigivolving
/// fires and offers hand selection prompt.
#[test]
fn bt21_017_condition_passes_with_zero_tamers_and_owen_in_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_owen("OWEN-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["OWEN-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();

    // P0 has 0 tamers on field → condition count_lte ≤1 passes.
    // Place Dimetromon on field to simulate it being on the field (WhenDigivolving context).
    let dimetro_handle = runner.place_on_field(0, "BT21-017", Some(0));
    fire_when_digivolving(&mut runner, dimetro_handle);

    // WhenDigivolving effect is optional — player should be offered hand selection.
    let pending = runner.pending_selection();
    assert!(
        pending.is_some(),
        "WhenDigivolving should install selection when 0 tamers and Owen in hand"
    );
}

/// Positive: P0 has exactly 1 Tamer → count ≤ 1 still passes (boundary).
#[test]
fn bt21_017_condition_passes_with_one_tamer_boundary() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_owen("OWEN-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .hand(0, &["OWEN-1"])
        .memory(10)
        .start();

    // Place 1 tamer on field.
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));
    let dimetro_handle = runner.place_on_field(0, "BT21-017", Some(0));
    fire_when_digivolving(&mut runner, dimetro_handle);

    // Still 1 tamer → condition passes (≤1 threshold).
    let pending = runner.pending_selection();
    assert!(
        pending.is_some(),
        "WhenDigivolving should install selection when exactly 1 tamer present (≤1 boundary passes)"
    );
}

/// Negative: P0 has 2 Tamers → condition count > 1, effect does NOT fire.
///
/// Regression coverage for G-COUNT-LTE-EVAL: non-security aggregate counts
/// must gate triggered effects instead of silently passing.
#[test]
fn bt21_017_condition_blocked_with_two_tamers() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_tamer("TAMER-1"))
        .add_card(make_tamer("TAMER-2"))
        .add_card(make_owen("OWEN-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .hand(0, &["OWEN-1"])
        .memory(10)
        .start();

    // Place 2 tamers on field.
    let _ = runner.place_on_field(0, "TAMER-1", Some(0));
    let _ = runner.place_on_field(0, "TAMER-2", Some(0));
    let dimetro_handle = runner.place_on_field(0, "BT21-017", Some(0));
    fire_when_digivolving(&mut runner, dimetro_handle);

    let pending = runner.pending_selection();
    assert!(
        pending.is_none(),
        "WhenDigivolving must NOT install selection when 2 tamers present (condition fails)"
    );
}

/// Negative: P0 has 0 Tamers but NO Owen Dreadnought in hand →
/// the select_hand filter yields no candidates; no prompt.
#[test]
fn bt21_017_condition_blocked_when_no_owen_in_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        // Empty hand (or hand with only non-Owen cards)
        .memory(10)
        .start();

    let dimetro_handle = runner.place_on_field(0, "BT21-017", Some(0));
    fire_when_digivolving(&mut runner, dimetro_handle);

    // No Owen in hand → select_hand has 0 valid candidates → no prompt installed.
    let pending = runner.pending_selection();
    assert!(
        pending.is_none(),
        "WhenDigivolving must NOT install selection when hand has no Owen Dreadnought"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcome (WhenDigivolving clause)
// ═══════════════════════════════════════════════════════════════════════════════

/// When player selects Owen Dreadnought, it is played for free onto the field.
#[test]
fn bt21_017_playing_owen_moves_it_to_field_for_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_owen("OWEN-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["OWEN-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();

    let dimetro_handle = runner.place_on_field(0, "BT21-017", Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0); // includes Dimetromon itself

    fire_when_digivolving(&mut runner, dimetro_handle);

    // Pending: Hand selection — pick Owen (the only valid option).
    assert!(
        runner.pending_selection().is_some(),
        "expect hand selection prompt for Owen"
    );
    {
        let view = runner.pending_selection_view().unwrap();
        // Execute selection: pick the first valid_action_id.
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select Owen from hand");
    }
    let _ = runner.auto_resolve();

    // Owen should now be on field (tamers go to battle area).
    let field_after = runner.battle_area_size(0);
    assert!(
        field_after > field_before,
        "Owen Dreadnought should be on field after playing free; field_before={field_before}, field_after={field_after}"
    );

    // Owen is no longer in hand.
    let hand_after = runner.hand_size(0);
    assert!(
        hand_after < hand_before,
        "Owen must no longer be in hand after being played; hand_before={hand_before}, hand_after={hand_after}"
    );
}

/// Declining (PASS) when prompt is optional leaves Owen in hand and no tamer appears.
#[test]
fn bt21_017_declining_leaves_owen_in_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_owen("OWEN-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["OWEN-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();

    let dimetro_handle = runner.place_on_field(0, "BT21-017", Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, dimetro_handle);

    // Prompt is optional → PASS is valid.
    assert!(
        runner.pending_is_optional(),
        "WhenDigivolving selection must be optional (player may decline)"
    );

    // Execute PASS action.
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("pass the optional selection");
    let _ = runner.auto_resolve();

    // Hand size unchanged (Owen stays).
    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after, hand_before,
        "Owen must remain in hand after declining; hand_before={hand_before}, hand_after={hand_after}"
    );

    // No extra permanent appeared from declining.
    let field_after = runner.battle_area_size(0);
    assert_eq!(
        field_after, field_before,
        "No new permanent from declining; field_before={field_before}, field_after={field_after}"
    );
}

/// Owen is played for free (memory stays the same after resolution).
#[test]
fn bt21_017_playing_owen_free_does_not_spend_memory() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_owen("OWEN-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["OWEN-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(5)
        .start();

    let dimetro_handle = runner.place_on_field(0, "BT21-017", Some(0));
    fire_when_digivolving(&mut runner, dimetro_handle);

    let memory_before = runner.memory();

    // Pick Owen from hand.
    {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(0, view.valid_action_ids[0])
            .expect("select Owen");
    }
    let _ = runner.auto_resolve();

    let memory_after = runner.memory();
    // Playing free should not deduct memory for Owen's cost.
    assert_eq!(
        memory_after, memory_before,
        "play_from_hand_free must not spend memory for Owen's cost; memory_before={memory_before}, memory_after={memory_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited clause behavioral
// (All behavioral dispatch tests are #[ignore] pending G-INHERITED-DISPATCH)
// ═══════════════════════════════════════════════════════════════════════════════

/// The inherited clause gains 1 memory when opponent security is removed on your turn.
///
/// BLOCKED: engine lacks digivolution-stack inherited triggered-effect dispatch.
/// `enqueue_from_permanent` only scans top card's effects (+ linked_cards + Training).
/// Until that gap closes, Dimetromon's inherited OPT clause never fires from a stack.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH — digivolution-stack inherited triggered-effect dispatch not wired"]
fn bt21_017_inherited_fires_when_source_under_carrier_your_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC-1"])
        .deck(0, &["FILLER-DECK"])
        .memory(5)
        .start();

    // Place CARRIER on field; insert Dimetromon as bottom source.
    let carrier_handle = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let dimetro_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-017")
            .expect("BT21-017 in card_data");
        let next = runner.game.next_card_index();
        let src = CardSource::new(dimetro_idx, 0, next);
        runner.game.players[0].battle_area[carrier_handle.index as usize]
            .card_sources
            .insert(0, src);
    }

    let memory_before = runner.memory();
    runner.attack_player(carrier_handle, 1, false);
    let _ = runner.auto_resolve();
    let memory_after = runner.memory();

    assert!(
        memory_after > memory_before,
        "inherited clause must gain 1 memory when opponent security removed; delta={}",
        memory_after - memory_before
    );
}

/// Negative: inherited clause must NOT fire on opponent's turn
/// (active_when: your_turn blocks it).
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH — digivolution-stack inherited triggered-effect dispatch not wired"]
fn bt21_017_inherited_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("ATTACKER-P1"))
        .add_card(make_filler("SEC-P0"))
        .add_card(make_filler("FILLER-DECK"))
        .security(0, &["SEC-P0"])
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    let carrier_handle = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let dimetro_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-017")
            .expect("BT21-017 in card_data");
        let next = runner.game.next_card_index();
        let src = CardSource::new(dimetro_idx, 0, next);
        runner.game.players[0].battle_area[carrier_handle.index as usize]
            .card_sources
            .insert(0, src);
    }

    // Switch to opponent's turn.
    runner.end_turn();
    let attacker_p1 = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    let memory_before = runner.memory();
    runner.attack_player(attacker_p1, 0, false);
    let _ = runner.auto_resolve();
    let memory_after = runner.memory();

    assert_eq!(
        memory_after, memory_before,
        "Dimetromon inherited must not fire on opponent's turn (active_when: your_turn)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT enforcement on inherited clause
// ═══════════════════════════════════════════════════════════════════════════════

/// OPT: second security removal in the same turn must NOT gain another memory.
///
/// BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED both apply.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH + G-OPT-TRIGGERED — inherited dispatch and OPT enforcement not wired"]
fn bt21_017_inherited_opt_blocks_second_trigger_same_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("SEC-2"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILLER-DECK"])
        .memory(10)
        .start();

    let carrier_handle = runner.place_on_field(0, "CARRIER", Some(0));
    {
        let dimetro_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT21-017")
            .expect("BT21-017 in card_data");
        let next = runner.game.next_card_index();
        let src = CardSource::new(dimetro_idx, 0, next);
        runner.game.players[0].battle_area[carrier_handle.index as usize]
            .card_sources
            .insert(0, src);
    }

    // First attack — OPT fires.
    let m0 = runner.memory();
    runner.attack_player(carrier_handle, 1, false);
    let _ = runner.auto_resolve();
    let m1 = runner.memory();
    let first_delta = m1 - m0;

    assert!(
        first_delta >= 1,
        "first trigger must fire; delta={first_delta}"
    );

    if runner.game_over() {
        return;
    }

    // Second attack — OPT locked out.
    let carrier2 = runner.perm_handle(0, 0);
    let m2 = runner.memory();
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    let m3 = runner.memory();
    let second_delta = m3 - m2;

    assert!(
        second_delta < first_delta,
        "OPT must block second trigger; first_delta={first_delta}, second_delta={second_delta}"
    );
}
