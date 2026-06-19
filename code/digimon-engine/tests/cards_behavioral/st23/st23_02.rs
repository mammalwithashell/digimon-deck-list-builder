//! ST23-02 Liollmon — Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Holy Beast, Glowing Dawn, BEATBREAK. Attribute: Vaccine.
//!
//! # Card text (data/cards.json — verbatim)
//!
//! **Own effect:**
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//! [Glowing Dawn] trait, reduce the cost by 1.
//!
//! **Inherited effect:**
//! ＜Barrier＞ (When this Digimon would be deleted in battle, by trashing your
//! top security card, it isn't deleted.)
//!
//! Printed digivolve box: [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Yellow/ST23_02.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 — BeforePayCost cost reduction gated on digivolve-target trait
//!   (cost_target + source_is_cost_target_permanent; BT23-005 idiom).
//! - H14 — inherited <Barrier> keyword grant.
//! - alt-path Lv.2 [Glowing Dawn] cost 0.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledScope,
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

use crate::dsl_card_data::card_data_from_compiled;

const CARD_ID: &str = "ST23-02";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A Lv.4 Yellow [Glowing Dawn] Digimon that digivolves from a Lv.3 yellow
/// (printed evo cost 1).
fn make_glowing_dawn_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("GD-LV4", "GlowingDawnLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 2, // Yellow
        memory_cost: 1,
    }];
    c
}

/// A Lv.4 Yellow Digimon WITHOUT the [Glowing Dawn] trait (printed evo cost 1).
fn make_non_gd_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PLAIN-LV4", "PlainLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Beast".to_string()];
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 2,
        memory_cost: 1,
    }];
    c
}

/// A non-Liollmon Lv.3 Yellow Digimon usable as a different digivolution source.
fn make_plain_lv3() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PLAIN-LV3", "PlainLv3");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: usize, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in card_data"));
    let card_index = runner.game.next_card_index();
    runner.game.players[p]
        .hand
        .push(CardSource::new(data_idx, p as u8, card_index));
    runner.game.players[p].hand.len() - 1
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st23_02_metadata() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-02 in embedded DSL pack")
        .memory(0)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Liollmon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    for t in ["Holy Beast", "Glowing Dawn", "BEATBREAK"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn st23_02_has_glowing_dawn_alt_path_cost_0() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .memory(0)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let gd = card.alt_paths.iter().find(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)
    });
    assert!(gd.is_some(), "Lv.2 [Glowing Dawn] alt-path present");
}

#[test]
fn st23_02_has_cost_reduction_and_inherited_barrier() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .memory(0)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(has_cr, "cost-reduction clause present");

    let has_barrier = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Barrier" && *scope == CompiledScope::Inherited
        )
    });
    assert!(has_barrier, "inherited <Barrier> keyword grant present");
}

#[test]
fn st23_02_cost_reduction_carries_your_turn_gate() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .memory(0)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    let active_when = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                active_when,
                ..
            }) => Some(active_when.clone()),
            _ => None,
        })
        .expect("CostReduction clause")
        .expect("active_when predicate present");

    fn has_your_turn(p: &CompiledPredicate) -> bool {
        p.your_turn == Some(true)
            || p.all_of.iter().any(has_your_turn)
            || p.any_of.iter().any(has_your_turn)
    }
    assert!(
        has_your_turn(&active_when),
        "cost-reduction clause carries a [Your Turn] gate"
    );
}

// ─── Section 2 — Behavioral: cost reduction on digivolve-into-target ─────────

/// POSITIVE: digivolving FROM Liollmon INTO a Lv.4 [Glowing Dawn] target reduces
/// the digivolution cost by 1 (printed evo cost 1 → effective 0; memory unchanged).
#[test]
fn st23_02_cost_reduction_fires_digivolving_into_glowing_dawn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_glowing_dawn_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let liollmon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "GD-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, liollmon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "Liollmon must digivolve into GD-LV4 (effective cost 1 - 1 = 0)"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "memory unchanged: 1 evo cost - 1 Liollmon reduction = 0"
    );
}

/// NEGATIVE (trait gate): digivolving FROM Liollmon INTO a non-Glowing-Dawn Lv.4
/// must NOT trigger the cost reduction — full evo cost 1 is paid.
#[test]
fn st23_02_cost_reduction_does_not_fire_for_non_glowing_dawn_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_non_gd_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let liollmon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "PLAIN-LV4");

    let memory_before = runner.game.memory;
    let _ =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, liollmon.index as usize, PlaySource::ByHand);
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "target without [Glowing Dawn] — reduction must not apply; full cost 1 paid"
    );
}

/// NEGATIVE ("THIS Digimon" gate): with Liollmon present on the field as a
/// BYSTANDER, digivolving a DIFFERENT Lv.3 permanent into a Glowing-Dawn target
/// gets NO cost reduction.
#[test]
fn st23_02_cost_reduction_does_not_fire_for_different_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_plain_lv3())
        .add_card(make_glowing_dawn_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let _liollmon = runner.place_on_field(0, CARD_ID, Some(0));
    let plain = runner.place_on_field(0, "PLAIN-LV3", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "GD-LV4");

    let memory_before = runner.game.memory;
    let _ = runner
        .game
        .digivolve_from_hand(0, hand_idx, plain.index as usize, PlaySource::ByHand);
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "a non-Liollmon source must not receive Liollmon's cost reduction"
    );
}
