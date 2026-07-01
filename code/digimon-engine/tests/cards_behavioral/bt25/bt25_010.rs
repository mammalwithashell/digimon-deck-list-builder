//! BT25-010 Hawkmon — Digimon, Lv.3, Red/Green, DP 2000, Cost 3.
//! Traits: Avian, Iliad, TS. Attribute: Free.
//! Evo: Lv.2 Red / cost 1 (printed). DCGO adds alt digivolution requirements:
//!   - Lv.2 base with [TS] trait → cost 0.
//!   - base named "Poromon" → cost 0.
//!
//! # Card text (cards.json — verbatim)
//!
//! [Your Turn] When this Digimon would digivolve into a Digimon card with
//! [Avian], [Bird], [Beast], [Animal] or [Sovereign], other than [Sea Animal],
//! in any of its traits, reduce the cost by 1.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_010.cs
//!   - AddSelfDigivolutionRequirementStaticEffect(TopCard.HasTSTraits, cost 0,
//!     level 2) and (TopCard == "Poromon", cost 0).
//!   - ChangeDigivolutionCostStaticEffect(changeValue: -1,
//!     permanentCondition: target == self, cardCondition: HasBird/HasBeast
//!     trait groups, isInheritedEffect: false, condition: IsOwnerTurn &&
//!     on battle area, setFixedCost: false).
//!   - ChangeSelfDPStaticEffect(changeValue: 2000, isInheritedEffect: true,
//!     condition: IsOwnerTurn) → inherited [Your Turn] +2000 DP.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 cost reduction (digivolve-target-trait gated; self-source only)
//! - D4 inherited declarative DP aura (your-turn gated)
//! - H alt digivolution paths (TS / Poromon → cost 0)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-010";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Lv.4 Red Digimon evolvable from a Lv.3 (cost 1) so it can digivolve over
/// Hawkmon. `traits` controls the cost-reduction gate.
fn lv4_target(card_id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Red];
    card.dp = Some(5000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 1,
    }];
    card
}

/// A Lv.3 Red carrier for the inherited-aura test (Hawkmon sits underneath).
fn lv3_carrier(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 3);
    card.colors = vec![CardColor::Red];
    card.dp = Some(3000);
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-010 YAML parses and compiles")
        .add_card(lv4_target("AVIAN-LV4", &["Avian"]))
        .add_card(lv4_target("BIRD-LV4", &["Bird"]))
        .add_card(lv4_target("BEAST-LV4", &["Beast"]))
        .add_card(lv4_target("ANIMAL-LV4", &["Animal"]))
        .add_card(lv4_target("SOVEREIGN-LV4", &["Sovereign"]))
        .add_card(lv4_target("SEA-ANIMAL-LV4", &["Sea Animal"]))
        .add_card(lv4_target("PLAIN-LV4", &["Plain"]))
        // A Bird Digimon that ALSO carries [Sea Animal] — the exclusion must
        // win: even though Bird matches, "other than [Sea Animal]" blocks it.
        .add_card(lv4_target("BIRD-SEA-LV4", &["Bird", "Sea Animal"]))
        .add_card(lv3_carrier("RED-CARRIER"))
        .add_card(make_test_card_with_level("FILL", "Filler", 2))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
}

/// Hawkmon standing on P0's field as its own top card, ready to be the
/// digivolving source for a Lv.4 hand card.
fn place_hawkmon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, CARD_ID, Some(0))
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_010_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-010 must be in the embedded DSL pack");

    assert_eq!(card.name, "Hawkmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    for trait_name in ["Avian", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-010 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_010_has_ts_and_poromon_cost0_alt_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(2) && f.trait_has.as_deref() == Some("TS"))
    });
    assert!(has_ts, "BT25-010 must have a Lv.2 [TS] → cost 0 alt-path");

    let has_poromon = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .is_some_and(|f| f.name_is.as_deref() == Some("Poromon"))
    });
    assert!(
        has_poromon,
        "BT25-010 must have a [Poromon] → cost 0 alt-path"
    );
}

#[test]
fn bt25_010_has_inherited_your_turn_dp_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier: Some(2000),
                ..
            })
        )
    });
    assert!(
        aura,
        "BT25-010 must have an inherited +2000 DP aura (DCGO ChangeSelfDP 2000, inherited)"
    );
}

#[test]
fn bt25_010_has_cost_reduction_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_reduction = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        has_reduction,
        "BT25-010 must declare a digivolve cost-reduction clause"
    );
}

// ─── Section 2 — Behavior: digivolve cost reduction (self-source only) ────────

/// Positive: Hawkmon digivolving into each matching trait reduces the cost from
/// 1 to 0.
#[test]
fn bt25_010_reduces_cost_for_matching_trait_targets() {
    for target_id in [
        "AVIAN-LV4",
        "BIRD-LV4",
        "BEAST-LV4",
        "ANIMAL-LV4",
        "SOVEREIGN-LV4",
    ] {
        let mut runner = base().start();
        runner.game.turn_count = 1;
        let hawkmon = place_hawkmon(&mut runner);
        let hand_index = push_to_hand(&mut runner, 0, target_id);

        let memory_before = runner.game.memory;
        let digivolved = runner.game.digivolve_from_hand(
            0,
            hand_index,
            hawkmon.index as usize,
            PlaySource::ByHand,
        );

        assert!(digivolved, "Hawkmon should digivolve into {target_id}");
        assert_eq!(
            runner.game.memory, memory_before,
            "base cost 1 must be reduced to 0 when digivolving into {target_id}"
        );
    }
}

/// Negative: a [Sea Animal] target is excluded ("other than [Sea Animal]") →
/// full cost is paid.
#[test]
fn bt25_010_does_not_reduce_for_sea_animal_target() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let hawkmon = place_hawkmon(&mut runner);
    let hand_index = push_to_hand(&mut runner, 0, "SEA-ANIMAL-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, hawkmon.index as usize, PlaySource::ByHand);

    assert!(
        digivolved,
        "Sea Animal target is still a legal digivolution"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "[Sea Animal] is excluded from the reduction → full cost 1 paid"
    );
}

/// Negative: the exclusion wins even when another matching trait is present —
/// a Bird + Sea Animal Digimon must NOT get the reduction.
#[test]
fn bt25_010_sea_animal_exclusion_overrides_other_matching_trait() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let hawkmon = place_hawkmon(&mut runner);
    let hand_index = push_to_hand(&mut runner, 0, "BIRD-SEA-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, hawkmon.index as usize, PlaySource::ByHand);

    assert!(digivolved, "Bird+Sea Animal target is a legal digivolution");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "'other than [Sea Animal]' must veto the reduction even with a Bird trait present"
    );
}

/// Negative: an unmatched trait (Plain) target is not reduced.
#[test]
fn bt25_010_does_not_reduce_for_unmatched_target() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let hawkmon = place_hawkmon(&mut runner);
    let hand_index = push_to_hand(&mut runner, 0, "PLAIN-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_index, hawkmon.index as usize, PlaySource::ByHand);

    assert!(digivolved, "plain target is a legal digivolution");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "a non-matching trait target pays the full cost"
    );
}

/// Negative: the reduction is scoped to Hawkmon being the digivolving SOURCE.
/// A different Lv.3 source digivolving into a Bird target (while Hawkmon merely
/// sits on the field as a bystander) pays full cost.
#[test]
fn bt25_010_reduction_only_when_hawkmon_is_the_source() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    let _bystander = place_hawkmon(&mut runner);
    let other_source = runner.place_on_field(0, "RED-CARRIER", Some(0));
    let hand_index = push_to_hand(&mut runner, 0, "BIRD-LV4");

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_index,
        other_source.index as usize,
        PlaySource::ByHand,
    );

    assert!(digivolved, "the other Lv.3 source digivolves into Bird");
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "Hawkmon only reduces when IT is the digivolving source"
    );
}

// ─── Section 3 — Behavior: inherited [Your Turn] +2000 DP aura ────────────────

#[test]
fn bt25_010_inherited_grants_plus_2000_on_your_turn() {
    let mut carrier = lv3_carrier("CARRIER");
    carrier.dp = Some(3000);

    let mut runner = base().add_card(carrier).memory(10).start();
    runner.game.turn_count = 1; // your turn
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    assert_eq!(
        runner.effective_dp(stack),
        Some(5000),
        "carrier base 3000 + inherited 2000 (your turn) = 5000"
    );
}

#[test]
fn bt25_010_inherited_does_not_grant_on_opponents_turn() {
    let mut carrier = lv3_carrier("CARRIER");
    carrier.dp = Some(3000);

    let mut runner = base().add_card(carrier).memory(10).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    // Switch to the opponent's turn.
    runner.game.turn_player_idx = 1;

    assert_eq!(
        runner.effective_dp(stack),
        Some(3000),
        "[Your Turn] inherited +2000 must not apply on the opponent's turn"
    );
}
