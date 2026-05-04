//! EX8-051 Proganomon
//!
//! Printed text:
//! - <Collision> <Piercing> <Fragment (3)>
//! - Inherited: When effects trash this card from a [Mineral] or [Rock] trait
//!   Digimon's digivolution cards, <De-Digivolve 1> 1 opponent Digimon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-051")
        .expect("EX8-051 YAML parses and compiles")
        .build()
}

#[test]
fn ex8_051_has_collision_piercing_and_fragment_three() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX8-051", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Collision));
    assert!(runner.game.has_keyword(handle, Keyword::Piercing));
    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}

#[test]
fn ex8_051_inherited_source_trash_dedigivolves_one_opponent_digimon() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-051")
        .expect("EX8-051 YAML parses and compiles")
        .dsl_card("BT24-011")
        .expect("BT24-011 YAML parses and compiles")
        .dsl_card("BT21-024")
        .expect("BT21-024 YAML parses and compiles")
        .add_card(host)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "EX8-051");
    let opponent = runner.place_stack(1, &["BT24-011", "BT21-024"]);
    assert_eq!(
        runner.game.player(1).battle_area[opponent.index as usize]
            .card_sources
            .len(),
        2
    );

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .auto_resolve()
        .expect("EX8-051 target selection resolves");

    assert_eq!(
        runner.game.player(1).battle_area[opponent.index as usize]
            .card_sources
            .len(),
        1,
        "EX8-051 inherited effect must De-Digivolve 1 after this exact source is trashed"
    );
    assert_eq!(
        runner.game.modifiers.sum(opponent, ModifierType::ChangeDp),
        0,
        "source-trash inherited effect should only de-digivolve, not add unrelated modifiers"
    );
}
