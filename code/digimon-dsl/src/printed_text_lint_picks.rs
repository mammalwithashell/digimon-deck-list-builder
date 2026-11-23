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

fn forced_kind(filter: &Value) -> Option<&str> {
    if let Some(kind) = filter["kind"].as_str() {
        return Some(kind);
    }
    if let Some(items) = filter["all_of"].as_array() {
        let kinds: Vec<_> = items.iter().filter_map(forced_kind).collect();
        if let Some(kind) = kinds.first() {
            if kinds.iter().all(|k| k == kind) {
                return Some(kind);
            }
        }
    }
    if let Some(items) = filter["any_of"].as_array() {
        if let Some(kind) = items.first().and_then(forced_kind) {
            if items.iter().all(|item| forced_kind(item) == Some(kind)) {
                return Some(kind);
            }
        }
    }
    None
}

fn restricts_kind(filter: &Value) -> bool {
    filter["kind"].is_string()
        || filter["all_of"]
            .as_array()
            .is_some_and(|items| items.iter().any(restricts_kind))
        || filter["any_of"]
            .as_array()
            .is_some_and(|items| !items.is_empty() && items.iter().all(restricts_kind))
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
            let printed_kind = preceding
                .last()
                .filter(|w| matches!(**w, "digimon" | "tamer" | "option"))
                .copied();
            if printed_kind.map_or_else(
                || restricts_kind(filter),
                |kind| forced_kind(filter) != Some(kind),
            ) {
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
