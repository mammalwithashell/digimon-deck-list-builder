//! Prepare a NEW prior through production validation. Never edit an old snapshot.
use digimon_engine::{card_data::CardData, determinize::beliefs::{PriorSnapshot, WeightedDeckPrior}};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path};
#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 3, "source_prior data_directory NEW_output");
    let out = Path::new(&args[2]);
    assert!(!out.exists(), "refusing to replace prior");
    let raw = std::fs::read(&args[0]).unwrap();
    // Deserialize old data only; it is never admitted directly as a runtime prior.
    let old: PriorSnapshot = serde_json::from_slice(&raw).unwrap();
    assert_eq!(old.format_id, "standard");
    assert_eq!(old.candidates.len(), 21);
    assert!(old.candidates.iter().all(|d| d.weight == 1.0));
    assert_eq!(old.unseen_mass, 0.1);
    assert_eq!(old.fallback_attempts, 128);
    let dir = Path::new(&args[1]);
    let data = CardData::load_from_str(&std::fs::read_to_string(dir.join("cards.json")).unwrap()).unwrap();
    admission::validate_population(&old.population.iter().cloned().collect::<Vec<_>>(), &data, dir).unwrap();
    let decks: Vec<_> = old.candidates.iter().map(|d| d.cards.clone()).collect();
    admission::validate_pool(&decks, &data, dir).unwrap();
    let source_hash = format!("{:x}", Sha256::digest(&raw));
    let provenance = serde_json::json!({"split":"train","source_prior_sha256":source_hash,
        "source_provenance":old.provenance,"preparation":"production PriorSnapshot::new; same candidate and fallback contents"}).to_string();
    let new = PriorSnapshot::new(&old.format_id, &provenance, old.population.clone(),
        old.candidates.clone(), old.unseen_mass, old.fallback_attempts).unwrap();
    assert_eq!(new.population, old.population);
    assert_eq!(serde_json::to_value(&new.candidates).unwrap(), serde_json::to_value(&old.candidates).unwrap());
    let prior = WeightedDeckPrior::new(new).unwrap();
    let bytes = serde_json::to_vec_pretty(prior.snapshot()).unwrap();
    std::fs::OpenOptions::new().write(true).create_new(true).open(out).unwrap().write_all(&bytes).unwrap();
    println!("{}", serde_json::json!({"admitted":true,"source_prior_sha256":source_hash,
        "prior_sha256":format!("{:x}",Sha256::digest(&bytes)),"prior_snapshot_hash":prior.snapshot_hash(),
        "source_schema":old.schema_version,"new_schema":prior.snapshot().schema_version,
        "source_legality_hash":old.legality_hash,"new_legality_hash":prior.snapshot().legality_hash,
        "candidates":21,"population":prior.snapshot().population.len(),"contents_preserved":true,
        "searches":0,"games":0,"full_search_qualification":false}));
}
