//! BT4-072 Gogmamon
//!
//! This pass covers inherited [All Turns] +1000 DP and the face-up
//! [Main] Digi-Burst 1 source-cost DP buff.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::ModifierType;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT4-072")
        .expect("BT4-072 YAML parses and compiles")
        .build()
}

#[test]
fn bt4_072_has_inherited_all_turns_dp_aura() {
    let runner = runner();
    let card = runner
        .compiled_card("BT4-072")
        .expect("BT4-072 compiled card present");

    let dp = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            dp_modifier,
            ..
        }) => *dp_modifier,
        _ => None,
    });

    assert_eq!(dp, Some(1000));
}

#[test]
fn bt4_072_has_main_on_field_digi_burst_source_cost_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT4-072")
        .expect("BT4-072 compiled card present");

    let has_digi_burst_clause = card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when == vec![CompiledTiming::MainOnField] =>
        {
            triggered.process.iter().any(|step| match step {
                CompiledStep::SelectOwnSources {
                    target,
                    min,
                    max,
                    bind_as,
                    then,
                    ..
                } => {
                    matches!(target, Some(CompiledBindingRef::Source))
                        && *min == 1
                        && *max == 1
                        && bind_as.as_deref() == Some("burst_source")
                        && then
                            .iter()
                            .any(|tail| matches!(tail, CompiledStep::TrashSelectedSources { .. }))
                        && then
                            .iter()
                            .any(|tail| matches!(tail, CompiledStep::AddDpModifier { .. }))
                }
                _ => false,
            })
        }
        _ => false,
    });

    assert!(
        has_digi_burst_clause,
        "BT4-072 must compile a MainOnField Digi-Burst source-cost clause"
    );
}

#[test]
fn bt4_072_source_contributes_1000_dp_on_both_turns() {
    let mut host = make_test_card("HOST", "Host");
    host.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-072")
        .expect("BT4-072 YAML parses and compiles")
        .add_card(host)
        .build();
    let carrier = runner.place_stack(0, &["BT4-072", "HOST"]);

    assert_eq!(runner.game.source_dp_contribution(carrier, 0), 1000);
    runner.end_turn();
    assert_eq!(runner.game.source_dp_contribution(carrier, 0), 1000);
}

#[test]
fn bt4_072_digi_burst_trashes_self_source_then_buffs_own_digimon() {
    let mut source = make_test_card("BURST-SRC", "Burst Source");
    source.dp = Some(2000);
    let mut other_source = make_test_card("OTHER-SRC", "Other Source");
    other_source.dp = Some(1000);
    let mut other_host = make_test_card("OTHER-HOST", "Other Host");
    other_host.dp = Some(3000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-072")
        .expect("BT4-072 YAML parses and compiles")
        .add_card(source)
        .add_card(other_source)
        .add_card(other_host)
        .build();
    let carrier = runner.place_stack(0, &["BURST-SRC", "BT4-072"]);
    let other = runner.place_stack(0, &["OTHER-SRC", "OTHER-HOST"]);

    assert!(
        runner.game.activate_field_main(0, carrier.index as usize),
        "BT4-072 MainOnField Digi-Burst effect should activate"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        }),
        "Digi-Burst must expose the source-trash cost as a source selection"
    );

    let burst_pick =
        encode_source_select(carrier.index as u16, 0).expect("carrier source pick action");
    let other_pick = encode_source_select(other.index as u16, 0).expect("other source pick action");
    let source_selection = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Digi-Burst source selection should be pending");
    assert!(
        source_selection.valid_action_ids.contains(&burst_pick),
        "BT4-072 should allow trashing a source from under itself"
    );
    assert!(
        !source_selection.valid_action_ids.contains(&other_pick),
        "BT4-072 Digi-Burst must not allow sources under another Digimon"
    );
    runner
        .execute_action(0, burst_pick)
        .expect("Digi-Burst source selection resolves");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "after paying Digi-Burst, BT4-072 must expose the DP target choice"
    );
    runner
        .auto_resolve()
        .expect("BT4-072 DP target selection resolves");

    assert_eq!(
        runner.game.player(0).battle_area[carrier.index as usize]
            .card_sources
            .len(),
        1,
        "Digi-Burst source should be trashed from under BT4-072"
    );
    assert_eq!(runner.game.player(0).trash.len(), 1);
    assert_eq!(
        runner.game.modifiers.sum(carrier, ModifierType::ChangeDp),
        2000,
        "BT4-072 Digi-Burst should apply +2000 DP to the chosen own Digimon"
    );
}
