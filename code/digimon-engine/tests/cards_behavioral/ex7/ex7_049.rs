//! EX7-049 Metallicdramon
//!
//! This pass covers the On Play/When Attacking De-Digivolve 4 clause. The
//! digivolve-prevention and replacement play clauses need later primitives.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML parses and compiles")
        .dsl_card("BT24-011")
        .expect("BT24-011 YAML parses and compiles")
        .dsl_card("BT21-024")
        .expect("BT21-024 YAML parses and compiles")
        .build()
}

#[test]
fn ex7_049_has_on_play_and_when_attacking_dedigivolve_clause() {
    let runner = runner();
    let card = runner.compiled_card("EX7-049").expect("EX7-049 compiled");
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnPlay)
                && triggered.when.contains(&CompiledTiming::WhenAttacking)
    )));
}

#[test]
fn ex7_049_on_play_dedigivolves_opponent_stack() {
    let mut runner = runner();
    let opponent = runner.place_stack(1, &["BT24-011", "BT21-024"]);

    let metallic = runner.place_on_field(0, "EX7-049", None);
    runner.fire_on_play(0, metallic.index as usize);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .auto_resolve()
        .expect("EX7-049 de-digivolve target resolves");

    assert_eq!(
        runner.game.player(1).battle_area[opponent.index as usize]
            .card_sources
            .len(),
        1
    );
}
