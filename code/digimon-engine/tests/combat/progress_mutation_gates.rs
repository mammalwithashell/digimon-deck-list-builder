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
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
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

/// Install a modifier as if `effect_player`'s effect is the source, then
/// drop the source attribution. Wraps the
/// `set_effect_source_player_for_test → EffectContext::add_modifier → clear`
/// boilerplate that every Phase B+ modifier-gate test repeats.
///
/// Tests that exercise other `EffectContext` mutation entry points
/// (`delete_permanent`, `return_to_hand`, `de_digivolve`, `suspend`, etc.)
/// or that need to call `add_dp_modifier` directly to cover its delegate
/// path do not use this helper — they keep the explicit `EffectContext::new`
/// scope so the API under test stays visible at the call site.
fn install_modifier_as(
    r: &mut DebugRunner,
    effect_player: PlayerId,
    target: PermanentHandle,
    modifier: ModifierType,
    value: i32,
    expiry: Expiry,
) {
    r.game.set_effect_source_player_for_test(Some(effect_player));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, effect_player);
        ctx.add_modifier(target, modifier, value, expiry);
    }
    r.game.set_effect_source_player_for_test(None);
}

/// Build a runner with one non-Progress attacker on P0 and one opponent
/// permanent on P1. Used for regression tests that need to confirm the
/// gate predicate evaluates to `false` (gate inactive) outside Progress
/// scope — e.g., to catch a future broadening of `progress_excludes` that
/// would accidentally suppress all opponent-sourced modifiers.
fn setup_plain_attacker() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .add_card(fighter("PLAIN", 4000, vec![])) // no Progress keyword
        .add_card(fighter("OPP", 4000, vec![]))
        .start();
    let plain = r.place_on_field(0, "PLAIN", None);
    let opp = r.place_on_field(1, "OPP", None);
    r.game.pending_attack = Some(PendingAttack {
        attacker: plain,
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
    (r, plain, opp)
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

#[test]
fn opponent_effect_suspend_does_not_suspend_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    // Confirm starting state: attacker is unsuspended (the fake PendingAttack
    // does not flip is_suspended; placement defaults to unsuspended).
    assert!(
        !r.game.players[0].battle_area[progress.index as usize].is_suspended,
        "precondition: attacker starts unsuspended"
    );

    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.suspend(progress);
    }
    r.game.set_effect_source_player_for_test(None);

    assert!(
        !r.game.players[0].battle_area[progress.index as usize].is_suspended,
        "Progress attacker must not be suspended by opponent effect"
    );
}

#[test]
fn opponent_effect_negative_dp_does_not_apply_to_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_dp_modifier(progress, -3000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    let dp_sum = r.game.modifiers.sum(progress, ModifierType::ChangeDp);
    assert_eq!(
        dp_sum, 0,
        "Progress attacker must not receive opponent-effect -DP modifier; \
         got accumulated ChangeDp = {}",
        dp_sum
    );
}

#[test]
fn opponent_effect_positive_dp_does_not_apply_to_progress_attacker() {
    // DCGO-faithful: Progress.cs's SkillCondition is `IsOpponentEffect(...)` —
    // a pure source-side check. CanNotBeAffected gates regardless of sign,
    // including positive DP grants. Flipped from the Phase B precedent that
    // let positive buffs through; see plan
    // docs/superpowers/plans/2026-04-24-progress-gate-broaden-modifier-scope.md.
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 1);
        ctx.add_dp_modifier(progress, 1000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    let dp_sum = r.game.modifiers.sum(progress, ModifierType::ChangeDp);
    assert_eq!(
        dp_sum, 0,
        "Progress attacker must not receive opponent-effect +DP modifier; \
         got accumulated ChangeDp = {}",
        dp_sum
    );
}

#[test]
fn own_effect_delete_still_removes_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.delete_permanent(progress);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "own-sourced delete must still apply to a Progress attacker"
    );
}

#[test]
fn own_effect_negative_dp_still_applies_to_progress_attacker() {
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.add_dp_modifier(progress, -1000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeDp),
        -1000,
        "own-sourced -DP must still apply to Progress carrier"
    );
}

#[test]
fn rule_driven_delete_still_removes_progress_attacker() {
    // No EffectContext, no script-API mutation — direct Game-level call
    // simulates a rule-driven cleanup (e.g. cost-payment cascade). Source is
    // None; progress_excludes returns false; deletion proceeds.
    let (mut r, progress, _opp) = setup_progress_attacker();
    // effect_source_player stays None.
    r.game.delete_permanent_with_effects(progress);
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "rule-driven (None-source) delete must still remove Progress attacker"
    );
}

#[test]
fn opponent_effect_cannot_unsuspend_does_not_freeze_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(&mut r, 1, progress, ModifierType::CannotUnsuspend, 0, Expiry::EndOfOpponentsTurn);
    assert!(
        !r.game.modifiers.has(progress, ModifierType::CannotUnsuspend),
        "Progress attacker must not be frozen by opponent CannotUnsuspend"
    );
}

#[test]
fn opponent_effect_cannot_attack_does_not_lock_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(&mut r, 1, progress, ModifierType::CannotAttack, 0, Expiry::EndOfTurn);
    assert!(
        !r.game.modifiers.has(progress, ModifierType::CannotAttack),
        "Progress attacker must not pick up opponent-effect CannotAttack lockdown"
    );
}

#[test]
fn opponent_effect_dont_have_dp_does_not_apply_to_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(&mut r, 1, progress, ModifierType::DontHaveDp, 0, Expiry::EndOfAttack);
    assert!(
        !r.game.modifiers.has(progress, ModifierType::DontHaveDp),
        "Progress attacker must not be DontHaveDp-clamped by opponent effect"
    );
}

#[test]
fn opponent_effect_negative_base_dp_does_not_apply_to_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(&mut r, 1, progress, ModifierType::ChangeBaseDp, -2000, Expiry::EndOfTurn);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeBaseDp),
        0,
        "Progress attacker must not receive opponent-effect ChangeBaseDp(-2000)"
    );
}

#[test]
fn opponent_effect_positive_base_dp_also_does_not_apply_to_progress_attacker() {
    // DCGO-faithful: positive base-DP grants from opponents are gated for
    // the same reason positive ChangeDp is gated — CanNotBeAffected is
    // hostility-blind.
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(&mut r, 1, progress, ModifierType::ChangeBaseDp, 1000, Expiry::EndOfTurn);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeBaseDp),
        0,
        "Progress attacker must not receive opponent-effect ChangeBaseDp(+1000) either"
    );
}

#[test]
fn opponent_effect_negative_security_attack_does_not_apply_to_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(&mut r, 1, progress, ModifierType::SecurityAttackChange, -1, Expiry::EndOfTurn);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::SecurityAttackChange),
        0,
        "Progress attacker must not receive opponent-effect SecurityAttackChange(-1)"
    );
}

#[test]
fn opponent_effect_protective_modifier_does_not_apply_to_progress_attacker() {
    // DCGO-faithful: even a notionally-protective modifier (e.g. global
    // "all Digimon can't be deleted by effects this turn" rider from an
    // opponent's option) does not reach the Progress attacker. The gate
    // is purely source-side per Progress.cs SkillCondition, not hostility-
    // classified. Mirrors the positive-DP test's logic.
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(
        &mut r,
        1,
        progress,
        ModifierType::CannotBeDestroyedByEffect,
        0,
        Expiry::EndOfTurn,
    );
    assert!(
        !r.game.modifiers.has(progress, ModifierType::CannotBeDestroyedByEffect),
        "Progress gate is source-side only — opponent-granted protection doesn't pass through"
    );
}

#[test]
fn own_effect_cannot_attack_still_locks_progress_attacker() {
    let (mut r, progress, _opp) = setup_progress_attacker();
    install_modifier_as(&mut r, 0, progress, ModifierType::CannotAttack, 0, Expiry::EndOfTurn);
    assert!(
        r.game.modifiers.has(progress, ModifierType::CannotAttack),
        "own-sourced CannotAttack must still install on Progress carrier"
    );
}

#[test]
fn opponent_effect_negative_dp_applies_to_non_progress_attacker() {
    // Regression guard: if a future change accidentally broadens
    // `progress_excludes` (e.g. drops the Progress-keyword check or the
    // current-attacker gate), this test would start failing — confirming
    // the predicate is still narrow enough to let opponent-sourced
    // modifiers land on a plain attacker. The gate must fire ONLY for
    // Progress carriers, not all attacking permanents.
    let (mut r, plain, _opp) = setup_plain_attacker();
    install_modifier_as(&mut r, 1, plain, ModifierType::ChangeDp, -3000, Expiry::EndOfTurn);
    assert_eq!(
        r.game.modifiers.sum(plain, ModifierType::ChangeDp),
        -3000,
        "opponent-sourced -DP must land on a non-Progress attacker; \
         progress_excludes should return false here (no Keyword::Progress, \
         no ImmunityToOpponentEffects modifier)"
    );
}

#[test]
fn own_effect_positive_dp_still_buffs_progress_attacker() {
    // Sanity: own buffs are not gated. progress_excludes returns false when
    // src == target.player, so own players can still buff their own
    // attacking Progress carrier mid-attack.
    use digimon_engine::enums::{Expiry, ModifierType};
    let (mut r, progress, _opp) = setup_progress_attacker();
    r.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.add_dp_modifier(progress, 2000, Expiry::EndOfTurn);
    }
    r.game.set_effect_source_player_for_test(None);
    assert_eq!(
        r.game.modifiers.sum(progress, ModifierType::ChangeDp),
        2000,
        "own-sourced positive DP must still install on Progress carrier"
    );
}
