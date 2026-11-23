//! G-DSL-IS-SOURCE-CARD — the `is_source_card: bool` predicate leaf admits a
//! subject iff it is (or, for a permanent, is topped by) the effect's own
//! source card. LM-068 Black Sabbath uses it to use "this card" from the
//! trash; here an inline [On Play] `select_own_permanent` shows both polarities
//! filter the field to exactly the source / exactly the others.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn selector_yaml(want: bool) -> String {
    format!(
        r#"
card: TEST-ISC
name: Is Source Selector
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: pick
          optional: true
          filter: {{ is_source_card: {want} }}
          prompt: "Pick a Digimon"
"#
    )
}

fn offered(want: bool) -> usize {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&selector_yaml(want))
        .expect("selector YAML parses")
        .add_card(make_test_card("OTHER", "Other"))
        .deck(0, &["OTHER"; 4])
        .deck(1, &["OTHER"; 4])
        .memory(5)
        .start();
    runner.place_on_field(0, "OTHER", Some(0));
    runner.place_on_field(0, "OTHER", Some(0));
    let sel = runner.place_on_field(0, "TEST-ISC", Some(0));
    runner.fire_on_play(0, sel.index as usize);
    runner
        .game
        .pending_selection
        .as_ref()
        .expect("the optional select installs a pending selection")
        .valid_action_ids
        .len()
}

#[test]
fn is_source_card_true_admits_only_the_source() {
    assert_eq!(offered(true), 1, "only the source Digimon is offered");
}

#[test]
fn is_source_card_false_excludes_the_source() {
    assert_eq!(offered(false), 2, "both other Digimon, not the source");
}
