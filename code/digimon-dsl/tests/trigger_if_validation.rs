//! `trigger_if:` qualifies a trigger-type effect's trigger condition
//! (§15-8-3-1, §15-8-3-4); an activation-type effect (§15-8-4) is declared,
//! not triggered, so the validator rejects it there.
//! G-ENGINE-OBSERVER-TRIGGER-QUALIFIER-NOT-CHECKED-AT-TRIGGER.

use digimon_dsl::raw_rust_registry::StubRegistry;
use digimon_dsl::validator::{validate, ValidationContext};
use digimon_dsl::CardSpec;

fn spec(when: &str) -> CardSpec {
    serde_yml::from_str(&format!(
        "card: PROBE-TRIGGER-IF
name: Trigger If Witness
kind: tamer
color: [red]
cost: 3
effects:
  - when: {when}
    trigger_if:
      event_target_owner: you
    process:
      - gain_memory: 1
"
    ))
    .expect("parses")
}

fn trigger_if_errors(when: &str) -> Vec<String> {
    match validate(&spec(when), &ValidationContext { raw_rust: &StubRegistry::empty() }) {
        Ok(()) => Vec::new(),
        Err(errs) => errs
            .into_iter()
            .filter(|e| e.path.ends_with(".trigger_if"))
            .map(|e| e.message)
            .collect(),
    }
}

#[test]
fn trigger_if_is_rejected_on_activation_type_timings() {
    for when in ["main", "main_from_hand", "main_on_field", "main_from_trash", "[on_play, main]"] {
        let errs = trigger_if_errors(when);
        assert_eq!(errs.len(), 1, "{when}: {errs:?}");
        assert!(
            errs[0].contains("activation-type timing (§15-8-4)")
                && errs[0].contains("trigger condition (§15-8-3-1); `"),
            "{when}: {}",
            errs[0]
        );
    }
}

#[test]
fn trigger_if_is_accepted_on_observer_timings() {
    for when in ["on_any_digimon_played", "on_ally_won_battle", "on_suspend"] {
        assert!(trigger_if_errors(when).is_empty(), "{when}");
    }
}
