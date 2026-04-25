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

#[test]
fn opponent_effect_return_to_hand_does_not_bounce_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        let _ = ctx.return_to_hand(progress);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Progress attacker must survive opponent-effect return-to-hand"
    );
    assert!(
        r.game.players[0].hand.is_empty(),
        "no card returned to hand"
    );
}

#[test]
fn opponent_effect_return_to_deck_does_not_bounce_progress_attacker() {
    use digimon_engine::enums::StackPosition;
    let (mut r, progress, _opp) = setup_progress_attacker();
    let deck_size_before = r.game.players[0].deck.len();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        let _ = ctx.return_to_deck(progress, StackPosition::Bottom);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Progress attacker must survive opponent-effect return-to-deck"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_size_before,
        "deck size unchanged"
    );
}

#[test]
fn opponent_effect_de_digivolve_does_not_pop_progress_attacker_stack() {
    // Build a Progress carrier with two stack sources so de_digivolve has
    // something to pop. We layer a second source manually because Phase B
    // doesn't depend on the digivolve action path.
    use digimon_engine::card_source::CardSource;
    let mut r = DebugRunner::builder()
        .add_card(fighter("PROG", 6000, vec![Keyword::Progress]))
        .add_card(fighter("BOTTOM", 2000, vec![]))
        .add_card(fighter("OPP", 4000, vec![]))
        .start();
    let progress = r.place_on_field(0, "PROG", None);
    let _opp = r.place_on_field(1, "OPP", None);
    // Inject a second card under the top so the stack has 2 sources.
    {
        let bottom_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "BOTTOM")
            .unwrap();
        let next = r.game.next_card_index();
        let bottom_card = CardSource::new(bottom_idx, 0, next);
        let perm = &mut r.game.players[0].battle_area[progress.index as usize];
        perm.card_sources.insert(0, bottom_card);
    }
    let stack_size_before = r.game.players[0].battle_area[progress.index as usize]
        .card_sources
        .len();

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
    r.game.set_effect_source_player_for_test(Some(1));
    let popped = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.de_digivolve(progress, None, Some(1))
    };
    r.game.set_effect_source_player_for_test(None);

    assert_eq!(popped, 0, "de_digivolve must report 0 pops on Progress carrier");
    assert_eq!(
        r.game.players[0].battle_area[progress.index as usize]
            .card_sources
            .len(),
        stack_size_before,
        "Progress attacker stack must be unchanged"
    );
}
