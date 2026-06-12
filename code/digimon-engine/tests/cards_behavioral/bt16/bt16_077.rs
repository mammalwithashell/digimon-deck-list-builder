//! BT16-077 Dinobeemon — Lv.5 Purple/Red, DP 8000, Cost 8.
//! The inherited <Partition> copy under Imperialdramon: Dragon Mode is the
//! judge-quiz Q30 board element; the interruptive Partition machinery is
//! pinned by `judge_quiz/c_declare_then_pay.rs` and
//! `keyword_phase_d/partition.rs`.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

#[test]
fn bt16_077_compiles_with_dna_path_and_dual_partition() {
    let r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 YAML parses and compiles")
        .build();
    let card = r.compiled_card("BT16-077").expect("present in pack");
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::DnaDigivolve)),
        "the DNA Digivolution (purple Lv.4 + red Lv.4 / 0) path compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving) && t.optional
        )),
        "[When Digivolving] DNA-only trash-play + Rush clause compiles"
    );
    // The inherited Partition copy is a distinct inherited-scope clause.
    let inherited_partitions = card
        .effects
        .iter()
        .filter(|c| matches!(
            c,
            CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::Partition { scope, .. }
            ) if *scope == CompiledScope::Inherited
        ))
        .count();
    assert_eq!(inherited_partitions, 1, "one inherited <Partition> clause");
}

#[test]
fn bt16_077_grants_raid_and_partition_on_field() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT16-077")
        .expect("BT16-077 loads")
        .build();
    let h = r.place_on_field(0, "BT16-077", None);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Raid), "<Raid> granted");
    assert!(r.game.has_keyword(h, Keyword::Partition), "<Partition> granted");
}
