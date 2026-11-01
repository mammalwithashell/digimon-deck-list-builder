//! Matched-root, known-list mulligan diagnostics. No training or continuation scoring.
use digimon_engine::{
    action::build_action_mask, card_data::CardData, card_registry::CardRegistry,
    game::Game, inference::BatchedPolicyValueEvaluator,
    observation::{build_observation_tensor, ObservationProfileId}, rules::Rules,
    search::{decision_player, determinized_search_batched, determinized_search_batched_observed,
        DeterminizedConfig, EvalRequest, Evaluation, PolicyValueFn},
};
use rand::SeedableRng;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path, time::Instant};

#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;
#[cfg(windows)]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;
const PROFILE: ObservationProfileId = ObservationProfileId::StandardLiteV2;

struct Probe<'a> { inner: &'a mut dyn PolicyValueFn, values: Vec<f32> }
impl PolicyValueFn for Probe<'_> {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        let outputs = self.inner.evaluate_batch(requests);
        self.values.extend(outputs.iter().map(|row| row.value));
        outputs
    }
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn hand(game: &Game, pid: u8) -> Vec<String> {
    game.players[pid as usize].hand.iter().map(|c| c.card_id(&game.card_data).to_string()).collect()
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "request output");
    let out = Path::new(&args[1]);
    assert!(!out.exists(), "preserve prior output");
    let request = read(Path::new(&args[0]));
    let plan = read(Path::new(request["plan"].as_str().unwrap()));
    let corpus = read(Path::new(request["corpus"].as_str().unwrap()));
    let data = CardData::load_from_str(&std::fs::read_to_string(plan["cards_json"].as_str().unwrap()).unwrap()).unwrap();
    let decks: Vec<Vec<String>> = plan["decks"].as_array().unwrap().iter()
        .map(|d| serde_json::from_value(d["cards"].clone()).unwrap()).collect();
    admission::validate_pool(&decks, &data, Path::new(plan["cards_json"].as_str().unwrap()).parent().unwrap()).unwrap();
    let registry = CardRegistry::from_cards(&data);
    let mut evaluator = BatchedPolicyValueEvaluator::load_for_profile(Path::new(plan["model"].as_str().unwrap()), PROFILE).unwrap();
    evaluator.require_terminal_value().unwrap();
    let mut config = DeterminizedConfig::default();
    config.worlds = request["worlds"].as_u64().unwrap().try_into().unwrap();
    config.search.simulations = request["simulations"].as_u64().unwrap().try_into().unwrap();
    config.search.seed = request["seed"].as_u64().unwrap();
    config.search.dirichlet_epsilon = 0.;
    config.search.observation_profile = PROFILE;
    assert!([4,8,16].contains(&config.worlds));
    assert_eq!(config.search.simulations, 320);
    let qualify = request["qualify"].as_bool().unwrap_or(false);
    let mut rows = Vec::new();
    for node in corpus["nodes"].as_array().unwrap() {
        let chosen: Vec<_> = node["deck_indices"].as_array().unwrap().iter()
            .map(|i| decks[i.as_u64().unwrap() as usize].clone()).collect();
        let mut game = Game::new(&chosen, &data, Rules::standard(), Some(node["game_seed"].as_u64().unwrap())).unwrap();
        game.turn_count = 1;
        for action in node["actions"].as_array().unwrap() {
            let pid = decision_player(&game);
            let a = action.as_u64().unwrap() as u16;
            assert!(build_action_mask(&game, pid)[a as usize] > 0.5);
            game.decode_action(a, pid);
        }
        let pid = decision_player(&game);
        assert_eq!(Some(pid), game.mulligan_current_player());
        assert_eq!(pid as u64, node["mover"].as_u64().unwrap());
        let mask = build_action_mask(&game, pid);
        let legal: Vec<_> = mask.iter().enumerate().filter_map(|(a,p)| (*p>0.5).then_some(a)).collect();
        assert_eq!(legal, vec![0,1]);
        assert_eq!(hand(&game, pid), serde_json::from_value::<Vec<String>>(node["hand"].clone()).unwrap());
        let obs = build_observation_tensor(&game, pid, &registry, PROFILE);
        let mut hash = Sha256::new();
        for value in &obs { hash.update(value.to_le_bytes()); }
        let observation_sha256 = format!("{:x}", hash.finalize());
        if request["require_observation_hash"].as_bool().unwrap_or(false) {
            assert_eq!(Some(observation_sha256.as_str()), node["observation_sha256"].as_str(),
                "reconstructed observation differs from the frozen replay binding");
        }
        let raw = evaluator.evaluate_batch(&[EvalRequest { obs, mask }]).pop().unwrap();
        let before = Instant::now();
        let mut sampled = Vec::new();
        let mut probe = Probe { inner: &mut evaluator, values: Vec::new() };
        let result = determinized_search_batched_observed(&registry, &game, pid, &mut probe, &config, 4,
            &mut |world, seed| {
                assert_eq!(hand(world,pid), hand(&game,pid));
                let mut redraw = world.clone();
                redraw.decode_action(1, pid);
                sampled.push(json!({"seed":seed,"replacement_hand":hand(&redraw,pid)}));
            });
        let seconds = before.elapsed().as_secs_f64();
        assert_eq!(result.simulations_run, config.search.simulations);
        assert_eq!(sampled.len(), config.worlds as usize);
        let values = std::mem::take(&mut probe.values);
        drop(probe);
        assert!(!values.is_empty() && values.iter().all(|v| v.is_finite()));
        if qualify {
            let reference = determinized_search_batched(&registry, &game, pid, &mut evaluator, &config, 4);
            assert_eq!(result.root_plans, reference.root_plans, "observer changed search");
            assert_eq!(result.root_value, reference.root_value);
            let mut changed = game.clone();
            changed.rng = rand::rngs::StdRng::seed_from_u64(129873612);
            changed.players[(1-pid) as usize].deck.reverse();
            let mut resampled = Vec::new();
            let other = determinized_search_batched_observed(&registry, &changed, pid, &mut evaluator, &config, 4,
                &mut |world, seed| {
                    let mut redraw = world.clone();
                    redraw.decode_action(1, pid);
                    resampled.push(json!({"seed":seed,"replacement_hand":hand(&redraw,pid)}));
                });
            assert_eq!(sampled, resampled, "real future RNG or hidden order entered replacement sampling");
            assert_eq!(result.root_plans, other.root_plans);
            assert_eq!(result.root_value, other.root_value);
        }
        let unique_hands: BTreeSet<_> = sampled.iter().map(|s| {
            let mut cards: Vec<String> = serde_json::from_value(s["replacement_hand"].clone()).unwrap();
            cards.sort(); cards
        }).collect();
        rows.push(json!({"node":node["id"],"archetype":node["archetype"],"observation_sha256":observation_sha256,"keep_prior":raw.policy[0],
            "redraw_prior":raw.policy[1],"policy_value":raw.value,"selected_plan":result.best_plan().unwrap(),
            "root_plans":result.root_plans,"root_visits":result.root_visits,"root_q":result.root_q,
            "root_value":result.root_value,"simulations_run":result.simulations_run,"seconds":seconds,
            "sampled_worlds":sampled,"unique_replacement_multisets":unique_hands.len(),
            "evaluated_nodes_including_roots":{"count":values.len(),"mean":values.iter().map(|v|*v as f64).sum::<f64>()/values.len() as f64,
                "min":values.iter().copied().fold(f32::INFINITY,f32::min),"max":values.iter().copied().fold(f32::NEG_INFINITY,f32::max)},
            "observer_and_future_rng_invariance_verified":qualify}));
    }
    let temp = out.with_extension("tmp");
    std::fs::write(&temp, serde_json::to_vec_pretty(&json!({"rows":rows,"request":request,
        "scope":"Matched-root diagnostic, not strength or expert truth. Replacement hands are one forced-redraw clone per actual sampled world; they do not count independent MCTS traversals. Evaluation values combine root and leaf requests."})).unwrap()).unwrap();
    std::fs::rename(temp,out).unwrap();
}
