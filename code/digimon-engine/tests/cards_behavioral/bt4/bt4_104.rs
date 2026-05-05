//! BT4-104 Blinding Ray - Option, Cost 0, Yellow.
//!
//! # Card text (cards.json)
//!
//! [Main] Trash the top card of your security stack. Then, gain 2 memory.
//!
//! No inherited or security text.
//!
//! # Patterns this test covers
//! - Option card MainFromHand effect.
//! - trash_top_security: { of: you }.
//! - No selection: mandatory sequential security trash, then memory gain.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn blinding_ray() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT4-104")
        .expect("BT4-104 YAML must be present in the embedded DSL pack")
        .add_card(make_test_card("SEC-BOTTOM", "Security Bottom"))
        .add_card(make_test_card("SEC-TOP", "Security Top"))
        .hand(0, &["BT4-104"])
        .security(0, &["SEC-BOTTOM", "SEC-TOP"])
        .memory(0)
        .start()
}

#[test]
fn bt4_104_yaml_loads_as_single_main_from_hand_clause() {
    let runner = blinding_ray();
    let compiled = runner
        .compiled_card("BT4-104")
        .expect("BT4-104 compiled card must be registered");

    assert_eq!(compiled.effects.len(), 1, "only one [Main] clause");

    match &compiled.effects[0] {
        CompiledClause::Triggered(clause) => {
            assert_eq!(clause.scope, CompiledScope::FaceUp);
            assert!(
                clause.when.contains(&CompiledTiming::MainFromHand),
                "Blinding Ray must activate from hand during Main"
            );
            assert!(
                !clause.optional,
                "printed text is mandatory and has no 'you may'"
            );
        }
        other => panic!("BT4-104 effect must be triggered; got {other:?}"),
    }
}

#[test]
fn bt4_104_process_trashes_security_then_gains_memory() {
    let runner = blinding_ray();
    let compiled = runner.compiled_card("BT4-104").expect("compiled card");

    let clause = match &compiled.effects[0] {
        CompiledClause::Triggered(clause) => clause,
        other => panic!("BT4-104 effect must be triggered; got {other:?}"),
    };

    assert!(
        matches!(
            clause.process.first(),
            Some(CompiledStep::TrashTopSecurity { .. })
        ),
        "first step must trash top own security"
    );
    assert!(
        matches!(clause.process.get(1), Some(CompiledStep::GainMemory(2))),
        "second step must gain 2 memory after the security trash"
    );
    assert_eq!(
        clause.process.len(),
        2,
        "BT4-104 must not have hidden choices or extra steps"
    );
}

#[test]
fn bt4_104_main_trashes_exactly_top_own_security_and_gains_2_memory() {
    let mut runner = blinding_ray();
    let memory_before = runner.memory();
    let security_before = runner.security_count(0);
    let own_trash_before = runner.trash_size(0);
    let opponent_security_before = runner.security_count(1);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must fire BT4-104 [Main]");
    runner.auto_resolve().expect("no selections expected");

    assert_eq!(
        runner.security_count(0),
        security_before - 1,
        "exactly one own security card must be trashed"
    );
    assert_eq!(
        runner.security_count(1),
        opponent_security_before,
        "opponent security must be unchanged"
    );
    assert_eq!(
        runner.trash_size(0),
        own_trash_before + 1,
        "trashed security card must move to own trash"
    );

    let trashed_id = runner.game.players[0]
        .trash
        .last()
        .expect("one security card in trash")
        .card_id(&runner.game.card_data);
    assert_eq!(trashed_id, "SEC-TOP", "last security entry is the top card");

    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "memory must increase by 2 after resolving the security trash"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "Blinding Ray has no player-visible choices"
    );
}

#[test]
fn bt4_104_main_with_empty_security_still_gains_2_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-104")
        .expect("BT4-104 YAML must be present in the embedded DSL pack")
        .hand(0, &["BT4-104"])
        .memory(0)
        .start();

    assert_eq!(runner.security_count(0), 0, "precondition: no security");
    let memory_before = runner.memory();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must fire even with empty security"
    );
    runner.auto_resolve().expect("no selections expected");

    assert_eq!(
        runner.security_count(0),
        0,
        "empty security remains empty and does not panic"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "local Then sequencing keeps gain_memory after a no-op security trash"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "no card is trashed from empty security"
    );
}
