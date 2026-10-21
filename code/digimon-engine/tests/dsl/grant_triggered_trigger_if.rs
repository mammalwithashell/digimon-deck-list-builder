//! G-DSL-GRANTED-TRIGGER-QUALIFIER: `grant_triggered_effect { trigger_if }`
//! (BT14-044 Palmon's granted "When THIS Digimon becomes suspended") lowers
//! the qualifier onto the compiled step; it is rejected on dynamic grants.

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_dsl::spec::CardSpec;

fn card(grant: &str) -> Result<digimon_dsl::compiled::CompiledCard, String> {
    let yaml = format!(
        r#"
card: X-GRANT
name: Granted Qualifier Test
kind: digimon
level: 3
color: [green]
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - select_opponent_permanent:
          bind_as: t
          filter: {{ kind: digimon }}
          prompt: "pick"
{grant}
"#
    );
    let spec: CardSpec = serde_yml::from_str(&yaml).map_err(|e| e.to_string())?;
    compile(&spec).map_err(|errs| format!("{errs:?}"))
}

#[test]
fn trigger_if_lowers_onto_the_granted_step() {
    let compiled = card(
        r#"      - grant_triggered_effect:
          target: t
          timing: on_suspend
          trigger_if: { event_target_is_source: true }
          expiry: end_of_opponents_turn
          body:
            - lose_memory: 2"#,
    )
    .expect("compiles");
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("triggered clause");
    };
    match &t.process[1] {
        CompiledStep::GrantTriggeredEffect { trigger_if, .. } => {
            assert_eq!(
                trigger_if.as_ref().and_then(|p| p.event_target_is_source),
                Some(true)
            );
        }
        other => panic!("expected GrantTriggeredEffect, got {other:?}"),
    }
}

#[test]
fn trigger_if_defaults_to_none() {
    let compiled = card(
        r#"      - grant_triggered_effect:
          target: t
          timing: on_suspend
          expiry: end_of_opponents_turn
          body:
            - lose_memory: 2"#,
    )
    .expect("compiles");
    let CompiledClause::Triggered(t) = &compiled.effects[0] else {
        panic!("triggered clause");
    };
    assert!(matches!(
        &t.process[1],
        CompiledStep::GrantTriggeredEffect {
            trigger_if: None,
            ..
        }
    ));
}

#[test]
fn trigger_if_is_rejected_on_dynamic_grants() {
    let err = card(
        r#"      - grant_triggered_effect:
          dynamic: true
          target: { of: opponent, zone: [battle_area], kind: digimon }
          timing: on_suspend
          trigger_if: { event_target_is_source: true }
          expiry: end_of_opponents_turn
          body:
            - lose_memory: 2"#,
    )
    .expect_err("dynamic + trigger_if is rejected");
    assert!(err.contains("trigger_if"), "{err}");
}
