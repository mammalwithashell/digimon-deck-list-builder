//! Admission-only probe: no model loading, search, or games.
use digimon_engine::{card_data::CardData, determinize::beliefs::{PriorSnapshot, WeightedDeckPrior}};
use std::path::Path;
#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "prior.json data_directory");
    let data_dir = Path::new(&args[1]);
    let data = CardData::load_from_str(&std::fs::read_to_string(data_dir.join("cards.json")).unwrap()).unwrap();
    let snapshot: PriorSnapshot = serde_json::from_slice(&std::fs::read(&args[0]).unwrap()).unwrap();
    let prior = WeightedDeckPrior::new(snapshot).unwrap();
    admission::validate_population(&prior.snapshot().population.iter().cloned().collect::<Vec<_>>(), &data, data_dir).unwrap();
    let decks: Vec<_> = prior.snapshot().candidates.iter().map(|d| d.cards.clone()).collect();
    admission::validate_pool(&decks, &data, data_dir).unwrap();
    assert_eq!(decks.len(), 21);
    assert!(prior.snapshot().candidates.iter().all(|d| d.weight == 1.0));
    println!("{}", serde_json::json!({"admitted":true,"prior_snapshot_hash":prior.snapshot_hash(),
        "candidates":decks.len(),"population":prior.snapshot().population.len(),
        "searches":0,"games":0,"full_search_qualification":false}));
}
