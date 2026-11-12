// A `cost_reduction` with only a `condition` lowers to an unscoped field
// reducer that discounts every card either player plays. "When this card
// would be played, if X, reduce the cost by N" must say `when_playing_this`.
// Regression: BT25-044 Junomon / BT25-018 Apollomon (specialist-league
// game 245, 2026-11-08 -- a free, infinitely repeatable Option use).
use crate::{compile::compile, CardSpec};

fn document(body: &str) -> String {
    format!("card: COST-SCOPE\nname: Cost scope\nkind: digimon\nlevel: 6\ncolor: [yellow]\ncost: 12\ndp: 12000\neffects:\n{body}\n")
}

fn compile_errors(body: &str) -> Option<String> {
    let spec: CardSpec = serde_yml::from_str(&document(body)).expect("fixture parses");
    compile(&spec).err().map(|e| format!("{e:?}"))
}

const CONDITION: &str = "    condition: { security_count_lte: 3 }\n";

#[test]
fn condition_only_cost_reduction_is_rejected() {
    let errors = compile_errors(&format!("  - kind: cost_reduction\n    amount: 5\n{CONDITION}"))
        .expect("condition-only reducer must not compile");
    assert!(errors.contains("when_playing_this"), "{errors}");
}

#[test]
fn self_play_cost_reduction_with_condition_compiles() {
    let body = format!("  - kind: cost_reduction\n    when_playing_this: true\n    amount: 5\n{CONDITION}");
    assert_eq!(compile_errors(&body), None);
}

#[test]
fn active_when_scoped_cost_reduction_still_compiles() {
    let body = "  - kind: cost_reduction\n    amount: 1\n    active_when: { your_turn: true }\n";
    assert_eq!(compile_errors(body), None);
}
