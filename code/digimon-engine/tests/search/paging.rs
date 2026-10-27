use super::support::run_big_stack;
use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledPlayerRef, CompiledPredicate, CompiledStep,
    CompiledUnionZoneFilters, CompiledZone,
};
use digimon_engine::{
    card_registry::CardRegistry,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, step::run_steps},
    effect_context::EffectContext,
    player::OriginalDeckCardCount,
    search::{EvalRequest, Evaluation, Mcts, PolicyValueFn, SearchConfig},
};

struct NavigationOnly(f32);
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
                // Score the selected card only after the resumable effect
                // places it in trash. Merely visiting its page earns nothing.
                use digimon_engine::tensor_profiles::standard::v2_lite as layout;
                let selected =
                    r.obs[layout::OFF_KNOWN_ZONE_CARDS + layout::KNOWN_ZONE_CARD_ID_OFFSET];
                Evaluation {
                    policy,
                    value: if selected == self.0 { 1. } else { 0. },
                }
            })
            .collect()
    }
}

fn fixture(count: usize, winner: usize, optional: bool) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("PAGE-SOURCE", "Source"))
        .add_card(make_test_card("PAGE-WINNER", "Winner"))
        .deck(0, &["PAGE-SOURCE"; 100])
        .deck(1, &["PAGE-SOURCE"; 100])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    let mut stack = vec!["PAGE-SOURCE"; count + 1];
    stack[winner] = "PAGE-WINNER";
    let host = r.place_stack(0, &stack);
    for player in &mut r.game.players {
        player.original_deck = vec![
            OriginalDeckCardCount {
                card_id: "PAGE-SOURCE".into(),
                count: 200,
                is_digitama: false,
            },
            OriginalDeckCardCount {
                card_id: "PAGE-WINNER".into(),
                count: 1,
                is_digitama: false,
            },
        ];
    }
    let source = r.game.player(0).battle_area[host.index as usize]
        .top_card()
        .handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("carrier", host);
    let steps = [CompiledStep::SelectUnionZone {
        of: CompiledPlayerRef::You,
        zones: vec![CompiledZone::Material],
        material_of: Some(CompiledBindingRef::Named("carrier".into())),
        filter: CompiledPredicate::default(),
        zone_filters: CompiledUnionZoneFilters::default(),
        material_carrier_filter: None,
        bind_as: Some("chosen".into()),
        prompt: "Choose from the complete pool".into(),
        prompt_key: None,
        optional,
        cost: false,
        then: vec![CompiledStep::TrashUnionBound {
            binding: "chosen".into(),
        }],
    }];
    run_steps(
        &steps,
        &mut EffectContext::new(&mut r.game, source, Some(host), 0),
        &mut bindings,
    );
    assert!(r.game.pending_selection_resume.is_some());
    r
}

fn registry() -> CardRegistry {
    CardRegistry::from_cards(&std::collections::HashMap::from([
        (
            "PAGE-SOURCE".into(),
            make_test_card("PAGE-SOURCE", "Source"),
        ),
        (
            "PAGE-WINNER".into(),
            make_test_card("PAGE-WINNER", "Winner"),
        ),
    ]))
}

fn evaluator(registry: &CardRegistry) -> NavigationOnly {
    NavigationOnly(registry.get_index("PAGE-WINNER") as f32)
}

#[test]
fn paging_cannot_absorb_search_and_hide_the_winning_card() {
    run_big_stack(|| {
        let r = fixture(32, 31, false);
        let registry = registry();
        let result = Mcts::new(&registry).search(
            &r.game,
            &mut evaluator(&registry),
            &SearchConfig {
                simulations: 640,
                max_decisions: 1,
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
        let registry = registry();
        let config = SearchConfig {
            simulations: 640,
            max_decisions: 1,
            dirichlet_epsilon: 0.,
            ..Default::default()
        };
        let mcts = Mcts::new(&registry);
        let result = mcts.search(&r.game, &mut evaluator(&registry), &config);
        assert_eq!(
            result,
            mcts.search_batch(&[r.game.clone()], &mut evaluator(&registry), &[config])[0]
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
            assert!(copy.pending_selection.is_none());
            assert_eq!(copy.player(0).trash.len(), 1);
            selected.insert(copy.player(0).trash[0].card_index);
        }
        assert_eq!(selected.len(), 65);
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
        let registry = registry();
        for mode in [DeterminizationMode::Pimc, DeterminizationMode::IsMcts] {
            let config = DeterminizedConfig {
                worlds: 4,
                mode,
                search: SearchConfig {
                    simulations: 640,
                    max_decisions: 1,
                    dirichlet_epsilon: 0.,
                    ..Default::default()
                },
            };
            let result = determinized_search_batched(
                &registry,
                &r.game,
                0,
                &mut evaluator(&registry),
                &config,
                4,
            );
            let serial = determinized_search_batched(
                &registry,
                &r.game,
                0,
                &mut evaluator(&registry),
                &config,
                1,
            );
            assert_eq!(result, serial);
            assert_eq!(result.root_plans.len(), 32);
            assert_eq!(result.simulations_run, 640);
            let mut copy = r.game.clone();
            for &a in result.best_plan().unwrap() {
                copy.decode_action(a, 0);
            }
            assert_eq!(
                copy.player(0).trash[0].card_id(&copy.card_data),
                "PAGE-WINNER"
            );
        }
    });
}

#[test]
fn zero_budget_preserves_every_card_and_one_optional_decline() {
    run_big_stack(|| {
        let mut r = fixture(65, 63, true);
        r.game.decode_action(59, 0);
        let registry = registry();
        let result = Mcts::new(&registry).search(
            &r.game,
            &mut evaluator(&registry),
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
