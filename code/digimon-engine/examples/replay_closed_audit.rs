//! Offline, explicitly authorized diagnostic replay; never a scored result.
//! Guarded search uses UniformEvaluator by default. --model-search-probe loads
//! the exact frozen root actor model for the recorded decision only. No retry
//! controller or writes to the original campaign.
use digimon_engine::{
    build_action_mask,
    card_data::CardData,
    determinize::{
        beliefs::{PriorSnapshot, WeightedDeckPrior},
        closed::materialize_closed,
    },
    game::Game,
    rules::Rules,
    search::decision_player,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[path = "search_quality/frozen_modes.rs"] mod frozen_modes;

#[derive(Default)]
struct TracedProbe { calls: usize, rows: usize, model: Option<digimon_engine::inference::BatchedPolicyValueEvaluator> }
impl digimon_engine::search::PolicyValueFn for TracedProbe {
    fn evaluate_batch(&mut self, requests: &[digimon_engine::search::EvalRequest]) -> Vec<digimon_engine::search::Evaluation> {
        self.calls += 1; self.rows += requests.len();
        if let Some(model) = self.model.as_mut() {
            digimon_engine::search::PolicyValueFn::evaluate_batch(model, requests)
        } else {
            digimon_engine::search::PolicyValueFn::evaluate_batch(&mut digimon_engine::search::UniformEvaluator, requests)
        }
    }
}

fn search_probe(game: &Game, registry: &digimon_engine::card_registry::CardRegistry,
    prior: &WeightedDeckPrior, seed: u64,
    model: Option<digimon_engine::inference::BatchedPolicyValueEvaluator>) -> Value {
    let mut config = digimon_engine::search::DeterminizedConfig::default();
    config.worlds = 4; config.search.simulations = 320; config.search.dirichlet_epsilon = 0.;
    config.search.observation_profile = digimon_engine::observation::ObservationProfileId::StandardLiteV2;
    config.search.seed = seed;
    let learned_model = model.is_some();
    let mut evaluator = TracedProbe { model, ..Default::default() };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        frozen_modes::search(registry, game, &mut evaluator, &config,
            frozen_modes::InformationRegime::ClosedList, Some(prior))
    }));
    let outcome = match result {
        Ok(Ok((result, diagnostics))) => {
            let plan = result.best_plan().expect("complete search must return a plan").to_vec();
            let mut check = game.clone(); let viewer = decision_player(game);
            for &action in &plan {
                assert_eq!(decision_player(&check), viewer);
                assert!(build_action_mask(&check, viewer)[action as usize] > 0.5);
                check.decode_action(action, viewer); check.drain_events();
            }
            json!({"completed":true,"simulations":result.simulations_run,"selected_plan":plan,
                "root_plans":result.root_plans,"root_visits":result.root_visits,"diagnostics":diagnostics,
                "selected_plan_legally_replayed":true})
        }
        Ok(Err(error)) => json!({"completed":false,"error":error,"kind":"returned_error"}),
        Err(error) => {
            let message = error.downcast_ref::<String>().cloned().or_else(|| error.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or_else(||"non-string panic".into());
            json!({"completed":false,"error":message,"kind":"panic","partial_search_result_returned":false})
        }
    };
    json!({"outcome":outcome,"uniform_evaluator_calls":(!learned_model).then_some(evaluator.calls),
        "uniform_evaluator_rows":(!learned_model).then_some(evaluator.rows),
        "evaluator_calls":evaluator.calls,"evaluator_rows":evaluator.rows,
        "scored":false,"learned_model":learned_model,"requested_simulations":320,"requested_worlds":4,"batch":4,"seed":seed})
}

fn read(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn hash(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
fn inspect(game: &Game, viewer: u8) -> Value {
    json!({"phase":format!("{:?}",game.current_phase), "actor":decision_player(game),
        "memory":game.memory,"turn":game.turn_count,
        "knowledge":game.belief_knowledge(viewer).unwrap(),
        "queue":format!("{:?}",game.effect_queue),
        "resume":format!("{:?}",game.pending_selection_resume),
        "context":format!("{:?}",game.current_trigger_context)})
}

// Diagnostic only: enumerate callback surfaces AFTER allocation from a prior.
// Removing the queue below changes the diagnostic root; this is never an
// admission result, a qualified world, or a substitute scored continuation.
fn hypothesis_readers(game: &Game, viewer: u8, prior: &WeightedDeckPrior) -> Value {
    let mut probe = game.clone();
    let removed = probe.effect_queue.len();
    probe.effect_queue.clear();
    let sampled = match materialize_closed(&probe, viewer,
        game.belief_knowledge(viewer).unwrap(), prior, 918731) {
        Ok(world) => world,
        Err(error) => return json!({"diagnostic_only":true,"accepted_for_execution":false,
            "removed_queue_entries":removed,"sample_error":error}),
    };
    let mut rows = Vec::new();
    for p in &sampled.players {
        let mut cards = Vec::new();
        for (zone, values) in [("hand", &p.hand), ("deck", &p.deck),
            ("security", &p.security), ("trash", &p.trash), ("eggs", &p.digitama_deck)] {
            for c in values { cards.push((zone, c)); }
        }
        for perm in p.battle_area.iter().chain(&p.breeding_area) {
            for c in perm.card_sources.iter().chain(&perm.linked_cards) { cards.push(("field", c)); }
        }
        for (zone, c) in cards {
            let id = c.card_id(&sampled.card_data);
            let effects = sampled.effects_for_card(id, c.handle()).unwrap_or_default();
            let effects: Vec<_> = effects.iter().enumerate().map(|(slot,e)| json!({
                "slot":slot,"timing":format!("{:?}",e.timing),"name":e.name,
                "inherited":e.inherited,"declarative":e.declarative,
                "condition":e.condition.is_some(),"trigger_qualifier":e.trigger_qualifier.is_some(),
                "process":e.process.is_some(),"cost_amount":e.cost_reduction_fn.is_some(),
                "cost_payment":e.pay_cost_fn.is_some(),"activation_cost":e.activation_cost_fn.is_some(),
                "dynamic_dp":e.dp_modifier_fn.is_some(),"dynamic_security_attack":e.security_attack_fn.is_some(),
                "replacement_condition":e.replacement_condition.is_some(),
                "replacement_process":e.replacement_process.is_some(),
                "alt_path_registration":e.alt_path_registration.is_some(),
                "existing_direct_clause_certificate":e.ignores_event_card_identity,
            })).collect();
            rows.push(json!({"player":p.id,"zone":zone,"card_id":id,
                "handle":c.handle(),"effects":effects}));
        }
    }
    json!({"diagnostic_only":true,"accepted_for_execution":false,
        "seed":918731,"removed_queue_entries":removed,"cards":rows})
}
fn run() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(args.len() == 3 || (args.len() == 4 && ["--hypothesis-reader-inventory", "--guarded-search-probe", "--model-search-probe"].contains(&args[3].as_str())),
        "panel-root active-snapshot new-output [--hypothesis-reader-inventory|--guarded-search-probe|--model-search-probe]");
    let root = PathBuf::from(&args[0]);
    let active = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    assert!(!out.exists(), "preserve existing diagnostic");
    let snap = read(&active);
    let request = &snap["request"];
    let planpath = root.join(request["plan"].as_str().unwrap());
    let plan = read(&planpath);
    let requestpath = root
        .join("requests")
        .join(format!("{}.json", request["id"].as_str().unwrap()));
    assert_eq!(
        read(&requestpath),
        *request,
        "snapshot must bind original request"
    );
    let cards = root.join(plan["cards_json"].as_str().unwrap());
    let priorpath = root.join(plan["prior"].as_str().unwrap());
    assert_eq!(hash(&cards), plan["cards_sha256"].as_str().unwrap());
    assert_eq!(
        hash(&priorpath),
        plan["prior_file_sha256"].as_str().unwrap()
    );
    let data = CardData::load_from_str(&std::fs::read_to_string(&cards).unwrap()).unwrap();
    let pair: Vec<Vec<String>> = request["deck_indices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            serde_json::from_value(plan["decks"][i.as_u64().unwrap() as usize]["cards"].clone())
                .unwrap()
        })
        .collect();
    let prior =
        WeightedDeckPrior::new(serde_json::from_value::<PriorSnapshot>(read(&priorpath)).unwrap())
            .unwrap();
    let viewer = snap["viewer"].as_u64().unwrap() as u8;
    let mut game = Game::new(
        &pair,
        &data,
        Rules::standard(),
        request["game_seed"].as_u64(),
    )
    .unwrap();
    game.enable_belief_knowledge().unwrap();
    game.turn_count = 1;
    game.drain_events();
    let mut history = vec![inspect(&game, viewer)];
    for action in snap["executed_actions"].as_array().unwrap() {
        let action = action.as_u64().unwrap() as u16;
        let actor = decision_player(&game);
        let mask = build_action_mask(&game, actor);
        assert!(
            !game.game_over && mask[action as usize] > 0.5,
            "illegal replay action at {}: {action}",
            history.len() - 1
        );
        game.decode_action(action, actor);
        let events = game.drain_events();
        let mut state = inspect(&game, viewer);
        state["action"] = json!(action);
        state["mover"] = json!(actor);
        state["events"] = json!(events);
        history.push(state);
    }
    assert_eq!(
        history.len() - 1,
        snap["decision"].as_u64().unwrap() as usize
    );
    assert_eq!(decision_player(&game), viewer);
    let audit = match materialize_closed(
        &game,
        viewer,
        game.belief_knowledge(viewer).unwrap(),
        &prior,
        0,
    ) {
        Ok(world) => json!({"accepted":true,"queue":format!("{:?}",world.effect_queue),"resume":format!("{:?}",world.pending_selection_resume)}),
        Err(e) => json!({"accepted":false,"error":e}),
    };
    let readers = (args.get(3).map(String::as_str) == Some("--hypothesis-reader-inventory")).then(|| hypothesis_readers(&game, viewer, &prior));
    let model_probe = args.get(3).map(String::as_str) == Some("--model-search-probe");
    let mut model_identity = Value::Null;
    let model = model_probe.then(|| {
        let role = if viewer as u64 == snap["candidate_seat"].as_u64().unwrap() { "candidate" } else { "opponent" };
        let (path_key, hash_key) = if role == "candidate" { ("model", "model_sha256") }
            else { ("opponent_model", "opponent_model_sha256") };
        let settings = &request[role];
        assert_eq!(settings["kind"], "search");
        assert_eq!(settings["simulations"], 320);
        assert_eq!(settings["worlds"], 4);
        assert_eq!(settings["batch"], 4);
        assert_eq!(settings["root_noise"].as_f64(), Some(0.));
        assert_eq!(settings["information_regime"], "closed-list");
        assert_eq!(request["runtime_sha256"], plan["native_sha256"]);
        assert_eq!(request["prior_snapshot_hash"], plan["prior_snapshot_hash"]);
        assert_eq!(prior.snapshot_hash(), plan["prior_snapshot_hash"].as_str().unwrap());
        let expected_seed = request["search_seed"].as_u64().unwrap()
            .wrapping_add(snap["decision"].as_u64().unwrap().wrapping_mul(0x9e3779b97f4a7c15))
            .wrapping_add(viewer as u64);
        assert_eq!(snap["search_seed"].as_u64().unwrap(), expected_seed);
        let path = root.join(plan[path_key].as_str().unwrap());
        let actual_hash = hash(&path);
        assert_eq!(actual_hash, plan[hash_key].as_str().unwrap());
        let evaluator = digimon_engine::inference::BatchedPolicyValueEvaluator::load_for_profile(
            &path, digimon_engine::observation::ObservationProfileId::StandardLiteV2).unwrap();
        evaluator.require_terminal_value().unwrap();
        model_identity = json!({"role":role,"viewer":viewer,"path":plan[path_key],
            "sha256":actual_hash,"profile":"standard_lite_v2","terminal_value_required":true});
        evaluator
    });
    let search = (model_probe || args.get(3).map(String::as_str) == Some("--guarded-search-probe")).then(||
        search_probe(&game, &digimon_engine::card_registry::CardRegistry::from_cards(&data), &prior,
            snap["search_seed"].as_u64().unwrap(), model));
    let proof = json!({"schema":"offline-closed-audit-replay-v1","scored":false,"model_inference":model_probe,"model_identity":model_identity,
        "active_sha256":hash(&active),"request_sha256":hash(&requestpath),"plan_sha256":hash(&planpath),
        "diagnostic_binary_sha256":hash(&std::env::current_exe().unwrap()),"legal_actions_replayed":history.len()-1,"audit":audit,"history":history,
        "hypothesis_reader_inventory":readers,"guarded_search_probe":search});
    std::fs::write(out, serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
    println!(
        "{}",
        json!({"legal_actions_replayed":history.len()-1,"audit_accepted":audit["accepted"],"search":search})
    );
}
fn main() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
