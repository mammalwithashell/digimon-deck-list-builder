//! End-to-end Phase 2a: compile a small synthetic YAML card, register it,
//! and verify the OnPlay triggered process closure actually mutates game
//! memory when invoked.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;

#[test]
fn dsl_triggered_on_play_gain_memory_closure_mutates_memory() {
    // Compile a one-line DSL card from YAML.
    let yaml = r#"
card: DSL-E2E-001
name: Gain 1
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("valid YAML");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DSL-E2E-001", "Gain 1"))
        .hand(0, &["DSL-E2E-001"])
        .build();

    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let card: CardHandle = runner.game.players[0].hand[0].handle();
    let effects = dsl_effect.effects(card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");
    assert_eq!(
        effects[0].timing,
        digimon_engine::enums::EffectTiming::OnPlay,
    );

    let before = runner.game.memory;
    // Invoke the process closure.
    let process = effects[0].process.as_ref().expect("has process closure");
    {
        let mut ctx = EffectContext::new(&mut runner.game, card, None, 0);
        process(&mut ctx);
    }
    let after = runner.game.memory;
    assert_eq!(
        after,
        before + 1,
        "OnPlay gain_memory step should increment memory by 1"
    );
}
