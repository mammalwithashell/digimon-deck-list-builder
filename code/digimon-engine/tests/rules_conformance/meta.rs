//! Meta-tests: the `cite!` macro, the Rust/Python normaliser parity, and the
//! structural guarantees on the chapter modules.

use crate::cite::*;

/// Chapter module sources scanned by the meta-tests below, as
/// `(file name, include_str!(file))`.
///
/// **Chapter tasks MUST append their file here** (e.g.
/// `("ch15_effects.rs", include_str!("ch15_effects.rs"))`). A forgotten entry
/// fails `chapter_source_list_is_complete`, which compares this list with the
/// `*.rs` files actually present in the directory.
const CHAPTER_SOURCES: &[(&str, &str)] = &[
    ("ch03_areas.rs", include_str!("ch03_areas.rs")),
    (
        "ch15_04_effect_states.rs",
        include_str!("ch15_04_effect_states.rs"),
    ),
    (
        "ch15_07_optional_conditions.rs",
        include_str!("ch15_07_optional_conditions.rs"),
    ),
    (
        "ch15_08_effect_categories.rs",
        include_str!("ch15_08_effect_categories.rs"),
    ),
    (
        "ch15_14_effect_icons.rs",
        include_str!("ch15_14_effect_icons.rs"),
    ),
    (
        "ch15_15_unaffected.rs",
        include_str!("ch15_15_unaffected.rs"),
    ),
    ("ch17_rule_checks.rs", include_str!("ch17_rule_checks.rs")),
    (
        "ch18_pending_overwrite.rs",
        include_str!("ch18_pending_overwrite.rs"),
    ),
    (
        "ch11_14_attack_procedure.rs",
        include_str!("ch11_14_attack_procedure.rs"),
    ),
    (
        "ch16_new_keywords.rs",
        include_str!("ch16_new_keywords.rs"),
    ),
    (
        "ch16_replacement_keywords.rs",
        include_str!("ch16_replacement_keywords.rs"),
    ),
    (
        "ch16_attack_keywords.rs",
        include_str!("ch16_attack_keywords.rs"),
    ),
    (
        "ch16_processing_keywords.rs",
        include_str!("ch16_processing_keywords.rs"),
    ),
];

/// `main.rs`, `cite.rs`, `meta.rs`, `support.rs` are infrastructure, not
/// chapter modules.
const INFRA_FILES: usize = 4;

#[test]
fn cite_known_rule_with_correct_fingerprint_passes() {
    cite!(
        "15-4-3-5-1",
        "d89bb63cefda9cf4c0485d6d3b9c0c7f799c40b360e7255e3e91e64c3cfd0616"
    );
}

#[test]
fn cite_known_rule_unpinned_passes() {
    cite!("15-4-3-5-1");
}

#[test]
#[should_panic(expected = "fingerprint mismatch")]
fn cite_with_wrong_fingerprint_panics() {
    cite!("15-4-3-5-1", "deadbeef");
}

#[test]
#[should_panic(expected = "unknown rule id")]
fn cite_unknown_id_panics() {
    cite!("99-9-9");
}

#[test]
fn rust_normalise_matches_python_fingerprints() {
    let mut total = 0usize;
    let mut mismatches = Vec::new();
    for (id, o) in obligations() {
        total += 1;
        let got = fingerprint(&normalise(&o.text));
        if got != o.fingerprint {
            mismatches.push(format!("{id}: rust={got} json={}", o.fingerprint));
        }
    }
    assert_eq!(total, 1119, "obligation count changed; update this guard");
    assert!(
        mismatches.is_empty(),
        "{} of {total} fingerprints diverge from Python:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

#[test]
fn rule_text_returns_the_obligation_text() {
    assert!(rule_text("15-4-3-5-1").starts_with("The turn player chooses 1 of their effects"));
}

/// Split a source file into `#[test]` blocks: the text from one `#[test]` to
/// the next (or EOF). The text before the first `#[test]` is not a block.
fn test_blocks(src: &str) -> Vec<&str> {
    src.split("#[test]").skip(1).collect()
}

#[test]
fn every_test_in_this_target_cites_a_rule() {
    let mut uncited = Vec::new();
    for (file, src) in CHAPTER_SOURCES {
        for (i, block) in test_blocks(src).iter().enumerate() {
            if !block.contains("cite!(") {
                let name = block
                    .split("fn ")
                    .nth(1)
                    .and_then(|s| s.split('(').next())
                    .unwrap_or("<unnamed>")
                    .trim();
                uncited.push(format!("{file}: test #{} `{name}`", i + 1));
            }
        }
    }
    assert!(
        uncited.is_empty(),
        "tests without a cite! call:\n{}",
        uncited.join("\n")
    );
}

#[test]
fn test_block_splitter_flags_an_uncited_test() {
    let src = "#[test]\nfn a() { cite!(\"1-1\"); }\n#[test]\nfn b() { assert!(true); }\n";
    let blocks = test_blocks(src);
    assert_eq!(blocks.len(), 2);
    assert!(blocks[0].contains("cite!("));
    assert!(!blocks[1].contains("cite!("));
}

/// Count of unpinned `cite!("id")` calls (single argument). Every cite in the
/// chapter modules is pinned to its rule fingerprint, so this must stay zero:
/// a new unpinned cite would silently skip the renumber/reword guard.
#[test]
fn no_unpinned_cites_remain() {
    let mut unpinned = Vec::new();
    for (file, src) in CHAPTER_SOURCES {
        for part in src.split("cite!(").skip(1) {
            let args = part.split(')').next().unwrap_or("");
            if !args.contains(',') {
                unpinned.push(format!("{file}: cite!({args})"));
            }
        }
    }
    assert!(
        unpinned.is_empty(),
        "{} unpinned cite! call(s) remain:\n{}",
        unpinned.len(),
        unpinned.join("\n")
    );
}

/// Statically verify every pinned `cite!("<id>", "<64 hex>")` in the chapter
/// sources, including those inside `#[ignore]`d tests (which never run, so
/// their runtime `cite!` never fires). Mismatches are all reported at once.
#[test]
fn every_pinned_cite_in_source_matches() {
    let re = regex::Regex::new(r#"cite!\(\s*"([^"]+)"\s*,\s*"([0-9a-f]{64})"\s*,?\s*\)"#).unwrap();
    let mut checked = 0usize;
    let mut failures = Vec::new();
    for (file, src) in CHAPTER_SOURCES {
        for cap in re.captures_iter(src) {
            let (id, fp) = (&cap[1], &cap[2]);
            checked += 1;
            let result = std::panic::catch_unwind(|| check(id, Some(fp)));
            if result.is_err() {
                failures.push(format!("{file}: cite!({id}, {fp}) does not match the denominator"));
            }
        }
    }
    assert!(checked > 0, "no pinned cites found; the scanner regex is broken");
    assert!(
        failures.is_empty(),
        "{} of {checked} pinned cites are stale or unknown:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn chapter_source_list_is_complete() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/rules_conformance");
    let on_disk: Vec<String> = std::fs::read_dir(dir)
        .expect("read tests/rules_conformance")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".rs"))
        .collect();
    assert_eq!(
        on_disk.len(),
        CHAPTER_SOURCES.len() + INFRA_FILES,
        "tests/rules_conformance has {} .rs files ({on_disk:?}) but CHAPTER_SOURCES lists {} \
         chapter modules (+{INFRA_FILES} infra files). Append the new chapter file to \
         CHAPTER_SOURCES in meta.rs.",
        on_disk.len(),
        CHAPTER_SOURCES.len()
    );
}
