//! BT19-089 Red Card — Option, Red, Cost 2. No traits.
//!
//! # Card text (card image / official Bandai DB bundle — authoritative;
//! cross-checked data/cards.json + DCGO C#)
//!
//! While your opponent has a white Digimon or Tamer, you may ignore this
//! card's color requirements.
//! [Main] Until the end of your opponent's turn, 1 of your Digimon is
//! unaffected by your opponent's Option cards and its DP can't be reduced.
//!
//! Inherited (Security): [Security] Add this card to the hand.
//!
//! Official Q&A: its DP becomes the value it would have when unaffected by
//! the DP reduction — e.g. base 5000, reduced -2000 to 3000, then chosen for
//! this effect's protection → back to 5000 (retroactive recompute, not a
//! one-time snapshot).
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT19/Red/BT19_089.cs`
//!
//! Clause shape (per DCGO):
//! - `EffectTiming.None` `IgnoreColorConditionClass`: `CanUseCondition` =
//!   `HasMatchConditionOpponentsPermanent(card, perm => (perm.IsDigimon ||
//!   perm.IsTamer) && perm.TopCard.CardColors.Contains(White))`.
//!   `CardCondition`: `cardSource == card` (bypass scopes to THIS card).
//! - `EffectTiming.OptionSkill` (`[Main]`): `ActivateClass`,
//!   `SetUpActivateClass(null, ActivateCoroutine, -1, false, ...)` — outer
//!   trigger is NOT optional. `SelectPermanentEffect` over own battle-area
//!   Digimon (`IsPermanentExistsOnOwnerBattleAreaDigimon`), `maxCount:
//!   Math.Min(1, count)`, `canNoSelect: false` (mandatory single-target
//!   select). On a selection: `GainImmuneFromDPMinus` (until opponent turn
//!   end) + a `CanNotAffectedClass` installed via
//!   `UntilOpponentTurnEndEffects` whose `SkillCondition` requires
//!   `cardEffect.EffectSourceCard.Owner == card.Owner.Enemy` AND
//!   `!IsDigimonEffect && !IsTamerEffect` (i.e. scoped to opponent-controlled
//!   NON-Digimon/Tamer effect sources — Option cards, matching printed
//!   text "your opponent's Option cards").
//! - `EffectTiming.SecuritySkill`: `AddThisCardToHand`.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - D3 Color ignore / bypass (`use_requirement`, opponent-scoped, positive
//!   gate on "opponent has a white Digimon or Tamer")
//! - F6 Effect immunity / IMMUNE_FROM_X (`grant_effect_immunity`,
//!   `source_kind: option`, `source_controller: opponent`)
//! - D4-adjacent declarative-style modifier grant (`add_modifier:
//!   ImmuneFromDPMinus`, blanket/`Any` scope — printed text has no "by your
//!   opponent's effects" qualifier, unlike BT16-055's narrower grant)
//! - A4-adjacent Security "Add this card to hand" (`add_this_option_to_hand`)
//!
//! # Faithfulness notes
//!
//! - The color-bypass condition is wired through the top-level
//!   `use_requirement:` field (NOT a `flood_gate` declarative clause) — per
//!   `code/digimon-engine/src/action/mask.rs`
//!   `option_use_requirement_or_color_available` and
//!   `code/digimon-engine/src/game_actions/options.rs` `play_option_core`,
//!   only `effect.option_color_requirement_bypass` (populated from
//!   `use_requirement:` via `lower_triggered.rs`) or a genuinely
//!   player-scoped `ModifierType::IgnoreColorRequirement` gate a hand-play's
//!   color legality. A `kind: flood_gate` clause materializes modifiers onto
//!   BATTLE-AREA permanents only (`Game::materialize_declaratives_full`
//!   never scans the hand zone) — it would be dead weight for a plain
//!   trash-after-use Option like Red Card that never places itself on the
//!   field. `use_requirement:` is therefore the faithful, behaviorally-
//!   verified encoding (see Section 2 below, which plays the card from hand
//!   end-to-end through `play_option_from_hand`).
//! - `ImmuneFromDPMinus` (installed via `add_modifier`, no
//!   `effect_immunity_filter`) defaults to the BROAD `Any` controller scope
//!   in `Game::effective_dp` (`code/digimon-engine/src/combat/dp.rs`) —
//!   suppressing negative `ChangeDp` deltas from ANY source, not just the
//!   opponent's. This matches the printed text: "its DP can't be reduced"
//!   carries no "by your opponent's effects" qualifier (contrast BT16-055,
//!   which IS opponent-scoped and uses the narrower
//!   `grant_narrow_opponent_effect_protection` bundle).
//! - The Option-effect immunity uses `grant_effect_immunity { source_kind:
//!   option, source_controller: opponent }` alone (not the 4-source bundle
//!   seen on EX6-011/BT16-102) because DCGO's `SkillCondition` scopes
//!   strictly to `!IsDigimonEffect && !IsTamerEffect` opponent sources —
//!   i.e. Option cards only, matching the printed "your opponent's Option
//!   cards" (narrower than "isn't affected by your opponent's effects").

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT19-089";

// ── Card-data factories ──────────────────────────────────────────────────

/// A neutral red filler card — used for deck padding.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// An own-side red Digimon — target for the [Main] protection grant.
fn make_own_digimon(card_id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A generic opponent-side Digimon (non-white) — used when the color-bypass
/// condition must be ABSENT.
fn make_opp_digimon_non_white(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// An opponent-side WHITE Digimon — satisfies the color-bypass condition.
fn make_opp_white_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::White];
    c
}

/// An opponent-side WHITE Tamer — also satisfies the color-bypass condition
/// (printed text: "a white Digimon or Tamer").
fn make_opp_white_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c
}

fn base_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-089 YAML must parse and compile")
        .add_card(filler("FILL"))
        .add_card(make_own_digimon("ALLY", "Ally", 5000))
        .add_card(make_opp_digimon_non_white("OPP-BLUE", "Opp Blue"))
        .add_card(make_opp_white_digimon("OPP-WHITE", "Opp White"))
        .add_card(make_opp_white_tamer("OPP-WHITE-TAMER", "Opp White Tamer"))
}

fn encode_perm(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

// ═══════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bt19_089_metadata_option_red_cost_2() {
    let runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("BT19-089 compiled");

    assert_eq!(card.name, "Red Card");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.color, vec![digimon_dsl::compiled::CompiledColor::Red]);
}

#[test]
fn bt19_089_has_use_requirement_for_color_bypass() {
    let runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("BT19-089 compiled");

    assert!(
        card.use_requirement.is_some(),
        "opponent-white-Digimon-or-Tamer color bypass must compile as a use_requirement \
         (flood_gate would be inert — it only materializes onto battle-area permanents, \
         and Red Card never places itself on the field)"
    );
}

#[test]
fn bt19_089_has_main_clause_mandatory_and_security_clause() {
    let runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("BT19-089 compiled");

    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have a MainFromHand clause");
    assert!(
        !main.optional,
        "[Main] target select is mandatory — DCGO canNoSelect: false, \
         SetUpActivateClass(..., isOptional: false, ...)"
    );
    assert_eq!(
        main.scope,
        CompiledScope::FaceUp,
        "[Main] clause must be FaceUp scope"
    );

    let security = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have an OnSecurity clause");
    assert!(
        !security.optional,
        "[Security] Add this card to hand is mandatory — DCGO SecuritySkill has no canNoSelect"
    );
    assert!(
        security
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddThisOptionToHand)),
        "[Security] process must contain AddThisOptionToHand; got {:?}",
        security.process
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 2 — Color bypass (use_requirement) — positive + negative
// ═══════════════════════════════════════════════════════════════════════

/// Negative: opponent has only a non-white Digimon and the controller has no
/// red source on field/breeding → Red Card is NOT playable from hand (normal
/// color match fails AND the bypass condition is absent).
#[test]
fn bt19_089_negative_no_white_opponent_permanent_blocks_off_color_play() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-BLUE", None);
    runner.game.enter_main_phase();

    let mask = digimon_engine::action::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[digimon_engine::action::space::PLAY_HAND_START as usize], 0.0,
        "Red Card must NOT be playable — no white opponent permanent and no \
         own red battle-area/breeding source"
    );

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Invalid,
        "playing Red Card off-color without the bypass condition must be rejected"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "Red Card must remain in hand after an invalid play attempt"
    );
}

/// Positive: opponent has a WHITE Digimon → the color bypass is active and
/// Red Card becomes playable from hand despite the controller having no red
/// permanent.
#[test]
fn bt19_089_positive_opponent_white_digimon_enables_off_color_play() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    let mask = digimon_engine::action::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[digimon_engine::action::space::PLAY_HAND_START as usize], 1.0,
        "Red Card must be playable — opponent has a white Digimon"
    );

    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(
        result,
        OptionPlayResult::Invalid,
        "playing Red Card with the bypass active must be accepted"
    );
}

/// Positive: opponent has a WHITE Tamer (not Digimon) → printed text "white
/// Digimon OR Tamer" — the bypass must also fire for a Tamer.
#[test]
fn bt19_089_positive_opponent_white_tamer_enables_off_color_play() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-WHITE-TAMER", None);
    runner.game.enter_main_phase();

    let mask = digimon_engine::action::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[digimon_engine::action::space::PLAY_HAND_START as usize], 1.0,
        "Red Card must be playable — opponent has a white Tamer"
    );
}

/// Negative: opponent has a NON-white Digimon only → bypass must NOT fire
/// even though the opponent has A Digimon (color must specifically be white).
/// This is the same assertion as the first negative test in this section,
/// phrased as a direct `play_option_from_hand` rejection to double as the
/// "any opponent Digimon is not enough — it must be white" edge case.
#[test]
fn bt19_089_negative_opponent_non_white_digimon_does_not_bypass() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-BLUE", None);
    runner.game.enter_main_phase();

    let mask = digimon_engine::action::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[digimon_engine::action::space::PLAY_HAND_START as usize], 0.0,
        "a non-white opponent Digimon must NOT satisfy the color bypass"
    );
    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Invalid,
        "playing Red Card off-color against a non-white opponent Digimon must be rejected"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 3 — [Main] behavioral: mandatory own-Digimon target select
// ═══════════════════════════════════════════════════════════════════════

/// Negative: no own Digimon on field → the mandatory select has zero
/// candidates and the [Main] effect silently no-ops (no prompt installs).
#[test]
fn bt19_089_main_no_own_digimon_no_prompt() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    // Satisfy the color/play legality via a same-color own Digimon is not
    // available here (no own Digimon at all) — so grant the bypass via an
    // opponent white permanent instead, keeping the controller's battle
    // area empty to exercise the "no own Digimon" no-op path.
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        !fired || runner.pending_selection().is_none(),
        "no own Digimon on field → [Main] must not install a target selection"
    );
}

/// Positive: exactly one own Digimon on field → the [Main] mandatory select
/// installs an OwnField selection with that Digimon as the sole candidate.
#[test]
fn bt19_089_main_one_own_digimon_installs_mandatory_own_field_select() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for BT19-089");

    let view = runner
        .pending_selection_view()
        .expect("[Main] must install a target selection when an own Digimon exists");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        !view.is_optional,
        "[Main] target selection is mandatory — DCGO canNoSelect: false"
    );
    assert_eq!(
        view.valid_action_ids,
        vec![encode_perm(ally)],
        "the single own Digimon must be the sole legal target"
    );
}

/// Positive end-to-end: picking the ally grants BOTH ImmuneFromDPMinus and
/// Option-effect immunity (source_kind: Option, controller: opponent).
#[test]
fn bt19_089_main_selecting_target_grants_dp_protection_and_option_immunity() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    assert!(runner.game.activate_hand_main(0, 0));
    let view = runner
        .pending_selection_view()
        .expect("[Main] selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose ally for protection");
    let _ = runner.auto_resolve();

    assert!(
        runner
            .modifiers()
            .has(ally, ModifierType::ImmuneFromDPMinus),
        "ally must carry ImmuneFromDPMinus after [Main] resolves"
    );
    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Option),
        "ally must be unaffected by opponent (player 1) Option-sourced effects"
    );
}

/// Narrowness: the Option-effect immunity must NOT extend to opponent
/// Digimon-effect sources (printed text scopes strictly to "your opponent's
/// Option cards", unlike the broader 4-source bundle used by EX6-011/
/// BT16-102).
#[test]
fn bt19_089_main_immunity_does_not_extend_to_opponent_digimon_effects() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    assert!(runner.game.activate_hand_main(0, 0));
    let view = runner
        .pending_selection_view()
        .expect("[Main] selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose ally for protection");
    let _ = runner.auto_resolve();

    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Digimon),
        "protection is narrow to Option cards — opponent Digimon-effect sources \
         must NOT be covered"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Tamer),
        "protection is narrow to Option cards — opponent Tamer-effect sources \
         must NOT be covered"
    );
}

/// DP protection is BROAD (`Any` controller scope) — the printed text has no
/// "by your opponent's effects" qualifier on "its DP can't be reduced", so
/// even the controller's OWN DP-reduction must be suppressed. Contrast
/// BT16-055, whose narrower grant leaves the controller's own reduction
/// unaffected.
#[test]
fn bt19_089_main_dp_protection_is_broad_blocks_own_and_opponent_reduction() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    assert!(runner.game.activate_hand_main(0, 0));
    let view = runner
        .pending_selection_view()
        .expect("[Main] selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose ally for protection");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(ally),
        Some(5000),
        "baseline DP before any reduction"
    );

    // OPPONENT (player 1) tries to reduce ally's DP by 2000 — must be blocked.
    {
        let source_card = runner.game.players[0].battle_area[ally.index as usize]
            .top_card()
            .handle();
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut runner.game, source_card, None, 1);
        ctx.add_dp_modifier(ally, -2000, Expiry::EndOfTurn);
    }
    assert_eq!(
        runner.effective_dp(ally),
        Some(5000),
        "opponent DP-reduction must be blocked by ImmuneFromDPMinus"
    );

    // CONTROLLER's OWN effect also tries to reduce ally's DP by 1000 — this
    // printed text has NO opponent-only qualifier, so it must ALSO be blocked.
    {
        let source_card = runner.game.players[0].battle_area[ally.index as usize]
            .top_card()
            .handle();
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut runner.game, source_card, None, 0);
        ctx.add_dp_modifier(ally, -1000, Expiry::EndOfTurn);
    }
    assert_eq!(
        runner.effective_dp(ally),
        Some(5000),
        "controller's OWN DP-reduction must ALSO be blocked — the printed text \
         has no opponent-only qualifier on the DP-reduction clause"
    );
}

/// Official Q&A retroactive-recompute check: a Digimon already reduced by an
/// EARLIER effect, once selected for this card's protection, reverts to its
/// original (unreduced) DP — not merely frozen at the already-reduced value.
#[test]
fn bt19_089_qa_retroactive_recompute_reverts_prior_reduction() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    // Prior opponent DP-reduction of -2000 applied BEFORE Red Card resolves:
    // 5000 base -> 3000 current.
    {
        let source_card = runner.game.players[0].battle_area[ally.index as usize]
            .top_card()
            .handle();
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut runner.game, source_card, None, 1);
        ctx.add_dp_modifier(ally, -2000, Expiry::EndOfTurn);
    }
    assert_eq!(runner.effective_dp(ally), Some(3000));

    assert!(runner.game.activate_hand_main(0, 0));
    let view = runner
        .pending_selection_view()
        .expect("[Main] selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the already-reduced ally for protection");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(ally),
        Some(5000),
        "official Q&A: once protected, DP recomputes to the unaffected value \
         (5000), not the already-reduced value frozen at grant time (3000)"
    );
}

/// The [Main] grant expires at the end of the opponent's turn — DP
/// protection and Option immunity both lapse afterward.
#[test]
fn bt19_089_main_protection_expires_after_opponents_turn_ends() {
    let mut runner = base_runner()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(10)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-WHITE", None);
    runner.game.enter_main_phase();

    assert!(runner.game.activate_hand_main(0, 0));
    let view = runner
        .pending_selection_view()
        .expect("[Main] selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose ally for protection");
    let _ = runner.auto_resolve();

    assert!(runner
        .modifiers()
        .has(ally, ModifierType::ImmuneFromDPMinus));
    assert!(runner
        .game
        .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Option));

    // End the controller's turn (player 0), then the opponent's turn
    // (player 1) — protection lasts "until end of opponent's turn" so it
    // must survive the first end_turn (controller's own) and clear after
    // the opponent's turn fully ends.
    runner.end_turn(); // player 0 -> player 1's turn begins
    assert!(
        runner
            .modifiers()
            .has(ally, ModifierType::ImmuneFromDPMinus),
        "protection must still be active during the opponent's own turn"
    );
    runner.end_turn(); // player 1's turn ends -> back to player 0

    assert!(
        !runner
            .modifiers()
            .has(ally, ModifierType::ImmuneFromDPMinus),
        "ImmuneFromDPMinus must expire once the opponent's turn ends"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Option),
        "Option-effect immunity must expire once the opponent's turn ends"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 4 — [Security] (inherited) — Add this card to hand
// ═══════════════════════════════════════════════════════════════════════

/// Drives the inherited [Security] clause via the real security-check path
/// (attack into the defender's security stack). BT19-089 sits atop the
/// defender's (player 1) security; on flip, it must be added to player 1's
/// hand.
#[test]
fn bt19_089_security_adds_self_to_hand() {
    let mut attacker = filler("ATK");
    attacker.dp = Some(6000);

    let mut runner = base_runner()
        .add_card(attacker.clone())
        .memory(10)
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .security(1, &["BT19-089"])
        .start();

    let attacker_handle = runner.place_on_field(0, "ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: BT19-089 sits in player 1's security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "BT19-089 must leave player 1's security"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "BT19-089 must be added to player 1's hand by add_this_option_to_hand; \
         hand was {:?}",
        runner.game.players[1]
            .hand
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>()
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 5 — Anti-helpers (silence unused-warning across test fork)
// ═══════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = PASS;
    let _ = CardKind::Digimon;
}
