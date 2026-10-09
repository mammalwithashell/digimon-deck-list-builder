//! Guard: production card data must carry every trait a DSL card declares.
//!
//! Behavioral tests build `CardData` from the *compiled YAML* (`card_data_from_compiled`),
//! whose traits are the card's whole trait line — its `form:`, `attribute:` and `traits:`
//! (`CompiledCard::all_traits`) — so a trait declared only in a card's YAML passes those tests
//! while being ABSENT in production, which builds `CardData` from `cards.json`
//! (`CardData::load_from_str`, traits = form_eng + attribute_eng + type_eng). That silent
//! divergence is exactly the Ice-Snow / Appmon class of bug: the digimoncard.io API drops
//! `(Rule) Trait: Has [X] Type.` grants and the `form` field, so a trait-keyed requirement
//! (e.g. an alt-digivolve `from: { trait_has: ... }`) matches in tests but fails in real games.
//!
//! This guard asserts the production trait set (cards.json-built) is a superset of every
//! DSL card's compiled trait line. Reconcile any failure from the official Bandai DB via
//! `code/tools/audit_digivolve/reconcile_traits.py` (see CLAUDE.md "Printed card data").
//!
//! Stage forms (In-Training … Mega) are exempt, as in `_STAGES` of
//! `code/tests/test_cards_json_integrity.py`: cards.json omits them for most cards and
//! `reconcile_traits.py` keeps them out of `form_eng`. That is safe only while no DSL card
//! matches a trait against a stage — `no_dsl_card_matches_a_trait_against_a_stage_form`.

use std::collections::HashSet;

/// The stage a Digimon's printed Form names; not carried by cards.json for most cards.
const STAGE_FORMS: [&str; 6] = [
    "Baby",
    "In-Training",
    "Rookie",
    "Champion",
    "Ultimate",
    "Mega",
];

/// Printed attributes cards.json gets wrong, checked against each card image; the YAML is
/// right. Remove an entry once cards.json is corrected — the guard fails on a stale entry.
const CARDS_JSON_ATTRIBUTE_ERRORS: [(&str, &str); 5] = [
    // Garurumon prints Champion | Vaccine; cards.json has attribute_eng ["Data"].
    ("AD1-010", "Vaccine"),
    // Craniamon prints Mega | Vaccine; cards.json has attribute_eng ["Data"].
    ("BT23-058", "Vaccine"),
    // SkullGreymon prints Ultimate | Virus; cards.json has attribute_eng ["Ultimate"].
    ("BT24-072", "Virus"),
    // SkullBaluchimon prints Ultimate | Data; cards.json has attribute_eng ["Ultimate"].
    ("BT24-075", "Data"),
    // Alphamon prints Mega | Vaccine; cards.json has attribute_eng ["Data"].
    ("EX13-060", "Vaccine"),
];

fn is_stage_form(name: &str) -> bool {
    STAGE_FORMS
        .iter()
        .any(|stage| stage.eq_ignore_ascii_case(name))
}

/// Parsing + lowering the full embedded pack is stack-heavy and overflows the test harness's
/// default ~2 MB thread stack; run the body on a large-stack thread so the guard is
/// self-contained (no RUST_MIN_STACK dependency).
fn on_big_stack(body: fn()) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(body)
        .expect("spawn large-stack thread")
        .join()
        .expect("trait-parity guard thread panicked");
}

#[test]
fn production_card_data_traits_superset_of_dsl_traits() {
    on_big_stack(check_trait_parity);
}

fn check_trait_parity() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded cards.pack must load");
    let production = digimon_engine::deck_tools::full_card_data();

    let mut violations: Vec<String> = Vec::new();
    let mut known_errors_seen: HashSet<(&str, &str)> = HashSet::new();
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
        // The lines `CompiledCard::all_traits` folds, minus a stage form.
        let declared = compiled
            .traits
            .iter()
            .chain(&compiled.attribute)
            .chain(compiled.form.iter().filter(|form| !is_stage_form(form)));
        for trait_name in declared {
            if have.contains(&trait_name.to_ascii_lowercase()) {
                continue;
            }
            if let Some(&known) = CARDS_JSON_ATTRIBUTE_ERRORS
                .iter()
                .find(|(id, attribute)| id == card_id && attribute == trait_name)
            {
                known_errors_seen.insert(known);
                continue;
            }
            violations.push(format!(
                "  {card_id}: YAML trait {trait_name:?} absent from production traits {:?}",
                card_data.traits
            ));
        }
    }
    for (card_id, attribute) in CARDS_JSON_ATTRIBUTE_ERRORS {
        if !known_errors_seen.contains(&(card_id, attribute)) {
            violations.push(format!(
                "  {card_id}: production now carries {attribute:?} — remove it from \
                 CARDS_JSON_ATTRIBUTE_ERRORS"
            ));
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

/// The stage-form exemption above holds only while no DSL card matches a trait against a
/// stage: production omits the stage for most cards, so such a predicate would match in
/// DebugRunner and miss in a real game. BT9-083 ("For each card with [Mega] in their
/// traits", not yet authored) is the printed card that needs one — reconcile stage forms
/// into cards.json before authoring it.
#[test]
fn no_dsl_card_matches_a_trait_against_a_stage_form() {
    on_big_stack(check_no_stage_form_predicates);
}

fn check_no_stage_form_predicates() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded cards.pack must load");

    let mut violations: Vec<String> = Vec::new();
    for (card_id, compiled) in registry.iter() {
        let tree = serde_json::to_value(compiled).expect("compiled card serializes to JSON");
        let mut found = Vec::new();
        stage_form_predicates(&tree, &mut found);
        violations.extend(found.into_iter().map(|hit| format!("  {card_id}: {hit}")));
    }

    assert!(
        violations.is_empty(),
        "{} DSL predicate(s) match a trait against a stage form, which production does not \
         carry for most cards. Reconcile stage forms into cards.json, then drop the stage-form \
         exemption in this guard:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

/// Collects `key: value` for every trait-matching predicate leaf under `node` — any key
/// naming a trait (`trait_has`, `event_target_trait_has`, …) or `form_is` / `attribute_is`,
/// but not a card's own `traits` list — whose value is a stage form.
fn stage_form_predicates(node: &serde_json::Value, found: &mut Vec<String>) {
    match node {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let matches_a_trait = key != "traits"
                    && (key.contains("trait") || key == "form_is" || key == "attribute_is");
                if matches_a_trait {
                    let names = value.as_str().into_iter().chain(
                        value
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|v| v.as_str()),
                    );
                    for name in names.filter(|name| is_stage_form(name)) {
                        found.push(format!("{key}: {name}"));
                    }
                }
                stage_form_predicates(value, found);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                stage_form_predicates(item, found);
            }
        }
        _ => {}
    }
}
