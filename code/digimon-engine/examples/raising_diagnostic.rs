//! Frozen raising counterfactuals. No fitting, training shards or promotion.
//! raising_diagnostic <collect|probe> <request.json> <result.json>
use digimon_engine::{
    action::{
        build_action_mask,
        space::{MOVE_FROM_BREEDING, PASS},
    },
    card_data::CardData,
    card_registry::CardRegistry,
    enums::GamePhase,
    game::Game,
    inference::BatchedPolicyValueEvaluator,
    observation::{build_observation_tensor, ObservationProfileId},
    rules::Rules,
    search::{
        decision_player, determinized_search_batched, DeterminizedConfig, EvalRequest, Evaluation,
        PolicyValueFn,
    },
};
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path, time::Instant};

#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;
#[cfg(windows)]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;
const PROFILE: ObservationProfileId = ObservationProfileId::StandardLiteV2;

fn read(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn write(path: &Path, value: &Value) {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    std::fs::rename(temporary, path).unwrap();
}
fn evaluate(game: &Game, registry: &CardRegistry, model: &mut dyn PolicyValueFn) -> Evaluation {
    let player = decision_player(game);
    model
        .evaluate_batch(&[EvalRequest {
            obs: build_observation_tensor(game, player, registry, PROFILE),
            mask: build_action_mask(game, player),
        }])
        .remove(0)
}
fn argmax(evaluation: &Evaluation, mask: &[f32]) -> u16 {
    let mut best = None;
    for (action, &legal) in mask.iter().enumerate() {
        if legal > 0.5 && best.is_none_or(|(_, score)| evaluation.policy[action] > score) {
            best = Some((action as u16, evaluation.policy[action]));
        }
    }
    best.expect("nonempty legal mask").0
}
fn policy(game: &Game, registry: &CardRegistry, model: &mut dyn PolicyValueFn) -> u16 {
    let mask = build_action_mask(game, decision_player(game));
    let legal: Vec<_> = mask.iter().enumerate().filter(|(_, m)| **m > 0.5).collect();
    if legal.len() == 1 {
        legal[0].0 as u16
    } else {
        argmax(&evaluate(game, registry, model), &mask)
    }
}
fn config(seed: u64) -> DeterminizedConfig {
    let mut config = DeterminizedConfig::default();
    config.worlds = 4;
    config.search.simulations = 320;
    config.search.seed = seed;
    config.search.dirichlet_epsilon = 0.;
    config.search.observation_profile = PROFILE;
    config
}
fn eligible(game: &Game, actor: u8) -> bool {
    if game.game_over
        || game.current_phase != GamePhase::Breeding
        || game.pending_selection.is_some()
        || decision_player(game) != actor
    {
        return false;
    }
    let mask = build_action_mask(game, actor);
    mask[MOVE_FROM_BREEDING as usize] > 0.5 && mask[PASS as usize] > 0.5
}
fn off_field_token(game: &Game) -> bool {
    game.players.iter().any(|p| {
        p.hand
            .iter()
            .chain(&p.deck)
            .chain(&p.security)
            .chain(&p.trash)
            .chain(&p.digitama_deck)
            .any(|c| c.is_token)
    })
}
fn describe(game: &Game, actor: u8) -> Value {
    let me = game.player(actor);
    let raising = me.breeding_area.as_ref().unwrap();
    json!({"turn":game.turn_count, "actor":actor, "memory":game.memory,
        "raising_card":raising.top_card().card_id(&game.card_data),
        "raising_level":raising.top_card().level(&game.card_data),
        "raising_stack_size":raising.card_sources.len(),
        "own_security":me.security.len(), "opponent_security":game.player(1-actor).security.len(),
        "own_board":me.battle_area.len(), "opponent_board":game.player(1-actor).battle_area.len(),
        "own_hand":me.hand.len(), "digest":game.verification_digest().to_string()})
}
fn start(data: &HashMap<String, CardData>, decks: &[Vec<String>], seed: u64) -> Game {
    let mut game = Game::new(decks, data, Rules::standard(), Some(seed)).unwrap();
    game.turn_count = 1;
    game
}
fn replay(data: &HashMap<String, CardData>, decks: &[Vec<String>], node: &Value) -> Game {
    let mut game = start(data, decks, node["game_seed"].as_u64().unwrap());
    for action in node["actions"].as_array().unwrap() {
        let player = decision_player(&game);
        let action = action.as_u64().unwrap() as u16;
        assert!(!game.game_over && build_action_mask(&game, player)[action as usize] > 0.5);
        game.decode_action(action, player);
    }
    assert_eq!(
        game.verification_digest().to_string(),
        node["position"]["digest"].as_str().unwrap()
    );
    assert!(eligible(&game, node["actor"].as_u64().unwrap() as u8));
    assert!(!off_field_token(&game));
    game
}
fn rollout(
    root: &Game,
    registry: &CardRegistry,
    models: &mut [BatchedPolicyValueEvaluator],
    actor: u8,
    own: usize,
    opponent: usize,
    action: u16,
    cap: usize,
    search_actor: bool,
    search_seed: u64,
) -> Value {
    let mut game = root.clone();
    let mut actions = vec![action];
    game.decode_action(action, actor);
    let mut noops = 0;
    let mut searched = 0;
    let mut reason = "decision_cap";
    for decision in 1..cap {
        if game.game_over {
            break;
        }
        if off_field_token(&game) {
            reason = "off_field_token";
            break;
        }
        let player = decision_player(&game);
        let index = if player == actor { own } else { opponent };
        let mask = build_action_mask(&game, player);
        if !mask.iter().any(|&m| m > 0.5) {
            reason = "empty_mask";
            break;
        }
        let plan =
            if search_actor && player == actor && mask.iter().filter(|&&m| m > 0.5).count() > 1 {
                let result = determinized_search_batched(
                    registry,
                    &game,
                    player,
                    &mut models[index],
                    &config(search_seed.wrapping_add(decision as u64)),
                    4,
                );
                searched += 1;
                result.best_plan().unwrap().to_vec()
            } else {
                vec![policy(&game, registry, &mut models[index])]
            };
        for chosen in plan {
            assert_eq!(decision_player(&game), player);
            assert!(build_action_mask(&game, player)[chosen as usize] > 0.5);
            let before = game.verification_digest();
            game.decode_action(chosen, player);
            actions.push(chosen);
            if game.verification_digest() == before {
                noops += 1;
            } else {
                noops = 0;
            }
        }
        if noops >= 3 {
            reason = "repeated_noop";
            break;
        }
    }
    let complete = game.game_over && !off_field_token(&game);
    let score = complete.then(|| match game.winner {
        Some(winner) => f64::from(winner == actor),
        None => 0.5,
    });
    json!({"status":if complete {"complete"} else {"incomplete"}, "score":score,
        "winner":game.winner, "decisions":actions.len(), "turn":game.turn_count,
        "searched":searched, "reason":if complete {None} else {Some(reason)},
        "actions":actions, "final_digest":game.verification_digest().to_string()})
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 3);
    let request = read(&args[1]);
    let plan = read(request["plan"].as_str().unwrap());
    let out = Path::new(&args[2]);
    assert!(!out.exists(), "preserve earlier results");
    let data = CardData::load_from_str(
        &std::fs::read_to_string(plan["cards_json"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let decks: Vec<Vec<String>> = plan["decks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| serde_json::from_value(d["cards"].clone()).unwrap())
        .collect();
    admission::validate_pool(
        &decks,
        &data,
        Path::new(plan["cards_json"].as_str().unwrap())
            .parent()
            .unwrap(),
    )
    .unwrap();
    let registry = CardRegistry::from_cards(&data);
    let mut models: Vec<_> = plan["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            let model = BatchedPolicyValueEvaluator::load_for_profile(
                Path::new(m["path"].as_str().unwrap()),
                PROFILE,
            )
            .unwrap();
            model.require_terminal_value().unwrap();
            model
        })
        .collect();
    let started = Instant::now();
    let node = if args[0] == "collect" {
        &request
    } else {
        &request["node"]
    };
    let actor = node["actor"].as_u64().unwrap() as u8;
    let pair: Vec<Vec<String>> = node["deck_indices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|index| decks[index.as_u64().unwrap() as usize].clone())
        .collect();
    let cap = plan["decision_cap"].as_u64().unwrap() as usize;
    if args[0] == "collect" {
        let mut game = start(&data, &pair, request["game_seed"].as_u64().unwrap());
        let mut rng = rand::rngs::StdRng::seed_from_u64(request["sample_seed"].as_u64().unwrap());
        let mut actions = Vec::new();
        let mut chosen = None;
        let mut seen = 0;
        let mut noops = 0;
        for _ in 0..cap {
            if game.game_over || off_field_token(&game) {
                break;
            }
            if eligible(&game, actor) {
                seen += 1;
                if rng.gen_range(0..seen) == 0 {
                    chosen = Some(json!({"id":request["id"], "actor":actor,
                        "source_model":request["source_model"], "deck_indices":request["deck_indices"],
                        "archetype":request["archetype"], "game_seed":request["game_seed"],
                        "actions":actions, "position":describe(&game, actor)}));
                }
            }
            let player = decision_player(&game);
            let model = if player == actor {
                request["source_model"].as_u64().unwrap()
            } else {
                plan["collection_opponent"].as_u64().unwrap()
            } as usize;
            let action = policy(&game, &registry, &mut models[model]);
            let before = game.verification_digest();
            game.decode_action(action, player);
            actions.push(action);
            if game.verification_digest() == before {
                noops += 1;
            } else {
                noops = 0;
            }
            if noops >= 3 {
                break;
            }
        }
        let complete = game.game_over && !off_field_token(&game);
        // A partial source game is retained as a failure, never quietly sampled.
        write(
            out,
            &json!({"status":if complete {"complete"} else {"incomplete"},
            "node":if complete {chosen} else {None}, "opportunities":seen,
            "source_actions":actions,"winner":game.winner,"seconds":started.elapsed().as_secs_f64()}),
        );
    } else if args[0] == "probe" {
        let game = replay(&data, &pair, node);
        let original = game.verification_digest();
        let mask = build_action_mask(&game, actor);
        let mut roots = Vec::new();
        for own in plan["actors"].as_array().unwrap() {
            let own = own.as_u64().unwrap() as usize;
            let evaluation = evaluate(&game, &registry, &mut models[own]);
            let policy_action = argmax(&evaluation, &mask);
            let result = determinized_search_batched(
                &registry,
                &game,
                actor,
                &mut models[own],
                &config(request["search_seed"].as_u64().unwrap()),
                4,
            );
            assert_eq!(result.simulations_run, 320);
            assert_eq!(game.verification_digest(), original);
            roots.push(json!({"model":own,"policy_action":policy_action,
                "promote_probability":evaluation.policy[MOVE_FROM_BREEDING as usize],
                "wait_probability":evaluation.policy[PASS as usize],"policy_value":evaluation.value,
                "search_plan":result.best_plan(),"search_visits":result.root_visits,
                "search_q":result.root_q,"search_value":result.root_value,
                "simulations":result.simulations_run}));
        }
        let mut result = json!({"status":"running","node":node,"roots":roots,"branches":[]});
        write(out, &result);
        let search_actor = request["search_continuations"].as_bool().unwrap_or(false);
        for own in plan["actors"].as_array().unwrap() {
            for opponent in plan["opponents"].as_array().unwrap() {
                let own = own.as_u64().unwrap() as usize;
                let opponent = opponent.as_u64().unwrap() as usize;
                for action in [MOVE_FROM_BREEDING, PASS] {
                    let outcome = rollout(
                        &game,
                        &registry,
                        &mut models,
                        actor,
                        own,
                        opponent,
                        action,
                        cap,
                        search_actor,
                        request["search_seed"].as_u64().unwrap(),
                    );
                    assert_eq!(game.verification_digest(), original);
                    result["branches"].as_array_mut().unwrap().push(json!({"model":own,
                        "opponent":opponent,"first_action":action,"search_actor":search_actor,"outcome":outcome}));
                    result["seconds"] = json!(started.elapsed().as_secs_f64());
                    write(out, &result);
                }
            }
        }
        result["status"] = json!("complete");
        write(out, &result);
    } else {
        panic!("unknown mode");
    }
}
