//! CI gate for the security-icon lint (`digimon_dsl::security_icon_lint`):
//! every production card spec's security-scoped clauses must match the pink
//! `{Security}` / blue `[Security]` icons on the printed card
//! (`data/card_official.json`, the official Bandai DB mirror).
//!
//! Pink `{Security}` = active while FACE UP in the security stack
//! (`scope: security`); blue `[Security]` = fires on a security check
//! (`when: on_security`). See docs/RUST_DSL_AGENT_GUIDE.md §"Security icons".

use digimon_dsl::security_icon_lint::{check_security_icons, PrintedSection, SecurityIconRule};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn printed_sections() -> HashMap<String, Vec<PrintedSection>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/card_official.json");
    let raw = std::fs::read_to_string(&path).expect("read data/card_official.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse card_official.json");
    let mut out = HashMap::new();
    for (id, card) in v["cards"].as_object().expect("cards object") {
        let sections = card["text_sections"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| PrintedSection {
                        label: s["label"].as_str().unwrap_or_default().to_string(),
                        text: s["text"].as_str().unwrap_or_default().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.insert(id.clone(), sections);
    }
    out
}

fn yaml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).expect("read cards dir").flatten() {
        let p = e.path();
        if p.is_dir() {
            // `_examples/` holds curated fixtures, not production cards.
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
fn production_specs_match_printed_security_icons() {
    let printed = printed_sections();
    let mut files = Vec::new();
    yaml_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards"),
        &mut files,
    );
    files.sort();

    let mut violations = Vec::new();
    let mut checked = 0usize;
    for f in &files {
        let spec =
            digimon_dsl::loader::load_file(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let Some(sections) = printed.get(&spec.card) else {
            continue;
        };
        checked += 1;
        for finding in check_security_icons(&spec, sections) {
            violations.push(format!(
                "{} {} [{:?}] {}",
                spec.card, finding.path, finding.rule, finding.message
            ));
        }
    }
    assert!(
        checked > 100,
        "only {checked} specs had printed text — is card_official.json present?"
    );
    assert!(
        violations.is_empty(),
        "{} security-icon mismatch(es):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

// ─── Unit coverage of the three rules ─────────────────────────────────────

fn spec(yaml: &str) -> digimon_dsl::CardSpec {
    serde_yml::from_str(yaml).expect("fixture parses")
}

fn sec(label: &str, text: &str) -> PrintedSection {
    PrintedSection {
        label: label.into(),
        text: text.into(),
    }
}

const PINK_AS_BLUE: &str = r#"
card: X-1
name: X
kind: digimon
level: 6
color: [purple]
cost: 12
dp: 12000
effects:
  - when: on_security
    summary: "wrongly authored pink effect"
    process:
      - play_from_security: {}
"#;

const PINK_OK: &str = r#"
card: X-2
name: X
kind: digimon
level: 6
color: [purple]
cost: 12
dp: 12000
effects:
  - when: end_of_opponents_turn
    scope: security
    summary: "pink"
    process:
      - play_from_security: {}
"#;

const DISCARD_OK: &str = r#"
card: X-3
name: X
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_discard_security
    scope: security
    summary: "when effects trash this card from security"
    process:
      - gain_memory: 1
"#;

#[test]
fn pink_effect_authored_as_on_security_is_flagged() {
    let printed = [sec(
        "Effect",
        "{Security} [End of Opponent's Turn] Play this card without paying the cost.",
    )];
    let f = check_security_icons(&spec(PINK_AS_BLUE), &printed);
    let rules: Vec<_> = f.iter().map(|x| x.rule).collect();
    assert!(
        rules.contains(&SecurityIconRule::PinkIconWithoutPinkClause),
        "{rules:?}"
    );
    assert!(
        rules.contains(&SecurityIconRule::BlueClauseWithoutBlueIcon),
        "{rules:?}"
    );
}

#[test]
fn pink_effect_authored_as_security_scope_passes() {
    let printed = [sec(
        "Effect",
        "{Security} [End of Opponent's Turn] Play this card without paying the cost.",
    )];
    assert!(check_security_icons(&spec(PINK_OK), &printed).is_empty());
}

#[test]
fn security_scope_on_a_blue_only_card_is_flagged() {
    let printed = [sec(
        "Security Effect",
        "[Security] Play this card without paying the cost.",
    )];
    let f = check_security_icons(&spec(PINK_OK), &printed);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rule, SecurityIconRule::PinkClauseWithoutPinkIcon);
}

#[test]
fn trashed_from_security_shape_is_exempt() {
    let printed = [sec(
        "Effect",
        "When effects trash this card from the security stack, gain 1 memory.",
    )];
    assert!(check_security_icons(&spec(DISCARD_OK), &printed).is_empty());
}

#[test]
fn older_bracket_spelling_of_the_pink_icon_is_recognised() {
    // ST20-15 "Island of Adventure": the face-up effect is printed
    // "[Security] [All Turns] …" inside the main effect box.
    let printed = [
        sec(
            "Effect",
            "[Security] [All Turns] All of your level 3 or higher Digimon get +2000 DP.",
        ),
        sec(
            "Security Effect",
            "[Security] You may play 1 Tamer card from your hand without paying the cost.",
        ),
    ];
    assert!(check_security_icons(&spec(PINK_OK), &printed).is_empty());
    let f = check_security_icons(&spec(PINK_AS_BLUE), &printed);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].rule, SecurityIconRule::PinkIconWithoutPinkClause);
}

#[test]
fn security_followed_by_an_event_timing_is_the_blue_icon() {
    // BT26-075 ScourgeChiropmon: "[Security] [On Deletion] By trashing …" — a
    // security-check trigger sharing its body with an [On Deletion] trigger
    // (a card face up in security can never be deleted). DCGO: SecuritySkill.
    let printed = [sec(
        "Effect",
        "<Execute> [Security] [On Deletion] By trashing the bottom face-down card from under any of your Tamers, you may play 1 card.",
    )];
    assert!(
        check_security_icons(&spec(PINK_AS_BLUE), &printed).is_empty(),
        "`when: on_security` is the right shape"
    );
    let f = check_security_icons(&spec(PINK_OK), &printed);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].rule, SecurityIconRule::PinkClauseWithoutPinkIcon);
}
