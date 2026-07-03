//! Task 2.4 — perfect-information validation of the search core.
//!
//! Positions here are fully known (no hidden-information sampling); the
//! search must find the known optimal line with only the stub
//! `UniformEvaluator` guiding it (spec: "Search finds the optimal line on
//! perfect-information games").

use std::collections::HashMap;

use digimon_engine::action::{encode_attack, HATCH, PASS, SECURITY_TARGET};
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::debug_runner::{make_test_egg, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};
use digimon_engine::search::{Mcts, SearchConfig, SearchResult, UniformEvaluator};

use crate::support::{make_digimon, run_big_stack};

fn q_of(result: &SearchResult, action: u16) -> f32 {
    result
        .root_q
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, q)| *q)
        .unwrap_or_else(|| panic!("action {action} not among root edges: {:?}", result.root_q))
}

fn visits_of(result: &SearchResult, action: u16) -> u32 {
    result
        .root_visits
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, v)| *v)
        .unwrap_or_else(|| {
            panic!(
                "action {action} not among root edges: {:?}",
                result.root_visits
            )
        })
}

fn most_visited(result: &SearchResult) -> u16 {
    result
        .root_visits
        .iter()
        .max_by_key(|(_, v)| *v)
        .expect("non-empty root visits")
        .0
}

/// THE headline test — a forced win with a plausible losing alternative.
///
/// Board (perfect information, both securities empty):
///   - turn player: one unsuspended attacker (slot 0), empty hand.
///   - opponent:   one unsuspended attacker of their own + a small deck.
///
/// Legal root actions: attack the opposing player (their security is 0 —
/// immediate win) or PASS (the opponent untaps into a symmetric position and
/// wins by attacking OUR empty security). The opposing Digimon is
/// unsuspended, so it is not a legal attack target — the root branching is
/// exactly {win-now, lose-later}. With enough simulations the most-visited
/// root action must be the attack, its Q near +1, and PASS's Q negative.
#[test]
fn search_finds_forced_win_over_losing_pass() {
    run_big_stack(|| {
        let mut cards = HashMap::new();
        cards.insert("ATK".to_string(), make_digimon("ATK", CardColor::Red, 5000));
        cards.insert("DEF".to_string(), make_digimon("DEF", CardColor::Blue, 3000));

        let mut r = DebugRunner::builder()
            .with_card_data(cards.clone())
            .deck(0, &["DEF", "DEF", "DEF", "DEF"])
            .deck(1, &["DEF", "DEF", "DEF", "DEF"])
            .start();
        let tp = r.game.turn_player();
        let opp = 1 - tp;
        r.place_on_field(tp, "ATK", Some(0));
        r.place_on_field(opp, "DEF", Some(0));
        r.game.enter_main_phase();

        // Sanity on the constructed position: both securities empty, the
        // winning attack and the losing PASS are both masked-legal.
        assert_eq!(r.security_count(tp), 0);
        assert_eq!(r.security_count(opp), 0);
        let mask = digimon_engine::build_action_mask(&r.game, tp);
        let win_action = encode_attack(0, SECURITY_TARGET);
        assert_eq!(mask[win_action as usize], 1.0, "attack-player must be legal");
        assert_eq!(mask[PASS as usize], 1.0, "PASS must be legal");

        let registry = CardRegistry::from_cards(&cards);
        let mcts = Mcts::new(&registry);
        let mut evaluator = UniformEvaluator;
        let config = SearchConfig {
            simulations: 200,
            seed: 42,
            ..SearchConfig::default()
        };
        let result = mcts.search(&r.game, &mut evaluator, &config);

        assert_eq!(result.simulations_run, 200);
        assert_eq!(
            most_visited(&result),
            win_action,
            "search must prefer the immediate winning attack; got {:?}",
            result.root_visits
        );
        assert!(
            q_of(&result, win_action) > 0.8,
            "winning attack Q should be near +1, got {}",
            q_of(&result, win_action)
        );
        assert!(
            q_of(&result, PASS) < 0.0,
            "PASS leads to a forced loss, Q should be negative, got {}",
            q_of(&result, PASS)
        );
        assert!(
            result.root_value > 0.5,
            "root value should be strongly positive, got {}",
            result.root_value
        );
    });
}

/// Sign-handling across CONSECUTIVE decisions by the SAME player.
///
/// Root is the turn player's Breeding-phase decision (HATCH vs PASS — two
/// legal actions, so it is not auto-advanced). BOTH lead to the same
/// player's Main-phase decision, where attacking the opponent's empty
/// security wins immediately. The opponent has no Digimon, no digitama, and
/// no hand — they can never win; every line from the root is winning for
/// the root mover (or neutral until the depth cap).
///
/// Backup must key value sign to PLAYER IDENTITY, not tree depth: a buggy
/// per-level negation would report the root value near -1 (crediting the
/// child level's win to "the opponent"). Correct handling reports both root
/// edges' Q strongly positive.
#[test]
fn q_does_not_flip_across_consecutive_same_player_decisions() {
    run_big_stack(|| {
        let mut cards = HashMap::new();
        cards.insert("ATK".to_string(), make_digimon("ATK", CardColor::Red, 5000));
        cards.insert("DEF".to_string(), make_digimon("DEF", CardColor::Blue, 3000));
        cards.insert("EGG".to_string(), make_test_egg("EGG", "Test Egg"));

        let mut r = DebugRunner::builder()
            .with_card_data(cards.clone())
            .deck(0, &["DEF", "DEF", "DEF", "DEF", "DEF", "DEF"])
            .deck(1, &["DEF", "DEF", "DEF", "DEF", "DEF", "DEF"])
            .digitama(0, &["EGG"])
            .digitama(1, &["EGG"])
            .start();
        let tp = r.game.turn_player();
        r.place_on_field(tp, "ATK", Some(0));
        r.set_phase(GamePhase::Breeding);

        // Root: Breeding decision with exactly two legal actions for `tp`.
        let mask = digimon_engine::build_action_mask(&r.game, tp);
        assert_eq!(mask[HATCH as usize], 1.0, "HATCH must be legal at root");
        assert_eq!(mask[PASS as usize], 1.0, "PASS must be legal at root");

        let registry = CardRegistry::from_cards(&cards);
        let mcts = Mcts::new(&registry);
        let mut evaluator = UniformEvaluator;
        let config = SearchConfig {
            simulations: 200,
            seed: 7,
            ..SearchConfig::default()
        };
        let result = mcts.search(&r.game, &mut evaluator, &config);

        assert_eq!(result.simulations_run, 200);
        // Both breeding actions funnel into the SAME player's winning Main
        // phase — a depth-flipping backup would drive these negative.
        assert!(
            q_of(&result, HATCH) > 0.5,
            "HATCH leads to the same player's win; Q must stay positive \
             across the same-player transition, got {}",
            q_of(&result, HATCH)
        );
        assert!(
            q_of(&result, PASS) > 0.5,
            "breeding PASS leads to the same player's win; Q must stay \
             positive across the same-player transition, got {}",
            q_of(&result, PASS)
        );
        assert!(
            result.root_value > 0.5,
            "root value must be strongly positive (win is 1-2 same-player \
             plies away), got {}",
            result.root_value
        );
        // Both edges are winning, so both must have been visited.
        assert!(visits_of(&result, HATCH) > 0);
        assert!(visits_of(&result, PASS) > 0);
    });
}
