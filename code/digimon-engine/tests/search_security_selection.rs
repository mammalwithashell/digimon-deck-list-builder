use digimon_engine::{action::build_action_mask, card_data::CardData,
    determinize::materialize, runners::HeadlessRunner, search::decision_player};

#[test]
fn schema2_game268_security_selection_has_same_root_actions_in_every_world() {
    let case: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/superpowers/research/2026-10-25-security-search-regression.json")).unwrap();
    let cards = CardData::load_from_str(&std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json")).unwrap()).unwrap();
    let deck = |key: &str| case[key].as_array().unwrap().iter()
        .map(|c| c.as_str().unwrap().to_owned()).collect();
    let mut runner = HeadlessRunner::new(deck("deck1"), deck("deck2"), &cards,
        false, false, false, Some(case["seed"].as_u64().unwrap())).unwrap();
    for row in case["prefix"].as_array().unwrap() {
        let player = row["player"].as_u64().unwrap() as u8;
        let action = row["action"].as_u64().unwrap() as u16;
        assert_eq!(decision_player(&runner.game), player);
        assert!(build_action_mask(&runner.game, player)[action as usize] > 0.5, "at {row}");
        runner.step(action);
    }
    let player = decision_player(&runner.game);
    let legal = |mask: Vec<f32>| mask.iter().enumerate().filter_map(|(i, v)| (*v > 0.5).then_some(i)).collect::<Vec<_>>();
    let expected = legal(build_action_mask(&runner.game, player));
    assert!(runner.game.pending_selection.as_ref().unwrap().card_choice.is_some());
    fn mix(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9E3779B97F4A7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^ (x >> 31)
    }
    let actual_worlds = (0..4u64).map(|i| mix(case["search_seed"].as_u64().unwrap()
        ^ mix(0xD1B54A32D192ED03u64.wrapping_add(mix(i)))));
    for seed in (0..32).chain(actual_worlds) {
        let world = materialize(&runner.game, player, seed);
        assert_eq!(legal(build_action_mask(&world, player)), expected, "world {seed}; source {:?}; sampled {:?}",
            runner.game.pending_selection, world.pending_selection);
        digimon_engine::determinize::check_world_invariants(&runner.game, player, &world).unwrap();
    }
    use digimon_engine::{card_registry::CardRegistry, search::{determinized_search_batched,
        DeterminizedConfig, SearchConfig, UniformEvaluator}};
    let config = DeterminizedConfig { worlds: 4, search: SearchConfig {
        simulations: 4, seed: case["search_seed"].as_u64().unwrap(), ..Default::default()
    }, ..Default::default() };
    let result = determinized_search_batched(&CardRegistry::from_cards(&cards),
        &runner.game, player, &mut UniformEvaluator, &config, 4);
    assert_eq!(result.root_visits.iter().map(|(a, _)| *a as usize).collect::<Vec<_>>(), expected);

    // Private inspection must survive canonicalization as well as sampling.
    // Check actual root evaluator inputs, not just the materializer in isolation.
    runner.game.pending_selection.as_mut().unwrap().card_choice.as_mut().unwrap()
        .grant_security_inspection(player, &[0]);
    use digimon_engine::search::{PolicyValueFn, EvalRequest, Evaluation};
    struct VerifyRoots { expected: Vec<f32>, seen: usize }
    impl PolicyValueFn for VerifyRoots {
        fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
            for request in requests {
                if self.seen < 4 { assert_eq!(request.obs, self.expected, "known inspection changed in a sampled root"); }
                self.seen += 1;
            }
            UniformEvaluator.evaluate_batch(requests)
        }
    }
    let registry = CardRegistry::from_cards(&cards);
    let mut evaluator = VerifyRoots { expected: digimon_engine::observation::build_observation_tensor(
        &runner.game, player, &registry, config.search.observation_profile), seen: 0 };
    determinized_search_batched(&registry, &runner.game, player, &mut evaluator, &config, 4);
    assert!(evaluator.seen >= 4);
}

#[test]
fn blind_security_pages_keep_aliases_but_do_not_freeze_hidden_identities() {
    use digimon_engine::{debug_runner::{DebugRunner, make_test_card}, effect_context::EffectContext};
    let mut r = DebugRunner::builder().add_card(make_test_card("PIN-A", "A"))
        .add_card(make_test_card("PIN-B", "B")).security(0, &["PIN-A"; 40])
        .deck(0, &["PIN-B"; 40]).deck(1, &["PIN-A"; 10]).start();
    r.skip_mulligan();
    let host = r.place_stack(0, &["PIN-A"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_security(0,
        "Blind security", false, |_, _| true, |_, _| {});
    r.execute_action(0, 59).unwrap();
    let source_view = r.game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&r.game, 0);
    let expected = build_action_mask(&r.game, 0);
    let mut changed = false;
    for seed in 0..8 {
        let world = materialize(&r.game, 0, seed);
        assert_eq!(build_action_mask(&world, 0), expected);
        let view = world.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().view(&world, 0);
        assert_eq!(view.page_index, "1");
        assert_eq!(view.page_generation, source_view.page_generation);
        assert!(view.rows.iter().all(|row| row.card_id.is_none()));
        changed |= world.players[0].security.iter().any(|c| c.card_id(&world.card_data) == "PIN-B");
    }
    assert!(changed, "blind security must still be resampled");
}

#[test]
fn inspected_security_is_private_known_information_and_stays_pinned_for_inspector() {
    use digimon_engine::{debug_runner::{DebugRunner, make_test_card}, effect_context::EffectContext};
    let mut r = DebugRunner::builder().add_card(make_test_card("PIN-A", "A"))
        .add_card(make_test_card("PIN-B", "B")).security(0, &["PIN-A"; 4])
        .deck(0, &["PIN-B"; 40]).deck(1, &["PIN-A"; 10]).start();
    r.skip_mulligan();
    let host = r.place_stack(0, &["PIN-A"]);
    let top = r.game.players[0].battle_area[0].top_card().handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_security(0,
        "Inspected security", false, |_, _| true, |_, _| {});
    r.game.pending_selection.as_mut().unwrap().card_choice.as_mut().unwrap().grant_security_inspection(0, &[0,1,2,3]);
    for seed in 0..8 {
        let world = materialize(&r.game, 0, seed);
        assert_eq!(world.players[0].security.iter().map(|c| c.handle()).collect::<Vec<_>>(),
            r.game.players[0].security.iter().map(|c| c.handle()).collect::<Vec<_>>());
        assert!(world.players[0].face_up_security.is_empty(), "private inspection must not become public");
    }
}
