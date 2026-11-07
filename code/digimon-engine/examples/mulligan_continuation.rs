//! NEW single-game paired mulligan continuation runner. Qualification required.
#[path = "search_quality/mulligan_continuation_root.rs"]
mod root;
#[path = "search_quality/telemetry.rs"]
mod telemetry;
#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;
use digimon_engine::{action::build_action_mask, card_data::CardData,
    card_registry::CardRegistry, inference::BatchedPolicyValueEvaluator,
    observation::ObservationProfileId, search::{decision_player, determinized_search_batched,
        DeterminizedConfig, PolicyValueFn}};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Instant};

const SCHEMA: &str = "mulligan-continuation-game-v1";
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn hash(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}
fn write(path: &Path, value: &Value) {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    std::fs::rename(tmp, path).unwrap();
}
fn num(v: &Value, k: &str) -> u64 { v[k].as_u64().unwrap() }
fn run() {
    assert!(cfg!(feature = "portable-policy-math"), "portable runtime required");
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "request output");
    let out = Path::new(&args[1]);
    assert!(!out.exists() && !out.with_extension("active.json").exists(), "preserve prior attempt");
    let request = read(&args[0]);
    assert_eq!(request["schema"], SCHEMA);
    let plan = read(request["plan"].as_str().unwrap());
    assert_eq!(plan["schema"], SCHEMA);
    assert_eq!(hash(request["plan"].as_str().unwrap()), request["plan_sha256"].as_str().unwrap());
    let runtime_hash = hash(std::env::current_exe().unwrap());
    assert_eq!(runtime_hash, plan["runtime_sha256"].as_str().unwrap());
    assert_eq!(request["runtime_sha256"], plan["runtime_sha256"]);
    assert_eq!(request["protocol_sha256"], plan["protocol_sha256"]);
    assert_eq!(hash(plan["protocol"].as_str().unwrap()), plan["protocol_sha256"].as_str().unwrap());
    for (path, key) in [("cards_json", "cards_sha256"), ("model", "model_sha256"),
                        ("opponent_model", "opponent_model_sha256")] {
        assert_eq!(hash(plan[path].as_str().unwrap()), plan[key].as_str().unwrap());
    }
    assert_eq!(request["checkpoint"], plan["checkpoint"]);
    assert_eq!(request["opponent_checkpoint"], 8);
    assert_eq!(plan["opponent_checkpoint"], 8);
    assert_eq!(plan["information_regime"], "known-list");
    assert_eq!([num(&plan,"simulations"),num(&plan,"worlds"),num(&plan,"batch"),num(&plan,"root_noise")], [320,4,4,0]);
    assert_eq!(num(&plan,"decision_cap"), 2000);
    assert_eq!(num(&plan,"deadline_seconds"), 3600);
    let data = CardData::load_from_str(&std::fs::read_to_string(plan["cards_json"].as_str().unwrap()).unwrap()).unwrap();
    let decks: Vec<Vec<String>> = plan["decks"].as_array().unwrap().iter()
        .map(|d| serde_json::from_value(d["cards"].clone()).unwrap()).collect();
    admission::validate_pool(&decks, &data, Path::new(plan["cards_json"].as_str().unwrap()).parent().unwrap()).unwrap();
    assert_eq!(hash(request["root_path"].as_str().unwrap()), request["root_sha256"].as_str().unwrap());
    let node: root::SavedRoot = serde_json::from_value(read(request["root_path"].as_str().unwrap())).unwrap();
    assert_eq!(node.id, request["root"].as_str().unwrap());
    let registry = CardRegistry::from_cards(&data);
    let mut original = root::reconstruct(&node, &decks, &data, &registry).unwrap();
    original.drain_events(); // Prefix events precede the intervention denominator.
    let candidate_seat = node.mover;
    let branch: u16 = num(&request,"forced_mulligan_action").try_into().unwrap();
    let initial_ui = telemetry::snapshot(&original);
    let original_digest = original.verification_digest();
    let mut game = root::intervene(&original, &registry, &node, branch, num(&request,"outcome_seed")).unwrap();
    let forced_digest = game.verification_digest();
    let first_player = game.turn_player();
    let mut candidate = BatchedPolicyValueEvaluator::load_for_profile(Path::new(plan["model"].as_str().unwrap()), ObservationProfileId::StandardLiteV2).unwrap();
    let mut opponent = BatchedPolicyValueEvaluator::load_for_profile(Path::new(plan["opponent_model"].as_str().unwrap()), ObservationProfileId::StandardLiteV2).unwrap();
    candidate.require_terminal_value().unwrap(); opponent.require_terminal_value().unwrap();
    let started = Instant::now();
    let mut actions = vec![branch];
    let mut steps = vec![json!({"mover":candidate_seat+1,"phase":"Mulligan","action":branch,
        "legal_actions":[0,1],"forced_intervention":true,"selection_seconds":0.,
        "after_ui":telemetry::snapshot(&game),"turn_count":game.turn_count,"events":game.drain_events()})];
    let mut searches = Vec::new();
    while !game.game_over && actions.len() < 2000 {
        assert!(started.elapsed().as_secs_f64() < 3600., "game deadline exceeded");
        let pid = decision_player(&game);
        let mask = build_action_mask(&game, pid);
        let legal: Vec<_> = mask.iter().enumerate().filter_map(|(a,m)| (*m>0.5).then_some(a as u16)).collect();
        assert!(!legal.is_empty(), "empty legal mask");
        let start = actions.len();
        let before = Instant::now();
        let seed = num(&request,"follow_on_search_seed").wrapping_add((start as u64).wrapping_mul(0x9e3779b97f4a7c15)).wrapping_add(pid as u64);
        write(&out.with_extension("active.json"), &json!({"request":request,"decision":start,"actor":pid,
            "search_seed":seed,"actions":actions,"root_observation_sha256":node.observation_sha256}));
        let route = if legal.len()==1 { legal.clone() } else {
            let mut config = DeterminizedConfig::default();
            config.worlds=4; config.search.simulations=320; config.search.dirichlet_epsilon=0.;
            config.search.observation_profile=ObservationProfileId::StandardLiteV2; config.search.seed=seed;
            // Same root-actor rollout evaluator convention as the frozen reference.
            let eval: &mut dyn PolicyValueFn = if pid==candidate_seat {&mut candidate} else {&mut opponent};
            let result=determinized_search_batched(&registry,&game,pid,eval,&config,4);
            assert_eq!(result.simulations_run,320);
            let selected=result.best_plan().expect("no complete root plan").to_vec();
            let opportunity=digimon_engine::search::opportunity::describe(&game,pid,&result,&selected,"argmax_tau0").unwrap();
            searches.push(json!({"decision":start,"actor":pid,"model_sha256":if pid==candidate_seat {&plan["model_sha256"]} else {&plan["opponent_model_sha256"]},
                "simulations":result.simulations_run,"worlds":4,"batch":4,"root_noise":0,"search_seed":seed,
                "information_regime":"known-list","selected_plan":selected,"root_plans":result.root_plans,"opportunity":opportunity}));
            selected
        };
        let seconds=before.elapsed().as_secs_f64();
        for (ix,&action) in route.iter().enumerate() {
            assert!(!game.game_over && actions.len()<2000 && started.elapsed().as_secs_f64()<3600., "incomplete route/cap/deadline");
            assert_eq!(decision_player(&game),pid,"route changed actor before commitment");
            let next_mask=build_action_mask(&game,pid);
            assert!(next_mask.get(action as usize).is_some_and(|v|*v>0.5),"route no longer legal");
            let valid: Vec<_>=next_mask.iter().enumerate().filter_map(|(a,m)|(*m>0.5).then_some(a)).collect();
            let phase=format!("{:?}",game.current_phase);
            game.decode_action(action,pid); actions.push(action);
            steps.push(json!({"mover":pid+1,"phase":phase,"action":action,"legal_actions":valid,
                "selection_seconds":if ix==0 {seconds}else{0.},"after_ui":telemetry::snapshot(&game),
                "turn_count":game.turn_count,"events":game.drain_events()}));
            write(&out.with_extension("active.json"), &json!({"request":request,"decision":actions.len(),"actor":pid,"actions":actions}));
        }
        if let Some(search)=searches.last_mut().filter(|s|s["decision"].as_u64()==Some(start as u64)) {
            digimon_engine::search::opportunity::record_execution(&mut search["opportunity"],&actions[start..]).unwrap();
        }
    }
    assert!(game.game_over && !matches!(game.terminal_outcome_reason,Some(digimon_engine::game::TerminalOutcomeReason::StepLimit|digimon_engine::game::TerminalOutcomeReason::Crash)),"incomplete game; no completed score");
    let score=match game.winner { None=>0.5,Some(w) if w==candidate_seat=>1.,_=>0. };
    let output=json!({"schema":SCHEMA,"request":request,"runtime_sha256":runtime_hash,
        "plan_sha256":hash(request["plan"].as_str().unwrap()),"model_sha256":plan["model_sha256"],"opponent_model_sha256":plan["opponent_model_sha256"],
        "root_observation_sha256":node.observation_sha256,"root_digest":original_digest,"forced_digest":forced_digest,
        "game":{"candidate_seat":candidate_seat,"first_player":first_player,"winner":game.winner,"score":score,"truncated":false,
            "terminal_reason":format!("{:?}",game.terminal_outcome_reason),"actions":actions,"decisions":actions.len(),
            "searched":searches.len(),"search_diagnostics":searches,"seconds":started.elapsed().as_secs_f64(),
            "card_metrics":{"schema":"native-card-eval-v1","event_version":1,"initial_ui":initial_ui,"steps":steps}}});
    use std::io::Write;
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(out).unwrap();
    file.write_all(&serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    println!("completed one continuation game");
}
fn main() {
    std::thread::Builder::new().stack_size(268435456).spawn(run).unwrap().join().unwrap();
}
