//! Guard: `(Rule) Trait: Has [X] ...` grants from the official Bandai DB must be
//! present in the trait data the engine actually matches on.
//!
//! Some cards print a rule box such as "(Rule) Trait: Has [Ice-Snow] Type." —
//! the card carries that trait everywhere, in addition to its printed trait
//! line. The digimoncard.io API behind `data/cards.json` drops these grants
//! entirely, so a trait-keyed predicate (effect targeting, digivolve
//! requirement, DSL FILTER) silently fails to match in real games (the
//! user-visible "Rule to give types not working" bug, observed on EX8-019 /
//! EX7-016 / EX7-023).
//!
//! The official mirror `data/card_official.json` (world.digimoncard.com) is
//! authoritative for printed data. This guard parses every Rule trait grant in
//! it and asserts, in both trait surfaces the engine uses:
//!
//! 1. **Production** — `deck_tools::full_card_data()` (cards.json-built
//!    `CardData.traits`, what every runtime trait predicate reads).
//! 2. **Compiled DSL** — the embedded pack's `traits:` (what DebugRunner
//!    behavioral tests match against).
//!
//! Reconcile any production failure from the official DB via
//! `code/tools/audit_digivolve/reconcile_traits.py --apply` (then fold the
//! overrides into cards.json); fix compiled failures in the card's YAML
//! `traits:` line.

use std::collections::HashMap;

const OFFICIAL_JSON_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/card_official.json"
);

/// Parse every `(Rule) Trait: Has [X] ...` / `[Rule] Trait: Has [X] and [Y].`
/// grant out of a card's joined printed text. Handles all forms observed in
/// the official mirror:
///   - `(Rule) Trait: Has [Ice-Snow] Type.` / `... type.`
///   - `(Rule) Trait: Has [Free] attribute.` / `... Attribute.`
///   - `(Rule) Trait: Has [Hybrid] Form.`
///   - `(Rule) Trait: Has [Olympos XII].` (no field word)
///   - `(Rule) Trait: Has [Boss] and [D-Brigade].` (compound)
///   - `[Rule] Trait: Has [Dragonkin] Type.` (square-bracket Rule marker)
fn rule_trait_grants(text: &str) -> Vec<String> {
    let mut grants = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find("Rule") {
        let preceded = rest[..pos].ends_with('(') || rest[..pos].ends_with('[');
        let after = &rest[pos + "Rule".len()..];
        if preceded {
            if let Some(s) = after.strip_prefix(')').or_else(|| after.strip_prefix(']')) {
                let s = s.trim_start();
                if let Some(s) = s.strip_prefix("Trait:") {
                    let s = s.trim_start();
                    if let Some(mut s) = s.strip_prefix("Has") {
                        // One or more `[Name]`, optionally joined by "and".
                        loop {
                            let t = s.trim_start();
                            let Some(open) = t.strip_prefix('[') else { break };
                            let Some(end) = open.find(']') else { break };
                            grants.push(open[..end].trim().to_string());
                            s = &open[end + 1..];
                            match s.trim_start().strip_prefix("and") {
                                Some(t) => s = t,
                                None => break,
                            }
                        }
                    }
                }
            }
        }
        rest = after;
    }
    grants
}

/// `{card_id: [granted trait, ...]}` for every card in the official mirror
/// that prints at least one Rule trait grant.
fn official_rule_grants() -> HashMap<String, Vec<String>> {
    let raw = std::fs::read_to_string(OFFICIAL_JSON_PATH)
        .expect("read data/card_official.json from repo root");
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).expect("card_official.json parses");
    let cards = parsed["cards"]
        .as_object()
        .expect("card_official.json has a `cards` object");

    let mut out = HashMap::new();
    for (card_id, card) in cards {
        let joined = card["text_sections"]
            .as_array()
            .map(|sections| {
                sections
                    .iter()
                    .filter_map(|s| s["text"].as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let grants = rule_trait_grants(&joined);
        if !grants.is_empty() {
            out.insert(card_id.clone(), grants);
        }
    }
    assert!(
        out.len() >= 100,
        "sanity: the official mirror carries 120+ Rule trait grants; found {} — \
         the parser or the mirror layout regressed",
        out.len()
    );
    out
}

#[test]
fn parser_handles_all_observed_rule_grant_forms() {
    assert_eq!(
        rule_trait_grants("effects. (Rule) Trait: Has [Ice-Snow] Type."),
        vec!["Ice-Snow"]
    );
    assert_eq!(
        rule_trait_grants("Digimon. (Rule) Trait: Has [Free] attribute."),
        vec!["Free"]
    );
    assert_eq!(
        rule_trait_grants("card. (Rule) Trait: Has [Hybrid] Form."),
        vec!["Hybrid"]
    );
    assert_eq!(
        rule_trait_grants("attack. (Rule) Trait: Has [Olympos XII]."),
        vec!["Olympos XII"]
    );
    assert_eq!(
        rule_trait_grants("checks. (Rule) Trait: Has [Boss] and [D-Brigade]."),
        vec!["Boss", "D-Brigade"]
    );
    assert_eq!(
        rule_trait_grants("attack. [Rule] Trait: Has [Dragonkin] Type."),
        vec!["Dragonkin"]
    );
    // Name rules and unrelated text must NOT parse as trait grants.
    assert!(rule_trait_grants("(Rule) Name: Also treated as [Agumon].").is_empty());
    assert!(rule_trait_grants("no rule box here [Ice-Snow] trait").is_empty());
}

/// PRODUCTION: every official Rule trait grant must be present in the
/// cards.json-built `CardData.traits` — the trait list every runtime predicate
/// (effect targeting, digivolve requirements, DSL FILTERs) matches against.
#[test]
fn production_card_data_carries_official_rule_trait_grants() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(check_production_grants)
        .expect("spawn large-stack thread")
        .join()
        .expect("production rule-grant guard thread panicked");
}

fn check_production_grants() {
    let production = digimon_engine::deck_tools::full_card_data();
    let grants = official_rule_grants();

    // Regression pins for the reported bug (EX8-019 / EX7-016 / EX7-023 —
    // "Rule to give types not working"): these must be flagged as granting
    // Ice-Snow, so the loop below cannot silently skip them.
    for pinned in ["EX8-019", "EX7-016", "EX7-023"] {
        assert!(
            grants
                .get(pinned)
                .is_some_and(|g| g.iter().any(|t| t == "Ice-Snow")),
            "{pinned} must parse as granting [Ice-Snow] (printed rule box)"
        );
    }

    let mut violations: Vec<String> = Vec::new();
    for (card_id, granted) in &grants {
        // Cards not yet ingested into cards.json have no production CardData
        // to check — the guard picks them up once the set is ingested.
        let Some(card_data) = production.get(card_id) else {
            continue;
        };
        for trait_name in granted {
            if !card_data
                .traits
                .iter()
                .any(|t| t.eq_ignore_ascii_case(trait_name))
            {
                violations.push(format!(
                    "  {card_id}: rule-granted trait {trait_name:?} absent from production \
                     traits {:?}",
                    card_data.traits
                ));
            }
        }
    }
    violations.sort();

    assert!(
        violations.is_empty(),
        "{} card(s) print a `(Rule) Trait: Has [X]` grant that production \
         (cards.json-built) CardData does not carry — trait predicates cannot see it \
         in-game.\nReconcile from the official Bandai DB:\n  python \
         code/tools/audit_digivolve/reconcile_traits.py --apply\nand fold the overrides \
         into data/cards.json (see CLAUDE.md \"Printed card data\"):\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

/// COMPILED DSL: every official Rule trait grant must also be present in the
/// card's compiled YAML `traits:` — that is the trait list DebugRunner
/// behavioral tests match against (`card_data_from_compiled`), so a grant
/// missing here makes tests diverge from production.
#[test]
fn compiled_dsl_traits_carry_official_rule_trait_grants() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(check_compiled_grants)
        .expect("spawn large-stack thread")
        .join()
        .expect("compiled rule-grant guard thread panicked");
}

fn check_compiled_grants() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded cards.pack must load");
    let grants = official_rule_grants();

    let mut violations: Vec<String> = Vec::new();
    for (card_id, compiled) in registry.iter() {
        let Some(granted) = grants.get(card_id) else {
            continue;
        };
        for trait_name in granted {
            if !compiled
                .traits
                .iter()
                .any(|t| t.eq_ignore_ascii_case(trait_name))
            {
                violations.push(format!(
                    "  {card_id}: rule-granted trait {trait_name:?} absent from YAML \
                     traits {:?}",
                    compiled.traits
                ));
            }
        }
    }
    violations.sort();

    assert!(
        violations.is_empty(),
        "{} DSL card(s) omit a printed `(Rule) Trait: Has [X]` grant from their YAML \
         `traits:` line — behavioral tests would miss trait interactions production \
         must support. Add the granted trait to each card's YAML `traits:`:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}
