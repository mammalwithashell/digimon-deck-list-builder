//! Abstract engine tactics; these are not card/archetype certification.
use super::*;
use digimon_engine::{
    action::{encode_attack, SECURITY_TARGET},
    debug_runner::{make_test_card, make_test_egg, DebugRunner},
    enums::{Expiry, GamePhase, Keyword},
};

struct Fixture {
    name: &'static str,
    game: Game,
    expected: Vec<u16>,
    minimum_value: Option<f32>,
}
fn fixture(kind: usize) -> Fixture {
    let mut cards = HashMap::new();
    for (id, dp) in [
        ("Q-ATK", 5000),
        ("Q-DEF", 3000),
        ("Q-SMALL", 1000),
        ("Q-BIG", 15000),
    ] {
        let mut card = make_test_card(id, id);
        card.dp = Some(dp);
        cards.insert(id.to_string(), card);
    }
    for i in 0..5 {
        let id = format!("Q-PLAY{i}");
        cards.insert(id.clone(), make_test_card(&id, &id));
    }
    cards.insert("Q-EGG".into(), make_test_egg("Q-EGG", "Q-EGG"));
    let mut builder = DebugRunner::builder()
        .with_card_data(cards)
        .deck(0, &["Q-SMALL", "Q-SMALL", "Q-SMALL"])
        .deck(1, &["Q-SMALL", "Q-SMALL", "Q-SMALL"]);
    if kind == 4 {
        builder = builder.digitama(0, &["Q-EGG"]).digitama(1, &["Q-EGG"]);
    }
    let mut r = builder.start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    r.place_on_field(tp, "Q-ATK", Some(0));
    r.place_on_field(opp, "Q-DEF", Some(0));
    r.game.enter_main_phase();
    let mut expected = vec![encode_attack(0, SECURITY_TARGET)];
    let mut minimum_value = None;
    let name = match kind {
        0 => "immediate_lethal_over_losing_pass",
        1 => {
            for i in 0..5 {
                r.add_to_hand(tp, &format!("Q-PLAY{i}"));
            }
            "lethal_with_five_play_distractions"
        }
        2 => {
            r.place_on_field(tp, "Q-ATK", Some(0));
            // One known weak security card; two attacks win this turn.
            r.game
                .stage_inject_card(opp, "Q-SMALL", "security_top")
                .unwrap();
            expected.push(encode_attack(1, SECURITY_TARGET));
            "two_attack_lethal"
        }
        3 => {
            r.game.players[opp as usize].battle_area[0].is_suspended = true;
            // Attacking a strong suspended target is a legal losing distraction.
            let big = r.place_on_field(opp, "Q-BIG", Some(0));
            r.game.players[opp as usize].battle_area[big.index as usize].is_suspended = true;
            "player_lethal_over_unnecessary_board_trade"
        }
        4 => {
            r.set_phase(GamePhase::Breeding);
            expected = legal(&build_action_mask(&r.game, tp));
            minimum_value = Some(0.0);
            "same_player_breeding_to_lethal_backup"
        }
        5 => {
            // Defending player must block an otherwise terminal attack.
            let blocker = r.place_on_field(opp, "Q-BIG", Some(0));
            r.game
                .modifiers
                .grant_keyword(blocker, Keyword::Blocker, Expiry::Permanent, opp);
            r.game.decode_action(encode_attack(0, SECURITY_TARGET), tp);
            assert_eq!(decision_player(&r.game), opp);
            let valid = legal(&build_action_mask(&r.game, opp));
            assert!(valid.len() > 1);
            expected = valid
                .into_iter()
                .filter(|&a| {
                    let mut g = r.game.clone();
                    g.decode_action(a, opp);
                    !(g.game_over && g.winner == Some(tp))
                })
                .collect();
            assert!(
                !expected.is_empty()
                    && expected.len() < legal(&build_action_mask(&r.game, opp)).len()
            );
            "block_to_avoid_immediate_loss"
        }
        _ => unreachable!(),
    };
    let pid = decision_player(&r.game);
    let mask = build_action_mask(&r.game, pid);
    assert!(expected.iter().all(|&a| mask[a as usize] > 0.5));
    Fixture {
        name,
        game: r.game,
        expected,
        minimum_value,
    }
}
pub fn run(registry: &CardRegistry, evaluator: &mut dyn PolicyValueFn, plan: &Value) -> Value {
    let mut rows = Vec::new();
    for kind in 0..6 {
        let f = fixture(kind);
        let pid = decision_player(&f.game);
        for settings in plan["grid"].as_array().unwrap() {
            for seed in plan["tactical_seeds"].as_array().unwrap() {
                let c = config(settings, seed.as_u64().unwrap(), 0.);
                for guidance in ["uniform", "frozen_model"] {
                    let before = Instant::now();
                    let result = if guidance == "uniform" {
                        determinized_search_batched(
                            registry,
                            &f.game,
                            pid,
                            &mut digimon_engine::search::UniformEvaluator,
                            &c,
                            4,
                        )
                    } else {
                        determinized_search_batched(registry, &f.game, pid, evaluator, &c, 4)
                    };
                    let chosen = choose(&result);
                    let passed = f.expected.contains(&chosen)
                        && f.minimum_value
                            .map(|v| result.root_value > v)
                            .unwrap_or(true);
                    rows.push(json!({"fixture":f.name,"config":settings,"seed":seed,"guidance":guidance,
                        "expected":f.expected,"action":chosen,"value":result.root_value,"passed":passed,
                        "seconds":before.elapsed().as_secs_f64()}));
                }
            }
        }
    }
    json!({"scope":"six abstract engine tactics; synthetic IDs map to unknown embeddings under the frozen production registry; not archetype certification",
        "rows":rows})
}
