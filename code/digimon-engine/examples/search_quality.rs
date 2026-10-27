//! Frozen-model diagnostics only. Never trains or emits training shards.
//! search_quality <corpus|sweep|tactics|match> <request.json> <result.json>
use digimon_engine::{
    action::build_action_mask,
    card_data::CardData,
    card_registry::CardRegistry,
    enums::PlayerId,
    game::Game,
    inference::BatchedPolicyValueEvaluator,
    observation::{build_observation_tensor, ObservationProfileId},
    policies::greedy::greedy_action,
    rules::Rules,
    search::{
        decision_player, determinized_search_batched, DeterminizedConfig, EvalRequest,
        PolicyValueFn, RootPolicy, SearchResult,
    },
};
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path, time::Instant};

#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;
#[path = "search_quality/tactics.rs"]
mod tactics;
#[path = "search_quality/telemetry.rs"]
mod telemetry;

#[cfg(windows)]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;
const PROFILE: ObservationProfileId = ObservationProfileId::StandardLiteV2;

fn number(v: &Value, key: &str) -> u64 {
    v[key].as_u64().unwrap_or_else(|| panic!("missing {key}"))
}
fn write(path: &Path, value: &Value) {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    std::fs::rename(tmp, path).unwrap();
}
fn legal(mask: &[f32]) -> Vec<u16> {
    mask.iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.5).then_some(i as u16))
        .collect()
}
fn choose(result: &SearchResult) -> u16 {
    // Evaluation play is deterministic: under a Gumbel root take the argmax
    // of the completed-Q improved policy (the survivor in `chosen_action`
    // carries the Gumbel exploration sample, which belongs to self-play);
    // otherwise first-max visits in original mask order, matching the
    // native driver.
    if let Some(policy) = &result.root_policy {
        let mut best = (f32::NEG_INFINITY, policy[0].0);
        for &(a, p) in policy {
            if p > best.0 {
                best = (p, a);
            }
        }
        return best.1;
    }
    let mut best = (0, result.root_visits[0].0);
    for &(a, v) in &result.root_visits {
        if v > best.0 {
            best = (v, a);
        }
    }
    best.1
}
/// Whether `game` sits at an unconstrained Main-phase root (the driver's
/// phase-budget rule): no pending selection, no mulligan, Main phase.
fn main_root(game: &Game) -> bool {
    game.pending_selection.is_none()
        && game.mulligan_current_player().is_none()
        && game.current_phase == digimon_engine::enums::GamePhase::Main
}
/// Search settings from a request object. Beyond `simulations`/`worlds`,
/// the search-budget experiments add optional `reduced_simulations` (the
/// budget for non-Main roots, applied when `main_root` is false), `root`
/// (`"puct"` default or `"gumbel"`) and `gumbel_candidates` (default 16).
fn config(v: &Value, seed: u64, noise: f32, main_root: bool) -> DeterminizedConfig {
    let mut c = DeterminizedConfig::default();
    c.worlds = number(v, "worlds") as u32;
    c.search.simulations = number(v, "simulations") as u32;
    if !main_root {
        if let Some(reduced) = v["reduced_simulations"].as_u64() {
            c.search.simulations = c.search.simulations.min(reduced as u32);
        }
    }
    c.search.seed = seed;
    c.search.dirichlet_epsilon = noise;
    c.search.observation_profile = PROFILE;
    c.search.root = match v["root"].as_str().unwrap_or("puct") {
        "puct" => RootPolicy::Puct,
        // Optional `gumbel_c_visit` / `gumbel_c_scale` override the paper's
        // σ(q) = (c_visit + max_visits) · c_scale · q constants (50 / 1).
        "gumbel" => RootPolicy::Gumbel {
            max_candidates: v["gumbel_candidates"].as_u64().unwrap_or(16) as u32,
            c_visit: v["gumbel_c_visit"].as_f64().unwrap_or(50.0) as f32,
            c_scale: v["gumbel_c_scale"].as_f64().unwrap_or(1.0) as f32,
        },
        other => panic!("unknown root policy {other}"),
    };
    c
}
fn policy(
    game: &Game,
    registry: &CardRegistry,
    evaluator: &mut dyn PolicyValueFn,
    mask: &[f32],
) -> u16 {
    let pid = decision_player(game);
    let obs = build_observation_tensor(game, pid, registry, PROFILE);
    let output = evaluator.evaluate_batch(&[EvalRequest {
        obs,
        mask: mask.to_vec(),
    }]);
    let mut best = (f32::NEG_INFINITY, 0);
    for a in legal(mask) {
        let p = output[0].policy[a as usize];
        if p > best.0 {
            best = (p, a);
        }
    }
    best.1
}
fn new_game(data: &HashMap<String, CardData>, decks: &[Vec<String>], seed: u64) -> Game {
    let mut g = Game::new(decks, data, Rules::standard(), Some(seed)).unwrap();
    g.turn_count = 1;
    g
}
fn replay(data: &HashMap<String, CardData>, decks: &[Vec<String>], node: &Value) -> Game {
    let mut g = new_game(data, decks, number(node, "game_seed"));
    for a in node["actions"].as_array().unwrap() {
        let pid = decision_player(&g);
        let a = a.as_u64().unwrap() as u16;
        assert!(
            !g.game_over && build_action_mask(&g, pid)[a as usize] > 0.5,
            "invalid corpus replay"
        );
        g.decode_action(a, pid);
    }
    assert!(!g.game_over, "terminal corpus root");
    g
}
fn decks_for(plan: &Value, node: &Value) -> Vec<Vec<String>> {
    node["deck_indices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            serde_json::from_value(plan["decks"][i.as_u64().unwrap() as usize]["cards"].clone())
                .unwrap()
        })
        .collect()
}
fn seed_for(master: u64, decision: u64, pid: PlayerId) -> u64 {
    // Identical decision/seat stream across candidate budgets; no budget in seed.
    master
        .wrapping_add(decision.wrapping_mul(0x9e3779b97f4a7c15))
        .wrapping_add(pid as u64)
}
fn run() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 3);
    let request: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let plan: Value =
        serde_json::from_slice(&std::fs::read(request["plan"].as_str().unwrap()).unwrap()).unwrap();
    let data = CardData::load_from_str(
        &std::fs::read_to_string(plan["cards_json"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let all_decks: Vec<Vec<String>> = plan["decks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| serde_json::from_value(d["cards"].clone()).unwrap())
        .collect();
    admission::validate_pool(
        &all_decks,
        &data,
        Path::new(plan["cards_json"].as_str().unwrap())
            .parent()
            .unwrap(),
    )
    .unwrap();
    let registry = CardRegistry::from_cards(&data);
    let mut evaluator = BatchedPolicyValueEvaluator::load_for_profile(
        Path::new(plan["model"].as_str().unwrap()),
        PROFILE,
    )
    .unwrap();
    evaluator.require_terminal_value().unwrap();
    let out = Path::new(&args[2]);
    assert!(!out.exists(), "preserve completed diagnostic");
    let started = Instant::now();
    match args[0].as_str() {
        "corpus" => {
            let mut nodes = Vec::new();
            let mut games = Vec::new();
            for index in 0..all_decks.len() {
                let other = (index + 3) % all_decks.len();
                let seed = number(&plan, "corpus_seed") + index as u64;
                let decks = vec![all_decks[index].clone(), all_decks[other].clone()];
                let mut game = new_game(&data, &decks, seed);
                let mut actions = Vec::new();
                let mut choices = Vec::new();
                for decision in 0..number(&plan, "decision_cap") {
                    if game.game_over {
                        break;
                    }
                    let pid = decision_player(&game);
                    let mask = build_action_mask(&game, pid);
                    let valid = legal(&mask);
                    assert!(!valid.is_empty());
                    if valid.len() > 1 && pid == 0 {
                        choices.push(json!({"id":format!("heldout-{index}-d{decision}"),"archetype":plan["decks"][index]["name"],
                            "game_seed":seed,"deck_indices":[index,other],"actions":actions.clone(),
                            "decision":decision,"mover":pid,"phase":format!("{:?}",game.current_phase),
                            "pending":game.pending_selection.is_some(),"legal_actions":valid}));
                    }
                    let a = if valid.len() == 1 {
                        valid[0]
                    } else if (pid == 0) == (index % 2 == 0) {
                        greedy_action(&game, &mask)
                    } else {
                        policy(&game, &registry, &mut evaluator, &mask)
                    };
                    assert!(mask[a as usize] > 0.5);
                    game.decode_action(a, pid);
                    actions.push(a);
                }
                assert!(choices.len() >= 2, "not enough corpus decisions");
                // Freeze mid/late-game roots, prefer a genuine pending choice for the second.
                let middle = choices.len() / 2;
                let second = (middle + 1..choices.len())
                    .find(|&i| choices[i]["pending"] == true)
                    .unwrap_or(choices.len() * 3 / 4);
                nodes.push(choices[middle].clone());
                nodes.push(choices[second.max(middle + 1).min(choices.len() - 1)].clone());
                games.push(json!({"deck_indices":[index,other],"seed":seed,"decisions":actions.len(),"game_over":game.game_over,"winner":game.winner}));
            }
            write(
                out,
                &json!({"diagnostic_only":true,"nodes":nodes,"games":games,"seconds":started.elapsed().as_secs_f64()}),
            );
        }
        "sweep" => {
            let corpus: Value = serde_json::from_slice(
                &std::fs::read(request["corpus"].as_str().unwrap()).unwrap(),
            )
            .unwrap();
            let seed = number(&request, "seed");
            let noise = request["noise"].as_f64().unwrap_or(0.) as f32;
            let mut rows = Vec::new();
            for node in corpus["nodes"].as_array().unwrap() {
                let game = replay(&data, &decks_for(&plan, node), node);
                let pid = decision_player(&game);
                let mask = build_action_mask(&game, pid);
                assert_eq!(
                    legal(&mask),
                    serde_json::from_value::<Vec<u16>>(node["legal_actions"].clone()).unwrap()
                );
                // Per node: a `reduced_simulations` setting applies to
                // non-Main roots only, so the effective budget is root-specific.
                let c = config(&request["config"], seed, noise, main_root(&game));
                let before = Instant::now();
                let result =
                    determinized_search_batched(&registry, &game, pid, &mut evaluator, &c, 4);
                assert_eq!(result.simulations_run, c.search.simulations);
                rows.push(json!({"node":node["id"],"archetype":node["archetype"],"action":choose(&result),
                    "visits":result.root_visits,"q":result.root_q,"value":result.root_value,
                    "policy":result.root_policy,"survivor":result.chosen_action,
                    "seconds":before.elapsed().as_secs_f64(),"simulations_run":result.simulations_run}));
                write(
                    &out.with_extension("progress.json"),
                    &json!({"completed":rows.len(),"requested":corpus["nodes"].as_array().unwrap().len()}),
                );
            }
            write(
                out,
                &json!({"config":request["config"],"seed":seed,"noise":request["noise"],"rows":rows,"seconds":started.elapsed().as_secs_f64()}),
            );
        }
        "tactics" => {
            write(out, &tactics::run(&registry, &mut evaluator, &plan));
        }
        "match" | "replay_metrics" => {
            let indices = &request["deck_indices"];
            let decks = decks_for(&plan, &json!({"deck_indices":indices}));
            let game_seed = number(&request, "game_seed");
            let mut games = Vec::new();
            for candidate_seat in 0..2u8 {
                let mut game = new_game(&data, &decks, game_seed);
                let mut decisions = 0;
                let mut searched = 0;
                let first_player = game.turn_player();
                let game_start = Instant::now();
                let mut action_trace = Vec::new();
                let collect_metrics = request["card_metrics"].as_bool().unwrap_or(false);
                let initial_ui = collect_metrics.then(|| telemetry::snapshot(&game));
                let mut metric_steps = Vec::new();
                if collect_metrics {
                    game.drain_events();
                }
                while !game.game_over && decisions < number(&plan, "decision_cap") {
                    let pid = decision_player(&game);
                    let phase = format!("{:?}", game.current_phase);
                    let mask = build_action_mask(&game, pid);
                    let valid = legal(&mask);
                    assert!(!valid.is_empty());
                    let selection_started = Instant::now();
                    let a = if args[0] == "replay_metrics" {
                        request["replay_games"][candidate_seat as usize]["actions"]
                            [decisions as usize]
                            .as_u64()
                            .expect("replay exhausted before terminal")
                            as u16
                    } else if valid.len() == 1 {
                        valid[0]
                    } else {
                        let settings = if pid == candidate_seat {
                            &request["candidate"]
                        } else {
                            &request["opponent"]
                        };
                        match settings["kind"].as_str().unwrap_or("search") {
                            "greedy" => greedy_action(&game, &mask),
                            "policy" => policy(&game, &registry, &mut evaluator, &mask),
                            "search" => {
                                let c = config(
                                    settings,
                                    seed_for(number(&request, "search_seed"), decisions, pid),
                                    0.,
                                    main_root(&game),
                                );
                                let result = determinized_search_batched(
                                    &registry,
                                    &game,
                                    pid,
                                    &mut evaluator,
                                    &c,
                                    4,
                                );
                                searched += 1;
                                choose(&result)
                            }
                            other => panic!("unknown policy {other}"),
                        }
                    };
                    let selection_seconds = selection_started.elapsed().as_secs_f64();
                    assert!(mask[a as usize] > 0.5);
                    game.decode_action(a, pid);
                    if collect_metrics {
                        let events = game.drain_events();
                        metric_steps.push(json!({"mover":pid + 1,"phase":phase,"legal_actions":valid,"selection_seconds":selection_seconds,
                            "action":a,"after_ui":telemetry::snapshot(&game),"turn_count":game.turn_count,
                            "events":events}));
                    }
                    decisions += 1;
                    action_trace.push(a);
                    if decisions % 20 == 0 {
                        write(
                            &out.with_extension("progress.json"),
                            &json!({"seat":candidate_seat,"decisions":decisions,"seconds":game_start.elapsed().as_secs_f64()}),
                        );
                    }
                }
                let truncated = !game.game_over
                    || matches!(
                        game.terminal_outcome_reason,
                        Some(
                            digimon_engine::game::TerminalOutcomeReason::StepLimit
                                | digimon_engine::game::TerminalOutcomeReason::Crash
                        )
                    );
                let score = if truncated || game.winner.is_none() {
                    0.5
                } else if game.winner == Some(candidate_seat) {
                    1.
                } else {
                    0.
                };
                games.push(json!({"candidate_seat":candidate_seat,"first_player":first_player,
                    "winner":game.winner,"truncated":truncated,"score":score,"decisions":decisions,"searched":searched,
                    "terminal_reason":format!("{:?}",game.terminal_outcome_reason),"seconds":game_start.elapsed().as_secs_f64(),"actions":action_trace,
                    "card_metrics":if collect_metrics {json!({"schema":"native-card-eval-v1","event_version":1,
                        "initial_ui":initial_ui,"steps":metric_steps})} else {Value::Null}}));
                if args[0] == "replay_metrics" {
                    assert_eq!(
                        games.last().unwrap()["winner"],
                        request["replay_games"][candidate_seat as usize]["winner"]
                    );
                    assert_eq!(
                        games.last().unwrap()["actions"],
                        request["replay_games"][candidate_seat as usize]["actions"]
                    );
                }
                write(
                    &out.with_extension("progress.json"),
                    &json!({"completed_games":games.len(),"games":games}),
                );
            }
            write(
                out,
                &json!({"candidate":request["candidate"],"opponent":request["opponent"],"game_seed":game_seed,
                "deck_indices":indices,"games":games,"seconds":started.elapsed().as_secs_f64()}),
            );
        }
        other => panic!("unknown mode {other}"),
    }
}
fn main() {
    std::thread::Builder::new()
        .stack_size(268435456)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
