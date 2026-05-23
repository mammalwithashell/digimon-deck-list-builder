//! BT21-055 Sunarizamon
//!
//! This file covers both clauses:
//! - Inherited: when trashed from Mineral/Rock digivolution stack, delete
//!   opponent Digimon cost 4 or less.
//! - [Your Turn] face-up cost_reduction: when THIS Digimon would digivolve
//!   into a [Mineral] or [Rock] trait card, reduce the digivolution cost by 1.
//!   Substrate: Phase 2 Track H (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::EvoCost;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-055")
        .expect("BT21-055 YAML parses and compiles")
        .build()
}

/// Build a Lv.4 black Digimon with the given trait and evo cost 1 from Lv.3 black.
fn make_lv4_black(id: &str, trait_name: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec![trait_name.to_string()];
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 5, // Black = 5
        memory_cost: 1,
    }];
    c
}

#[test]
fn bt21_055_has_inherited_source_trash_delete_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT21-055")
        .expect("BT21-055 compiled card present");

    let inherited = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed] =>
        {
            Some(triggered)
        }
        _ => None,
    });

    assert!(
        inherited.is_some(),
        "BT21-055 must compile inherited source-trash timing"
    );
}

#[test]
fn bt21_055_source_trash_deletes_only_opponent_digimon_cost_four_or_less() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());
    let mut cheap = make_test_card("CHEAP4", "Cheap Cost 4");
    cheap.play_cost = 4;
    let mut expensive = make_test_card("EXPENSIVE5", "Expensive Cost 5");
    expensive.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-055")
        .expect("BT21-055 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(expensive)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "BT21-055");
    runner.place_on_field(1, "CHEAP4", None);
    runner.place_on_field(1, "EXPENSIVE5", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only play-cost-4-or-less opponent Digimon should be selectable"
    );
    runner
        .auto_resolve()
        .expect("BT21-055 delete target resolves");

    let remaining: Vec<&str> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data))
        .collect();
    assert_eq!(remaining, vec!["EXPENSIVE5"]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Face-up [Your Turn] cost-reduction clause tests
// Substrate: Phase 2 Track H (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
// Pattern: `kind: cost_reduction` + `source_is_cost_target_permanent: true`
// + `cost_target: { any_of: [{ trait_has: Mineral }, { trait_has: Rock }] }`.
// ─────────────────────────────────────────────────────────────────────────────

/// BT21-055 compiled card must contain exactly 2 clauses: one CostReduction
/// (face-up, [Your Turn] digivolve-into-Mineral/Rock) and one Triggered
/// (inherited source-trash delete).
#[test]
fn bt21_055_has_two_clauses_including_cost_reduction() {
    let runner = runner();
    let card = runner
        .compiled_card("BT21-055")
        .expect("BT21-055 compiled card present");

    assert_eq!(
        card.effects.len(),
        2,
        "BT21-055 must have exactly 2 clauses (CostReduction + Triggered inherited); got {}",
        card.effects.len()
    );

    let has_cost_reduction = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        has_cost_reduction,
        "BT21-055 must have a CostReduction clause for the [Your Turn] digivolve-cost reduction"
    );
}

/// POSITIVE (Mineral trait): BT21-055 on field, Lv.4 Mineral Digimon in hand
/// with base evo cost 1. After reduction, effective cost is 0 — memory unchanged.
#[test]
fn bt21_055_cost_reduction_fires_when_digivolving_into_mineral() {
    let mineral_lv4 = make_lv4_black("MINERAL-LV4", "Mineral");
    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-055")
        .expect("parses")
        .add_card(mineral_lv4)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();
    runner.game.turn_count = 1; // [Your Turn]

    let bt21_055 = runner.place_on_field(0, "BT21-055", Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "MINERAL-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        bt21_055.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "BT21-055 must digivolve into MINERAL-LV4 (evo cost 1 - 1 reduction = 0)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before,
        "digivolution cost must be 0 after BT21-055 reduction (Mineral trait); \
         memory_before={memory_before}"
    );
}

/// POSITIVE (Rock trait): same as Mineral but with [Rock] trait — cost still reduced.
#[test]
fn bt21_055_cost_reduction_fires_when_digivolving_into_rock() {
    let rock_lv4 = make_lv4_black("ROCK-LV4", "Rock");
    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-055")
        .expect("parses")
        .add_card(rock_lv4)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let bt21_055 = runner.place_on_field(0, "BT21-055", Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "ROCK-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        bt21_055.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "BT21-055 must digivolve into ROCK-LV4 (evo cost 1 - 1 reduction = 0)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before,
        "digivolution cost must be 0 after BT21-055 reduction (Rock trait); \
         memory_before={memory_before}"
    );
}

/// NEGATIVE (non-Mineral/non-Rock trait): BT21-055 on field, Lv.4 [Reptile]
/// Digimon in hand with base evo cost 1. Cost reduction must NOT fire.
/// Memory decreases by 1 (the base evo cost).
#[test]
fn bt21_055_cost_reduction_does_not_fire_for_non_mineral_rock_target() {
    let reptile_lv4 = make_lv4_black("REPTILE-LV4", "Reptile");
    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-055")
        .expect("parses")
        .add_card(reptile_lv4)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let bt21_055 = runner.place_on_field(0, "BT21-055", Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "REPTILE-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let _ = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        bt21_055.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "cost reduction must NOT fire for [Reptile] target (not Mineral/Rock); \
         base evo cost 1 must be paid in full; memory_before={memory_before}"
    );
}
