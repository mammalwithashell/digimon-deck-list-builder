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

#[test]
fn group7_migrated_cards_do_not_use_raw_rust_or_empty_placeholders() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let migrated_cards = [(
        "BT8-097",
        manifest_dir.join("cards").join("bt8").join("BT8-097.yaml"),
    )];

    for (card_id, path) in migrated_cards {
        let yaml = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read migrated YAML for {card_id} at {path:?}: {err}"));
        assert!(
            !yaml.contains("raw_rust"),
            "{card_id} should remain fully native DSL after Group 7 migration"
        );
        assert!(
            !yaml.contains("process: []"),
            "{card_id} should not regress to an empty process placeholder"
        );
    }
}
