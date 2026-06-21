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
