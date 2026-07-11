//! BT21-017 Dimetromon — Digimon, Lv.4, Red, DP 4000, Cost 4.
//! Traits: Reptile, LIBERATOR
//! Evo: Red Lv.3 / 2 memory (standard circle — official Bandai DB / cards.json)
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
//! # Known gaps (historical)
//! - **G-INHERITED-DISPATCH**: closed 2026-05-17 (Phase 2 Track D). Clause (b)
//!   behavioral fire and your-turn-gate tests are live.
//! - **G-OPT-TRIGGERED**: closed 2026-05-16 (Phase 2 Track C). OPT lockout
//!   test is live.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource};
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
// (G-INHERITED-DISPATCH closed 2026-05-17 in Phase 2 Track D)
// ═══════════════════════════════════════════════════════════════════════════════

/// The inherited clause gains 1 memory when opponent security is removed on your turn.
#[test]
fn bt21_017_inherited_fires_when_source_under_carrier_your_turn() {
    // SEC-1 is a Tamer so the on-opp-security-removed trigger fires
    // without the carrier mutual-destructing against a same-DP Digimon
    // security (RULES_CONTEXT 14-2-1-3 post-fix).
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_tamer("SEC-1"))
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

    // Switch to opponent's turn. Use `pass_turn` (memory → +3 for P1), not a
    // raw `end_turn` (which would flip to -5, an impossible real-game state):
    // an attack now correctly ends a turn whose memory sits on the opponent's
    // side, so P1 must start their turn with valid non-negative memory.
    runner.pass_turn();
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
#[test]
fn bt21_017_inherited_opt_blocks_second_trigger_same_turn() {
    // Tamer security so the carrier can attack security multiple times
    // without mutual-destructing on the first attack (post-fix per
    // RULES_CONTEXT 14-2-1-3 in `equal_dp_security_battle_deletes_attacker`).
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_filler("CARRIER"))
        .add_card(make_tamer("SEC-1"))
        .add_card(make_tamer("SEC-2"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILLER-DECK"; 10])
        .deck(1, &["FILLER-DECK"; 10])
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

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Digivolve cost fidelity (user-reported bug 2026-07-09)
//
// Reported: "Digivolving to play a tamer for free made the digivolution free
// (observed on BT21-017)". Root cause: the YAML mis-authored the printed
// standard circle (Red Lv.3 / cost 2 — official Bandai DB + cards.json agree)
// as an ungated `alt_paths: kind: digivolve, from: {level_eq: 3}, cost: 0`,
// which `collect_dsl_alt_digivolve_routes` surfaces as a phantom FREE digivolve
// route from ANY-colour Lv.3. The [When Digivolving] free tamer play is
// innocent: `commit_digivolve_from_hand_no_replace` pays memory BEFORE the
// trigger fires, so `play_from_hand_free` cannot retroactively unpay it.
// ═══════════════════════════════════════════════════════════════════════════════

/// A red Lv.3 Digimon matching BT21-017's printed digivolve circle.
fn make_red_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.colors = vec![CardColor::Red];
    c.dp = Some(2000);
    c
}

/// A blue Lv.3 Digimon — NOT a legal base for BT21-017 (circle is Red-only).
fn make_blue_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.colors = vec![CardColor::Blue];
    c.dp = Some(2000);
    c
}

/// Inject BT21-017's PRINTED evo-cost table (Red Lv.3 / cost 2) into the
/// runner's card store. DSL-loaded fixtures never see cards.json, which is
/// where the printed circle lives in production — this mirrors that state.
fn inject_printed_evo_cost(runner: &mut DebugRunner) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BT21-017")
        .expect("BT21-017 in card_data");
    runner.game.card_data[idx].evo_costs = vec![EvoCost {
        card_color: 0, // Red
        level: 3,
        memory_cost: 2,
    }];
}

/// End-to-end reproduction of the reported bug: digivolve BT21-017 over a red
/// Lv.3, accept the free Owen Dreadnought play, and assert
///   (a) the DIGIVOLUTION cost (2) WAS paid in memory, and
///   (b) the TAMER's cost was NOT paid (total spend == 2 exactly).
///
/// Pre-fix this failed: the bogus cost-0 alt-path made the engine offer a
/// second (free) digivolve route, so the digivolve paused on a cost-choice
/// prompt that includes an illegal cost-0 option.
#[test]
fn bt21_017_digivolve_pays_printed_cost_and_only_tamer_is_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_red_lv3("BASE-RED3"))
        .add_card(make_owen("OWEN-1"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-017", "OWEN-1"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();
    inject_printed_evo_cost(&mut runner);
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "BASE-RED3", Some(0));

    let mem_before = runner.memory();
    let proceeded = runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);

    // The card prints exactly ONE digivolve requirement (Red Lv.3 / cost 2),
    // so there must be NO cost-choice prompt — a prompt here means a phantom
    // route (the bug: an ungated cost-0 alt-path) leaked into the route set.
    if let Some(view) = runner.pending_selection_view() {
        assert_ne!(
            view.kind,
            SelectionKind::EffectChoice,
            "BT21-017 has exactly one printed digivolve route (Red Lv.3 / 2); \
             a digivolve cost-choice prompt means a phantom route leaked in: {:?}",
            view.effect_choices
        );
    }
    assert!(
        proceeded,
        "digivolve must proceed directly at the single printed cost"
    );

    // (a) The digivolution cost was paid: exactly 2 memory.
    assert_eq!(
        runner.memory(),
        mem_before - 2,
        "the digivolve cost (2) must be paid in memory"
    );

    // WhenDigivolving fires → optional Owen hand pick. Accept it.
    let view = runner
        .pending_selection_view()
        .expect("WhenDigivolving must offer the Owen Dreadnought hand pick");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .expect("a selectable Owen action id");
    runner.execute_action(0, pick).expect("select Owen");
    let _ = runner.auto_resolve();

    // (b) Owen was played WITHOUT paying its cost: total spend is still
    // exactly the digivolve cost.
    assert_eq!(
        runner.memory(),
        mem_before - 2,
        "playing Owen free must not change memory — total spend == digivolve cost only"
    );

    // Owen is on the field (digivolved stack + Owen = 2 permanents), hand empty.
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "field must hold the digivolved Digimon and the freely played Owen"
    );
    // Both staged hand cards were used; the digivolve itself drew 1 card.
    assert_eq!(
        runner.hand_size(0),
        1,
        "BT21-017 digivolved and Owen was played; only the digivolve draw remains in hand"
    );
}

/// The printed circle is Red-only: a BLUE Lv.3 base must NOT be digivolvable
/// into BT21-017 at all. Pre-fix the ungated `{level_eq: 3} / cost 0` alt-path
/// made this succeed — for FREE.
#[test]
fn bt21_017_cannot_digivolve_from_off_color_base() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(DIMETROMON_YAML)
        .expect("BT21-017 YAML parses")
        .add_card(make_blue_lv3("BASE-BLUE3"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &["BT21-017"])
        .deck(0, &["FILLER-DECK", "FILLER-DECK", "FILLER-DECK"])
        .memory(10)
        .start();
    inject_printed_evo_cost(&mut runner);
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "BASE-BLUE3", Some(0));

    let mem_before = runner.memory();
    let proceeded = runner.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand);

    assert!(
        !proceeded,
        "a blue Lv.3 base does not satisfy BT21-017's Red Lv.3 circle — digivolve must be rejected"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no prompt of any kind for an illegal digivolve"
    );
    assert_eq!(runner.memory(), mem_before, "no memory paid");
    assert_eq!(runner.hand_size(0), 1, "BT21-017 stays in hand");
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the blue base is untouched (no digivolution happened)"
    );
}
