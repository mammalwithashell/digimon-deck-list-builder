//! Offline reconstruction/intervention audit. No inference, search or full games.
#[path = "search_quality/mulligan_continuation_root.rs"]
mod root;
#[path = "../../digimon-engine-cli/src/admission.rs"]
mod admission;
use digimon_engine::{card_data::CardData, card_registry::CardRegistry};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::Path, fs};

fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 4, "roots.json deck-plan.json cards.json output.json");
    assert!(!Path::new(&args[3]).exists(), "preserve prior output");
    let root_bytes = fs::read(&args[0]).unwrap();
    let plan_bytes = fs::read(&args[1]).unwrap();
    let card_bytes = fs::read(&args[2]).unwrap();
    let nodes: Vec<root::SavedRoot> = serde_json::from_slice(&root_bytes).unwrap();
    assert_eq!(nodes.len(), 52, "preregistered root count");
    let plan: Value = serde_json::from_slice(&plan_bytes).unwrap();
    let decks: Vec<Vec<String>> = plan["decks"].as_array().unwrap().iter()
        .map(|d| serde_json::from_value(d["cards"].clone()).unwrap()).collect();
    let data = CardData::load_from_str(std::str::from_utf8(&card_bytes).unwrap()).unwrap();
    admission::validate_pool(&decks, &data, Path::new(&args[2]).parent().unwrap()).unwrap();
    let registry = CardRegistry::from_cards(&data);
    let mut rows = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for node in &nodes {
        assert!(ids.insert(&node.id), "duplicate saved root");
        let game = root::reconstruct(node, &decks, &data, &registry).unwrap();
        let before = game.verification_digest();
        for seed in [51, 52] {
            for branch in [0, 1] {
                let fork = root::intervene(&game, &registry, node, branch, seed).unwrap();
                assert_eq!(game.verification_digest(), before);
                rows.push(json!({"root":node.id,"actor":node.mover,"branch":branch,
                    "test_outcome_seed":seed,"before":before,"after":fork.verification_digest(),
                    "observation_sha256":node.observation_sha256}));
            }
        }
    }
    let output = json!({"schema":"mulligan-continuation-root-audit-v1", "passed":true,
        "roots":nodes.len(),"interventions":rows.len(),"searches":0,"completed_games":0,
        "roots_sha256":hash(&root_bytes),"deck_plan_sha256":hash(&plan_bytes),
        "cards_sha256":hash(&card_bytes),"rows":rows});
    // create_new prevents replacing a previous audit, including failed attempts.
    use std::io::Write;
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&args[3]).unwrap();
    file.write_all(&serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    println!("52 roots and 208 forced branch checks verified; zero searches or completed games");
}
