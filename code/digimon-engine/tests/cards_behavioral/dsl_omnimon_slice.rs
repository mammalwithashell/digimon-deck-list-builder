//! First vertical DSL archetype slice: Nokia / WarGreymon / Omnimon.
//!
//! These tests use real YAML from `cards/_examples` through the embedded DSL
//! pack, then drive the engine behavior through `DebugRunner`.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

fn resolve_first_pending(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a pending selection exists");
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("pending selection resolves");
}

#[test]
fn bt22_084_nokia_compiles_as_memory_play_and_aura_tamer() {
    let card = compiled("BT22-084");
    assert_eq!(card.kind, CompiledCardKind::Tamer);

    let triggered = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(triggered.len(), 3);
    assert!(triggered
        .iter()
        .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]));
    assert!(triggered
        .iter()
        .any(|t| t.when == vec![CompiledTiming::StartOfYourMainPhase, CompiledTiming::OnPlay]));
    assert!(triggered
        .iter()
        .any(|t| t.when == vec![CompiledTiming::OnSecurity]));

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
    )));
}

#[test]
fn bt17_015_wargreymon_compiles_choice_cost_reduction_and_inherited_clause() {
    let card = compiled("BT17-015");
    assert_eq!(card.level, Some(6));
    assert!(card
        .alt_paths
        .iter()
        .any(|path| path.kind == CompiledAltPathKind::Digivolve));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when == vec![CompiledTiming::WhenAttacking]
                && t.once_per_turn
    )));
}

#[test]
fn nokia_on_play_free_plays_agumon_from_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(card_data_from_compiled("BT22-084"))
        .add_card(card_data_from_compiled("BT17-007"))
        .hand(0, &["BT22-084", "BT17-007"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Nokia plays from hand");
    assert!(
        runner.game.pending_selection.is_some(),
        "Nokia's On Play should ask which Agumon/Gabumon to play"
    );

    resolve_first_pending(&mut runner);

    assert_eq!(runner.battle_area_size(0), 2);
    assert_eq!(
        runner.game.players[0].battle_area[1]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT17-007"
    );
    assert_eq!(runner.hand_size(0), 0);
}

#[test]
fn wargreymon_delete_branch_installs_8000_dp_or_less_target_prompt() {
    let mut target = make_test_card("TARGET-ROOKIE", "Target Rookie");
    target.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .add_card(card_data_from_compiled("BT17-015"))
        .add_card(target)
        .hand(0, &["BT17-015"])
        .hand(1, &["TARGET-ROOKIE"])
        .memory(20)
        .start();
    runner.place_on_field(1, "TARGET-ROOKIE", Some(0));

    runner.play(0, 0).expect("WarGreymon plays from hand");
    resolve_first_pending(&mut runner); // Choose the delete branch.

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("delete branch should install a target prompt");
    assert!(pending.prompt.contains("8000 DP"));
    assert_eq!(pending.valid_action_ids.len(), 1);
}
