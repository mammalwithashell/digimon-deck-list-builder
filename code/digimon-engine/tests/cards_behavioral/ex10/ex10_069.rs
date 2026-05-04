//! EX10-069 Unique Emblem: Gravel Hearts
//!
//! This pass covers the Main hand/trash play choice. Battle-area placement and
//! on-suspend Delay digivolve remain shared option-permanent gaps.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

fn has_select_hand(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::SelectHand { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => has_select_hand(then) || has_select_hand(else_branch),
        _ => false,
    })
}

fn has_select_trash(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::SelectTrash { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => has_select_trash(then) || has_select_trash(else_branch),
        _ => false,
    })
}

#[test]
fn ex10_069_has_main_play_from_hand_or_trash_paths() {
    let runner = DebugRunner::builder()
        .dsl_card("EX10-069")
        .expect("EX10-069 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("EX10-069").expect("EX10-069 compiled");
    let main = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::MainFromHand) =>
        {
            Some(triggered)
        }
        _ => None,
    });
    let main = main.expect("EX10-069 must have MainFromHand clause");
    assert!(has_select_hand(&main.process));
    assert!(has_select_trash(&main.process));
}
