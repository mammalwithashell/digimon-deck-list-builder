use serde_json::Value;

fn selections<'a>(value: &'a Value, out: &mut Vec<(&'a str, &'a Value)>) {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                if key.starts_with("select_") || key == "choose_from_reveal" {
                    out.push((key, child));
                }
                if !matches!(key.as_str(), "text" | "summary" | "summary_key") {
                    selections(child, out);
                }
            }
        }
        Value::Array(items) => {
            for child in items {
                selections(child, out);
            }
        }
        _ => (),
    }
}

const DIGIMON: u8 = 1;
const PLAYABLE: u8 = 3;
const ALL_KINDS: u8 = 7;

fn kind_bit(kind: &str) -> u8 {
    match kind {
        "digimon" => DIGIMON,
        "tamer" => 2,
        "option" => 4,
        _ => 0,
    }
}

// Guaranteed/possible matches preserve unknown name/trait constraints under NOT.
fn kind_bounds(filter: &Value) -> (u8, u8) {
    let mut result = (ALL_KINDS, ALL_KINDS);
    if let Some(fields) = filter.as_object() {
        for (key, value) in fields {
            if value.is_null() {
                continue;
            }
            let bounds = match key.as_str() {
                "kind" => {
                    let kind = kind_bit(value.as_str().unwrap_or_default());
                    (kind, kind)
                }
                "all_of" => value
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(kind_bounds)
                    .fold((ALL_KINDS, ALL_KINDS), |(a, b), (c, d)| (a & c, b & d)),
                "any_of" | "none_of" => {
                    let items = value.as_array().unwrap();
                    if items.is_empty() {
                        continue;
                    }
                    let (low, high) = items
                        .iter()
                        .map(kind_bounds)
                        .fold((0, 0), |(a, b), (c, d)| (a | c, b | d));
                    if key == "none_of" {
                        (ALL_KINDS & !high, ALL_KINDS & !low)
                    } else {
                        (low, high)
                    }
                }
                "not" => {
                    let (low, high) = kind_bounds(value);
                    (ALL_KINDS & !high, ALL_KINDS & !low)
                }
                "dp" | "dp_eq" | "dp_lte" | "dp_gte" | "level" | "level_eq" | "level_lte"
                | "level_gte" => (0, DIGIMON),
                "any_color" => continue,
                _ => (0, ALL_KINDS),
            };
            result = (result.0 & bounds.0, result.1 & bounds.1);
        }
    }
    result
}

fn kind_mismatch(filter: &Value, preceding: &[&str], tail: &[&str]) -> bool {
    let action_kinds = if preceding.first() == Some(&"play") {
        PLAYABLE
    } else {
        ALL_KINDS
    };
    let mut printed = preceding
        .last()
        .map(|k| kind_bit(k))
        .filter(|k| *k != 0)
        .unwrap_or(action_kinds);
    if preceding.contains(&"level")
        || matches!(tail, ["with", amount, "dp", ..] if amount.chars().all(|c| c.is_ascii_digit()))
        || tail.starts_with(&["with", "a", "level"])
    {
        printed &= DIGIMON;
    }
    kind_bounds(filter).1 & action_kinds != printed
}

fn leaves<'a>(filter: &'a Value, out: &mut Vec<(&'a str, &'a str)>) {
    if let Some(fields) = filter.as_object() {
        for (key, value) in fields {
            if matches!(
                key.as_str(),
                "name_is" | "name_contains" | "trait_has" | "trait_contains"
            ) {
                if let Some(value) = value.as_str() {
                    out.push((key, value));
                }
            }
            if matches!(key.as_str(), "all_of" | "any_of") {
                if let Some(items) = value.as_array() {
                    for item in items {
                        leaves(item, out);
                    }
                }
            }
        }
    }
}

pub(super) fn check(value: &Value, text: &str) -> Vec<&'static str> {
    let mut picks = Vec::new();
    selections(value, &mut picks);
    let [(verb, args)] = picks.as_slice() else {
        return vec![];
    };
    if !matches!(
        *verb,
        "select_hand"
            | "select_trash"
            | "select_reveal"
            | "select_security"
            | "select_zone_cards"
            | "select_union_zone"
            | "select_material"
            | "choose_from_reveal"
    ) {
        return vec![];
    }
    let Some(filter) = args.get("filter") else {
        return vec![];
    };
    let mut findings = Vec::new();
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .collect();
    let nouns: Vec<_> = words
        .iter()
        .enumerate()
        .filter(|(_, w)| matches!(**w, "card" | "cards"))
        .map(|(i, _)| i)
        .collect();
    if let [i] = nouns.as_slice() {
        let start = words[..*i]
            .iter()
            .rposition(|word| {
                matches!(
                    *word,
                    "add" | "play" | "select" | "choose" | "place" | "trash" | "return"
                )
            })
            .unwrap_or(0);
        let preceding = &words[start..*i];
        if !preceding.iter().any(|w| {
            matches!(
                *w,
                "or" | "and" | "except" | "digivolution" | "this" | "that"
            )
        }) {
            if kind_mismatch(filter, preceding, &words[i + 1..]) {
                findings.push("kind_mismatch");
            }
        }
    }
    let mut references = Vec::new();
    let mut rest = text;
    while let Some((before, tail)) = rest.split_once('[') {
        let Some((name, after)) = tail.split_once(']') else {
            break;
        };
        references.push((name, before, after));
        rest = after;
    }
    let mut predicates = Vec::new();
    leaves(filter, &mut predicates);
    for (key, name) in predicates {
        let name = super::normalize(name);
        let matching: Vec<_> = references
            .iter()
            .filter(|(printed, _, _)| *printed == name)
            .collect();
        let [(.., before, after)] = matching.as_slice() else {
            continue;
        };
        let after = after.trim_start();
        let in_name = after.starts_with("in its name") || after.starts_with("in their names");
        let in_trait = [
            "in its traits",
            "in their traits",
            "in its type",
            "in their types",
        ]
        .iter()
        .any(|s| after.starts_with(s));
        let exact_trait = after.starts_with("trait");
        let bare_name = before
            .split_whitespace()
            .last()
            .is_some_and(|w| matches!(w, "1" | "a" | "an" | "named"))
            && !in_name
            && !in_trait
            && !exact_trait
            && !after.starts_with("in its text");
        if (key == "name_is" && in_name) || (key == "name_contains" && bare_name) {
            if !findings.contains(&"name_match_mismatch") {
                findings.push("name_match_mismatch");
            }
        }
        if (key == "trait_has" && in_trait) || (key == "trait_contains" && exact_trait) {
            if !findings.contains(&"trait_match_mismatch") {
                findings.push("trait_match_mismatch");
            }
        }
    }
    findings
}
