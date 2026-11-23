use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Edit {
    path: String,
    before: PathBuf,
    after: PathBuf,
}

fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest = std::env::args_os().nth(1).ok_or("expected edit manifest")?;
    let edits: Vec<Edit> = serde_json::from_slice(&std::fs::read(manifest)?)?;
    for edit in &edits {
        let before = digimon_dsl::loader::load_file(&edit.before)?;
        let after = digimon_dsl::loader::load_file(&edit.after)?;
        let original = digimon_dsl::compile::compile(&before)
            .map_err(|e| format!("{} before: {e:?}", edit.path))?;
        let updated = digimon_dsl::compile::compile(&after)
            .map_err(|e| format!("{} after: {e:?}", edit.path))?;
        if original != updated {
            return Err(format!("{}: compiled output changed", edit.path).into());
        }
    }
    println!("compile-equivalence: {} files loaded before/after; {} identical compiled cards; 0 differences", edits.len(), edits.len());
    Ok(())
}

fn main() {
    let result = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(run)
        .expect("spawn verifier")
        .join()
        .expect("verifier panicked");
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
