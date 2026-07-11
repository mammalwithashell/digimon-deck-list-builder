//! Determinism + robustness of the search on messy mid-game states from
//! real starter decks (tasks 2.1 / 2.4).

use digimon_engine::card_registry::CardRegistry;
use digimon_engine::game::Game;
use digimon_engine::runners::HeadlessRunner;
use digimon_engine::search::{Mcts, SearchConfig, UniformEvaluator};

use crate::support::{
    advance_random, decision_player, legal_actions, load_card_data, run_big_stack, Lcg, ST1_DECK,
    ST5_DECK,
};

fn owned(deck: &[&str]) -> Vec<String> {
    deck.iter().map(|s| s.to_string()).collect()
}

/// Build a real ST1-vs-ST5 game and advance it `decisions` random steps.
fn midgame_state(
    card_data: &std::collections::HashMap<String, digimon_engine::card_data::CardData>,
    seed: u64,
    decisions: u32,
) -> Game {
    let runner = HeadlessRunner::new(
        owned(ST1_DECK),
        owned(ST5_DECK),
        card_data,
        false,
        false,
        false,
        Some(seed),
    )
    .expect("build headless runner");
    let mut game = runner.game;
    let mut lcg = Lcg(seed ^ 0x9E3779B97F4A7C15);
    advance_random(&mut game, &mut lcg, decisions);
    game
}

/// Same seed => bit-identical SearchResult; different seeds => (almost
/// surely) a different result. Root Dirichlet noise and all tree randomness
/// flow from `StdRng::seed_from_u64(config.seed)` only.
#[test]
fn search_is_deterministic_per_seed() {
    run_big_stack(|| {
        let card_data = load_card_data();
        let registry = CardRegistry::from_cards(&card_data);
        // Advance far enough for a branchy, non-trivial root.
        let game = midgame_state(&card_data, 11, 24);
        assert!(!game.game_over, "mid-game fixture ended too early");

        let mcts = Mcts::new(&registry);
        let config = SearchConfig {
            simulations: 160,
            seed: 7,
            ..SearchConfig::default()
        };

        let mut eval_a = UniformEvaluator;
        let a = mcts.search(&game, &mut eval_a, &config);
        let mut eval_b = UniformEvaluator;
        let b = mcts.search(&game, &mut eval_b, &config);
        assert_eq!(a, b, "same seed must reproduce the exact SearchResult");

        // A different seed reshuffles the root Dirichlet noise; at least one
        // of a few nearby seeds must produce a different result (each
        // individually differs almost surely; the loop kills residual flake).
        let differs = (8u64..=10).any(|seed| {
            let mut evaluator = UniformEvaluator;
            let c = mcts.search(
                &game,
                &mut evaluator,
                &SearchConfig {
                    seed,
                    ..config.clone()
                },
            );
            c != a
        });
        assert!(
            differs,
            "different seeds should (almost surely) change the visit spread"
        );
    });
}

/// Fuzz-ish robustness: from several random mid-game states, a 200-sim
/// search must complete, spend every simulation, and only ever visit
/// root actions that were legal in the root mask (masked-only expansion).
#[test]
fn search_completes_and_respects_mask_on_midgame_states() {
    run_big_stack(|| {
        let card_data = load_card_data();
        let registry = CardRegistry::from_cards(&card_data);
        let mcts = Mcts::new(&registry);

        let mut states_searched = 0u32;
        let mut total_sims = 0u32;
        let started = std::time::Instant::now();
        for (seed, decisions) in [(21u64, 10u32), (22, 25), (23, 40)] {
            let game = midgame_state(&card_data, seed, decisions);
            if game.game_over {
                continue; // random playout ended early; not a search case
            }
            let mover = decision_player(&game);
            let root_mask = digimon_engine::build_action_mask(&game, mover);
            let legal = legal_actions(&root_mask);
            assert!(
                !legal.is_empty(),
                "non-terminal state must have legal actions"
            );

            let mut evaluator = UniformEvaluator;
            let config = SearchConfig {
                simulations: 200,
                seed,
                ..SearchConfig::default()
            };
            let result = mcts.search(&game, &mut evaluator, &config);

            assert_eq!(result.simulations_run, 200);
            assert!(!result.root_visits.is_empty());
            // Every simulation passes through exactly one root edge.
            let visit_sum: u32 = result.root_visits.iter().map(|(_, v)| *v).sum();
            assert_eq!(visit_sum, 200, "each sim must visit exactly one root edge");
            // Masked-only expansion at the root.
            for (action, visits) in &result.root_visits {
                assert!(
                    legal.contains(action),
                    "root edge {action} (visits {visits}) was not legal in the root mask"
                );
            }
            states_searched += 1;
            total_sims += result.simulations_run;
        }
        assert!(
            states_searched >= 2,
            "too few usable mid-game states ({states_searched})"
        );
        let elapsed = started.elapsed().as_secs_f64();
        eprintln!(
            "search midgame fuzz: {states_searched} states, {total_sims} sims in {elapsed:.2}s \
             ({:.0} sims/sec, debug build)",
            total_sims as f64 / elapsed
        );
    });
}
