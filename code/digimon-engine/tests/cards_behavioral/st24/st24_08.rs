//! ST24-08 Lalamon — Digimon, Lv.3, Green, DP 1000, Cost 3.
//! Traits: Vegetation, DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim)
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//!   [DATA SQUAD] trait, reduce the cost by 1.
//! Inherited [All Turns]: This Digimon gets +1000 DP.
//!
//! Printed digivolve box: [Digivolve] Lv.2 w/[DATA SQUAD] trait: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Green/ST24_08.cs
//!
//! # Patterns — sister to ST23-02 (cost-reduction, trait-swapped); inherited
//! +1000 DP is [All Turns] (no your-turn gate, unlike the digivolve-cost clause).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledScope,
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

use crate::dsl_card_data::card_data_from_compiled;

const CARD_ID: &str = "ST24-08";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A Lv.4 Green [DATA SQUAD] Digimon that digivolves from a Lv.3 green
/// (printed evo cost 1).
fn make_ds_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("DS-LV4", "DataSquadLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["DATA SQUAD".to_string()];
    c.colors = vec![CardColor::Green];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 3, // Green
        memory_cost: 1,
    }];
    c
}

/// A Lv.4 Green Digimon WITHOUT the [DATA SQUAD] trait (printed evo cost 1).
fn make_non_ds_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PLAIN-LV4", "PlainLv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Beast".to_string()];
    c.colors = vec![CardColor::Green];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 3,
        memory_cost: 1,
    }];
    c
}

/// A non-Lalamon Lv.3 Green Digimon usable as a different digivolution source.
fn make_plain_lv3() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PLAIN-LV3", "PlainLv3");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
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
fn st24_08_metadata_and_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-08 in embedded DSL pack")
        .memory(0)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Lalamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(card.traits.contains(&"DATA SQUAD".to_string()));
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "DATA SQUAD")
            .unwrap_or(false)
    });
    assert!(ds, "Lv.2 [DATA SQUAD] alt-path present");
}

#[test]
fn st24_08_has_cost_reduction_and_inherited_dp() {
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

    let has_inherited_dp = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier: Some(1000), ..
        }) if *scope == CompiledScope::Inherited)
    });
    assert!(has_inherited_dp, "inherited +1000 DP aura present");
}

/// [All Turns] DP buff carries NO your-turn gate (unlike the digivolve-cost
/// clause which IS your-turn-gated).
#[test]
fn st24_08_inherited_dp_is_all_turns_no_your_turn_gate() {
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
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                active_when,
                dp_modifier: Some(1000),
                ..
            }) => Some(active_when.clone()),
            _ => None,
        })
        .expect("DP aura clause");

    fn has_your_turn(p: &CompiledPredicate) -> bool {
        p.your_turn == Some(true)
            || p.all_of.iter().any(has_your_turn)
            || p.any_of.iter().any(has_your_turn)
    }
    assert!(
        active_when.as_ref().map_or(true, |p| !has_your_turn(p)),
        "[All Turns] DP buff must NOT carry a your-turn gate"
    );
}

// ─── Section 2 — Behavioral: cost reduction on digivolve-into-target ─────────

/// POSITIVE: digivolving FROM Lalamon INTO a Lv.4 [DATA SQUAD] target reduces the
/// digivolution cost by 1 (printed evo cost 1 → effective 0; memory unchanged).
#[test]
fn st24_08_cost_reduction_fires_digivolving_into_data_squad() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_ds_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let lalamon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "DS-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, lalamon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "Lalamon must digivolve into DS-LV4 (effective cost 1 - 1 = 0)"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "memory unchanged: 1 evo cost - 1 Lalamon reduction = 0"
    );
}

/// NEGATIVE (trait gate): digivolving INTO a non-[DATA SQUAD] Lv.4 must NOT
/// trigger the cost reduction — full evo cost 1 is paid.
#[test]
fn st24_08_cost_reduction_does_not_fire_for_non_data_squad_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_non_ds_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let lalamon = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "PLAIN-LV4");

    let memory_before = runner.game.memory;
    let _ =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, lalamon.index as usize, PlaySource::ByHand);
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "target without [DATA SQUAD] — reduction must not apply; full cost 1 paid"
    );
}

/// NEGATIVE ("THIS Digimon" gate): a DIFFERENT Lv.3 source does not get
/// Lalamon's cost reduction.
#[test]
fn st24_08_cost_reduction_does_not_fire_for_different_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_plain_lv3())
        .add_card(make_ds_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let _lalamon = runner.place_on_field(0, CARD_ID, Some(0));
    let plain = runner.place_on_field(0, "PLAIN-LV3", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "DS-LV4");

    let memory_before = runner.game.memory;
    let _ = runner
        .game
        .digivolve_from_hand(0, hand_idx, plain.index as usize, PlaySource::ByHand);
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "a non-Lalamon source must not receive Lalamon's cost reduction"
    );
}
