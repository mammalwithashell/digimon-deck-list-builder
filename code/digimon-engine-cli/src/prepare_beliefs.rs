use digimon_engine::{
    card_data::CardData,
    determinize::beliefs::{CandidateDeck, PriorSnapshot},
};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    process::ExitCode,
};

pub fn run(
    decks: PathBuf,
    out: PathBuf,
    format_id: String,
    provenance: String,
    data: HashMap<String, CardData>,
    data_dir: PathBuf,
) -> ExitCode {
    let result = (|| -> Result<(), String> {
        let (lists, _) = crate::selfplay::load_deck_pool(&decks)?;
        let admission: serde_json::Value = serde_json::from_slice(
            &std::fs::read(data_dir.join("card_admissibility.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let registered: BTreeSet<_> = digimon_engine::cards::build_registry_cached()
            .registered_card_ids()
            .into_iter()
            .collect();
        let population: BTreeSet<_> = data
            .keys()
            .filter(|id| registered.contains(*id) && admission["cards"][*id]["admissible"] == true)
            .cloned()
            .collect();
        crate::admission::validate_population(&population.iter().cloned().collect::<Vec<_>>(), &data, &data_dir)?;
        let mut accepted = Vec::new();
        let mut excluded = Vec::new();
        for (i, cards) in lists.iter().enumerate() {
            let legal = digimon_engine::deck_tools::validate_deck_for_game_mode(cards, &format_id)?;
            if legal.is_valid && cards.iter().all(|id| population.contains(id)) {
                accepted.push(CandidateDeck {
                    cards: cards.clone(),
                    weight: 1.0,
                });
            } else {
                excluded.push(serde_json::json!({"index":i,"legality_errors":legal.errors,"outside_population":cards.iter().filter(|id|!population.contains(*id)).collect::<Vec<_>>()}));
            }
        }
        if accepted.is_empty() {
            return Err("no legal admitted candidate lists".into());
        }
        let snapshot = PriorSnapshot::new(&format_id, &provenance, population, accepted, 0.1, 128)?;
        if out.exists() {
            return Err("refusing to replace a frozen prior snapshot".into());
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&out, serde_json::to_vec_pretty(&snapshot).unwrap())
            .map_err(|e| e.to_string())?;
        let report = serde_json::json!({"input_lists":lists.len(),"accepted_unique":snapshot.candidates.len(),"excluded":excluded});
        std::fs::write(
            out.with_extension("admission.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("prior preparation failed: {e}");
            ExitCode::from(1)
        }
    }
}
