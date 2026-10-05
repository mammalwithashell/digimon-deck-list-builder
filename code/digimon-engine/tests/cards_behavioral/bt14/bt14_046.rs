//! BT14-046 Togemon — Digimon, Lv.4, Green, DP 3000, Cost 4. Trait: Vegetation.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT14-046.md)
//! [Your Turn][Once Per Turn] When you would play a green Tamer card from your
//! hand, by suspending 1 of your green Digimon, reduce the play cost by 3.
//! Inherited: [Your Turn][Once Per Turn] When this Digimon would digivolve, if
//! you have a green Tamer, reduce the digivolution cost by 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Green/BT14_046.cs
//!
//! # Patterns
//! - BeforePayCost play-cost reducer (`when_any_ally_played`) with an
//!   INTERACTIVE pay_cost (select own green Digimon + suspend) — the
//!   play-path park of `cost_hooks/pay_cost_play_delete_reducer.rs`
//! - optional accept/decline, [Once Per Turn], `cost_target_from_hand`
//! - Inherited [Your Turn][OPT] digivolve cost -1 gated on a green Tamer

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledPredicate, CompiledScope,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT14-046";

fn tamer(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![color];
    c
}

fn digimon(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![color];
    c.play_cost = 5;
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT14-046 YAML parses")
        .add_card(tamer("GTAMER", CardColor::Green, 5))
        .add_card(tamer("GTAMER2", CardColor::Green, 5))
        .add_card(tamer("RTAMER", CardColor::Red, 5))
        .add_card(digimon("GDIGI", CardColor::Green))
        .add_card(digimon("RDIGI", CardColor::Red))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
}

fn hand_idx(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[0].battle_area[h.index as usize].is_suspended
}

fn on_field(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

#[test]
fn bt14_046_structural_play_cost_reducer() {
    let runner = builder().build();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Togemon");
    let (optional, opt, amount, active_when, ally, condition, pay_len) = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope: CompiledScope::FaceUp,
                optional,
                once_per_turn,
                amount,
                active_when,
                when_any_ally_played,
                condition,
                pay_cost,
                ..
            }) => Some((
                *optional,
                *once_per_turn,
                *amount,
                active_when.clone(),
                when_any_ally_played.clone(),
                condition.clone(),
                pay_cost.len(),
            )),
            _ => None,
        })
        .expect("face-up play-cost reducer");
    assert!(optional, "'by suspending' is a cost the player may decline");
    assert!(opt, "[Once Per Turn]");
    assert_eq!(amount, Some(3));
    assert!(ally.is_some(), "keyed on a played card (green Tamer)");
    assert_eq!(pay_len, 2, "select + suspend");
    fn any(p: &CompiledPredicate, f: &dyn Fn(&CompiledPredicate) -> bool) -> bool {
        f(p) || p.all_of.iter().any(|q| any(q, f))
    }
    assert!(any(active_when.as_ref().unwrap(), &|p| p.your_turn == Some(true)), "[Your Turn]");
    assert!(
        any(condition.as_ref().unwrap(), &|p| p.cost_target_from_hand == Some(true)),
        "'from your hand'"
    );
}

/// Accept: suspend the chosen green Digimon; green Tamer costs 5 - 3 = 2.
#[test]
fn bt14_046_accept_suspends_chosen_green_digimon_and_reduces_by_3() {
    let mut runner = builder().hand(0, &["GTAMER"]).start();
    let toge = runner.place_on_field(0, CARD_ID, Some(0));
    let g = runner.place_on_field(0, "GDIGI", Some(0));
    let r = runner.place_on_field(0, "RDIGI", Some(0));
    let mem = runner.memory();

    runner.play(0, hand_idx(&runner, "GTAMER"));
    assert!(runner.pending_selection().is_some(), "reducer offered");
    assert!(runner.pending_is_optional(), "optional reducer");
    assert_eq!(runner.memory(), mem, "nothing paid yet");
    runner.accept_optional_trigger().expect("accept");

    let pending = runner.pending_selection().expect("suspend pick parks");
    assert!(matches!(pending.kind, SelectionKind::OwnField));
    assert!(!pending.is_optional, "once accepted, the suspend cost must be paid");
    let pick_g = encode_attack(0, g.index as u16);
    assert!(pending.valid_action_ids.contains(&pick_g));
    assert!(
        pending.valid_action_ids.contains(&encode_attack(0, toge.index as u16)),
        "Togemon itself is a green Digimon"
    );
    assert!(
        !pending.valid_action_ids.contains(&encode_attack(0, r.index as u16)),
        "red Digimon is not eligible"
    );
    runner.execute_action(0, pick_g).expect("suspend GDIGI");
    let _ = runner.auto_resolve();

    assert!(suspended(&runner, g), "chosen green Digimon suspended");
    assert!(!suspended(&runner, toge));
    assert!(on_field(&runner, "GTAMER"), "Tamer played");
    assert_eq!(mem - runner.memory(), 2, "play cost 5 reduced by 3");
}

/// Decline: full cost, nothing suspended.
#[test]
fn bt14_046_decline_pays_full_cost() {
    let mut runner = builder().hand(0, &["GTAMER"]).start();
    let toge = runner.place_on_field(0, CARD_ID, Some(0));
    let mem = runner.memory();
    runner.play(0, 0);
    runner.decline_optional_trigger().expect("decline");
    let _ = runner.auto_resolve();
    assert!(on_field(&runner, "GTAMER"));
    assert!(!suspended(&runner, toge));
    assert_eq!(mem - runner.memory(), 5);
}

/// Non-green Tamer → not offered.
#[test]
fn bt14_046_red_tamer_not_reduced() {
    let mut runner = builder().hand(0, &["RTAMER"]).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let mem = runner.memory();
    runner.play(0, 0);
    assert!(runner.pending_selection().is_none(), "no prompt for a red Tamer");
    assert_eq!(mem - runner.memory(), 5);
}

/// A green DIGIMON play (not a Tamer) → not offered.
#[test]
fn bt14_046_green_digimon_play_not_reduced() {
    let mut runner = builder().hand(0, &["GDIGI"]).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let mem = runner.memory();
    runner.play(0, 0);
    assert!(runner.pending_selection().is_none(), "no prompt for a Digimon");
    assert_eq!(mem - runner.memory(), 5);
}

/// No unsuspended green Digimon to pay with → not offered.
#[test]
fn bt14_046_not_offered_without_unsuspended_green_digimon() {
    let mut runner = builder().hand(0, &["GTAMER"]).start();
    let toge = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "RDIGI", Some(0));
    runner.game.players[0].battle_area[toge.index as usize].is_suspended = true;
    let mem = runner.memory();
    runner.play(0, 0);
    assert!(runner.pending_selection().is_none(), "cost unpayable → no prompt");
    assert_eq!(mem - runner.memory(), 5);
}

/// [Once Per Turn]: a second green Tamer the same turn is not offered.
#[test]
fn bt14_046_once_per_turn() {
    let mut runner = builder().hand(0, &["GTAMER", "GTAMER2"]).memory(10).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "GDIGI", Some(0));
    runner.play(0, hand_idx(&runner, "GTAMER"));
    runner.accept_optional_trigger().expect("accept");
    let _ = runner.auto_resolve(); // only one pick needed
    // Memory after the first (reduced) play: 10 - 2 = 8.
    let mem = runner.memory();
    runner.play(0, hand_idx(&runner, "GTAMER2"));
    assert!(runner.pending_selection().is_none(), "OPT: not offered again this turn");
    assert!(on_field(&runner, "GTAMER2"));
    assert_eq!(mem - runner.memory(), 5, "second green Tamer pays full cost");
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
