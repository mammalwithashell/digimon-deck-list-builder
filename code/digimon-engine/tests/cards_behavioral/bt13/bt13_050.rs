//! BT13-050 Sunflowmon — Digimon, Lv.4, Green, DP 3000, Cost 4. Trait: Vegetation.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT13-050.md)
//! [Main] By suspending this Digimon, 1 of your Digimon may digivolve into a
//! Digimon card with [Fairy] in one of its traits in the hand for the
//! digivolution cost. When it would digivolve by this effect, reduce the
//! digivolution cost by 2.
//! Inherited: [Your Turn][Once Per Turn] When this Digimon would digivolve, if
//! you have a green Tamer, reduce the digivolution cost by 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Green/BT13_050.cs
//!
//! # Patterns
//! - `main_on_field` suspend-self activated ability gated `source_is_unsuspended`
//! - optional own-Digimon pick filtered by `has_digivolve_candidate` (BT25-092)
//! - hand pick filtered by `can_digivolve_onto` + `effect_initiated_digivolve`
//!   with `cost: { reduce: 2 }`
//! - Inherited [Your Turn][OPT] digivolve cost -1 gated on a green Tamer

use digimon_dsl::compiled::{CompiledClause, CompiledCostDelta, CompiledStep, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
    PLAY_HAND_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT13-050";

/// Green Lv.5 Digimon digivolving from green Lv.4 for 3; `traits` given.
fn lv5(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Green];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c.evo_costs = vec![EvoCost { level: 4, card_color: 3, memory_cost: 3 }];
    c
}

/// A Lv.3 red Digimon — no Fairy card can digivolve onto it.
fn red_lv3(id: &str) -> CardData {
    make_test_card(id, id)
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-050 YAML parses")
        .add_card(lv5("FAIRY5", &["Fairy"]))
        .add_card(lv5("PLAIN5", &["Beast"]))
        .add_card(red_lv3("RED3"))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(5)
}

fn main_action(h: PermanentHandle) -> u16 {
    FIELD_EFFECT_START + h.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn activate(runner: &mut DebugRunner, h: PermanentHandle) {
    let action = main_action(h);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[action as usize], 1.0, "[Main] must be legal");
    runner.game.decode_action(action, 0);
    runner.game.drain_effect_queue();
}

fn hand_idx(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[0].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

#[test]
fn bt13_050_structural_main_clause() {
    let runner = builder().build();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Sunflowmon");
    assert_eq!(card.level, Some(4));
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainOnField] => Some(t),
            _ => None,
        })
        .expect("[Main] on-field clause");
    assert!(
        matches!(main.process.first(), Some(CompiledStep::Suspend { .. })),
        "suspending this Digimon is the cost, first step"
    );
    assert!(
        main.process.iter().any(|s| matches!(
            s,
            CompiledStep::EffectInitiatedDigivolve {
                cost: CompiledCostDelta::Reduce(2),
                ignore_requirements: false,
                ..
            }
        )),
        "digivolve for the digivolution cost reduced by 2"
    );
}

#[test]
fn bt13_050_main_illegal_when_suspended() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[me.index as usize].is_suspended = true;
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[main_action(me) as usize], 0.0, "needs an unsuspended carrier");
}

/// Sunflowmon itself digivolves into the Fairy Lv.5: suspend → optional
/// own-Digimon pick → hand pick → pays 3 - 2 = 1.
#[test]
fn bt13_050_main_digivolves_into_fairy_with_cost_minus_2() {
    let mut runner = builder().hand(0, &["FAIRY5", "PLAIN5"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let mem = runner.game.memory;
    activate(&mut runner, me);
    assert!(runner.game.players[0].battle_area[me.index as usize].is_suspended, "cost paid");

    let view = runner.pending_selection_view().expect("own-Digimon pick");
    assert!(view.is_optional, "'may' digivolve");
    let pick = encode_attack(0, me.index as u16);
    assert!(view.valid_action_ids.contains(&pick));
    runner.execute_action(0, pick).expect("pick Sunflowmon");

    let view = runner.pending_selection_view().expect("hand pick");
    let fairy = PLAY_HAND_START + hand_idx(&runner, "FAIRY5") as u16;
    let plain = PLAY_HAND_START + hand_idx(&runner, "PLAIN5") as u16;
    assert!(view.valid_action_ids.contains(&fairy), "Fairy card offered");
    assert!(!view.valid_action_ids.contains(&plain), "non-Fairy card not offered");
    runner.execute_action(0, fairy).expect("pick FAIRY5");
    let _ = runner.auto_resolve();

    assert_eq!(top_id(&runner, me), "FAIRY5", "digivolved into the Fairy card");
    assert_eq!(mem - runner.game.memory, 1, "digivolution cost 3 reduced by 2");
}

/// Declining the "may" leaves the carrier suspended and nothing digivolves.
#[test]
fn bt13_050_main_decline_still_suspends() {
    let mut runner = builder().hand(0, &["FAIRY5"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let mem = runner.game.memory;
    activate(&mut runner, me);
    let view = runner.pending_selection_view().expect("own-Digimon pick");
    assert!(view.valid_action_ids.contains(&PASS));
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, me), CARD_ID);
    assert!(runner.game.players[0].battle_area[me.index as usize].is_suspended);
    assert_eq!(runner.game.memory, mem);
    assert_eq!(runner.game.players[0].hand.len(), 1);
}

/// Only Digimon with a legal Fairy result in hand are offered: a red Lv.3
/// (no route for the Lv.5) is not a candidate.
#[test]
fn bt13_050_main_only_offers_digimon_with_a_fairy_route() {
    let mut runner = builder().hand(0, &["FAIRY5"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let red = runner.place_on_field(0, "RED3", Some(0));
    activate(&mut runner, me);
    let view = runner.pending_selection_view().expect("own-Digimon pick");
    assert!(view.valid_action_ids.contains(&encode_attack(0, me.index as u16)));
    assert!(
        !view.valid_action_ids.contains(&encode_attack(0, red.index as u16)),
        "RED3 has no Fairy digivolution in hand"
    );
}

/// With only a non-Fairy card in hand, activation just suspends — no prompt.
#[test]
fn bt13_050_main_no_fairy_in_hand_only_suspends() {
    let mut runner = builder().hand(0, &["PLAIN5"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    activate(&mut runner, me);
    assert!(runner.pending_selection().is_none(), "no candidate → no prompt");
    assert!(runner.game.players[0].battle_area[me.index as usize].is_suspended);
    assert_eq!(top_id(&runner, me), CARD_ID);
}

/// The -2 is scoped to "by this effect": a normal digivolve into the same
/// Fairy card pays the full 3.
#[test]
fn bt13_050_reduction_only_by_this_effect() {
    let mut runner = builder().hand(0, &["FAIRY5"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let mem = runner.game.memory;
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, me.index as usize, PlaySource::ByHand));
    assert_eq!(mem - runner.game.memory, 3);
}

// ═══════════════════════════════════════════════════════════════════════════
// Inherited — [Your Turn][Once Per Turn] When this Digimon would digivolve,
// if you have a green Tamer, reduce the digivolution cost by 1.
// ═══════════════════════════════════════════════════════════════════════════

mod inherited {
    use super::CARD_ID;
    use digimon_dsl::compiled::{
        CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledScope,
    };
    use digimon_engine::card_data::{CardData, EvoCost};
    use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
    use digimon_engine::enums::{CardColor, CardKind, PlaySource};

    /// Printed level of the card under test.
    const LV: u8 = 4;

    fn tamer(id: &str, color: CardColor) -> CardData {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Tamer;
        c.level = None;
        c.dp = None;
        c.play_cost = 2;
        c.colors = vec![color];
        c
    }

    /// Plain green Digimon of `level`, digivolving from a green `level - 1`
    /// for `evo` memory.
    fn green_digimon(id: &str, level: u8, evo: u8) -> CardData {
        let mut c = make_test_card(id, id);
        c.level = Some(level);
        c.dp = Some(1000 * level as i32);
        c.play_cost = level as u16 + 2;
        c.colors = vec![CardColor::Green];
        c.evo_costs = vec![EvoCost {
            level: level - 1,
            card_color: 3, // Green
            memory_cost: evo as u16,
        }];
        c
    }

    fn builder() -> DebugRunnerBuilder {
        let fill = make_test_card("INH-FILL", "Filler");
        DebugRunner::builder()
            .dsl_card(CARD_ID)
            .expect("card YAML parses")
            .add_card(tamer("GREEN-TAMER", CardColor::Green))
            .add_card(tamer("RED-TAMER", CardColor::Red))
            // TOP sits on the card under test; EVO1 / EVO2 digivolve onto it.
            .add_card(green_digimon("TOP", LV + 1, 3))
            .add_card(green_digimon("EVO1", LV + 2, 3))
            .add_card(green_digimon("EVO2", LV + 3, 3))
            // Digivolves directly onto the card under test (face-up case).
            .add_card(green_digimon("EVO-DIRECT", LV + 1, 3))
            .add_card(fill)
            .deck(0, &["INH-FILL"; 6])
            .deck(1, &["INH-FILL"; 6])
            .memory(10)
    }

    fn digivolve(runner: &mut DebugRunner, field_index: u8, card_id: &str) -> i16 {
        let hand_idx = runner.add_to_hand(0, card_id);
        let before = runner.game.memory;
        let ok = runner
            .game
            .digivolve_from_hand(0, hand_idx, field_index as usize, PlaySource::ByHand);
        assert!(ok, "digivolve into {card_id} must succeed");
        assert!(runner.pending_selection().is_none(), "no prompt expected");
        (before - runner.game.memory).abs()
    }

    #[test]
    fn inherited_clause_is_structurally_inherited_opt_your_turn() {
        let runner = builder().build();
        let card = runner.compiled_card(CARD_ID).expect("compiled");
        let found: Vec<(bool, Option<CompiledPredicate>, Option<i32>)> = card
            .effects
            .iter()
            .filter_map(|c| match c {
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                    scope: CompiledScope::Inherited,
                    once_per_turn,
                    active_when,
                    amount,
                    ..
                }) => Some((*once_per_turn, active_when.clone(), *amount)),
                _ => None,
            })
            .collect();
        assert_eq!(found.len(), 1, "exactly one inherited cost reducer");
        let (opt, active_when, amount) = found.into_iter().next().unwrap();
        assert!(opt, "[Once Per Turn]");
        assert_eq!(amount, Some(1), "reduce by 1");
        fn has_your_turn(p: &CompiledPredicate) -> bool {
            p.your_turn == Some(true) || p.all_of.iter().any(has_your_turn)
        }
        assert!(
            active_when.as_ref().map_or(false, has_your_turn),
            "[Your Turn] gate present"
        );
    }

    /// POSITIVE: buried under TOP with a green Tamer → digivolving the stack
    /// costs 3 - 1 = 2.
    #[test]
    fn inherited_reduces_digivolve_cost_with_green_tamer() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        runner.place_on_field(0, "GREEN-TAMER", Some(0));
        let stack = runner.place_stack(0, &[CARD_ID, "TOP"]);
        assert_eq!(digivolve(&mut runner, stack.index, "EVO1"), 2);
    }

    /// NEGATIVE (green-Tamer gate): a red Tamer only → full cost 3.
    #[test]
    fn inherited_no_reduction_with_only_non_green_tamer() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        runner.place_on_field(0, "RED-TAMER", Some(0));
        let stack = runner.place_stack(0, &[CARD_ID, "TOP"]);
        assert_eq!(digivolve(&mut runner, stack.index, "EVO1"), 3);
    }

    /// NEGATIVE (green-Tamer gate): no Tamer at all → full cost 3.
    #[test]
    fn inherited_no_reduction_without_tamer() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        let stack = runner.place_stack(0, &[CARD_ID, "TOP"]);
        assert_eq!(digivolve(&mut runner, stack.index, "EVO1"), 3);
    }

    /// NEGATIVE (inherited scope): face-up (card is the top) → not active.
    #[test]
    fn inherited_inactive_while_card_is_the_top() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        runner.place_on_field(0, "GREEN-TAMER", Some(0));
        let me = runner.place_on_field(0, CARD_ID, Some(0));
        assert_eq!(digivolve(&mut runner, me.index, "EVO-DIRECT"), 3);
    }

    /// NEGATIVE ("this Digimon"): another Digimon's digivolve is not reduced.
    #[test]
    fn inherited_does_not_reduce_other_digimon() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        runner.place_on_field(0, "GREEN-TAMER", Some(0));
        let _carrier = runner.place_stack(0, &[CARD_ID, "TOP"]);
        let other = runner.place_on_field(0, "TOP", Some(0));
        assert_eq!(digivolve(&mut runner, other.index, "EVO1"), 3);
    }

    /// OPT: the second digivolve of the same stack in the same turn is not
    /// reduced; it resets on the controller's next turn.
    #[test]
    fn inherited_once_per_turn_and_resets_next_turn() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        runner.place_on_field(0, "GREEN-TAMER", Some(0));
        let stack = runner.place_stack(0, &[CARD_ID, "TOP"]);
        assert_eq!(digivolve(&mut runner, stack.index, "EVO1"), 2, "first reduced");
        assert_eq!(
            digivolve(&mut runner, stack.index, "EVO2"),
            3,
            "OPT: second digivolve same turn pays full cost"
        );
    }

    #[test]
    fn inherited_opt_resets_on_next_own_turn() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        runner.place_on_field(0, "GREEN-TAMER", Some(0));
        let stack = runner.place_stack(0, &[CARD_ID, "TOP"]);
        assert_eq!(digivolve(&mut runner, stack.index, "EVO1"), 2);
        runner.end_turn();
        runner.end_turn();
        assert_eq!(runner.game.turn_player(), 0);
        let _ = runner.auto_resolve();
        assert_eq!(
            digivolve(&mut runner, stack.index, "EVO2"),
            2,
            "OPT resets: reduced again on the next own turn"
        );
    }

    /// [Your Turn] only: during the opponent's turn the reducer is inactive.
    #[test]
    fn inherited_inactive_on_opponents_turn() {
        let mut runner = builder().start();
        runner.game.turn_count = 1;
        runner.place_on_field(0, "GREEN-TAMER", Some(0));
        let stack = runner.place_stack(0, &[CARD_ID, "TOP"]);
        runner.end_turn();
        assert_eq!(runner.game.turn_player(), 1);
        let _ = runner.auto_resolve();
        assert_eq!(
            digivolve(&mut runner, stack.index, "EVO1"),
            3,
            "[Your Turn]: no reduction during the opponent's turn"
        );
    }
}
