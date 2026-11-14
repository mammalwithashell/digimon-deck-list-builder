//! Exact sourced EX13 Royal Knights lists, production metadata and legal masked play.
use digimon_engine::{action::space::CONCEDE_GAME, card_data::CardData, runners::headless::HeadlessRunner};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[test]
fn admitted_ex13_lists_construct_and_finish_both_seed_parities_without_conceding() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let load = |p: &str| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(root.join(p)).unwrap()).unwrap()
    };
    let report = load("qa/archetype-qa/dsl/royal-knights-ex13-training-gate/admission.json");
    let admission = load("data/card_admissibility.json");
    assert_eq!(load("data/mechanics_readiness.json")["ready"], true);
    let data = CardData::load_from_file(&root.join("data/cards.json")).unwrap();
    let mut admitted = 0;
    for row in report["lists"].as_array().unwrap() {
        if !row["after_blockers"].as_object().unwrap().is_empty() { continue; }
        admitted += 1;
        let deck: Vec<String> = serde_json::from_value(row["cards"].clone()).unwrap();
        for id in &deck {
            assert_eq!(admission["cards"][id]["admissible"], true, "{}: {id}", row["deck_id"]);
            assert!(data.contains_key(id));
        }
        for seed in 0..2 {
            let mut runner = HeadlessRunner::new(deck.clone(), deck.clone(), &data, false, false, false, Some(seed)).unwrap();
            let mut rng = StdRng::seed_from_u64(0x524b ^ seed);
            for _ in 0..4000 {
                if runner.is_game_over() { break; }
                let mask = runner.get_action_mask();
                let legal: Vec<u16> = mask.iter().enumerate()
                    .filter(|(i, value)| **value > 0.5 && *i != CONCEDE_GAME as usize)
                    .map(|(i, _)| i as u16).collect();
                assert!(!legal.is_empty(), "no non-concede action: {} seed {seed}", row["deck_id"]);
                runner.step(legal[rng.gen_range(0..legal.len())]);
            }
            assert!(runner.is_game_over(), "step cap: {} seed {seed}", row["deck_id"]);
        }
    }
    assert_eq!(admitted, 16);
}
