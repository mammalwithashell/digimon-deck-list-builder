//! EX3-008 Flamedramon — Lv.4 Red, DP 5000, Cost 5. Armor Form / Free.
//! Its inherited [End of Your Turn] DNA-digivolve is the judge-quiz Q30
//! driver (pinned end-to-end in `judge_quiz/c_declare_then_pay.rs`).

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex3_008_compiles_with_wd_choice_and_inherited_eot_dna() {
    let r = DebugRunner::builder()
        .dsl_card("EX3-008")
        .expect("EX3-008 YAML parses and compiles")
        .build();
    let card = r.compiled_card("EX3-008").expect("present in pack");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "[When Digivolving] two-effect choice clause compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::EndOfYourTurn)
                    && t.optional
        )),
        "inherited [End of Your Turn] DNA-digivolve clause compiles"
    );
}
