//! Lowering pins for the folded-"may" [Once Per Turn] refund
//! (`dsl_cards::lower_triggered::folded_may_refund_binding` /
//! `folded_may_refund_process`, G-DSL-FOLDED-MAY-OPT-REFUND-AUDIT).
//!
//! Official Q&A Q1818: declining a "you may" [Once Per Turn] effect does not
//! use up the [Once Per Turn]; CR 4.3 §15-14-1-4 counts a use when the player
//! chooses to activate. A clause whose "may" is folded into its first pick has
//! no outer confirm, so the lowering inserts `if binding_absent: <pick> then
//! refund_opt` right after that pick. Declining an optional COST
//! (`cost: true`) of an activated effect still spends the use (BT24-026 /
//! P-250, owner ruling #20), and a tail that runs without the pick ("You may
//! X. Then, Y", Q1643) means the effect was activated: neither is rewritten.
//!
//! The rule-level behaviour is pinned in
//! `tests/rules_conformance/ch15_14_effect_icons.rs`.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTriggeredClause};
use digimon_engine::dsl_cards::lower_triggered::{
    folded_may_refund_binding, folded_may_refund_process, then_may_refund_bindings,
    then_may_refund_process,
};

fn clause(yaml: &str) -> CompiledTriggeredClause {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("test YAML parses");
    let card = digimon_dsl::compile::compile(&spec).expect("test YAML compiles");
    card.effects
        .into_iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("one triggered clause")
}

fn card(clause_yaml: &str) -> String {
    let clause_yaml = clause_yaml
        .replace("  - when:", "  - summary: \"test clause\"\n    when:")
        .replace("          bind_as:", "          prompt: \"pick\"\n          bind_as:");
    format!(
        "card: FOLD\nname: Foldmon\nkind: digimon\nlevel: 4\ncolor: [red]\ncost: 5\ndp: 5000\neffects:\n{clause_yaml}"
    )
}

/// "[On Play] [Once Per Turn] You may delete 1 of your opponent's Digimon."
const FOLDED_MAY: &str = r#"
  - when: on_play
    once_per_turn: true
    optional: true
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          filter: { kind: digimon }
      - delete_permanent: { target: victim }
"#;

#[test]
fn folded_may_pick_gets_a_refund_after_the_pick() {
    let c = clause(&card(FOLDED_MAY));
    assert_eq!(folded_may_refund_binding(&c).as_deref(), Some("victim"));
    let steps = folded_may_refund_process(&c).expect("rewritten");
    assert_eq!(steps.len(), 2, "the pick, then the refund gate over the rest");
    match &steps[0] {
        CompiledStep::SelectOpponentPermanent {
            continue_on_decline,
            ..
        } => assert!(*continue_on_decline, "a PASS must reach the refund"),
        other => panic!("expected the pick first, got {other:?}"),
    }
    match &steps[1] {
        CompiledStep::If {
            condition,
            then,
            else_branch,
        } => {
            assert_eq!(condition.binding_absent.as_deref(), Some("victim"));
            assert_eq!(then, &vec![CompiledStep::RefundOpt]);
            assert!(
                matches!(else_branch.as_slice(), [CompiledStep::DeletePermanent { .. }]),
                "the rest of the body runs only after a pick: {else_branch:?}"
            );
        }
        other => panic!("expected the refund `if` after the pick, got {other:?}"),
    }
}

#[test]
fn optional_cost_pick_is_not_rewritten() {
    // "By trashing 1 card in your hand, ..." authored as an optional cost.
    let c = clause(&card(
        r#"
  - when: on_play
    once_per_turn: true
    optional: true
    process:
      - select_hand:
          of: you
          bind_as: cost_card
          optional: true
          cost: true
          filter: {}
      - trash_from_hand_by_index: { of: you, hand_index: cost_card }
"#,
    ));
    assert_eq!(folded_may_refund_binding(&c), None);
}

#[test]
fn tail_that_runs_without_the_pick_is_not_rewritten() {
    // "You may delete 1 of your opponent's Digimon. Then, <Draw 1>." (Q1643)
    let c = clause(&card(
        r#"
  - when: on_play
    once_per_turn: true
    optional: true
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          filter: { kind: digimon }
      - delete_permanent: { target: victim }
      - draw: { of: you, count: 1 }
"#,
    ));
    assert_eq!(folded_may_refund_binding(&c), None);
}

#[test]
fn binding_present_branch_counts_as_dependent() {
    let c = clause(&card(
        r#"
  - when: on_play
    once_per_turn: true
    optional: true
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          filter: { kind: digimon }
      - if:
          condition: { binding_present: victim }
          then:
            - delete_permanent: { target: victim }
            - draw: { of: you, count: 1 }
"#,
    ));
    assert_eq!(folded_may_refund_binding(&c).as_deref(), Some("victim"));
}

#[test]
fn activation_type_main_clause_is_not_rewritten() {
    let main = FOLDED_MAY.replace("  - when: on_play
", "  - when: main
");
    assert!(main.contains("when: main"));
    assert_eq!(folded_may_refund_binding(&clause(&card(&main))), None);
}

#[test]
fn outer_prompt_non_optional_uncapped_or_self_refunding_clauses_are_not_rewritten() {
    let outer = FOLDED_MAY.replace("    optional: true\n    process:", "    optional: true\n    outer_prompt: true\n    process:");
    assert_eq!(folded_may_refund_binding(&clause(&card(&outer))), None, "outer_prompt");
    let mandatory = FOLDED_MAY.replace("    optional: true\n    process:", "    process:");
    assert_eq!(folded_may_refund_binding(&clause(&card(&mandatory))), None, "non-optional clause");
    let uncapped = FOLDED_MAY.replace("    once_per_turn: true\n", "");
    assert_eq!(folded_may_refund_binding(&clause(&card(&uncapped))), None, "no per-turn cap");
    let refunding = format!(
        "{}      - if:\n          condition: {{ binding_absent: victim }}\n          then:\n            - refund_opt: {{}}\n",
        FOLDED_MAY.trim_end_matches('\n').to_string() + "\n"
    );
    assert_eq!(folded_may_refund_binding(&clause(&card(&refunding))), None, "authored refund_opt");
}

/// [Main] / [Counter] clauses (EX12-013#0, EX12-017#3, EX12-043#0) are
/// excluded: their activation is the player's declaration.
///
/// Every production clause the rewrite applies to, reviewed by hand against
/// its printed text (each is a whole-effect "may" whose pass leaves nothing to
/// process). A clause entering or leaving this list is a behaviour change and
/// must be reviewed against Q1818 / Q1643.
const REVIEWED: &[&str] = &[
    // Light Hybrid: both printed inherited clauses are whole-effect "you may"
    // with no independent Then tail (Q1818). Their card tests prove a decline
    // preserves OPT for a later attack, while successful use spends it.
    "BT17-020#1", "BT18-094#1",
    "AD1-017#2", "BT13-008#1", "BT21-001#0", "BT13-012#1", "BT21-009#3", "BT21-042#0", "BT21-044#1",
    "BT21-044#2", "BT21-082#2", "BT22-023#1", "BT24-001#0", "BT24-051#2", "BT24-067#3",
    "BT25-025#3", "BT25-039#3", "BT25-052#3", "BT25-053#3", "BT25-055#1", "BT25-055#2",
    "EX11-017#2", "EX12-045#1", "EX12-046#2",
    "EX12-065#1", "EX12-074#0", "EX9-033#3", "P-207#2",
    // P-176's whole-effect may: p_176.rs tests decline preserving OPT and
    // successful activation spending it until the next turn.
    "P-176#0",
];

#[test]
fn production_inventory_matches_the_reviewed_list() {
    let registry = digimon_engine::dsl_registry::from_embedded_cached().expect("embedded pack");
    let mut found: Vec<String> = Vec::new();
    for (id, card) in registry.iter() {
        for (i, c) in card.effects.iter().enumerate() {
            if let CompiledClause::Triggered(t) = c {
                if folded_may_refund_binding(t).is_some() {
                    found.push(format!("{id}#{i}"));
                }
            }
        }
    }
    found.sort();
    let mut want: Vec<String> = REVIEWED.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(found, want);
}

// ─── "You may A. Then, you may B." (owner-decided 2026-10-21, W13) ─────────

/// "[On Play] [Once Per Turn] You may delete 1 of your opponent's Digimon.
/// Then, you may return 1 of your opponent's Digimon to the hand."
const THEN_MAY: &str = r#"
  - when: on_play
    once_per_turn: true
    optional: true
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          filter: { kind: digimon }
      - delete_permanent: { target: victim }
      - select_opponent_permanent:
          bind_as: bounce
          optional: true
          filter: { kind: digimon }
      - return_to_hand: { target: bounce }
"#;

#[test]
fn then_may_clause_refunds_when_both_halves_are_empty() {
    let c = clause(&card(THEN_MAY));
    assert_eq!(folded_may_refund_binding(&c), None, "two halves: not the single-pick shape");
    assert_eq!(
        then_may_refund_bindings(&c),
        Some(vec!["victim".to_string(), "bounce".to_string()])
    );
    let steps = then_may_refund_process(&c).expect("rewritten");
    assert_eq!(steps.len(), 5, "the body, then the refund gate");
    for i in [0, 2] {
        match &steps[i] {
            CompiledStep::SelectOpponentPermanent {
                continue_on_decline,
                ..
            } => assert!(*continue_on_decline, "a PASS on half {i} must reach the refund"),
            other => panic!("expected a pick at {i}, got {other:?}"),
        }
    }
    match &steps[4] {
        CompiledStep::If { condition, then, .. } => {
            let counts: Vec<_> = condition
                .all_of
                .iter()
                .map(|p| p.binding_count_eq.clone())
                .collect();
            assert_eq!(
                counts,
                vec![Some(("victim".to_string(), 0)), Some(("bounce".to_string(), 0))]
            );
            assert_eq!(then, &vec![CompiledStep::RefundOpt]);
        }
        other => panic!("expected the refund gate last, got {other:?}"),
    }
}

#[test]
fn then_half_with_mandatory_content_is_not_rewritten() {
    // "You may delete .... Then, <Draw 1>." (Q1643 / BT7-079 shape).
    let c = clause(&card(
        r#"
  - when: on_play
    once_per_turn: true
    optional: true
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          filter: { kind: digimon }
      - delete_permanent: { target: victim }
      - select_opponent_permanent:
          bind_as: bounce
          optional: true
          filter: { kind: digimon }
      - return_to_hand: { target: bounce }
      - draw: { of: you, count: 1 }
"#,
    ));
    assert_eq!(then_may_refund_bindings(&c), None);
}

/// Production clauses with the "you may A. Then, you may B." shape. A clause
/// entering or leaving this list is a behaviour change to review.
/// BT25-028#2: "you may trash any 4 digivolution cards ... Then, 2 of your
/// Digimon may DNA digivolve" (DCGO `RemoveUse` when nothing executed).
/// EX8-074#4: "you may activate 1 of this Digimon's [When Digivolving]
/// effects", inlined as "You may suspend 1 Digimon. Then, you may delete ..."
/// (DCGO `isOptional: true`: declining keeps the use).
const REVIEWED_THEN_MAY: &[&str] = &["BT25-028#2", "EX8-074#4"];

#[test]
fn production_then_may_inventory_matches_the_reviewed_list() {
    let registry = digimon_engine::dsl_registry::from_embedded_cached().expect("embedded pack");
    let mut found: Vec<String> = Vec::new();
    for (id, card) in registry.iter() {
        for (i, c) in card.effects.iter().enumerate() {
            if let CompiledClause::Triggered(t) = c {
                if then_may_refund_bindings(t).is_some() {
                    found.push(format!("{id}#{i}"));
                }
            }
        }
    }
    found.sort();
    assert_eq!(found, REVIEWED_THEN_MAY);
}
