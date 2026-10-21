//! G-DSL-PLAY-FROM-HAND-FREE-SUSPENDED: `play_from_hand_free { suspended }`
//! ("play 1 ... from your hand suspended without paying its memory cost",
//! BT2-047 Argomon) parses and lowers the flag; omitting it keeps `false`.

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_dsl::spec::CardSpec;

fn play_step(suspended_line: &str) -> CompiledStep {
    let yaml = format!(
        r#"
card: X-SUSP
name: Suspended Play Test
kind: digimon
level: 5
color: [green]
cost: 8
dp: 6000
effects:
  - when: when_attacking
    optional: true
    process:
      - select_hand:
          of: you
          bind_as: pick
          optional: true
          filter: {{ kind: digimon }}
          prompt: "pick"
      - play_from_hand_free:
          of: you
          hand_index: pick
{suspended_line}
"#
    );
    let spec: CardSpec = serde_yml::from_str(&yaml).expect("card YAML parses");
    let compiled = compile(&spec).expect("card compiles");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => t.process[1].clone(),
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

#[test]
fn suspended_true_lowers_to_the_compiled_flag() {
    assert!(matches!(
        play_step("          suspended: true"),
        CompiledStep::PlayFromHandFree {
            suspended: true,
            ..
        }
    ));
}

#[test]
fn suspended_defaults_to_false() {
    assert!(matches!(
        play_step(""),
        CompiledStep::PlayFromHandFree {
            suspended: false,
            ..
        }
    ));
}
