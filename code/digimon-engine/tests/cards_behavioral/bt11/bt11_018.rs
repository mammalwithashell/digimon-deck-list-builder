//! BT11-018 Shoutmon DX — Digimon, Lv.6, Red+Blue, DP 12000, Cost 13.
//! Traits: [Composite, Xros Heart, Blue Flare]. Attribute: Data.
//!
//! # Card text (data/card_bundles/BT11-018.md — official Bandai DB, verbatim)
//!
//! The name of this card/Digimon is also treated as [OmniShoutmon] and
//! [ZeigGreymon]. <Material Save 2> [On Play] Delete 1 of your opponent's
//! Digimon with 8000 DP or less. 1 of your opponent's Digimon can't attack
//! until the end of their turn. [End of Attack] By deleting this Digimon,
//! gain 1 memory.
//!
//! Special Play Condition: DigiXros -3: [OmniShoutmon] x [ZeigGreymon].
//!
//! Standard digivolve circles (official Bandai DB bundle — AUTHORITATIVE;
//! cards.json evo_costs drops the Blue circle, the classic lossy multi-colour
//! ingest failure):
//!   - Red  Lv.5 / cost 4  (present in cards.json evo_costs)
//!   - Blue Lv.6 / cost 3  (missing from cards.json — authored explicitly)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT11/Red/BT11_018.cs
//!
//! # Patterns this test covers
//! - Name Rule: also_treated_as [OmniShoutmon, ZeigGreymon]
//! - <Material Save 2> keyword grant
//! - [On Play] two independent mandatory sub-pools: delete opp DP<=8000,
//!   separately lock 1 opp Digimon from attacking until end of their turn
//! - [End of Attack] optional self-delete -> gain 1 memory
//! - Standard digivolve alt-paths: Red Lv.5/cost4 AND Blue Lv.6/cost3
//! - DigiXros -3 [OmniShoutmon] x [ZeigGreymon] alt-path materials

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType, PlaySource};
use digimon_engine::selection::TriggerSource;

// ─── Inlined production YAML ─────────────────────────────────────────────────

const YAML: &str = include_str!("../../../cards/bt11/BT11-018.yaml");

// ─── Card data helpers ────────────────────────────────────────────────────────

/// A Red Lv.5 Digimon (eligible base for the standard Red Lv.5/cost4 circle).
fn make_red_lv5(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.colors = vec![CardColor::Red];
    c.dp = Some(6000);
    c
}

/// A Blue Lv.6 Digimon (eligible base for the added Blue Lv.6/cost3 circle).
fn make_blue_lv6(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card_with_level(id, id, 6);
    c.colors = vec![CardColor::Blue];
    c.dp = Some(9000);
    c
}

/// A plain opponent Digimon with a specific DP value.
fn make_opp_digimon(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(dp);
    c.level = Some(4);
    c
}

fn find_hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(data) == card_id)
}

// ─── Base runner ─────────────────────────────────────────────────────────────

fn dx() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .memory(15)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// [Rule] Name: also treated as [OmniShoutmon] and [ZeigGreymon].
#[test]
fn bt11_018_also_treated_as_omnishoutmon_and_zeiggreymon() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");
    assert_eq!(
        compiled.also_treated_as,
        vec!["OmniShoutmon".to_string(), "ZeigGreymon".to_string()],
        "also_treated_as must be exactly [OmniShoutmon, ZeigGreymon]"
    );
}

/// Exactly 2 standard Digivolve alt-paths (Red Lv.5/cost4, Blue Lv.6/cost3)
/// plus 1 DigiXros alt-path = 3 total.
#[test]
fn bt11_018_has_three_alt_paths_total() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");
    assert_eq!(
        compiled.alt_paths.len(),
        3,
        "must have exactly 3 alt_paths (2 digivolve circles + 1 digixros); got {:?}",
        compiled.alt_paths
    );
}

/// Standard digivolve circle: Red Lv.5 / cost 4 (cards.json evo_costs).
#[test]
fn bt11_018_has_red_lv5_cost4_digivolve_alt_path() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");

    let found = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from
                .as_ref()
                .map(|f| f.level_eq == Some(5))
                .unwrap_or(false)
    });
    assert!(
        found,
        "must have a Digivolve alt-path: Lv.5 Red / cost 4; alt_paths={:?}",
        compiled.alt_paths
    );
}

/// Standard digivolve circle: Blue Lv.6 / cost 3 — the circle the official
/// Bandai DB confirms but cards.json evo_costs drops (fix under test).
#[test]
fn bt11_018_has_blue_lv6_cost3_digivolve_alt_path() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");

    let found = compiled.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .map(|f| f.level_eq == Some(6))
                .unwrap_or(false)
    });
    assert!(
        found,
        "must have a Digivolve alt-path: Lv.6 Blue / cost 3 (official Bandai DB circle \
         dropped by cards.json evo_costs); alt_paths={:?}",
        compiled.alt_paths
    );
}

/// DigiXros -3 alt-path has exactly 2 material slots, each name_is exact
/// match ([OmniShoutmon], [ZeigGreymon]) with cost_delta -3, sourced from
/// hand + battle_area.
#[test]
fn bt11_018_digixros_has_two_named_material_slots_cost_delta_3() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");

    let digixros = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DigiXros)
        .expect("must have a DigiXros alt-path");

    assert_eq!(
        digixros.materials.len(),
        2,
        "DigiXros -3 must have exactly 2 material slots (OmniShoutmon, ZeigGreymon)"
    );

    let omni_slot = digixros
        .materials
        .iter()
        .find(|m| m.filter.name_is.as_deref() == Some("OmniShoutmon"))
        .expect("must have an OmniShoutmon exact-name material slot");
    assert_eq!(
        omni_slot.cost_delta,
        Some(-3),
        "OmniShoutmon material slot must reduce cost by 3"
    );

    let zeig_slot = digixros
        .materials
        .iter()
        .find(|m| m.filter.name_is.as_deref() == Some("ZeigGreymon"))
        .expect("must have a ZeigGreymon exact-name material slot");
    assert_eq!(
        zeig_slot.cost_delta,
        Some(-3),
        "ZeigGreymon material slot must reduce cost by 3"
    );
}

/// <Material Save 2> keyword grant is present.
#[test]
fn bt11_018_has_material_save_2_keyword_grant() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");

    let has_material_save = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword {
                    keyword,
                    value: Some(2),
                    ..
                }
            ) if keyword == "MaterialSave"
        )
    });
    assert!(
        has_material_save,
        "must grant MaterialSave value 2; effects={:?}",
        compiled.effects
    );
}

/// The [On Play] clause exists as a mandatory (non-optional) triggered clause.
#[test]
fn bt11_018_has_on_play_mandatory_triggered_clause() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");

    let op = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
        _ => None,
    });
    assert!(op.is_some(), "must have an OnPlay triggered clause");
    assert!(
        !op.unwrap().optional,
        "[On Play] delete/lock clause is mandatory, not \"you may\""
    );
}

/// The [End of Attack] clause exists as an optional triggered clause.
#[test]
fn bt11_018_has_end_of_attack_optional_triggered_clause() {
    let runner = dx();
    let compiled = runner
        .compiled_card("BT11-018")
        .expect("BT11-018 in compiled_cards");

    let eoa = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfAttack) => Some(t),
        _ => None,
    });
    assert!(eoa.is_some(), "must have an EndOfAttack triggered clause");
    assert!(
        eoa.unwrap().optional,
        "\"By deleting this Digimon, gain 1 memory\" must be optional (You may)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] delete DP<=8000 + independently lock 1 opp Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// With one opponent Digimon at DP<=8000 and none above, [On Play] deletes it
/// AND (independently) locks the SAME (only) opponent Digimon from attacking.
#[test]
fn bt11_018_on_play_deletes_low_dp_and_locks_remaining_opponent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .add_card(make_opp_digimon("OPP-LOW", 5000))
        .add_card(make_opp_digimon("OPP-HIGH", 9000))
        .memory(15)
        .start();

    runner.place_on_field(1, "OPP-LOW", Some(0));
    runner.place_on_field(1, "OPP-HIGH", Some(0));
    let dx_perm = runner.place_on_field(0, "BT11-018", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dx_perm));
    runner.game.drain_effect_queue();

    // Sub-pool 1: delete the DP<=8000 target. Only OPP-LOW qualifies, so the
    // engine may auto-resolve a single-candidate mandatory select; drive
    // through with auto_resolve to pick the sole valid target each step.
    runner.auto_resolve().expect("resolve on_play chain");

    // OPP-LOW must be deleted (was the only DP<=8000 candidate).
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-LOW"),
        "OPP-LOW (DP 5000 <= 8000) must be deleted by the On Play clause"
    );

    // OPP-HIGH must survive (DP 9000 > 8000, ineligible for the delete pool)
    // but IS eligible for the cannot-attack pool (no DP filter there) and is
    // the only remaining opponent Digimon, so it should be the lock target.
    // NOTE: deleting OPP-LOW shifts battle_area indices, so `opp_high`'s
    // ORIGINAL handle is stale — re-resolve the live handle by card_id
    // before checking its modifiers (mirrors how a real client re-reads
    // state from the engine after a mutation rather than caching handles).
    let opp_high_live = runner.game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-HIGH")
        .map(|index| digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: index as u8,
        });
    assert!(
        opp_high_live.is_some(),
        "OPP-HIGH (DP 9000 > 8000) must survive the delete pool"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(opp_high_live.unwrap(), ModifierType::CannotAttack),
        "the surviving opponent Digimon must be locked from attacking"
    );
}

/// When the opponent has ONLY a high-DP Digimon (no DP<=8000 candidate), the
/// delete sub-pool is skipped but the cannot-attack sub-pool STILL fires
/// (the two pools are independent — no silent drop of pool 2 when pool 1 has
/// no candidates).
#[test]
fn bt11_018_on_play_no_low_dp_target_still_locks_high_dp_opponent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .add_card(make_opp_digimon("OPP-HIGH", 9000))
        .memory(15)
        .start();

    let opp_high = runner.place_on_field(1, "OPP-HIGH", Some(0));
    let dx_perm = runner.place_on_field(0, "BT11-018", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dx_perm));
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("resolve on_play chain");

    // OPP-HIGH survives (no eligible delete target) but must still be locked
    // by the independent cannot-attack sub-pool.
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-HIGH"),
        "OPP-HIGH must survive — no DP<=8000 delete target exists"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(opp_high, ModifierType::CannotAttack),
        "cannot-attack sub-pool must fire independently even when the delete \
         sub-pool has no eligible target"
    );
}

/// When the opponent has no Digimon at all, [On Play] fires without a panic
/// and neither sub-pool has any effect (both silently skip via their `if`
/// guards).
#[test]
fn bt11_018_on_play_no_opponent_digimon_no_op() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .memory(15)
        .start();

    let dx_perm = runner.place_on_field(0, "BT11-018", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(dx_perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "no opponent Digimon exist — neither sub-pool should install a selection"
    );
    runner.auto_resolve().ok();
    assert_eq!(runner.battle_area_size(1), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [End of Attack] optional self-delete -> gain 1 memory
// ═══════════════════════════════════════════════════════════════════════════════

/// End of Attack installs an optional (Yes/No) prompt — the outer confirm is
/// auto-inserted because `delete_permanent` (the first body step) is not
/// itself declinable.
#[test]
fn bt11_018_end_of_attack_installs_optional_prompt() {
    let mut runner = dx();
    let dx_perm = runner.place_on_field(0, "BT11-018", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(dx_perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "end_of_attack must install a selection (the outer accept/decline confirm)"
    );
    assert!(
        runner.pending_is_optional(),
        "end_of_attack clause must be optional (PASS = decline, exposing the choice)"
    );
}

/// Declining End of Attack: Shoutmon DX stays on the field, memory unchanged.
#[test]
fn bt11_018_end_of_attack_decline_leaves_self_and_memory_unchanged() {
    use digimon_engine::action::space::PASS;

    // Memory is a seesaw clamped to [-10, 10] (rules.memory_range); start
    // below the ceiling so a +1 gain is observable (not clamped away).
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .memory(5)
        .start();
    let dx_perm = runner.place_on_field(0, "BT11-018", Some(0));
    let mem_before = runner.memory();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(dx_perm));
    runner.game.drain_effect_queue();

    runner
        .execute_action(0, PASS)
        .expect("decline end_of_attack");
    runner.auto_resolve().expect("post-decline drain");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT11-018"),
        "declining must leave Shoutmon DX on the field"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "declining must not gain memory"
    );
}

/// Accepting End of Attack: Shoutmon DX is deleted and the controller gains
/// exactly 1 memory.
#[test]
fn bt11_018_end_of_attack_accept_deletes_self_and_gains_1_memory() {
    // Memory is a seesaw clamped to [-10, 10] (rules.memory_range); start
    // below the ceiling so a +1 gain is observable (not clamped away).
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .memory(5)
        .start();
    let dx_perm = runner.place_on_field(0, "BT11-018", Some(0));
    let mem_before = runner.memory();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(dx_perm));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("selection must appear");
    let accept_action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != digimon_engine::action::space::PASS)
        .expect("an accept action must exist alongside PASS");
    runner
        .execute_action(0, accept_action)
        .expect("accept end_of_attack");
    runner
        .auto_resolve()
        .expect("resolve self-delete + memory gain");

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT11-018"),
        "accepting must delete Shoutmon DX from the field"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "accepting must gain exactly 1 memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral digivolve: both standard circles
// ═══════════════════════════════════════════════════════════════════════════════

/// Behavioral: Shoutmon DX digivolves from a Red Lv.5 base at cost 4.
#[test]
fn bt11_018_digivolves_from_red_lv5_at_cost_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .add_card(make_red_lv5("RED-LV5-BASE"))
        .hand(0, &["BT11-018"])
        .deck(0, &["RED-LV5-BASE", "RED-LV5-BASE"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "RED-LV5-BASE", Some(0));
    let hand_idx = find_hand_index(&runner, 0, "BT11-018").expect("BT11-018 in hand");
    let mem_before = runner.memory();

    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "BT11-018 must digivolve from a Red Lv.5 base");
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT11-018",
        "stack top must become Shoutmon DX"
    );
    assert_eq!(
        mem_before - runner.memory(),
        4,
        "Red Lv.5 circle must cost exactly 4 memory"
    );
}

/// Behavioral: Shoutmon DX digivolves from a Blue Lv.6 base at cost 3 — the
/// circle cards.json evo_costs dropped and this fix adds explicitly.
#[test]
fn bt11_018_digivolves_from_blue_lv6_at_cost_3() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .add_card(make_blue_lv6("BLUE-LV6-BASE"))
        .hand(0, &["BT11-018"])
        .deck(0, &["BLUE-LV6-BASE", "BLUE-LV6-BASE"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "BLUE-LV6-BASE", Some(0));
    let hand_idx = find_hand_index(&runner, 0, "BT11-018").expect("BT11-018 in hand");
    let mem_before = runner.memory();

    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        ok,
        "BT11-018 must digivolve from a Blue Lv.6 base (the official Bandai DB \
         circle that cards.json evo_costs dropped)"
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT11-018",
        "stack top must become Shoutmon DX"
    );
    assert_eq!(
        mem_before - runner.memory(),
        3,
        "Blue Lv.6 circle must cost exactly 3 memory"
    );
}

/// Negative: a Red Lv.6 base (wrong color for the Lv.6 circle, wrong level
/// for the Lv.5 circle) cannot digivolve into Shoutmon DX via either
/// standard circle.
#[test]
fn bt11_018_red_lv6_base_cannot_digivolve_via_standard_circles() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT11-018 YAML parses")
        .add_card(make_test_card_with_level("RED-LV6-BASE", "RED-LV6-BASE", 6))
        .hand(0, &["BT11-018"])
        .deck(0, &["RED-LV6-BASE", "RED-LV6-BASE"])
        .memory(10)
        .start();

    let base = runner.place_on_field(0, "RED-LV6-BASE", Some(0));
    let hand_idx = find_hand_index(&runner, 0, "BT11-018").expect("BT11-018 in hand");

    let ok =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        !ok,
        "a Red Lv.6 base matches neither the Red Lv.5 circle nor the Blue Lv.6 circle"
    );
}
