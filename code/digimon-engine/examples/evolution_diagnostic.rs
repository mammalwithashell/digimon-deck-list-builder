//! Frozen-model research on abstract, bounded-minimax-checked evolution tactics.
//! Synthetic IDs use unknown production embeddings: this is not meta-card QA.
use digimon_engine::{
    action::build_action_mask,
    card_data::{CardData, DnaCost, DnaRequirement, EvoCost},
    card_registry::CardRegistry,
    debug_runner::{make_test_card, make_test_egg, DebugRunner},
    enums::{CardColor, GamePhase, Keyword},
    game::Game,
    inference::BatchedPolicyValueEvaluator,
    observation::{build_observation_tensor, ObservationProfileId},
    search::{
        decision_player, determinized_search_batched, DeterminizedConfig, EvalRequest,
        PolicyValueFn, UniformEvaluator,
    },
};
use serde_json::{json, Value};
use std::{path::Path, time::Instant};

fn legal(g: &Game) -> Vec<u16> {
    build_action_mask(g, decision_player(g))
        .iter()
        .enumerate()
        .filter_map(|(i, &x)| (x > 0.5).then_some(i as u16))
        .collect()
}

fn fixture(kind: usize) -> (&'static str, Game, u16) {
    let mut red = make_test_card("EVO-RED", "Red material");
    red.level = Some(4);
    let mut blue = red.clone();
    blue.card_id = "EVO-BLUE".into();
    blue.card_name = "Blue material".into();
    blue.colors = vec![CardColor::Blue];
    let mut rush = make_test_card("EVO-RUSH", "Rush evolution");
    rush.level = Some(5);
    rush.dp = Some(6000);
    rush.play_cost = 9;
    rush.keywords = vec![Keyword::Rush];
    rush.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 1,
    }];
    let mut dna = make_test_card("EVO-DNA", "DNA evolution");
    dna.level = Some(5);
    dna.dp = Some(8000);
    dna.play_cost = 9;
    let req = |color| DnaRequirement {
        level: 4,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    };
    dna.dna_costs = vec![DnaCost {
        requirement1: req(CardColor::Red),
        requirement2: req(CardColor::Blue),
        memory_cost: 0,
    }];
    let mut rookie = make_test_card("EVO-ROOKIE", "Raising rookie");
    rookie.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 2,
        memory_cost: 0,
    }];
    let mut r = DebugRunner::builder()
        .add_card(red)
        .add_card(blue)
        .add_card(rush)
        .add_card(dna)
        .add_card(rookie)
        .add_card(make_test_egg("EVO-EGG", "Egg"))
        .add_card(make_test_card("EVO-FILL", "Filler"))
        .deck(0, &["EVO-FILL"; 8])
        .deck(1, &["EVO-FILL"; 8])
        .memory(3)
        .start();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    let (name, expected) = match kind {
        0 => {
            r.place_on_field(0, "EVO-RED", None);
            r.place_on_field(1, "EVO-FILL", Some(0));
            r.add_to_hand(0, "EVO-RUSH");
            ("battle_evolve_into_rush_for_lethal", 400)
        }
        1 => {
            r.place_on_field(0, "EVO-RED", None);
            r.place_on_field(0, "EVO-BLUE", None);
            r.place_on_field(1, "EVO-FILL", Some(0));
            r.add_to_hand(0, "EVO-DNA");
            ("dna_fresh_materials_for_lethal", 63)
        }
        2 => {
            r.place_in_breeding(0, "EVO-ROOKIE");
            r.place_on_field(1, "EVO-FILL", Some(0));
            r.set_phase(GamePhase::Breeding);
            ("promote_for_lethal", 61)
        }
        3 => {
            r.place_in_breeding(0, "EVO-EGG");
            r.add_to_hand(0, "EVO-ROOKIE");
            // No present attacker: development can win after passing the turn.
            r.add_to_hand(1, "EVO-FILL");
            ("raising_evolution_then_next_turn_promotion", 414)
        }
        _ => unreachable!(),
    };
    // DebugRunner stages zones outside the normal deal. Declare their exact
    // card multisets so debug PIMC conservation checks remain enabled.
    for pid in 0..2 {
        let counts = if pid == 0 {
            vec![
                ("EVO-FILL", 8),
                ("EVO-RED", u16::from(kind < 2)),
                ("EVO-BLUE", u16::from(kind == 1)),
                ("EVO-RUSH", u16::from(kind == 0)),
                ("EVO-DNA", u16::from(kind == 1)),
                ("EVO-ROOKIE", u16::from(kind >= 2)),
                ("EVO-EGG", u16::from(kind == 3)),
            ]
        } else {
            vec![("EVO-FILL", 9)]
        };
        r.game.players[pid].original_deck = counts
            .into_iter()
            .filter(|(_, n)| *n > 0)
            .map(
                |(id, count)| digimon_engine::player::OriginalDeckCardCount {
                    card_id: id.into(),
                    count,
                    is_digitama: id == "EVO-EGG",
                },
            )
            .collect();
    }
    assert_eq!(decision_player(&r.game), 0);
    assert!(
        legal(&r.game).contains(&expected),
        "{name}: expected action illegal"
    );
    (name, r.game, expected)
}

// Exact outcomes within a finite horizon; unknown is never treated as a draw.
// The adversary minimizes player zero's result. Short-circuit proven wins/losses.
fn solve(g: &Game, depth: u32, budget: &mut u32) -> Option<i8> {
    if g.game_over {
        return Some(match g.winner {
            Some(0) => 1,
            Some(_) => -1,
            None => 0,
        });
    }
    if depth == 0 || *budget == 0 {
        return None;
    }
    *budget -= 1;
    let maximize = decision_player(g) == 0;
    let mut best = if maximize { -1 } else { 1 };
    let mut unknown = false;
    // Attacks first avoid spending the bounded proof budget on avoidable passes.
    let mut actions = legal(g);
    actions.sort_by_key(|a| {
        if (100..400).contains(a) {
            0
        } else if *a == 62 {
            2
        } else {
            1
        }
    });
    assert!(!actions.is_empty());
    for action in actions {
        let mut child = g.clone();
        child.decode_action(action, decision_player(g));
        match solve(&child, depth - 1, budget) {
            Some(value) => {
                best = if maximize {
                    best.max(value)
                } else {
                    best.min(value)
                };
                if (maximize && best == 1) || (!maximize && best == -1) {
                    return Some(best);
                }
            }
            None => unknown = true,
        }
    }
    (!unknown).then_some(best)
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 4, "cards.json model-directory output.json");
    let out = Path::new(&args[3]);
    assert!(!out.exists());
    let cards = CardData::load_from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let registry = CardRegistry::from_cards(&cards);
    let profile = ObservationProfileId::StandardLiteV2;
    let mut models: Vec<_> = (0..3)
        .map(|i| {
            let path = Path::new(&args[2]).join(format!("gen{i}/model.onnx"));
            let model = BatchedPolicyValueEvaluator::load_for_profile(&path, profile).unwrap();
            model.require_terminal_value().unwrap();
            model
        })
        .collect();
    let mut results = Vec::new();
    for kind in 0..4 {
        let (name, game, expected) = fixture(kind);
        let mut proof = Vec::new();
        for a in legal(&game) {
            let mut child = game.clone();
            child.decode_action(a, 0);
            let mut budget = 100_000;
            let value = solve(&child, 14, &mut budget);
            proof.push(json!({"action":a,"value":value,"nodes":100_000-budget}));
        }
        assert_eq!(
            proof.iter().find(|p| p["action"] == expected).unwrap()["value"],
            1,
            "fixture expected action must have a proven win: {name} {proof:?}"
        );
        assert!(proof.iter().all(|p| !p["value"].is_null()));
        let winning_actions: Vec<u16> = proof
            .iter()
            .filter(|p| p["value"] == 1)
            .map(|p| p["action"].as_u64().unwrap() as u16)
            .collect();
        if kind < 3 {
            assert_eq!(winning_actions, vec![expected], "unique winning tactic");
        } else {
            // This control must not grade an equally winning hard-play as wrong.
            assert_eq!(winning_actions, vec![0, expected]);
        }
        let mask = build_action_mask(&game, 0);
        let obs = build_observation_tensor(&game, 0, &registry, profile);
        for guidance in 0..4 {
            let mut uniform = UniformEvaluator;
            let evaluator: &mut dyn PolicyValueFn = if guidance == 3 {
                &mut uniform
            } else {
                &mut models[guidance]
            };
            let direct = evaluator.evaluate_batch(&[EvalRequest {
                obs: obs.clone(),
                mask: mask.clone(),
            }]);
            let chosen = legal(&game)
                .into_iter()
                .max_by(|a, b| {
                    direct[0].policy[*a as usize]
                        .total_cmp(&direct[0].policy[*b as usize])
                        .then_with(|| b.cmp(a))
                })
                .unwrap();
            for sims in [320, 1280] {
                for seed in [77101, 77102, 77103] {
                    let mut cfg = DeterminizedConfig::default();
                    cfg.worlds = 4;
                    cfg.search.simulations = sims;
                    cfg.search.seed = seed;
                    cfg.search.dirichlet_epsilon = 0.0;
                    cfg.search.observation_profile = profile;
                    let start = Instant::now();
                    let search =
                        determinized_search_batched(&registry, &game, 0, evaluator, &cfg, 4);
                    let action = search
                        .root_visits
                        .iter()
                        .max_by(|(aa, av), (ba, bv)| av.cmp(bv).then_with(|| ba.cmp(aa)))
                        .unwrap()
                        .0;
                    let row = json!({"fixture":name,"expected":expected,"proof":proof,"guidance":guidance,
                        "winning_actions":winning_actions,
                        "search_proven_win":winning_actions.contains(&action),
                        "direct_proven_win":winning_actions.contains(&chosen),
                        "direct_action":chosen,"direct_value":direct[0].value,
                        "direct_expected_probability":direct[0].policy[expected as usize],
                        "simulations":sims,"worlds":4,"batch":4,"seed":seed,
                        "search_action":action,"search_value":search.root_value,
                        "root_visits":search.root_visits,"seconds":start.elapsed().as_secs_f64()});
                    println!("{name} guidance={guidance} sims={sims} seed={seed}: direct={chosen} search={action} expected={expected}");
                    results.push(row);
                }
            }
        }
    }
    let report: Value = json!({"scope":"abstract engine fixtures; synthetic unknown embeddings; no card certification; finite-horizon perfect-information proof; PIMC search with hidden arrangement sampling",
        "guidance":"0/1/2 = frozen campaign generations; 3 = uniform policy, zero value",
        "rows":results});
    std::fs::write(out, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
