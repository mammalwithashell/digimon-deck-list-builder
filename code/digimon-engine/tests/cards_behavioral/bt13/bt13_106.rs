//! BT13-106 Odin's Breath — Option, Yellow, Cost 5.
//!
//! # Card text (code/digimon-engine/cards/bt13/BT13-106.json +
//! data/card_bundles/BT13-106.md — authoritative, verbatim)
//!
//! effect_description_eng:
//!   "When an effect trashes this card from the security stack, activate
//!   this card's [Main] effect.
//!   [Main] 1 of your opponent's Digimon gets -3000 DP until the end of your
//!   opponent's turn. Then, if there're 6 or fewer total cards in both
//!   players' security stacks, all of your opponent's Digimon gain
//!   <Security A. -1> (This Digimon checks 1 fewer security card.) until the
//!   end of your opponent's turn."
//!
//! inherited_effect_description_eng:
//!   "Security Effect [Security] Activate this card's [Main] effects."
//!
//! Official Q&A: "No, you can't. You can only activate the 'when an effect
//! trashes this card from the security stack' [effect] when this card is
//! directly trashed from the security stack."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Yellow/BT13_106.cs
//!
//! DCGO registers THREE clause builders funneling into the same [Main] body:
//!   - `EffectTiming.OnDiscardSecurity` (CanTriggerOnTrashSelfSecurity):
//!     re-runs `OptionMainEffect(card)`. -> `when: on_discard_security`.
//!   - `EffectTiming.OptionSkill` ([Main] from hand): mandatory
//!     SelectPermanentEffect over 1 opponent Digimon (silently no-ops via
//!     `HasMatchConditionPermanent` when none exists) ->
//!     `ChangeDigimonDP(-3000, UntilOpponentTurnEnd)`. THEN, unconditionally:
//!     `if (Owner.SecurityCards.Count + Enemy.SecurityCards.Count <= 6)` ->
//!     mass `ChangeDigimonSAttackPlayerEffect(-1, UntilOpponentTurnEnd)` over
//!     ALL opponent battle-area Digimon. -> `when: main_from_hand`.
//!   - `EffectTiming.SecuritySkill` ([Security]):
//!     `AddActivateMainOptionSecurityEffect` re-runs the same [Main] body. ->
//!     `scope: inherited, when: on_security`.
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - F5-adjacent  OnDiscardSecurity re-activates [Main] (not a normal
//!   security check).
//! - F5-adjacent  mandatory delete/debuff select gated on candidate
//!   existence.
//! - D1  turn-scoped DP debuff (`add_dp_modifier`, `end_of_opponents_turn`).
//! - G-DSL-TOTAL-SECURITY-COUNT-PREDICATE  conditional mass `<Security A.
//!   -1>` grant gated on total (both players') security count <= 6.
//! - E1  main_from_hand / on_security body duplication ([Security] Activate
//!   [Main]).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT13-106";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Green];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

/// A YELLOW Digimon for P0's own field — satisfies BT13-106's Option color
/// requirement (playing a Yellow Option requires a Yellow Digimon/Tamer on
/// the controller's own field or breeding area; DTCG color-match rule,
/// `option_color_match_available`).
fn make_yellow_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn odins_breath_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-106 YAML parses and compiles")
        .add_card(make_digimon("OPP-A", 4))
        .add_card(make_digimon("OPP-B", 5))
        .add_card(make_yellow_digimon("YELLOW-HOST", 3))
        .add_card(make_test_card("FILLER", "Filler Security Card"))
}

fn battle_area_len(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player].battle_area.len()
}

/// Drive `play_option_from_hand`; BT13-106 is single-mode (no link / no dual
/// mode), so no mode-select prompt is expected.
fn play_odins_breath(runner: &mut DebugRunner) -> OptionPlayResult {
    runner.game.play_option_from_hand(0, 0)
}

// ═══════════════════════════════════════════════════════════════════════════
// §1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_106_compiles_as_yellow_option_cost_5() {
    let runner = odins_breath_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    assert_eq!(compiled.card, CARD_ID);
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(5));
    assert!(compiled.color.iter().any(|c| *c == CompiledColor::Yellow));
}

#[test]
fn bt13_106_has_on_discard_security_clause_with_dp_and_conditional_security_attack() {
    let runner = odins_breath_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnDiscardSecurity] => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT13-106 must have an on_discard_security triggered clause");

    assert!(
        clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectOpponentPermanent { .. })),
        "OnDiscardSecurity body must select an opponent permanent"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::AddDpModifier { .. })),
        "OnDiscardSecurity body must apply the -3000 DP modifier"
    );
    assert!(
        clause.process.iter().any(|step| matches!(
            step,
            CompiledStep::If { then, .. }
            if then.iter().any(|s| matches!(
                s,
                CompiledStep::AddModifier { modifier, continuous: true, .. }
                    if modifier == "SecurityAttackChange"
            ))
        )),
        "OnDiscardSecurity body must conditionally install the mass SecurityAttackChange grant"
    );
}

#[test]
fn bt13_106_has_main_from_hand_clause() {
    let runner = odins_breath_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("BT13-106 must have a main_from_hand triggered clause");

    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectOpponentPermanent { .. })));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddDpModifier { .. })));
}

#[test]
fn bt13_106_has_inherited_on_security_clause_activating_main() {
    let runner = odins_breath_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when == vec![CompiledTiming::OnSecurity] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT13-106 must have an inherited on_security clause");

    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectOpponentPermanent { .. })));
    assert!(clause
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddDpModifier { .. })));
}

// ═══════════════════════════════════════════════════════════════════════════
// §2 — [Main] from hand: mandatory -3000 DP select
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_106_main_from_hand_installs_mandatory_dp_target_prompt() {
    let mut runner = odins_breath_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "YELLOW-HOST", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(play_odins_breath(&mut runner), OptionPlayResult::Pending);
    let view = runner
        .pending_selection_view()
        .expect("Main body must install the mandatory DP-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, target.index as u16)));
}

#[test]
fn bt13_106_main_from_hand_applies_minus_3000_dp_until_end_of_opponents_turn() {
    let mut runner = odins_breath_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "YELLOW-HOST", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.enter_main_phase();
    let base_dp = runner.effective_dp(target).expect("target has DP");

    assert_eq!(play_odins_breath(&mut runner), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("DP-target prompt");
    runner
        .execute_action(view.selecting_player, encode_attack(0, target.index as u16))
        .expect("apply -3000 DP");

    assert_eq!(
        runner.effective_dp(target),
        Some(base_dp - 3000),
        "the selected opponent Digimon must get -3000 DP"
    );
}

#[test]
fn bt13_106_main_from_hand_no_prompt_when_opponent_has_no_digimon() {
    let mut runner = odins_breath_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "YELLOW-HOST", Some(0));
    // Opponent controls no Digimon at all — the mandatory select must
    // silently no-op (DCGO HasMatchConditionPermanent gate).
    runner.game.enter_main_phase();

    assert_eq!(play_odins_breath(&mut runner), OptionPlayResult::Trashed);
    assert!(
        runner.pending_selection().is_none(),
        "no DP-target prompt should install with no opponent Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 — [Main]: conditional mass <Security A. -1> (total_security_count <= 6)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_106_main_grants_security_attack_minus_1_when_total_security_lte_6() {
    // P0 (controller) has 3 security, P1 (opponent) has 3 security = 6 total.
    let mut runner = odins_breath_runner()
        .hand(0, &[CARD_ID])
        .security(0, &["FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER"])
        .memory(20)
        .start();
    runner.place_on_field(0, "YELLOW-HOST", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));
    let bystander = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(play_odins_breath(&mut runner), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("DP-target prompt");
    runner
        .execute_action(view.selecting_player, encode_attack(0, target.index as u16))
        .expect("apply -3000 DP");
    runner.auto_resolve().ok();

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(target, ModifierType::SecurityAttackChange),
        -1,
        "6 total security cards: the DP-target opponent Digimon must also gain <Security A. -1>"
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(bystander, ModifierType::SecurityAttackChange),
        -1,
        "the mass grant covers ALL opponent Digimon, not just the DP target"
    );
}

#[test]
fn bt13_106_main_no_security_attack_grant_when_total_security_gt_6() {
    // P0 has 4 security, P1 has 4 security = 8 total > 6.
    let mut runner = odins_breath_runner()
        .hand(0, &[CARD_ID])
        .security(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(20)
        .start();
    runner.place_on_field(0, "YELLOW-HOST", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(play_odins_breath(&mut runner), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("DP-target prompt");
    runner
        .execute_action(view.selecting_player, encode_attack(0, target.index as u16))
        .expect("apply -3000 DP");
    runner.auto_resolve().ok();

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(target, ModifierType::SecurityAttackChange),
        0,
        "8 total security cards (> 6): no <Security A. -1> grant should apply"
    );
    // The DP debuff must still have applied — the two clauses are independent.
    assert!(runner.effective_dp(target).unwrap() < 4000);
}

#[test]
fn bt13_106_main_security_attack_grant_still_fires_when_dp_select_has_no_target() {
    // Total security <= 6 (0 + 0), but the opponent has NO Digimon at all, so
    // the DP-select silently no-ops. The mass Security A. grant is
    // unconditional relative to the DP select (DCGO: two independent `if`
    // blocks) — but with no opponent Digimon on field there is nothing to
    // grant it to; this test asserts the clause does not panic / block and
    // resolves cleanly with no Digimon left ungranted.
    let mut runner = odins_breath_runner().hand(0, &[CARD_ID]).memory(20).start();
    runner.place_on_field(0, "YELLOW-HOST", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(play_odins_breath(&mut runner), OptionPlayResult::Trashed);
    assert!(runner.pending_selection().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// §4 — OnDiscardSecurity: activate [Main] only for effect-driven trash
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_106_effect_trash_from_security_activates_main() {
    let mut runner = odins_breath_runner()
        .security(0, &[CARD_ID])
        .memory(0)
        .start();
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    // Simulate an opponent effect trashing P0's top security card (BT13-106
    // itself) directly — the OnDiscardSecurity path, not a normal attack
    // security check.
    let attacker = runner.place_on_field(1, "OPP-B", Some(0));
    let source_card = runner.game.players[1].battle_area[attacker.index as usize]
        .top_card()
        .handle();

    runner.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(attacker), 1);
        assert!(ctx.trash_top_security(0));
    }
    runner.game.set_effect_source_player_for_test(None);

    let view = runner
        .pending_selection_view()
        .expect("effect-driven security trash must activate [Main]'s mandatory DP-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, target.index as u16)));
}

#[test]
fn bt13_106_normal_attack_security_check_does_not_double_fire_on_discard_security() {
    // A normal attack security check reveals + trashes the security card via
    // the security-check path, NOT via an effect trashing it — so
    // OnDiscardSecurity itself must not additionally fire (Official Q&A:
    // "you can only activate [OnDiscardSecurity] when this card is directly
    // trashed from the security stack" [by an EFFECT]). The card's OWN
    // `on_security` ([Security] Activate [Main]) clause is a *different*
    // clause and DOES fire on a normal security check — that path is
    // covered by `bt13_106_security_check_activates_main_and_applies_dp_debuff`
    // below. This test only asserts the printed effect activates exactly
    // once (not twice, once per clause) on a normal attack.
    let mut runner = odins_breath_runner()
        .security(0, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(1, "OPP-A", Some(0));
    let dp_target = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.enter_main_phase();

    runner.attack_player(attacker, 0, false);

    // [Security] Activate [Main] must fire (the card's own on_security
    // clause) — exactly one mandatory DP-target prompt, not a second
    // independent activation from OnDiscardSecurity.
    let view = runner
        .pending_selection_view()
        .expect("[Security] Activate [Main] must install the DP-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    let base_dp = runner.effective_dp(dp_target).expect("dp_target has DP");
    runner
        .execute_action(
            view.selecting_player,
            encode_attack(0, dp_target.index as u16),
        )
        .expect("resolve the single Main activation");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.security_count(0),
        0,
        "the security card was checked and trashed to trash"
    );
    assert_eq!(runner.game.players[0].trash.len(), 1);
    // Only ONE Main activation occurred: only `dp_target` (the selected
    // target) carries the -3000 DP modifier; nothing double-applied.
    assert_eq!(runner.effective_dp(dp_target), Some(base_dp - 3000));
}

// ═══════════════════════════════════════════════════════════════════════════
// §5 — [Security] Activate [Main] effects (security check reveals the card)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_106_security_check_activates_main_and_applies_dp_debuff() {
    let mut runner = odins_breath_runner()
        .security(0, &[CARD_ID])
        .memory(0)
        .start();
    let target = runner.place_on_field(1, "OPP-A", Some(0));
    let attacker = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.enter_main_phase();

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("[Security] must activate [Main]'s mandatory DP-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    let base_dp = runner.effective_dp(target).expect("target has DP");
    runner
        .execute_action(view.selecting_player, encode_attack(0, target.index as u16))
        .expect("apply -3000 DP via [Security] Activate [Main]");

    assert_eq!(runner.effective_dp(target), Some(base_dp - 3000));
}
