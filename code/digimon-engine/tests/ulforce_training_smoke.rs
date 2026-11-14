//! Exact exported 50+4 list: canonical gate, production metadata, and masked games.
use digimon_engine::{
    action::space::CONCEDE_GAME, card_data::CardData, runners::headless::HeadlessRunner,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
fn run_seed(seed: u64) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let load = |p: &str| -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(root.join(p)).unwrap()).unwrap()
    };
    let fixture = load("qa/archetype-qa/dsl/ulforce-exact-deck-2026-10-19.json");
    let admission = load("data/card_admissibility.json");
    assert_eq!(load("data/mechanics_readiness.json")["ready"], true);
    let data = CardData::load_from_file(&root.join("data/cards.json")).unwrap();
    let mut deck = Vec::new();
    assert_eq!(fixture["counts"].as_object().unwrap().len(), 19);
    for (id, count) in fixture["counts"].as_object().unwrap() {
        assert_eq!(admission["cards"][id]["admissible"], true, "{id}");
        assert!(data.contains_key(id));
        deck.extend(std::iter::repeat_n(
            id.clone(),
            count.as_u64().unwrap() as usize,
        ));
    }
    let library = load("data/deck_library.json");
    let entry = library["archetypes"]["Ulforce"]["decklists"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["deck_id"] == "dcg_nexus_b760a865ce88")
        .expect("original sourced EX13 list remains in the library");
    let mut library_deck: Vec<String> =
        serde_json::from_str(entry["decklist"].as_str().unwrap()).unwrap();
    library_deck.sort();
    deck.sort();
    assert_eq!(deck, library_deck, "qualify the exact original list");
    assert_eq!(deck.len(), 54);
    assert_eq!(
        deck.iter().filter(|id| id.as_str() == "EX13-002").count(),
        4
    );
    eprintln!("Ulforce smoke seed {seed}: constructing");
    let mut runner = HeadlessRunner::new(
        deck.clone(),
        deck.clone(),
        &data,
        false,
        false,
        false,
        Some(seed),
    )
    .unwrap();
    let mut rng = StdRng::seed_from_u64(0x554c46 ^ seed);
    let mut steps = 0;
    for step in 0..4000 {
        if runner.is_game_over() {
            break;
        }
        if step % 100 == 0 {
            eprintln!("Ulforce smoke seed {seed}: step {step}");
        }
        steps = step + 1;
        let mask = runner.get_action_mask();
        let legal: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter(|(i, value)| **value > 0.5 && *i != CONCEDE_GAME as usize)
            .map(|(i, _)| i as u16)
            .collect();
        assert!(!legal.is_empty(), "no non-concede action: seed {seed}");
        runner.step(legal[rng.gen_range(0..legal.len())]);
    }
    assert!(runner.is_game_over(), "step cap: seed {seed}");
    eprintln!("Ulforce smoke seed {seed}: finished after {steps} steps");
}

// Each independent game rechecks exact identity and both canonical gate layers.
#[test]
fn exact_list_seed_0() {
    run_seed(0);
}

#[test]
fn exact_list_seed_1() {
    run_seed(1);
}

#[test]
fn exact_list_seed_2() {
    run_seed(2);
}

#[test]
fn exact_list_seed_3() {
    run_seed(3);
}

#[test]
fn exact_list_seed_4() {
    run_seed(4);
}

#[test]
fn exact_list_seed_5() {
    run_seed(5);
}

#[test]
fn exact_list_seed_6() {
    run_seed(6);
}

#[test]
fn exact_list_seed_7() {
    run_seed(7);
}
