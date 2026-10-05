//! BT13-049 Lalamon — Digimon, Lv.3, Green, DP 1000, Cost 3. Trait: Vegetation.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT13-049.md)
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Vegetation], [Plant], or [Fairy] in one of its traits and 1 [Yoshino
//! Fujieda] among them to the hand. Place the rest at the bottom of the deck
//! in any order.
//! Inherited: [Your Turn][Once Per Turn] When this Digimon would digivolve, if
//! you have a green Tamer, reduce the digivolution cost by 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Green/BT13_049.cs
//!
//! # Patterns
//! - A1/A3 reveal-top-N, two mandatory buckets + no_duplicate_cards (BT16-029 idiom)
//! - `trait_contains` "in one of its traits" (Vegetation / Plant / Fairy)
//! - Remainder to deck bottom with player-chosen order (OrderedPermutation)
//! - Inherited [Your Turn][OPT] digivolve cost -1 gated on a green Tamer
//!   (`scope: inherited` cost_reduction, BT11-061 idiom)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT13-049";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn digimon_with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn yoshino(id: &str) -> CardData {
    let mut c = make_test_card(id, "Yoshino Fujieda");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Green];
    c
}

fn vegetation_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.traits = vec!["Vegetation".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .unwrap_or_else(|| panic!("{id} is revealed"))
}

fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let action = revealed_action_for_id(runner, id);
    let view = runner.pending_selection_view().expect("pending pick");
    assert!(
        view.valid_action_ids.contains(&action),
        "{id} must be a legal pick; valid = {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect("pick revealed card");
}

fn ids(cards: &[digimon_engine::card_source::CardSource], runner: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn on_play_runner(deck: &[&str], extra: Vec<CardData>) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-049 YAML parses");
    for c in extra {
        b = b.add_card(c);
    }
    b.deck(0, deck).hand(0, &[CARD_ID]).memory(10).start()
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt13_049_metadata_and_mandatory_on_play() {
    let runner = DebugRunner::builder().dsl_card(CARD_ID).expect("parses").build();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Lalamon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    let on_play = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::OnPlay] && t.scope == CompiledScope::FaceUp =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("face-up On Play clause");
    assert!(!on_play.optional, "reveal-and-add is mandatory");
}

// ─── [On Play] behavioral ────────────────────────────────────────────────────

/// Both buckets: a Vegetation Digimon and Yoshino are added; the filler goes
/// to the deck bottom.
#[test]
fn bt13_049_on_play_adds_vegetation_digimon_and_yoshino() {
    let mut runner = on_play_runner(
        &["FILL", "YOSHI", "VEG"],
        vec![digimon_with_traits("VEG", &["Vegetation"]), yoshino("YOSHI"), filler("FILL")],
    );
    runner.play(0, 0).expect("play Lalamon");

    assert!(!runner.pending_is_optional(), "bucket 1 is mandatory");
    assert!(!runner.pending_selection_view().unwrap().valid_action_ids.contains(&PASS));
    pick_revealed(&mut runner, "VEG");
    assert!(!runner.pending_is_optional(), "bucket 2 is mandatory");
    pick_revealed(&mut runner, "YOSHI");
    runner.auto_resolve().expect("remainder");

    let hand = ids(&runner.game.players[0].hand, &runner);
    assert!(hand.contains(&"VEG".to_string()));
    assert!(hand.contains(&"YOSHI".to_string()));
    assert_eq!(ids(&runner.game.players[0].deck, &runner), vec!["FILL".to_string()]);
}

/// "[Plant]" and "[Fairy]" *in one of its traits* (substring) both qualify.
#[test]
fn bt13_049_on_play_plant_and_fairy_substring_traits_qualify() {
    let mut runner = on_play_runner(
        &["FILL", "FAIRY", "PLANT"],
        vec![
            digimon_with_traits("PLANT", &["Plant"]),
            digimon_with_traits("FAIRY", &["Fairy Spirit"]),
            filler("FILL"),
        ],
    );
    runner.play(0, 0).expect("play Lalamon");
    let view = runner.pending_selection_view().expect("bucket 1");
    let plant = revealed_action_for_id(&runner, "PLANT");
    let fairy = revealed_action_for_id(&runner, "FAIRY");
    assert!(view.valid_action_ids.contains(&plant), "[Plant] trait qualifies");
    assert!(view.valid_action_ids.contains(&fairy), "trait containing 'Fairy' qualifies");
    let fill = revealed_action_for_id(&runner, "FILL");
    assert!(!view.valid_action_ids.contains(&fill), "plain filler does not qualify");
}

/// Bucket 1 requires a Digimon card: a [Vegetation] Tamer and a non-plant
/// Digimon are not offered (bucket finalizes empty → straight to Yoshino).
#[test]
fn bt13_049_on_play_rejects_non_digimon_and_wrong_trait() {
    let mut runner = on_play_runner(
        &["DRAGON", "VEGTAMER", "YOSHI"],
        vec![
            vegetation_tamer("VEGTAMER"),
            digimon_with_traits("DRAGON", &["Dragon"]),
            yoshino("YOSHI"),
        ],
    );
    runner.play(0, 0).expect("play Lalamon");
    // Bucket 1 has no candidate → the only pick offered is Yoshino (bucket 2).
    let view = runner.pending_selection_view().expect("bucket 2 pending");
    let yoshi = revealed_action_for_id(&runner, "YOSHI");
    assert_eq!(view.valid_action_ids, vec![yoshi], "only Yoshino is pickable");
    pick_revealed(&mut runner, "YOSHI");
    runner.auto_resolve().expect("remainder");
    let hand = ids(&runner.game.players[0].hand, &runner);
    assert!(!hand.contains(&"VEGTAMER".to_string()));
    assert!(!hand.contains(&"DRAGON".to_string()));
    assert_eq!(runner.game.players[0].deck.len(), 2, "rest bottomed");
}

/// "in any order": the 2-card remainder surfaces an OrderedPermutation and
/// the player's chosen order is honored at the deck bottom.
#[test]
fn bt13_049_on_play_remainder_order_is_player_chosen() {
    for first_pick in ["F1", "F2"] {
        let mut runner = on_play_runner(
            &["F2", "F1", "VEG"],
            vec![digimon_with_traits("VEG", &["Vegetation"]), filler("F1"), filler("F2")],
        );
        runner.play(0, 0).expect("play Lalamon");
        pick_revealed(&mut runner, "VEG");
        // No Yoshino revealed → bucket 2 finalizes empty; ordering prompt.
        let view = runner.pending_selection_view().expect("ordering prompt");
        assert!(
            matches!(view.kind, SelectionKind::OrderedPermutation { remaining: 2 }),
            "remainder ordering is a player choice; got {:?}",
            view.kind
        );
        let first = revealed_action_for_id(&runner, first_pick);
        runner.execute_action(view.selecting_player, first).expect("order 1");
        runner.auto_resolve().expect("order 2");
        let deck = ids(&runner.game.players[0].deck, &runner);
        // ordered[0] lands closest to the top among the bottomed cards.
        let other = if first_pick == "F1" { "F2" } else { "F1" };
        assert_eq!(deck, vec![other.to_string(), first_pick.to_string()]);
    }
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
