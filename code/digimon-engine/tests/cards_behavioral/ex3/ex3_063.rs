//! EX3-063 Imperialdramon: Dragon Mode — Lv.6 Purple/Red, DP 12000, Cost 12.
//! The full DNA-digivolve + opponent-keeps-1 + Partition interaction is the
//! judge-quiz Q30 pin (`judge_quiz/c_declare_then_pay.rs`), which drives
//! this card end-to-end from Flamedramon's inherited [EoT] DNA digivolve.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex3_063_compiles_with_dna_path_and_both_clauses() {
    let r = DebugRunner::builder()
        .dsl_card("EX3-063")
        .expect("EX3-063 YAML parses and compiles")
        .build();
    let card = r.compiled_card("EX3-063").expect("present in pack");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::DnaDigivolve)),
        "the DNA Digivolution (purple Lv.5 + red Lv.5 / 0) path compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "[When Digivolving] opponent-keeps-1 clause compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenAttacking) && t.once_per_turn
        )),
        "[When Attacking][Once Per Turn] +2000/Fighter Mode clause compiles"
    );
}
