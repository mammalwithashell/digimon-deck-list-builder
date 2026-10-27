use super::support::run_big_stack;
use digimon_engine::{
    card_registry::CardRegistry,
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
    search::{EvalRequest, Evaluation, Mcts, PolicyValueFn, SearchConfig},
    selection::{UnionZoneOrigin, UnionZoneSet},
};

struct NavigationOnly;
impl PolicyValueFn for NavigationOnly {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        requests
            .iter()
            .map(|r| {
                let mut policy = vec![0.; r.mask.len()];
                for a in [58, 59] {
                    if r.mask[a] > 0.5 {
                        policy[a] = 1.;
                    }
                }
                Evaluation { policy, value: 0. }
            })
            .collect()
    }
}

fn fixture(count: usize, winner: usize, optional: bool) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("PAGE-SOURCE", "Source"))
        .deck(0, &["PAGE-SOURCE"; 100])
        .deck(1, &["PAGE-SOURCE"; 100])
        .memory(3)
        .start();
    r.skip_mulligan();
    let host = r.place_stack(0, &vec!["PAGE-SOURCE"; count + 1]);
    let source = r.game.player(0).battle_area[host.index as usize]
        .top_card()
        .handle();
    EffectContext::new(&mut r.game, source, Some(host), 0).select_union_zone(
        0,
        UnionZoneSet::MATERIAL,
        Some(host),
        None,
        "Choose from the complete pool",
        optional,
        |_, _, _| true,
        move |ctx, _, origin| {
            let UnionZoneOrigin::Material { source_index, .. } = origin else {
                panic!()
            };
            ctx.game.memory = source_index as i16;
            ctx.game
                .declare_winner(if source_index == winner { 0 } else { 1 });
        },
    );
    r
}

#[test]
fn paging_cannot_absorb_search_and_hide_the_winning_card() {
    run_big_stack(|| {
        let r = fixture(32, 31, false);
        let registry = CardRegistry::from_cards(&std::collections::HashMap::from([(
            "PAGE-SOURCE".to_string(),
            make_test_card("PAGE-SOURCE", "Source"),
        )]));
        let result = Mcts::new(&registry).search(
            &r.game,
            &mut NavigationOnly,
            &SearchConfig {
                simulations: 640,
                dirichlet_epsilon: 0.,
                ..Default::default()
            },
        );
        assert!(
            result.root_value > 0.5,
            "search stayed in presentation pages: {result:?}"
        );
    });
}

#[test]
fn all_pages_remain_reachable_from_a_middle_page_and_batching_is_exact() {
    run_big_stack(|| {
        use digimon_engine::action::build_action_mask;
        let mut r = fixture(65, 63, false);
        r.game.decode_action(59, 0);
        let registry = CardRegistry::from_cards(&std::collections::HashMap::from([(
            "PAGE-SOURCE".to_string(),
            make_test_card("PAGE-SOURCE", "Source"),
        )]));
        let config = SearchConfig {
            simulations: 640,
            dirichlet_epsilon: 0.,
            ..Default::default()
        };
        let mcts = Mcts::new(&registry);
        let result = mcts.search(&r.game, &mut NavigationOnly, &config);
        assert_eq!(
            result,
            mcts.search_batch(&[r.game.clone()], &mut NavigationOnly, &[config])[0]
        );
        assert_eq!(result.root_plans.len(), 65);
        assert_eq!(result.simulations_run, 640);
        assert_eq!(result.root_visits.iter().map(|(_, v)| v).sum::<u32>(), 640);
        let mut selected = std::collections::BTreeSet::new();
        for (route, _, _) in &result.root_plans {
            assert!(
                !(route.contains(&58) && route.contains(&59)),
                "route must not cycle"
            );
            let mut copy = r.game.clone();
            for &action in route {
                assert!(build_action_mask(&copy, 0)[action as usize] > 0.5);
                copy.decode_action(action, 0);
            }
            assert!(copy.game_over);
            selected.insert(copy.memory);
        }
        assert_eq!(selected, (0..65).collect());
        assert!(result.best_plan().unwrap().starts_with(&[59]));
        assert_eq!(result.visits_after(&[59]).len(), 9);
        let mut next = r.game.clone();
        next.decode_action(59, 0);
        let mask = build_action_mask(&next, 0);
        let targets = result.training_visits(&[59], &mask);
        assert!(
            targets.contains(&(58, 0)),
            "previous page remains legal with zero target mass"
        );
        assert_eq!(targets.len(), mask.iter().filter(|&&m| m > 0.5).count());
        assert_eq!(r.game.memory, 3, "search mutated the original state");
    });
}

#[test]
fn determinized_modes_choose_a_complete_card_and_preserve_the_budget() {
    run_big_stack(|| {
        use digimon_engine::search::{
            determinized_search_batched, DeterminizationMode, DeterminizedConfig,
        };
        let r = fixture(32, 31, false);
        let registry = CardRegistry::from_cards(&std::collections::HashMap::from([(
            "PAGE-SOURCE".to_string(),
            make_test_card("PAGE-SOURCE", "Source"),
        )]));
        for mode in [DeterminizationMode::Pimc, DeterminizationMode::IsMcts] {
            let config = DeterminizedConfig {
                worlds: 4,
                mode,
                search: SearchConfig {
                    simulations: 640,
                    dirichlet_epsilon: 0.,
                    ..Default::default()
                },
            };
            let result =
                determinized_search_batched(&registry, &r.game, 0, &mut NavigationOnly, &config, 4);
            let serial =
                determinized_search_batched(&registry, &r.game, 0, &mut NavigationOnly, &config, 1);
            assert_eq!(result, serial);
            assert_eq!(result.root_plans.len(), 32);
            assert_eq!(result.simulations_run, 640);
            let mut copy = r.game.clone();
            for &a in result.best_plan().unwrap() {
                copy.decode_action(a, 0);
            }
            assert_eq!(copy.winner, Some(0));
        }
    });
}

#[test]
fn zero_budget_preserves_every_card_and_one_optional_decline() {
    run_big_stack(|| {
        let mut r = fixture(65, 63, true);
        r.game.decode_action(59, 0);
        let registry = CardRegistry::from_cards(&std::collections::HashMap::from([(
            "PAGE-SOURCE".to_string(),
            make_test_card("PAGE-SOURCE", "Source"),
        )]));
        let result = Mcts::new(&registry).search(
            &r.game,
            &mut NavigationOnly,
            &SearchConfig {
                simulations: 0,
                dirichlet_epsilon: 0.,
                ..Default::default()
            },
        );
        assert_eq!(result.root_plans.len(), 66);
        assert!(result.root_plans.iter().all(|(_, visits, _)| *visits == 0));
        assert_eq!(
            result
                .root_plans
                .iter()
                .filter(|(p, _, _)| p.last() == Some(&62))
                .count(),
            1
        );
        assert!(result.root_plans.iter().any(|(p, _, _)| p == &[62]));
        assert!(result
            .root_plans
            .iter()
            .all(|(p, _, _)| !matches!(p.last(), Some(58 | 59))));
    });
}
