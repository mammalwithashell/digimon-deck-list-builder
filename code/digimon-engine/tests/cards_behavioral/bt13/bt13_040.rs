//! BT13-040 Magnamon

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledZone,
};
use digimon_engine::action::space::{encode_source_select, PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt13_040_has_blocker_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT13-040").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
    )));
}

#[test]
fn bt13_040_when_leaving_uses_native_union_zone_for_hand_or_sources() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT13-040").expect("compiled card");

    let replacement = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { process, .. }) => {
            Some(process)
        }
        _ => None,
    });
    let process = replacement.expect("BT13-040 replacement process");

    assert!(
        !process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "BT13-040 must use native DSL for the optional Veemon play, not raw_rust"
    );
    assert!(
        process.iter().any(|step| matches!(
            step,
            CompiledStep::SelectUnionZone { zones, .. }
                if zones.contains(&CompiledZone::Hand) && zones.contains(&CompiledZone::Material)
        )),
        "BT13-040 must select Veemon from hand or source materials with select_union_zone"
    );
    assert!(
        process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlayUnionBoundFree { .. })),
        "BT13-040 must play the bound hand/material card with play_union_bound_free"
    );
}

#[test]
fn bt13_040_when_leaving_draws_and_may_play_veemon_from_hand_or_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .dsl_card("BT12-021")
        .expect("BT12-021 Veemon must load from embedded DSL pack")
        .deck(0, &["BT12-021"])
        .hand(0, &["BT12-021"])
        .start();

    let magnamon = runner.place_stack(0, &["BT12-021", "BT13-040"]);

    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);

    let pending = runner
        .pending_selection()
        .expect("Magnamon should offer a play-from-hand/source choice after drawing");
    assert!(pending.is_optional, "the Veemon play is optional");
    assert!(
        pending.valid_action_ids.contains(&(PLAY_HAND_START + 0)),
        "Veemon already in hand should be a legal pick"
    );
    let source_action = encode_source_select(0, 0).expect("source action id");
    assert!(
        pending.valid_action_ids.contains(&source_action),
        "Veemon in Magnamon's sources should be a legal pick"
    );

    let selecting_player = pending.selecting_player;
    runner
        .execute_action(selecting_player, source_action)
        .expect("play source Veemon");

    assert!(
        runner.pending_selection().is_none(),
        "source choice should resolve the full would-leave observer"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT12-021"),
        "selected source Veemon should be played"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT13-040"),
        "the original leave event should still proceed"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        2,
        "draw 1 should happen before the optional play, leaving original hand Veemon plus drawn card"
    );
}

#[test]
fn bt13_040_when_leaving_can_decline_veemon_play_and_still_leaves() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .dsl_card("BT12-021")
        .expect("BT12-021 Veemon must load from embedded DSL pack")
        .deck(0, &["BT12-021"])
        .hand(0, &["BT12-021"])
        .start();

    let magnamon = runner.place_stack(0, &["BT12-021", "BT13-040"]);

    runner
        .game
        .delete_permanent_with_cause(magnamon, ReplacementCause::OpponentEffect);

    let selecting_player = runner
        .pending_selection()
        .expect("optional Veemon play prompt")
        .selecting_player;
    runner
        .execute_action(selecting_player, PASS)
        .expect("decline optional play");

    assert!(runner.pending_selection().is_none());
    assert!(
        runner.game.players[0].battle_area.is_empty(),
        "declining the play should not cancel the original leave"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        2,
        "the draw still resolves even when the optional play is declined"
    );
}
