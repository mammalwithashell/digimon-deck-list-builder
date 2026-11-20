//! EX8-057 DemiDevimon — Digimon, Purple, Lv.3, cost 3, 1000 DP. Traits: Evil,
//! NSo. Attribute: Virus.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX8-057.md)
//! [Digivolve] Purple Lv.2: Cost 0
//! [Digivolve] Lv.2 w/[NSo] trait: Cost 0
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [NSo]
//! trait and 1 card with the [Fallen Angel] trait among them to the hand.
//! Return the rest to the bottom of the deck.
//! Inherited Effect
//! [When Attacking] [Once Per Turn] <Draw 1> and trash 1 card in your hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Purple/EX8_057.cs — see the YAML header.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, GamePhase};

use super::nso_support::*;

const CARD_ID: &str = "EX8-057";

fn builder() -> DebugRunnerBuilder {
    let mut angel = plain("ANGEL", 3, 3000, CardColor::White);
    angel.traits = vec!["Fallen Angel".to_string()];
    let mut both = nso("BOTH", 3, 3000, 3, CardColor::Purple);
    both.traits = vec!["NSo".to_string(), "Fallen Angel".to_string()];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-057 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(nso("NSO-CARD", 3, 3000, 3, CardColor::Purple))
        .add_card(angel)
        .add_card(both)
        .add_card(nso("N2-BLUE", 2, 0, 0, CardColor::Blue))
        .add_card(plain("X2-PURPLE", 2, 0, CardColor::Purple))
        .add_card(plain("X2-BLUE", 2, 0, CardColor::Blue))
        .add_card(plain("HOST-L4", 4, 4000, CardColor::Purple))
        .add_card(plain("OPP-A", 4, 9000, CardColor::Blue))
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
}

fn digivolve_over(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().deck(0, &["FILL"; 6]).hand(0, &[CARD_ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

fn offered(r: &DebugRunner, slot: usize) -> bool {
    build_action_mask(&r.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.5
}

/// Walk the reveal prompts taking the first legal non-PASS action each time.
fn take_first_picks(r: &mut DebugRunner) {
    for _ in 0..6 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .expect("a legal pick");
        r.execute_action(view.selecting_player, action).unwrap();
    }
}

#[test]
fn ex8_057_digivolves_from_a_purple_or_nso_level_2_for_free() {
    for base in ["X2-PURPLE", "N2-BLUE"] {
        let (r, slot) = digivolve_over(base);
        assert!(offered(&r, slot), "{base}");
    }
    let (r, slot) = digivolve_over("X2-BLUE");
    assert!(!offered(&r, slot), "off-colour without [NSo]");
}

#[test]
fn ex8_057_on_play_adds_one_nso_and_one_fallen_angel_from_the_top_3() {
    let mut r = builder()
        .deck(0, &["FILL", "FILL", "FILL", "NSO-CARD", "ANGEL"])
        .start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    let deck_before = r.game.players[0].deck.len();
    r.fire_on_play(0, me.index as usize);
    take_first_picks(&mut r);
    let hand = hand_ids(&r, 0);
    assert!(hand.contains(&"NSO-CARD".to_string()), "{hand:?}");
    assert!(hand.contains(&"ANGEL".to_string()), "{hand:?}");
    assert_eq!(hand.len(), 2, "exactly one card per condition: {hand:?}");
    assert_eq!(r.game.players[0].deck.len(), deck_before - 2, "2 cards left the deck; the third returned to the bottom");
}

#[test]
fn ex8_057_a_card_with_both_traits_cannot_fill_both_conditions() {
    let mut r = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "BOTH"])
        .start();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.fire_on_play(0, me.index as usize);
    take_first_picks(&mut r);
    assert_eq!(hand_ids(&r, 0), vec!["BOTH".to_string()], "one card, added once");
}

#[test]
fn ex8_057_inherited_draws_one_then_trashes_one_when_attacking() {
    let mut r = builder().deck(0, &["FILL"; 6]).start();
    let attacker = r.place_stack(0, &[CARD_ID, "HOST-L4"]);
    r.place_on_field(1, "OPP-A", Some(0));
    push_hand(&mut r, 0, "NSO-CARD");
    let deck_before = r.game.players[0].deck.len();
    r.attack_player(attacker, 1, false);
    // <Draw 1> resolves, then a MANDATORY trash of 1 card.
    let view = r.pending_selection_view().expect("trash 1 card prompt");
    assert!(!view.is_optional, "the trash is mandatory (canNoSelect: false)");
    assert_eq!(r.game.players[0].deck.len(), deck_before - 1, "drew 1");
    assert_eq!(hand_ids(&r, 0).len(), 2);
    pick_hand(&mut r, 0, "NSO-CARD");
    assert_eq!(hand_ids(&r, 0).len(), 1);
    assert!(trash_ids(&r, 0).contains(&"NSO-CARD".to_string()));
}
