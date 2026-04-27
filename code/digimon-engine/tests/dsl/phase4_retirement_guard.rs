use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED_DIRS: &[&str] = &["raw_rust", "test", "tokens"];
const ALLOWED_FILES: &[&str] = &["keyword_effects.rs", "mod.rs"];

fn collect_unretired_entries(src_cards: &Path) -> Vec<PathBuf> {
    let mut offenders = Vec::new();
    for entry in fs::read_dir(src_cards).expect("read src/cards") {
        let path = entry.expect("read src/cards entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("utf-8 src/cards entry");

        if path.is_dir() {
            if !ALLOWED_DIRS.contains(&name) {
                offenders.push(path);
            }
        } else if !ALLOWED_FILES.contains(&name) {
            offenders.push(path);
        }
    }
    offenders.sort();
    offenders
}

#[test]
fn src_cards_contains_only_test_tokens_keyword_and_raw_rust_shells() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_cards = manifest_dir.join("src").join("cards");

    let offenders = collect_unretired_entries(&src_cards);
    assert!(
        offenders.is_empty(),
        "production hand-written card modules must migrate to DSL YAML or cards/raw_rust:\n{}",
        offenders
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}
