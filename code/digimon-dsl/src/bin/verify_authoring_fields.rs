use std::collections::BTreeMap;
use std::path::PathBuf;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 || !matches!(args[1].as_str(), "snapshot" | "verify") {
        return Err(
            "usage: verify_authoring_fields <snapshot|verify> <cards-root> <snapshot.json>".into(),
        );
    }
    let root = PathBuf::from(&args[2]);
    let mut cards = BTreeMap::new();
    for path in digimon_dsl::authoring_lint::pool_card_files(&root) {
        let spec = digimon_dsl::loader::load_file(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let card = digimon_dsl::compile::compile(&spec)
            .map_err(|e| format!("{}: {e:?}", path.display()))?;
        cards.insert(
            path.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            serde_json::to_value(card).map_err(|e| e.to_string())?,
        );
    }
    if cards.is_empty() {
        return Err("no production cards found".into());
    }
    if args[1] == "snapshot" {
        std::fs::write(
            &args[3],
            serde_json::to_vec(&cards).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!(
            "compile snapshot: {} files loaded and compiled",
            cards.len()
        );
    } else {
        let before: BTreeMap<String, serde_json::Value> =
            serde_json::from_slice(&std::fs::read(&args[3]).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let differences: Vec<_> = before
            .keys()
            .chain(cards.keys())
            .filter(|key| before.get(*key) != cards.get(*key))
            .collect();
        if !differences.is_empty() {
            return Err(format!("compiled output differs: {differences:?}"));
        }
        println!("compile-equivalence: {} files loaded before/after; {} identical compiled cards; 0 differences", cards.len(), cards.len());
    }
    Ok(())
}

fn main() {
    let result = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
