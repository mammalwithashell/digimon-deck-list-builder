//! EX10-063 Close
//!
//! This pass covers the source-trash memory trigger. The start-main hand play
//! branch and trash-to-play fallback need separate hand/trash play routing.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

#[test]
fn ex10_063_source_trash_trigger_suspends_tamer_and_gains_memory() {
    let mut host = make_test_card("EX10-063-HOST", "Mineral Host");
    host.traits.push("Mineral".to_string());
    let source_card = make_test_card("EX10-063-SRC", "Source Card");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-063")
        .expect("EX10-063 YAML parses and compiles")
        .add_card(host)
        .add_card(source_card)
        .memory(0)
        .build();

    let close = runner.place_on_field(0, "EX10-063", None);
    let host = runner.place_on_field(0, "EX10-063-HOST", None);
    let source = runner.push_source(host, "EX10-063-SRC");

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.memory(), 1);
    assert!(runner.game.players[0].battle_area[close.index as usize].is_suspended);
}
