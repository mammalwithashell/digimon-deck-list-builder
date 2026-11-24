//! Grade a model's search on a saved search exam (`qa/search-exams/<id>.json`).
//!
//! An exam is a real recorded position plus what the search should do there.
//! The position is rebuilt from (decks, seed, recorded actions), searched at
//! the exam's budget over several seeds, and each expectation is reported as
//! "passed k of N seeds". Lines in the exam are scored with the value head
//! step by step, from the searching seat's side.
//!
//!   cargo run --release -p digimon-engine --example search_exam -- \
//!       qa/search-exams/<id>.json --model <model.onnx> [--seeds 8] \
//!       [--simulations N] [--worlds W] [--json out.json]
//!
//! Exams are saved with `code/tools/search_exam/make_exam.py`. This is a
//! diagnostic, not a gate: one position says little about overall strength.
use digimon_engine::{
    action::build_action_mask,
    card_data::CardData,
    card_registry::CardRegistry,
    determinize::{
        beliefs::{CandidateDeck, PriorSnapshot, WeightedDeckPrior},
        closed::materialize_closed_with_diagnostics,
    },
    game::Game,
    inference::BatchedPolicyValueEvaluator,
    observation::{build_observation_tensor_with_mask, ObservationProfileId},
    rules::Rules,
    search::{
        decision_player, determinized_search_batched, determinized_search_sampled,
        DeterminizedConfig, EvalRequest, PolicyValueFn, SearchResult,
    },
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

const SCHEMA: &str = "search-exam-v1";

struct Args {
    exam: String,
    model: String,
    seeds: u64,
    simulations: Option<u32>,
    worlds: Option<u32>,
    json: Option<String>,
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1);
    let exam = it
        .next()
        .expect("usage: search_exam <exam.json> --model <onnx> [options]");
    let mut args = Args {
        exam,
        model: String::new(),
        seeds: 8,
        simulations: None,
        worlds: None,
        json: None,
    };
    while let Some(flag) = it.next() {
        let value = it.next().unwrap_or_else(|| panic!("{flag} needs a value"));
        match flag.as_str() {
            "--model" => args.model = value,
            "--seeds" => args.seeds = value.parse().unwrap(),
            "--simulations" => args.simulations = Some(value.parse().unwrap()),
            "--worlds" => args.worlds = Some(value.parse().unwrap()),
            "--json" => args.json = Some(value),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!args.model.is_empty(), "--model is required");
    args
}

fn cards_json_path() -> String {
    std::env::var("DIGIMON_CARDS_JSON").unwrap_or_else(|_| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/cards.json")
            .to_string_lossy()
            .into()
    })
}

fn plan_of(v: &Value) -> Vec<u16> {
    serde_json::from_value(v.clone()).expect("plan must be a list of action ids")
}

fn share(result: &SearchResult, plan: &[u16]) -> f64 {
    let total: u32 = result.root_plans.iter().map(|p| p.1).sum();
    let visits = result
        .root_plans
        .iter()
        .find(|p| p.0 == plan)
        .map_or(0, |p| p.1);
    if total == 0 {
        0.0
    } else {
        visits as f64 / total as f64
    }
}

fn run() {
    let args = parse_args();
    let exam: Value = serde_json::from_slice(&std::fs::read(&args.exam).unwrap()).unwrap();
    assert_eq!(exam["schema"], SCHEMA, "not a search exam");
    let model_hash = format!("{:x}", Sha256::digest(std::fs::read(&args.model).unwrap()));
    let exam_model = exam["model"]["sha256"].as_str().unwrap_or("");
    if model_hash != exam_model {
        eprintln!("note: model sha256 {model_hash} differs from the exam's recorded {} ({exam_model}); grading a different model.",
            exam["model"]["name"]);
    }

    let data =
        CardData::load_from_str(&std::fs::read_to_string(cards_json_path()).unwrap()).unwrap();
    let registry = CardRegistry::from_cards(&data);
    let setup = &exam["setup"];
    let decks: Vec<Vec<String>> = serde_json::from_value(setup["decks"].clone()).unwrap();
    let mut game = Game::new(
        &decks,
        &data,
        Rules::standard(),
        Some(setup["game_seed"].as_u64().unwrap()),
    )
    .unwrap();
    game.enable_belief_knowledge().unwrap();
    game.turn_count = setup["turn_count_reset"].as_u64().unwrap_or(1) as _;
    for (i, a) in plan_of(&setup["actions"]).into_iter().enumerate() {
        let pid = decision_player(&game);
        assert!(build_action_mask(&game, pid)[a as usize] > 0.5,
            "recorded action {a} at move {i} is illegal on this engine; the exam no longer reproduces");
        game.decode_action(a, pid);
    }
    let search = &exam["search"];
    let viewer = search["viewer"].as_u64().unwrap() as u8;
    assert_eq!(
        decision_player(&game),
        viewer,
        "exam position belongs to a different seat on this engine"
    );
    let decision = setup["actions"].as_array().unwrap().len() as u64;

    let closed = search["regime"] == "closed-list";
    let prior = closed.then(|| {
        let p = &exam["prior"];
        let population: BTreeSet<String> = serde_json::from_value(p["population"].clone()).unwrap();
        let candidates: Vec<CandidateDeck> =
            serde_json::from_value(p["candidates"].clone()).unwrap();
        let snapshot = PriorSnapshot::new(
            p["format_id"].as_str().unwrap(),
            &format!("search exam {}", exam["id"]),
            population,
            candidates,
            p["unseen_mass"].as_f64().unwrap(),
            p["fallback_attempts"].as_u64().unwrap() as usize,
        )
        .unwrap();
        WeightedDeckPrior::new(snapshot).unwrap()
    });

    let profile = ObservationProfileId::StandardLiteV2;
    let mut eval =
        BatchedPolicyValueEvaluator::load_for_profile(Path::new(&args.model), profile).unwrap();
    eval.require_terminal_value().unwrap();
    let simulations = args
        .simulations
        .unwrap_or(search["simulations"].as_u64().unwrap() as u32);
    let worlds = args
        .worlds
        .unwrap_or(search["worlds"].as_u64().unwrap() as u32);
    let batch = search["batch"].as_u64().unwrap_or(4) as usize;
    let base_seed = search["search_seed"]
        .as_u64()
        .unwrap()
        .wrapping_add(decision.wrapping_mul(0x9e3779b97f4a7c15))
        .wrapping_add(viewer as u64);

    println!(
        "{} — {}",
        exam["id"].as_str().unwrap(),
        exam["title"].as_str().unwrap()
    );
    println!("{simulations} simulations, {worlds} worlds, {} regime, {} seeds (seed 0 = the recorded search)",
        search["regime"].as_str().unwrap(), args.seeds);

    let expectations = exam["expectations"].as_array().cloned().unwrap_or_default();
    let mut passes = vec![0u64; expectations.len()];
    let mut runs = Vec::new();
    for k in 0..args.seeds {
        let mut config = DeterminizedConfig::default();
        config.worlds = worlds;
        config.search.simulations = simulations;
        config.search.dirichlet_epsilon = 0.;
        config.search.observation_profile = profile;
        config.search.seed = base_seed.wrapping_add(k.wrapping_mul(0x5851f42d4c957f2d));
        let result = match &prior {
            Some(prior) => {
                let memory = game.belief_knowledge(viewer).unwrap();
                determinized_search_sampled(
                    &registry,
                    &game,
                    viewer,
                    &mut eval,
                    &config,
                    batch,
                    &mut |seed| {
                        materialize_closed_with_diagnostics(&game, viewer, memory, prior, seed)
                            .map(|(w, _)| w)
                    },
                )
                .unwrap()
            }
            None => {
                determinized_search_batched(&registry, &game, viewer, &mut eval, &config, batch)
            }
        };
        let chosen = result.best_plan().unwrap().to_vec();
        let mut verdicts = Vec::new();
        for (i, e) in expectations.iter().enumerate() {
            let plan = plan_of(&e["plan"]);
            let ok = match e["kind"].as_str().unwrap() {
                "visit_share_at_least" => share(&result, &plan) >= e["share"].as_f64().unwrap(),
                "not_chosen" => chosen != plan,
                "chosen" => chosen == plan,
                other => panic!("unknown expectation kind {other}"),
            };
            passes[i] += u64::from(ok);
            verdicts.push(ok);
        }
        let mut plans = result.root_plans.clone();
        plans.sort_by(|a, b| b.1.cmp(&a.1));
        runs.push(json!({"seed_offset":k,"chosen":chosen,"verdicts":verdicts,"root_value":result.root_value,
            "plans":plans.iter().map(|p| json!({"plan":p.0,"visits":p.1,"q":p.2})).collect::<Vec<_>>()}));
        let top: Vec<String> = plans
            .iter()
            .take(3)
            .map(|p| {
                format!(
                    "{:?} {:.0}% q{:+.2}",
                    p.0,
                    100.0 * share(&result, &p.0),
                    p.2
                )
            })
            .collect();
        println!("  seed {k}: chose {chosen:?}  top {}", top.join(", "));
    }
    println!("expectations:");
    for (i, e) in expectations.iter().enumerate() {
        let what = match e["kind"].as_str().unwrap() {
            "visit_share_at_least" => format!(
                "gets ≥{:.0}% of visits",
                100.0 * e["share"].as_f64().unwrap()
            ),
            "not_chosen" => "is not chosen".into(),
            _ => "is chosen".into(),
        };
        println!(
            "  {:>2}/{} seeds  {} ({}) {what}",
            passes[i],
            args.seeds,
            e["label"].as_str().unwrap_or("?"),
            e["plan"]
        );
    }

    // Value head along the exam's lines, from the searching seat's side.
    let mut lines = json!({});
    for (name, acts) in exam["lines"].as_object().cloned().unwrap_or_default() {
        let mut g = game.clone();
        let mut values = Vec::new();
        let acts = plan_of(&acts);
        for step in 0..=acts.len() {
            if g.game_over {
                break;
            }
            let mover = decision_player(&g);
            let mask = build_action_mask(&g, mover);
            let obs = build_observation_tensor_with_mask(&g, mover, &registry, profile, &mask);
            let v = PolicyValueFn::evaluate_batch(
                &mut eval,
                &[EvalRequest {
                    obs,
                    mask: mask.clone(),
                }],
            )[0]
            .value;
            values.push(if mover == viewer { v } else { -v });
            if step == acts.len() {
                break;
            }
            assert!(
                mask[acts[step] as usize] > 0.5,
                "line {name}: action {} illegal at step {step}",
                acts[step]
            );
            g.decode_action(acts[step], mover);
        }
        println!(
            "  value along {name}: {}",
            values
                .iter()
                .map(|v| format!("{v:+.2}"))
                .collect::<Vec<_>>()
                .join(" → ")
        );
        lines[&name] = json!(values);
    }
    if let Some(path) = args.json {
        let report = json!({"exam":exam["id"],"model_sha256":model_hash,"simulations":simulations,"worlds":worlds,
            "seeds":args.seeds,"passes":passes,"runs":runs,"lines":lines});
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

fn main() {
    // Deep effect chains recurse; match the panel drivers' stack.
    std::thread::Builder::new()
        .stack_size(268435456)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
