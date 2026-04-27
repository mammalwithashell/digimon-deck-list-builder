//! Phase 2f2 Task 2 — runtime evaluator for `CompiledFormula`.
//!
//! One test per `CompiledFormula` variant + per-selector / aggregate-selector
//! variants. The evaluator is pure read-only against `EffectContext` +
//! a target permanent handle.

use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledFormula, CompiledPerSelector, CompiledPlayerRef,
    CompiledZone,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::formula_eval::evaluate;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

// ─── Helpers ─────────────────────────────────────────────────────────

/// Build a runner with a single Digimon (`card_id`) on player 0's battle
/// area. Returns the runner and the handle to that permanent.
fn one_digimon_on_field(card_id: &str) -> (DebugRunner, PermanentHandle) {
    let card = make_test_card(card_id, card_id);
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(0, &[card_id])
        .build();
    let h = runner.place_on_field(0, card_id, None);
    (runner, h)
}

/// Push an extra `CardSource` (using the same CardData) onto the
/// digivolution stack of the permanent at `(player, perm_index)`.
fn push_extra_source_same_data(
    runner: &mut DebugRunner,
    player: u8,
    perm_index: usize,
    card_id: &str,
) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card_id registered");
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, player, next);
    runner.game.players[player as usize].battle_area[perm_index]
        .card_sources
        .push(src);
}

/// Push an extra `CardSource` from a different CardData (different colors,
/// different DP, etc.) onto the digivolution stack of `(player, perm_index)`.
fn push_extra_source_with_data(
    runner: &mut DebugRunner,
    player: u8,
    perm_index: usize,
    other_card_id: &str,
) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == other_card_id)
        .expect("other card_id registered");
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, player, next);
    runner.game.players[player as usize].battle_area[perm_index]
        .card_sources
        .push(src);
}

// A blue-color, level-5, dp 4000 test card.
fn make_blue_card(id: &str) -> digimon_engine::card_data::CardData {
    use digimon_engine::card_data::CardData;
    use digimon_engine::enums::CardKind;
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(4000),
        play_cost: 5,
        colors: vec![CardColor::Blue],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// A red-color, level-3, dp 1000 test card.
fn make_low_dp_red_card(id: &str) -> digimon_engine::card_data::CardData {
    use digimon_engine::card_data::CardData;
    use digimon_engine::enums::CardKind;
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(1000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// A red-color, level-7, dp 3000 test card.
fn make_high_dp_red_card(id: &str) -> digimon_engine::card_data::CardData {
    use digimon_engine::card_data::CardData;
    use digimon_engine::enums::CardKind;
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(7),
        dp: Some(3000),
        play_cost: 7,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── 1. Literal ──────────────────────────────────────────────────────

#[test]
fn evaluate_literal() {
    let (mut runner, target) = one_digimon_on_field("T-LIT");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    assert_eq!(evaluate(&CompiledFormula::Literal(7), &ctx, target), 7);
}

// ─── 2-3. BasePerDelta + StackSize / MaterialCount ───────────────────

#[test]
fn evaluate_base_per_stack_size_delta() {
    let (mut runner, target) = one_digimon_on_field("T-SS");
    // Add 2 extra sources → stack_size = 3 (1 base + 2 materials).
    push_extra_source_same_data(&mut runner, 0, 0, "T-SS");
    push_extra_source_same_data(&mut runner, 0, 0, "T-SS");
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 3);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::BasePerDelta {
        base: 1000,
        per: CompiledPerSelector::StackSize,
        delta: 200,
    };
    // 1000 + 3 * 200 = 1600
    assert_eq!(evaluate(&f, &ctx, target), 1600);
}

#[test]
fn evaluate_base_per_material_count_delta() {
    let (mut runner, target) = one_digimon_on_field("T-MC");
    // 1 base + 2 extra sources → 2 materials.
    push_extra_source_same_data(&mut runner, 0, 0, "T-MC");
    push_extra_source_same_data(&mut runner, 0, 0, "T-MC");

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::MaterialCount,
        delta: 1000,
    };
    // 2 materials * 1000 = 2000
    assert_eq!(evaluate(&f, &ctx, target), 2000);
}

#[test]
fn evaluate_base_per_material_count_lone_card_is_zero() {
    // No materials → 0 even with a non-zero delta.
    let (mut runner, target) = one_digimon_on_field("T-MC0");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::BasePerDelta {
        base: 500,
        per: CompiledPerSelector::MaterialCount,
        delta: 1000,
    };
    assert_eq!(evaluate(&f, &ctx, target), 500);
}

// ─── 4-5. FloorDiv ───────────────────────────────────────────────────

#[test]
fn evaluate_floor_div() {
    let (mut runner, target) = one_digimon_on_field("T-FD");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::FloorDiv(vec![
        CompiledFormula::Literal(10),
        CompiledFormula::Literal(3),
    ]);
    assert_eq!(evaluate(&f, &ctx, target), 3);
}

#[test]
fn evaluate_floor_div_zero_divisor_returns_zero() {
    let (mut runner, target) = one_digimon_on_field("T-FD0");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::FloorDiv(vec![
        CompiledFormula::Literal(10),
        CompiledFormula::Literal(0),
    ]);
    assert_eq!(evaluate(&f, &ctx, target), 0);
}

#[test]
fn evaluate_floor_div_wrong_arity_returns_zero() {
    let (mut runner, target) = one_digimon_on_field("T-FDA");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    // Single-arg FloorDiv is malformed: returns 0 rather than panicking.
    let f = CompiledFormula::FloorDiv(vec![CompiledFormula::Literal(10)]);
    assert_eq!(evaluate(&f, &ctx, target), 0);
}

// ─── 6-7. Max / Min ─────────────────────────────────────────────────

#[test]
fn evaluate_max() {
    let (mut runner, target) = one_digimon_on_field("T-MAX");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::Max(vec![
        CompiledFormula::Literal(5),
        CompiledFormula::Literal(2),
        CompiledFormula::Literal(7),
    ]);
    assert_eq!(evaluate(&f, &ctx, target), 7);
}

#[test]
fn evaluate_min() {
    let (mut runner, target) = one_digimon_on_field("T-MIN");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::Min(vec![
        CompiledFormula::Literal(5),
        CompiledFormula::Literal(2),
        CompiledFormula::Literal(7),
    ]);
    assert_eq!(evaluate(&f, &ctx, target), 2);
}

// ─── 8-9. Aggregate(LowestDp / HighestDp) ───────────────────────────

#[test]
fn evaluate_aggregate_lowest_dp() {
    let low = make_low_dp_red_card("T-LOW"); // dp 1000
    let high = make_high_dp_red_card("T-HIGH"); // dp 3000
    let mut runner = DebugRunner::builder()
        .add_card(low)
        .add_card(high)
        .hand(0, &["T-LOW", "T-HIGH"])
        .build();
    let h_low = runner.place_on_field(0, "T-LOW", None);
    let _h_high = runner.place_on_field(0, "T-HIGH", None);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::LowestDp);
    assert_eq!(evaluate(&f, &ctx, h_low), 1000);
}

#[test]
fn evaluate_aggregate_highest_dp() {
    let low = make_low_dp_red_card("T-LOW2");
    let high = make_high_dp_red_card("T-HIGH2");
    let mut runner = DebugRunner::builder()
        .add_card(low)
        .add_card(high)
        .hand(0, &["T-LOW2", "T-HIGH2"])
        .build();
    let h_low = runner.place_on_field(0, "T-LOW2", None);
    let _h_high = runner.place_on_field(0, "T-HIGH2", None);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::HighestDp);
    assert_eq!(evaluate(&f, &ctx, h_low), 3000);
}

#[test]
fn evaluate_aggregate_lowest_level() {
    let low = make_low_dp_red_card("T-LL"); // level 3
    let high = make_high_dp_red_card("T-LH"); // level 7
    let mut runner = DebugRunner::builder()
        .add_card(low)
        .add_card(high)
        .hand(0, &["T-LL", "T-LH"])
        .build();
    let h_low = runner.place_on_field(0, "T-LL", None);
    let _h_high = runner.place_on_field(0, "T-LH", None);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::LowestLevel);
    assert_eq!(evaluate(&f, &ctx, h_low), 3);
}

#[test]
fn evaluate_aggregate_highest_level() {
    let low = make_low_dp_red_card("T-HLL"); // level 3
    let high = make_high_dp_red_card("T-HLH"); // level 7
    let mut runner = DebugRunner::builder()
        .add_card(low)
        .add_card(high)
        .hand(0, &["T-HLL", "T-HLH"])
        .build();
    let h_low = runner.place_on_field(0, "T-HLL", None);
    let _h_high = runner.place_on_field(0, "T-HLH", None);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::HighestLevel);
    assert_eq!(evaluate(&f, &ctx, h_low), 7);
}

#[test]
fn evaluate_aggregate_empty_battle_area_returns_zero() {
    // Build a runner with a card on player 1's field, but evaluate aggregate
    // for ctx.player = 0 (whose battle_area is empty).
    let card = make_test_card("T-EBA", "T-EBA");
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(1, &["T-EBA"])
        .build();
    let target = runner.place_on_field(1, "T-EBA", None);

    let src_card = runner.game.players[1].battle_area[0].top_card().handle();
    // ctx.player = 0 → no permanents under that controller.
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::Aggregate(CompiledAggregateSelector::LowestDp);
    assert_eq!(evaluate(&f, &ctx, target), 0);
}

// ─── 10. AllyCount ───────────────────────────────────────────────────

#[test]
fn evaluate_per_ally_count() {
    let card = make_test_card("T-AC", "T-AC");
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["T-AC", "T-AC", "T-AC"])
        .build();
    let h_self = runner.place_on_field(0, "T-AC", None);
    let _h2 = runner.place_on_field(0, "T-AC", None);
    let _h3 = runner.place_on_field(0, "T-AC", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 3);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::AllyCount,
        delta: 1,
    };
    // 3 perms total - 1 self = 2 allies.
    assert_eq!(evaluate(&f, &ctx, h_self), 2);
}

// ─── 11. RawRust unregistered ────────────────────────────────────────

#[test]
fn evaluate_raw_rust_returns_zero_when_unregistered() {
    let (mut runner, target) = one_digimon_on_field("T-RR");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::RawRust("nonexistent_formula_fn".to_string());
    assert_eq!(evaluate(&f, &ctx, target), 0);
}

// ─── 12. DigivolutionColorCount ──────────────────────────────────────

#[test]
fn evaluate_per_digivolution_color_count() {
    // Build a permanent with two CardSources of different colors:
    // base = red T-DCC (level 3), top = blue T-DCC-B (level 5).
    let red = make_test_card("T-DCC", "T-DCC"); // Red
    let blue = make_blue_card("T-DCC-B");
    let mut runner = DebugRunner::builder()
        .add_card(red)
        .add_card(blue)
        .hand(0, &["T-DCC"])
        .build();
    let target = runner.place_on_field(0, "T-DCC", None);
    push_extra_source_with_data(&mut runner, 0, 0, "T-DCC-B");
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 2);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);

    let f = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::DigivolutionColorCount,
        delta: 1,
    };
    assert_eq!(evaluate(&f, &ctx, target), 2);
}

// ─── 13. CardCountInZone ─────────────────────────────────────────────

#[test]
fn evaluate_per_card_count_in_zone_counts_your_hand() {
    let card = make_test_card("T-CZ-H", "T-CZ-H");
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(0, &["T-CZ-H", "T-CZ-H", "T-CZ-H"])
        .build();
    let target = runner.place_on_field(0, "T-CZ-H", None);
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::CardCountInZoneScoped {
            of: CompiledPlayerRef::You,
            zone: CompiledZone::Hand,
        },
        delta: 100,
    };
    assert_eq!(evaluate(&f, &ctx, target), 300);
}

#[test]
fn evaluate_per_card_count_in_zone_counts_opponent_trash() {
    let card = make_test_card("T-CZ-T", "T-CZ-T");
    let mut runner = DebugRunner::builder()
        .add_card(card)
        .hand(1, &["T-CZ-T", "T-CZ-T"])
        .build();
    let target = runner.place_on_field(0, "T-CZ-T", None);
    while let Some(card) = runner.game.players[1].hand.pop() {
        runner.game.players[1].trash.push(card);
    }

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let f = CompiledFormula::BasePerDelta {
        base: 1,
        per: CompiledPerSelector::CardCountInZoneScoped {
            of: CompiledPlayerRef::Opponent,
            zone: CompiledZone::Trash,
        },
        delta: 10,
    };
    assert_eq!(evaluate(&f, &ctx, target), 21);
}

// ─── Defensive: missing target permanent ─────────────────────────────

#[test]
fn evaluate_per_with_missing_target_returns_zero() {
    let (mut runner, _real) = one_digimon_on_field("T-MIS");
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
    let bogus = PermanentHandle {
        player: 0,
        index: 99,
    };
    let f = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::StackSize,
        delta: 1000,
    };
    assert_eq!(evaluate(&f, &ctx, bogus), 0);
}
