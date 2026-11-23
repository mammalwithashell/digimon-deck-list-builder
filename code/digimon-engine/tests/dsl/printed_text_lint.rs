use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn production_printed_text_lint_baseline() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let official: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("../../data/card_official.json")).unwrap(),
    )
    .unwrap();
    let mut actual = BTreeSet::new();
    let files = digimon_dsl::authoring_lint::pool_card_files(&root.join("cards"));
    assert!(files.len() > 1000, "production pool must be found");
    for file in files {
        let spec = digimon_dsl::loader::load_file(&file)
            .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let sections = official["cards"][&spec.card]["text_sections"]
            .as_array()
            .map(|sections| {
                sections
                    .iter()
                    .map(|s| digimon_dsl::security_icon_lint::PrintedSection {
                        label: s["label"].as_str().unwrap_or_default().into(),
                        text: s["text"].as_str().unwrap_or_default().into(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for finding in digimon_dsl::printed_text_lint::check_printed_text(&spec, &sections) {
            actual.insert(format!(
                "{} {} {}",
                spec.card, finding.kind, finding.clause_index
            ));
        }
    }
    let path = root.join("tests/dsl/printed_text_lint_baseline.txt");
    if std::env::var("PRINTED_TEXT_LINT_BLESS").as_deref() == Ok("1") {
        std::fs::write(
            &path,
            actual.iter().cloned().collect::<Vec<_>>().join("\n") + "\n",
        )
        .unwrap();
    }
    let baseline: BTreeSet<_> = std::fs::read_to_string(path)
        .expect("read printed-text baseline")
        .lines()
        .map(str::to_owned)
        .collect();
    let new: Vec<_> = actual.difference(&baseline).collect();
    let stale: Vec<_> = baseline.difference(&actual).collect();
    assert!(
        new.is_empty() && stale.is_empty(),
        "new findings: {new:#?}\nstale baseline entries: {stale:#?}"
    );
}
