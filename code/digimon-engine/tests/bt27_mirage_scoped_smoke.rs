//! Exact Bandai composition, with non-production metadata fixtures for new IDs.
//! This exercises runtime choices only. It is NOT production training admission.
#[path = "support/dsl_card_data.rs"]
mod dsl_card_data;
use digimon_engine::{
    action::space::CONCEDE_GAME, card_data::CardData, runners::headless::HeadlessRunner,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
fn run(seed: u64) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("qa/archetype-qa/dsl/bt27-mirage-official-sample.json"))
            .unwrap(),
    )
    .unwrap();
    let mut data = CardData::load_from_file(&root.join("data/cards.json")).unwrap();
    let registry: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join("data/card_registry.json")).unwrap()).unwrap();
    let capacity = registry["capacity"].as_u64().unwrap() as f32;
    let mut next = data.values().map(|c| c.index).max().unwrap() + 1;
    let mut deck = Vec::new();
    for (id, count) in fixture["counts"].as_object().unwrap() {
        if !data.contains_key(id) {
            assert!(
                id.starts_with("BT27-"),
                "unexpected missing prerequisite: {id}"
            );
            // Isolated test fixture only; never written to cards.json or the registry.
            let mut c = dsl_card_data::card_data_from_compiled(id);
            c.index = next;
            c.norm_id = next as f32 / capacity;
            next += 1;
            data.insert(id.clone(), c);
        }
        deck.extend(std::iter::repeat_n(
            id.clone(),
            count.as_u64().unwrap() as usize,
        ));
    }
    assert_eq!(deck.len(), 54);
    let mut r =
        HeadlessRunner::new(deck.clone(), deck, &data, false, false, false, Some(seed)).unwrap();
    let mut rng = StdRng::seed_from_u64(seed ^ 0x42543237);
    let mut steps = 0;
    for _ in 0..4000 {
        if r.is_game_over() {
            break;
        }
        let mask = r.get_action_mask();
        let legal: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter(|(i, x)| **x > 0.5 && *i != CONCEDE_GAME as usize)
            .map(|(i, _)| i as u16)
            .collect();
        assert!(
            !legal.is_empty(),
            "no non-concede action at seed{seed}, step{steps}"
        );
        r.step(legal[rng.gen_range(0..legal.len())]);
        steps += 1;
    }
    assert!(r.is_game_over(), "step cap at seed{seed}");
    eprintln!(
        "NON-PRODUCTION metadata: exact Mirage sample seed{seed}, finished {steps} masked actions"
    );
}
#[test]
fn scoped_exact_sample_seed0() {
    run(0)
}
#[test]
fn scoped_exact_sample_seed1() {
    run(1)
}
#[test]
fn scoped_exact_sample_seed2() {
    run(2)
}
