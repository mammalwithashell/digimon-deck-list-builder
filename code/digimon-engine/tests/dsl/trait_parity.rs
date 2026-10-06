//! Guard: production card data must carry every trait a DSL card declares.
//!
//! Behavioral tests build `CardData` from the *compiled YAML* (`card_data_from_compiled`),
//! so a trait declared only in a card's YAML `traits:` field passes those tests while being
//! ABSENT in production — which builds `CardData` from `cards.json` (`CardData::load_from_str`,
//! traits = form_eng + attribute_eng + type_eng). That silent divergence is exactly the
//! Ice-Snow / Appmon class of bug: the digimoncard.io API drops `(Rule) Trait: Has [X] Type.`
//! grants and the `form` field, so a trait-keyed requirement (e.g. an alt-digivolve
//! `from: { trait_has: ... }`) matches in tests but fails in real games.
//!
//! This guard asserts the production trait set (cards.json-built) is a superset of every
//! DSL card's compiled `traits:`. Reconcile any failure from the official Bandai DB via
//! `code/tools/audit_digivolve/reconcile_traits.py` (see CLAUDE.md "Printed card data").
//!
//! The other direction matters too: a YAML that leaves out a printed type trait makes its
//! behavioral tests blind to it (EX10-069's [LIBERATOR], BT22-084's [CS], until
//! 2026-10-05), and a YAML `attribute:` copied from a wrong cards.json row stays wrong
//! after the row is fixed (AD1-010, BT23-058). cards.json's traits are themselves held to
//! the official DB pool-wide by code/tests/test_cards_json_integrity.py.

use std::collections::HashSet;

#[test]
fn production_card_data_traits_superset_of_dsl_traits() {
    // Parsing + lowering the full embedded pack is stack-heavy and overflows the test
    // harness's default ~2 MB thread stack; run the body on a large-stack thread so the
    // guard is self-contained (no RUST_MIN_STACK dependency).
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(check_trait_parity)
        .expect("spawn large-stack thread")
        .join()
        .expect("trait-parity guard thread panicked");
}

fn check_trait_parity() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded cards.pack must load");
    let production = digimon_engine::deck_tools::full_card_data();

    let mut violations: Vec<String> = Vec::new();
    for (card_id, compiled) in registry.iter() {
        // Tokens and test-only cards are not in cards.json — they have no production
        // CardData to compare against, so skip them.
        let Some(card_data) = production.get(card_id) else {
            continue;
        };
        let have: HashSet<String> = card_data
            .traits
            .iter()
            .map(|t| t.to_ascii_lowercase())
            .collect();
        for trait_name in &compiled.traits {
            if !have.contains(&trait_name.to_ascii_lowercase()) {
                violations.push(format!(
                    "  {card_id}: YAML trait {trait_name:?} absent from production traits {:?}",
                    card_data.traits
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} DSL card(s) declare a trait missing from production (cards.json-built) CardData.\n\
         The digimoncard.io API drops Rule grants / the form field; reconcile from the official\n\
         Bandai DB with code/tools/audit_digivolve/reconcile_traits.py, or fix the DSL spec if\n\
         the official DB does not list the trait:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

const CARDS_JSON_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json");

/// data/cards.json rows by card id. Production folds a row's form_eng, attribute_eng and
/// type_eng into one trait list; the YAML files the type line as `traits:` (plus whatever a
/// card's tests need) and the attribute as `attribute:`, so they are compared field by field.
fn cards_json_trait_fields() -> std::collections::HashMap<String, serde_json::Value> {
    let raw = std::fs::read_to_string(CARDS_JSON_PATH).expect("read data/cards.json");
    serde_json::from_str(&raw).expect("cards.json parses")
}

fn field(card: &serde_json::Value, name: &str) -> Vec<String> {
    card[name]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Every printed type trait of a DSL card (its cards.json `type_eng`, minus the API's
/// "X (App Name)" copies of an app name) must be in the YAML `traits:`, and a YAML
/// `attribute:` must be one of the card's printed attributes.
#[test]
fn dsl_yaml_carries_the_printed_types_and_attribute() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(check_printed_types_and_attribute)
        .expect("spawn large-stack thread")
        .join()
        .expect("printed-trait guard thread panicked");
}

fn check_printed_types_and_attribute() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded cards.pack must load");
    let cards = cards_json_trait_fields();

    let mut violations: Vec<String> = Vec::new();
    for (card_id, compiled) in registry.iter() {
        let Some(card) = cards.get(card_id) else {
            continue;
        };
        let yaml: HashSet<String> = compiled
            .traits
            .iter()
            .map(|t| t.to_ascii_lowercase())
            .collect();
        for printed in field(card, "type_eng") {
            if !printed.ends_with("(App Name)") && !yaml.contains(&printed.to_ascii_lowercase()) {
                violations.push(format!(
                    "  {card_id}: printed type {printed:?} absent from YAML traits {:?}",
                    compiled.traits
                ));
            }
        }
        let attributes = field(card, "attribute_eng");
        if let Some(attribute) = &compiled.attribute {
            if !attributes.iter().any(|a| a.eq_ignore_ascii_case(attribute)) {
                violations.push(format!(
                    "  {card_id}: YAML attribute {attribute:?}, but the card prints {attributes:?}"
                ));
            }
        }
    }
    violations.sort();

    assert!(
        violations.is_empty(),
        "{} DSL card(s) disagree with the type line or attribute the card prints \
         (data/cards.json, itself held to the official Bandai DB by \
         code/tests/test_cards_json_integrity.py). Fix the YAML `traits:` / `attribute:`:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}
