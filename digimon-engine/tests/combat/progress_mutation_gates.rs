//! Phase B §B1–B4 — Progress gates every opponent-sourced mutation entry point.
//!
//! Each test sets up a Progress carrier on player 0's field as the active
//! attacker, then has player 1 (opponent) drive a script-API mutation against
//! the carrier via `EffectContext`. The mutation must be skipped because of
//! the Progress gate.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack};

fn fighter(id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Build a runner with one Progress carrier on P0 (attacker) and one
/// opponent permanent on P1. Marks the Progress carrier as the active
/// attacker via a fake `PendingAttack` so `progress_excludes` engages.
/// Returns `(runner, progress_handle, opp_handle)`.
fn setup_progress_attacker() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut r = DebugRunner::builder()
        .add_card(fighter("PROG", 6000, vec![Keyword::Progress]))
        .add_card(fighter("OPP", 4000, vec![]))
        .start();
    let progress = r.place_on_field(0, "PROG", None);
    let opp = r.place_on_field(1, "OPP", None);
    r.game.pending_attack = Some(PendingAttack {
        attacker: progress,
        original_target: AttackTarget::Player(1),
        effective_target: AttackTarget::Player(1),
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        cancelled: false,
        battle_occurred: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
    (r, progress, opp)
}

#[test]
fn opponent_effect_delete_does_not_remove_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();

    // Simulate "P1's effect is resolving" so infer_deletion_cause returns
    // OpponentEffect for a target on P0.
    r.game.set_effect_source_player_for_test(Some(1));

    {
        // Opponent (P1) script API.
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.delete_permanent(progress);
    }

    r.game.set_effect_source_player_for_test(None);

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Progress attacker must survive opponent-effect delete"
    );
}
