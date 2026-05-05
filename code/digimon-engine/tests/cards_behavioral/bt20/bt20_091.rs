//! BT20-091 Cool Boy - Tamer, White, LIBERATOR.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT20-091")
        .expect("BT20-091 YAML loads")
        .memory(10)
        .start()
}

#[test]
fn bt20_091_has_printed_metadata() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    assert_eq!(card.name, "Cool Boy");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "LIBERATOR"));
}

#[test]
fn bt20_091_has_rk_play_or_digivolve_suspend_draw_memory_clause() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    && triggered.when.contains(&CompiledTiming::OnDigivolve) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Royal Knight observer exists");
    assert!(clause.optional);
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::Suspend { .. })));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::Draw { count: 1, .. })));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::GainMemory(1))));
}

#[test]
fn bt20_091_security_plays_self() {
    let runner = runner();
    let card = runner.compiled_card("BT20-091").expect("BT20-091 present");
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnSecurity)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::PlayFromSecurity))
        }
        _ => false,
    }));
}

#[test]
#[ignore = "pending: RK-G004 — would-leave Royal Knight observer into optional Omekamon hand play"]
fn bt20_091_opponent_turn_may_play_omekamon_when_royal_knight_would_leave() {
    panic!("requires would-leave observer that plays a selected hand card without cancelling leave");
}
