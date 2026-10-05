//! BT14-044 Palmon — Digimon, Lv.3, Green, DP 1000, Cost 3. Trait: Vegetation.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT14-044.md)
//! [Start of Your Main Phase] 1 of your opponent's Digimon gains "[All Turns]
//! When this Digimon becomes suspended, lose 2 memory." until the end of their
//! turn.
//! Inherited: [Your Turn][Once Per Turn] When this Digimon would digivolve, if
//! you have a green Tamer, reduce the digivolution cost by 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Green/BT14_044.cs — the granted body is
//! `selectedPermanent.TopCard.Owner.AddMemory(-2)`: the SUSPENDED Digimon's
//! controller (the opponent) loses the memory.
//!
//! # Patterns
//! - start_of_your_main_phase trigger, mandatory opponent-Digimon pick
//! - grant_triggered_effect on a selected binding (EX10-034 fixture) with
//!   `on_suspend` timing + `end_of_opponents_turn` expiry
//! - Inherited [Your Turn][OPT] digivolve cost -1 gated on a green Tamer

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT14-044";

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.dp = Some(1000);
    c
}

fn sec_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c
}

/// P0 has Palmon on field; P1 has OPP-A (and optionally OPP-B). Returns
/// (runner, opp_a, opp_b). Turn 1 is P0's; we end P0's turn and P1's turn so
/// P0's next Start of Main Phase fires Palmon.
fn setup(two_targets: bool) -> (DebugRunner, PermanentHandle, Option<PermanentHandle>) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT14-044 YAML parses")
        .add_card(opp_digimon("OPP-A"))
        .add_card(opp_digimon("OPP-B"))
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(sec_option("SEC"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["SEC"; 3])
        .security(1, &["SEC"; 3])
        .memory(0)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = if two_targets {
        Some(runner.place_on_field(1, "OPP-B", Some(0)))
    } else {
        None
    };
    (runner, a, b)
}

/// End P0's turn, then P1's turn, arriving at P0's Start of Main Phase.
fn to_p0_next_main(runner: &mut DebugRunner) {
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.end_turn();
}

fn grants(runner: &DebugRunner, h: PermanentHandle) -> usize {
    runner
        .game
        .modifiers
        .granted_triggered_for_timing(h, EffectTiming::OnSuspend)
        .len()
}

#[test]
fn bt14_044_structural_start_of_main_grant_on_suspend() {
    let runner = DebugRunner::builder().dsl_card(CARD_ID).expect("parses").build();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Palmon");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::StartOfYourMainPhase] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("Start of Your Main Phase clause");
    assert!(!clause.optional, "mandatory");
    let grant = clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::GrantTriggeredEffect { timing, expiry, body, .. } => {
                Some((timing.clone(), expiry.clone(), body.clone()))
            }
            _ => None,
        })
        .expect("grant_triggered_effect step");
    assert_eq!(grant.0, "on_suspend");
    assert_eq!(grant.1, "end_of_opponents_turn");
    assert!(format!("{:?}", grant.2).contains("LoseMemory"), "body loses memory: {:?}", grant.2);
}

/// Start of P0's main phase: mandatory pick among opponent Digimon; only the
/// selected one gains the granted effect.
#[test]
fn bt14_044_start_of_main_grants_selected_opponent_digimon_only() {
    let (mut runner, a, b) = setup(true);
    let b = b.unwrap();
    to_p0_next_main(&mut runner);
    assert_eq!(runner.game.turn_player(), 0);
    let pending = runner.pending_selection().expect("P0 picks an opponent Digimon");
    assert_eq!(pending.selecting_player, 0);
    assert!(!pending.is_optional, "the pick is mandatory");
    let pick_a = encode_attack(0, a.index as u16);
    assert!(pending.valid_action_ids.contains(&pick_a));
    runner.game.resolve_selection(0, pick_a).expect("select OPP-A");
    assert_eq!(grants(&runner, a), 1, "selected Digimon gains the effect");
    assert_eq!(grants(&runner, b), 0, "unselected Digimon does not");
}

/// No opponent Digimon → no prompt, nothing granted.
#[test]
fn bt14_044_no_prompt_without_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("parses")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(0)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    to_p0_next_main(&mut runner);
    assert!(runner.pending_selection().is_none(), "no target → no prompt");
}

/// [All Turns]: suspending the carrier during P0's own turn makes its
/// controller (P1) lose 2 memory — i.e. the counter moves +2 toward P0.
#[test]
fn bt14_044_suspend_on_grantor_turn_opponent_loses_2() {
    let (mut runner, a, _) = setup(false);
    to_p0_next_main(&mut runner);
    let _ = runner.auto_resolve(); // single target pick
    assert_eq!(grants(&runner, a), 1);
    let before = runner.game.memory;
    runner.game.suspend(a);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.memory - before,
        2,
        "P1 (the suspended Digimon's controller) loses 2 → P0-perspective +2"
    );
}

/// The carrier attacking on its controller's turn (attack suspends it)
/// loses that player 2 memory.
#[test]
fn bt14_044_attack_suspension_on_their_turn_loses_2() {
    let (mut runner, a, _) = setup(false);
    to_p0_next_main(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(grants(&runner, a), 1);
    runner.end_turn(); // P0 → P1 (grant lasts until the end of P1's turn)
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.turn_player(), 1);
    assert_eq!(grants(&runner, a), 1, "still active during their turn");
    let before = runner.game.memory;
    let _ = runner.attack_player(a, 0, false);
    let _ = runner.auto_resolve();
    assert!(runner.game.players[1].battle_area.iter().any(|p| p.is_suspended));
    assert_eq!(
        before - runner.game.memory,
        2,
        "attacking suspends the carrier → its controller (turn player) loses 2"
    );
}

/// Expiry: after the opponent's turn ends, the granted effect is gone.
#[test]
fn bt14_044_grant_expires_at_end_of_their_turn() {
    let (mut runner, a, _) = setup(false);
    to_p0_next_main(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(grants(&runner, a), 1);
    runner.end_turn(); // → P1
    let _ = runner.auto_resolve();
    runner.end_turn(); // → P0 again (P1's turn ended)
    // The Start of Main trigger fires again and re-prompts; check OPP-A's
    // grant from the PREVIOUS activation expired before resolving it.
    assert_eq!(grants(&runner, a), 0, "expired at the end of the opponent's turn");
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
    const LV: u8 = 3;

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
