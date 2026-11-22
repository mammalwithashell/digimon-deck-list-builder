//! Authoring lints over the RAW card YAML: shapes that parse, compile and run
//! but that no printed card means. Each rule is a class that reached logged
//! games before it was caught (bug scan over 1,224 games), so the rule is an
//! error, not a warning.
//!
//! The walk is over the untyped YAML value on purpose: both shapes can sit at
//! any depth (a formula inside a nested `if`, a granted effect's `when:`), and
//! a typed walk would have to be kept in step with every new step variant.
//!
//! Rules
//!
//! * `unfiltered-battle-area-count` -- `card_count_in_zone` over `battle_area`
//!   with no `filter`. An unfiltered count is `battle_area.len()`: every
//!   permanent, including an Option card an effect placed in the battle area
//!   (CR 4.3 4-28-1, 17-1-3-2-2: it stays there as an Option card) and Tamers
//!   (4-4-1). No printed card counts "permanents"; they count "Digimon",
//!   "Digimon and Tamers", "Tamers", ... so the filter must be spelled out.
//!   ShineGreymon (AD1-016) "-3000 DP for each of your Digimon and Tamers" and
//!   Medusamon (BT24-017) "+2000 DP for each of your opponent's Digimon" both
//!   counted Digital Accident Tactics Squad (P-235) / a Tamer this way.
//!
//! * `self-and-observer-timing` -- one `when:` list mixing a card's OWN timing
//!   (`on_play`, `when_digivolving`, `when_attacking`, `on_deletion`) with the
//!   board-wide OBSERVER timing of the same event family (`on_digivolve`,
//!   `on_enter_field_anyone`, `on_any_digimon_played`, `on_ally_played`,
//!   `on_ally_attack`, `on_opponent_attack`, `on_any_deletion`). The printed
//!   tag pair "[On Play] [When Digivolving]" is `[on_play, when_digivolving]`;
//!   `on_digivolve` fires whenever ANY Digimon on either side digivolves.
//!   Snatchmon (BT21-058) was authored `[on_play, on_digivolve]` and re-ran its
//!   whole effect on every digivolution in the game. A printed observer ("When
//!   any of your Digimon are played or digivolve") uses observer timings only;
//!   "When Digimon attack" is `[on_ally_attack, on_opponent_attack]` (the
//!   observer's own attack is inside `on_ally_attack`), never `when_attacking`.

use serde_yml::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringFinding {
    pub rule: &'static str,
    pub path: String,
    pub message: String,
}

const SELF_TIMINGS: &[&str] = &["on_play", "when_digivolving", "when_attacking", "on_deletion"];
const OBSERVER_TIMINGS: &[&str] = &[
    "on_digivolve",
    "on_enter_field_anyone",
    "on_any_digimon_played",
    "on_ally_played",
    "on_ally_attack",
    "on_opponent_attack",
    "on_any_deletion",
];

/// Lint one card's raw YAML text. A file that does not parse yields no
/// findings here (the loader reports the parse error).
pub fn check_authoring_yaml(raw: &str) -> Vec<AuthoringFinding> {
    let mut out = Vec::new();
    if let Ok(value) = serde_yml::from_str::<Value>(raw) {
        walk(&value, "", &mut out);
    }
    out
}

/// Every production card spec under `cards_root` (`code/digimon-engine/cards`), sorted. `_examples/` holds DSL
/// documentation samples and `test/` synthetic engine fixtures; neither is a printed card. For the pool guards.
pub fn pool_card_files(cards_root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let Ok(sets) = std::fs::read_dir(cards_root) else {
        return files;
    };
    for set in sets.flatten() {
        let set = set.path();
        let name = set.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if !set.is_dir() || name.starts_with('_') || name == "test" {
            continue;
        }
        let Ok(cards) = std::fs::read_dir(&set) else {
            continue;
        };
        for card in cards.flatten() {
            let card = card.path();
            if card.extension().and_then(|e| e.to_str()) == Some("yaml") {
                files.push(card);
            }
        }
    }
    files.sort();
    files
}

/// Run [`check_authoring_yaml`] over every production card spec; one line per finding.
pub fn check_pool(cards_root: &std::path::Path) -> Vec<String> {
    let mut findings = Vec::new();
    for file in pool_card_files(cards_root) {
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        for finding in check_authoring_yaml(&raw) {
            findings.push(format!(
                "{} [{}] {}: {}",
                file.file_name().and_then(|n| n.to_str()).unwrap_or_default(),
                finding.rule,
                finding.path,
                finding.message
            ));
        }
    }
    findings
}

fn zone_includes_battle_area(zone: Option<&Value>) -> bool {
    match zone {
        Some(Value::String(s)) => s == "battle_area",
        Some(Value::Sequence(items)) => items
            .iter()
            .any(|item| matches!(item, Value::String(s) if s == "battle_area")),
        _ => false,
    }
}

fn walk(value: &Value, path: &str, out: &mut Vec<AuthoringFinding>) {
    match value {
        Value::Mapping(map) => {
            for (key, child) in map {
                let Some(key) = key.as_str() else {
                    continue;
                };
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                if key == "card_count_in_zone" {
                    if let Value::Mapping(spec) = child {
                        // Absent, null, or an empty mapping (`filter: {}`): a
                        // present key that constrains nothing still counts
                        // every permanent.
                        let filter_missing = spec.get("filter").is_none_or(|filter| match filter {
                            Value::Null => true,
                            Value::Mapping(map) => map.is_empty(),
                            _ => false,
                        });
                        if zone_includes_battle_area(spec.get("zone")) && filter_missing {
                            out.push(AuthoringFinding {
                                rule: "unfiltered-battle-area-count",
                                path: child_path.clone(),
                                message: "card_count_in_zone over battle_area has no filter: it counts EVERY permanent, Option cards placed in the battle area and Tamers included (CR 4.3 4-28-1, 4-4-1). Spell out what the card counts, e.g. `filter: { kind: digimon }` or `filter: { any_of: [{ kind: digimon }, { kind: tamer }] }`".into(),
                            });
                        }
                    }
                }
                if key == "when" {
                    if let Value::Sequence(items) = child {
                        let timings: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
                        let own = timings.iter().copied().find(|t| SELF_TIMINGS.contains(t));
                        let observer = timings.iter().copied().find(|t| OBSERVER_TIMINGS.contains(t));
                        if let (Some(own), Some(observer)) = (own, observer) {
                            out.push(AuthoringFinding {
                                rule: "self-and-observer-timing",
                                path: child_path.clone(),
                                message: format!(
                                    "`when` mixes the card's own timing `{own}` with the board-wide observer timing `{observer}`. A printed \"[On Play] [When Digivolving]\" is `[on_play, when_digivolving]` (`on_digivolve` fires on EVERY Digimon's digivolution); a printed observer (\"When any ... are played or digivolve\", \"When Digimon attack\") uses observer timings only"
                                ),
                            });
                        }
                    }
                }
                walk(child, &child_path, out);
            }
        }
        Value::Sequence(items) => {
            for (index, child) in items.iter().enumerate() {
                walk(child, &format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(raw: &str) -> Vec<&'static str> {
        check_authoring_yaml(raw).into_iter().map(|f| f.rule).collect()
    }

    #[test]
    fn unfiltered_battle_area_count_is_rejected() {
        let raw = "effects:\n  - when: on_play\n    process:\n      - add_dp_modifier:\n          value:\n            formula:\n              base: 0\n              per:\n                card_count_in_zone:\n                  of: you\n                  zone: battle_area\n              delta: -3000\n";
        let findings = check_authoring_yaml(raw);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "unfiltered-battle-area-count");
        assert!(findings[0].path.ends_with("per.card_count_in_zone"));
    }

    #[test]
    fn filtered_battle_area_count_and_other_zones_pass() {
        let filtered = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: opponent, zone: battle_area, filter: { kind: digimon } }\n";
        assert!(rules(filtered).is_empty());
        let trash = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: you, zone: trash }\n";
        assert!(rules(trash).is_empty());
    }

    /// `filter: {}` is a present key with no constraint: the count is still
    /// every permanent, so it is the same finding as a missing filter.
    #[test]
    fn empty_filter_mapping_is_an_unfiltered_battle_area_count() {
        for filter in ["{}", "{ }", "null", "~"] {
            let raw = format!(
                "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: {{ of: you, zone: battle_area, filter: {filter} }}\n"
            );
            assert_eq!(rules(&raw), vec!["unfiltered-battle-area-count"], "filter: {filter}");
        }
        let list_zone = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: you, zone: [battle_area], filter: {} }\n";
        assert_eq!(rules(list_zone), vec!["unfiltered-battle-area-count"]);
    }

    #[test]
    fn on_play_with_board_wide_on_digivolve_is_rejected() {
        // Snatchmon (BT21-058): printed "[On Play] [When Digivolving]".
        let raw = "effects:\n  - when: [on_play, on_digivolve]\n    process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }

    #[test]
    fn when_attacking_with_opponent_attack_observer_is_rejected() {
        let raw = "effects:\n  - when: [when_attacking, on_opponent_attack]\n    process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }

    #[test]
    fn self_only_and_observer_only_lists_pass() {
        for when in [
            "[on_play, when_digivolving]",
            "[on_play, when_digivolving, when_attacking]",
            "[on_enter_field_anyone, on_digivolve]",
            "[on_ally_played, on_digivolve]",
            "[on_ally_attack, on_opponent_attack]",
            "on_digivolve",
        ] {
            let raw = format!("effects:\n  - when: {when}\n    process: []\n");
            assert!(rules(&raw).is_empty(), "{when}");
        }
    }

    #[test]
    fn nested_granted_effect_is_walked() {
        let raw = "effects:\n  - when: on_play\n    process:\n      - grant_triggered_effect:\n          effect:\n            when: [on_deletion, on_any_deletion]\n            process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }
}
