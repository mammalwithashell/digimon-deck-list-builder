//! Bounded synthetic search-call abort evidence. No model, scored game,
//! materializer admission or full executable-output proof is established.
use super::*;
use crate::{
    action::build_action_mask,
    card_registry::CardRegistry,
    debug_runner::{make_test_card, DebugRunner},
    game::Game,
};
use std::panic::{catch_unwind, AssertUnwindSafe};

struct CountingEvaluator { rows: usize }
impl PolicyValueFn for CountingEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        self.rows += requests.len();
        UniformEvaluator.evaluate_batch(requests)
    }
}
fn fixture(seat: u8) -> (Game, CardRegistry) {
    let card = make_test_card("ABORT-FIXTURE", "Abort fixture");
    let cards = [(card.card_id.clone(), card)].into_iter().collect();
    let registry = CardRegistry::from_cards(&cards);
    let mut runner = DebugRunner::builder().with_card_data(cards)
        .hand(0, &["ABORT-FIXTURE"]).hand(1, &["ABORT-FIXTURE"])
        .deck(0, &["ABORT-FIXTURE"; 8]).deck(1, &["ABORT-FIXTURE"; 8])
        .start();
    runner.game.turn_player_idx = seat as usize;
    runner.game.memory = 3;
    runner.game.enter_main_phase();
    assert!(!runner.game.game_over);
    assert_eq!(decision_player(&runner.game), seat);
    assert!(build_action_mask(&runner.game, seat).iter().filter(|&&v| v > 0.5).count() >= 2);
    (runner.game, registry)
}
fn configs() -> Vec<(DeterminizedConfig, usize)> {
    [ (DeterminizationMode::Pimc, 1), (DeterminizationMode::Pimc, 2),
      (DeterminizationMode::Pimc, 4), (DeterminizationMode::IsMcts, 1) ]
        .into_iter().map(|(mode,batch)| (DeterminizedConfig {
            mode, worlds: 4,
            search: SearchConfig { simulations: 8, seed: 918731,
                dirichlet_epsilon: 0.0, ..SearchConfig::default() }
        }, batch)).collect()
}
fn root_surface(game: &Game, seat: u8) -> (Vec<f32>, i32, usize, bool, bool) {
    (build_action_mask(game, seat), game.memory as i32, game.turn_player_idx,
     game.game_over, game.projected_world_reads_guarded())
}
#[test]
fn sampler_error_discards_earlier_world_work_without_returning_partial_result() {
    for seat in 0..2 { for (config,batch) in configs() { for fail_at in [0,1,3] {
        let (root, registry) = fixture(seat);
        let before = root_surface(&root, seat);
        let mut samples = 0;
        let mut evaluator = CountingEvaluator { rows: 0 };
        let result = determinized_search_sampled(&registry, &root, seat, &mut evaluator,
            &config, batch, &mut |_| {
                let index = samples; samples += 1;
                if index == fail_at { Err(format!("audit-rejected-world-{index}")) }
                else { Ok(root.clone()) }
            });
        assert_eq!(result.unwrap_err(), format!("audit-rejected-world-{fail_at}"));
        assert_eq!(samples, fail_at+1, "no successor world is sampled after failure");
        if fail_at >= batch { assert!(evaluator.rows > 0, "earlier search work must be real"); }
        if fail_at == 0 { assert_eq!(evaluator.rows, 0); }
        assert_eq!(root_surface(&root, seat), before);
    } } }
}
#[test]
fn world_factory_guard_panics_escape_search_without_returning_partial_result() {
    for seat in 0..2 { for (config,batch) in configs() { for fail_at in [0,1,3] {
        let (root, registry) = fixture(seat);
        let before = root_surface(&root, seat);
        let mut samples = 0;
        let mut evaluator = CountingEvaluator { rows: 0 };
        let result = catch_unwind(AssertUnwindSafe(|| determinized_search_sampled(
            &registry, &root, seat, &mut evaluator, &config, batch, &mut |_| {
                let index = samples; samples += 1;
                let mut world = root.clone();
                if index == fail_at { world.require_projected_world_read_guard_for_test(); }
                Ok(world)
            })));
        let error = result.expect_err("marked world must unwind the entire search call");
        let message = error.downcast_ref::<String>().map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied()).unwrap_or("");
        assert!(message.contains("closed-list projected world reached unproven factory"), "{message}");
        assert_eq!(samples, fail_at+1);
        if fail_at >= batch { assert!(evaluator.rows > 0, "earlier search work must be real"); }
        if fail_at == 0 { assert_eq!(evaluator.rows, 0); }
        assert_eq!(root_surface(&root, seat), before);
    } } }
}
#[test]
fn unmarked_control_returns_full_search_budget_and_legal_root_plans() {
    for seat in 0..2 { for (config,batch) in configs() {
        let (root, registry) = fixture(seat);
        let before = root_surface(&root, seat);
        let mut samples = 0;
        let mut evaluator = CountingEvaluator { rows: 0 };
        let result = determinized_search_sampled(&registry, &root, seat, &mut evaluator,
            &config, batch, &mut |_| { samples += 1; Ok(root.clone()) }).unwrap();
        assert_eq!(result.simulations_run, 8);
        assert_eq!(samples, if matches!(config.mode, DeterminizationMode::Pimc) {4} else {8});
        assert!(evaluator.rows > 0);
        assert!(!result.root_plans.is_empty());
        for (route,_,_) in result.root_plans {
            assert!(!route.is_empty());
            assert!(before.0[route[0] as usize] > 0.5);
        }
        assert_eq!(root_surface(&root, seat), before);
    } }
}
