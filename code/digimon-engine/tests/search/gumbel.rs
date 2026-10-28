//! Gumbel root (`RootPolicy::Gumbel`) on the perfect-information forced-win
//! fixture from `perfect_information.rs`, plus the PIMC aggregation path.
//!
//! With a uniform prior and a tiny budget the sequential-halving root must
//! still pick the immediate winning attack, report an improved policy that
//! sums to one and concentrates on the win, spend exactly the budget, and
//! produce bit-identical results serially and through `search_batch`.

use std::collections::HashMap;

use digimon_engine::action::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardColor;
use digimon_engine::player::OriginalDeckCardCount;
use digimon_engine::search::{
    determinized_search_batched, DeterminizedConfig, Mcts, RootPolicy, SearchConfig,
    SearchResult, UniformEvaluator,
};

use crate::support::{make_digimon, run_big_stack};

fn policy_of(result: &SearchResult, action: u16) -> f32 {
    result
        .root_policy
        .as_ref()
        .expect("Gumbel root reports an improved policy")
        .iter()
        .find(|(a, _)| *a == action)
        .map(|(_, p)| *p)
        .unwrap_or_else(|| panic!("action {action} missing from root policy"))
}

fn forced_win_position() -> (DebugRunner, HashMap<String, digimon_engine::card_data::CardData>, u16) {
    let mut cards = HashMap::new();
    cards.insert("ATK".to_string(), make_digimon("ATK", CardColor::Red, 5000));
    cards.insert(
        "DEF".to_string(),
        make_digimon("DEF", CardColor::Blue, 3000),
    );
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
    // DebugRunner boards are staged outside the normal deal, so `original_deck`
    // starts empty; the debug-build world-invariant checker in the determinized
    // path audits every sampled card against it — declare the decklist (as the
    // PUCT fixture in `determinized.rs` does).
    for p in r.game.players.iter_mut() {
        p.original_deck = vec![
            OriginalDeckCardCount { card_id: "ATK".to_string(), count: 4, is_digitama: false },
            OriginalDeckCardCount { card_id: "DEF".to_string(), count: 8, is_digitama: false },
        ];
    }
    let win_action = encode_attack(0, SECURITY_TARGET);
    let mask = digimon_engine::build_action_mask(&r.game, tp);
    assert_eq!(mask[win_action as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);
    (r, cards, win_action)
}

#[test]
fn gumbel_root_finds_the_forced_win_with_a_small_budget() {
    run_big_stack(|| {
        let (r, cards, win_action) = forced_win_position();
        let registry = CardRegistry::from_cards(&cards);
        let mcts = Mcts::new(&registry);
        let config = SearchConfig {
            simulations: 24,
            seed: 42,
            root: RootPolicy::gumbel(16),
            ..SearchConfig::default()
        };
        let result = mcts.search(&r.game, &mut UniformEvaluator, &config);
        assert_eq!(
            mcts.search_batch(
                &[r.game.clone(), r.game.clone()],
                &mut UniformEvaluator,
                &[config.clone(), config.clone()]
            ),
            vec![result.clone(), result.clone()],
            "batched Gumbel search must reproduce the serial result"
        );

        assert_eq!(result.simulations_run, 24, "sequential halving spends the whole budget");
        assert_eq!(
            result.chosen_action,
            Some(win_action),
            "the survivor must be the immediate win; visits {:?} q {:?}",
            result.root_visits,
            result.root_q
        );
        let policy = result.root_policy.as_ref().expect("improved policy");
        assert_eq!(policy.len(), result.root_visits.len(), "policy parallels root edges");
        assert!(policy
            .iter()
            .zip(&result.root_visits)
            .all(|((a, _), (b, _))| a == b));
        let total: f32 = policy.iter().map(|(_, p)| p).sum();
        assert!((total - 1.0).abs() < 1e-4, "improved policy sums to one: {total}");
        assert!(
            policy_of(&result, win_action) > 0.8,
            "completed-Q policy must concentrate on the win, got {:?}",
            policy
        );
        assert!(policy_of(&result, PASS) < 0.2);

        // Determinism per seed; a different Gumbel sample is a different run
        // (same winner, but the exact visit split may differ).
        assert_eq!(mcts.search(&r.game, &mut UniformEvaluator, &config), result);

        // PUCT in the same budget still reports no Gumbel outputs.
        let puct = mcts.search(
            &r.game,
            &mut UniformEvaluator,
            &SearchConfig {
                root: RootPolicy::Puct,
                ..config.clone()
            },
        );
        assert!(puct.root_policy.is_none() && puct.chosen_action.is_none());
    });
}

#[test]
fn pimc_aggregates_gumbel_worlds_into_one_policy_and_action() {
    run_big_stack(|| {
        let (r, cards, win_action) = forced_win_position();
        let registry = CardRegistry::from_cards(&cards);
        let tp = r.game.turn_player();
        let config = DeterminizedConfig {
            worlds: 2,
            search: SearchConfig {
                simulations: 32,
                seed: 9,
                root: RootPolicy::gumbel(16),
                ..SearchConfig::default()
            },
            ..DeterminizedConfig::default()
        };
        let result =
            determinized_search_batched(&registry, &r.game, tp, &mut UniformEvaluator, &config, 2);
        assert_eq!(result.simulations_run, 32);
        assert_eq!(result.chosen_action, Some(win_action));
        let policy = result.root_policy.as_ref().expect("aggregated improved policy");
        let total: f32 = policy.iter().map(|(_, p)| p).sum();
        assert!((total - 1.0).abs() < 1e-4, "world-averaged policy sums to one: {total}");
        assert!(policy_of(&result, win_action) > 0.8);
        // Serial world evaluation (batch one) must agree exactly.
        assert_eq!(
            determinized_search_batched(&registry, &r.game, tp, &mut UniformEvaluator, &config, 1),
            result
        );
    });
}
