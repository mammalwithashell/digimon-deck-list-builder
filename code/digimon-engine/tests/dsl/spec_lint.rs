//! CI gate for `digimon_dsl::spec_lint` over the production card pool
//! (`dsl-lint` itself is not run in CI):
//!
//! - no unconditional self `security_attack` aura on a card whose printed face
//!   (official "Effect" section, `data/card_official.json`) carries an innate
//!   `<Security A. ±N>` — the engine counts both
//!   (G-ENGINE-SECURITY-ATTACK-AURA-PLUS-FACE-DOUBLE-COUNT);
//! - no `select_count_capped_multi { min: 0 }` without `optional_zero`.

use digimon_dsl::spec_lint::{check_capped_multi_min_zero, check_security_attack_self_aura};
use digimon_engine::card_data::parse_printed_keywords;
use digimon_engine::enums::Keyword;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// card id → official "Effect" section text.
fn printed_faces() -> HashMap<String, String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/card_official.json");
    let raw = std::fs::read_to_string(&path).expect("read data/card_official.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse card_official.json");
    let mut out = HashMap::new();
    for (id, card) in v["cards"].as_object().expect("cards object") {
        let face: Vec<&str> = card["text_sections"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["label"].as_str() == Some("Effect"))
            .filter_map(|s| s["text"].as_str())
            .collect();
        out.insert(id.clone(), face.join(" "));
    }
    out
}

fn face_prints_security_attack(face: &str) -> bool {
    parse_printed_keywords(face, "", "").iter().any(|k| {
        matches!(
            k,
            Keyword::SecurityAttackPlus(_) | Keyword::SecurityAttackMinus(_)
        )
    })
}

fn yaml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).expect("read cards dir").flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name()
                .map_or(false, |n| n == "_examples" || n == "test")
            {
                continue;
            }
            yaml_files(&p, out);
        } else if p.extension().map_or(false, |x| x == "yaml") {
            out.push(p);
        }
    }
}

#[test]
fn production_specs_pass_spec_lint() {
    let faces = printed_faces();
    let mut files = Vec::new();
    yaml_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards"),
        &mut files,
    );
    files.sort();

    let mut violations = Vec::new();
    let mut printed_sa = 0usize;
    for f in &files {
        let raw = std::fs::read_to_string(f).expect("read spec");
        for finding in check_capped_multi_min_zero(&raw) {
            violations.push(format!(
                "{} {} {}",
                f.display(),
                finding.path,
                finding.message
            ));
        }
        let spec =
            digimon_dsl::loader::load_file(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let prints = faces
            .get(&spec.card)
            .map_or(false, |t| face_prints_security_attack(t));
        printed_sa += prints as usize;
        for finding in check_security_attack_self_aura(&spec, prints) {
            violations.push(format!(
                "{} {} {}",
                spec.card, finding.path, finding.message
            ));
        }
    }
    assert!(
        printed_sa >= 10,
        "only {printed_sa} specs print <Security A.> — is card_official.json present?"
    );
    assert!(
        violations.is_empty(),
        "{} spec-lint violation(s):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn face_parse_distinguishes_innate_from_granted_security_attack() {
    // BT26-060's face (innate) vs a prose grant (a conditional DSL effect).
    assert!(face_prints_security_attack(
        "＜Security A. +1＞ ＜Reboot＞ ＜Blocker＞ [On Play] Delete 1 Digimon."
    ));
    assert!(!face_prints_security_attack(
        "[Your Turn] This Digimon gains ＜Security A. +1＞."
    ));
}
