//! Source-presence checks that typed default values cannot preserve.

use crate::errors::ValidationError;
use serde_json::Value;

pub const OPTIONAL_STEPS: &[&str] = &[
    "trash_bottom_face_down_source_under_tamer",
    "trash_link_card_of_own_digimon",
    "trash_option_from_own_stacks",
    "may_attack_now",
    "may_dna_digivolve_now",
    "redirect_attack_target",
    "refire_effect",
    "choose_from_reveal",
    "search_own_security_stack",
    "use_option_from_hand",
    "use_option_from_trash",
    "app_fuse",
    "select_own_permanent",
    "select_opponent_permanent",
    "select_any_permanent",
    "select_dna_pair",
    "select_hand",
    "select_trash",
    "select_material",
    "select_reveal",
    "select_security",
    "select_own_breeding_permanent",
    "select_union_zone",
    "link_to_own_digimon",
    "pay_source_prefix",
];

fn has_level(value: &Value) -> bool {
    value.as_object().is_some_and(|fields| {
        fields.iter().any(|(key, value)| {
            key == "level"
                || key.starts_with("level_")
                || (matches!(key.as_str(), "all_of" | "any_of")
                    && value
                        .as_array()
                        .is_some_and(|items| items.iter().any(has_level)))
        })
    })
}

fn explicit_color(value: &Value) -> bool {
    value.get("color_is").is_some_and(Value::is_string)
        || value.get("any_color") == Some(&Value::Bool(true))
        || value["all_of"]
            .as_array()
            .is_some_and(|items| items.iter().any(explicit_color))
        || value["any_of"]
            .as_array()
            .is_some_and(|items| !items.is_empty() && items.iter().all(explicit_color))
}

pub fn check(value: &Value) -> Vec<ValidationError> {
    fn require_optional(
        args: &Value,
        verb: &str,
        path: &str,
        card: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        if !args.get("optional").is_some_and(Value::is_boolean) {
            errors.push(ValidationError {
                card_id: card.into(),
                path: path.into(),
                message: format!("step `{verb}` must declare `optional:` as true or false"),
            });
        }
    }
    fn walk(value: &Value, path: &str, card: &str, errors: &mut Vec<ValidationError>) {
        match value {
            Value::Object(fields) => {
                for (key, child) in fields {
                    let next = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    if OPTIONAL_STEPS.contains(&key.as_str()) {
                        require_optional(child, key, &next, card, errors);
                    }
                    if key == "reveal_search" {
                        if let Some(buckets) = child["buckets"].as_array() {
                            for (i, bucket) in buckets.iter().enumerate() {
                                require_optional(
                                    bucket,
                                    "reveal_search bucket",
                                    &format!("{next}.buckets[{i}]"),
                                    card,
                                    errors,
                                );
                            }
                        }
                    }
                    if key == "alt_paths" {
                        if let Some(paths) = child.as_array() {
                            for (i, entry) in paths.iter().enumerate() {
                                if entry["kind"] == "digivolve"
                                    && entry.get("direction").map_or(true, |v| v == "from")
                                    && has_level(&entry["from"])
                                    && !explicit_color(&entry["from"])
                                {
                                    errors.push(ValidationError {
                                        card_id: card.into(),
                                        path: format!("{next}[{i}].from"),
                                        message: "digivolve source with a level gate must declare color_is, colour alternatives, or any_color: true".into(),
                                    });
                                }
                            }
                        }
                    }
                    walk(child, &next, card, errors);
                }
            }
            Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    walk(child, &format!("{path}[{i}]"), card, errors);
                }
            }
            _ => (),
        }
    }
    let mut errors = Vec::new();
    walk(
        value,
        "",
        value["card"].as_str().unwrap_or_default(),
        &mut errors,
    );
    errors
}
