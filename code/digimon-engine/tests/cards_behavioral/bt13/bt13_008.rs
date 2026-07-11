//! BT13-008 Agumon — Digimon, Lv.3, Red+Yellow, DP 2000, Cost 3, Dinosaur.
//! Attribute: Vaccine.
//!
//! # Card text (code/digimon-engine/cards/bt13/BT13-008.json)
//!
//! ```text
//! [Main] [Once Per Turn] For the turn, 1 of your [Marcus Damon]s is also
//! treated as a 3000 DP Digimon and can't digivolve.
//! ```
//!
//! Inherited Effect:
//! ```text
//! [Your Turn] [Once Per Turn] When one of your red or yellow Tamers becomes
//! suspended, you may delete 1 of your opponent's Digimon with 3000 DP or
//! less.
//! ```
//!
//! Alt-path (xros_req / official Bandai DB): `[Digivolve] [Koromon]: Cost 0`.
//!
//! Official Q&A (world.digimoncard.com): "It causes that card to also be
//! treated as a Digimon with X000 DP. For example, if a Tamer is also
//! treated as a Digimon, it can attack like a standard Digimon, and it will
//! gain inherited effects from cards stacked under it. However, even if a
//! Tamer is played and then treated as a Digimon in the same turn, it can't
//! attack during that turn."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Red/BT13_008.cs
//!
//! - `EffectTiming.None`: `AddSelfDigivolutionRequirementStaticEffect` —
//!   digivolve from [Koromon], cost 0 (alt-path, static rule).
//! - `EffectTiming.OnDeclaration` (own [Main][OPT], hash
//!   "BecomeDigimon_BT13_008"): `CanUseCondition` requires >=1 own
//!   battle-area permanent whose top card name contains "Marcus Damon" (or
//!   "MarcusDamon"). Body: mandatory `SelectPermanentEffect` (maxCount 1,
//!   `canNoSelect: false`) over own Marcus Damons ->
//!   `CardEffectCommons.BecomeDigimonThatCantDigivolve(DP: 3000,
//!   EffectDuration.UntilEachTurnEnd)`. That helper
//!   (`TamerBecomesDigimonThatCanNotDigivolve.cs`) grants THREE effects to
//!   the target, all `UntilEachTurnEnd`: `TreatAsDigimonStaticEffect`,
//!   `ChangeBaseDPStaticEffect(3000)`, and `CanNotDigivolveStaticEffect`.
//! - `EffectTiming.OnTappedAnyone` (inherited [Your Turn][OPT], hash
//!   "Delete_BT13_008"): `CanUseCondition` = own-battle-area + owner's turn +
//!   `CanTriggerWhenPermanentSuspends` where the suspended permanent is an
//!   own Tamer whose colors contain Red or Yellow.
//!   `CanActivateCondition`/`ActivateCoroutine`: if an opponent battle-area
//!   Digimon exists with `DP <= MaxDP_DeleteEffect(3000)`, optional
//!   (`canNoSelect: true`) `SelectPermanentEffect` (`Mode.Destroy`, maxCount
//!   1) deletes it.
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - Main [Main][OPT] activated effect on a Digimon permanent (`main_on_field`)
//! - G1-adjacent: TreatAsDigimon (Tamer treated as a 3000 DP Digimon, for the
//!   turn) + CannotDigivolve modifier grant
//! - B3-adjacent: inherited triggered observer of own-Tamer-suspend events,
//!   red/yellow color gate, DP<=3000 delete
//! - F-OPT: [Once Per Turn] on BOTH the own [Main] clause and the inherited
//!   triggered observer
//! - F-Predicate: dp_lte filter on opponent permanent select
//! - Alt-path: `[Digivolve] [Koromon]: Cost 0`
//!
//! # Clause map
//! | Clause | Timing | Effect |
//! |--------|--------|--------|
//! | 1 (own) | [Main][OPT] | 1 of your [Marcus Damon]s also treated as a 3000 DP Digimon, can't digivolve, for the turn |
//! | 2 (inherited) | [Your Turn][OPT] on_suspend | when own red/yellow Tamer suspends, may delete 1 opp Digimon DP<=3000 |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use digimon_engine::EffectTiming;

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;
use dsl_card_data::compiled;

const CARD_ID: &str = "BT13-008";

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// A [Marcus Damon] Tamer of a chosen color. The card name MUST be exactly
/// "Marcus Damon" so `name_contains: "Marcus Damon"` filters match it.
fn make_marcus(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, "Marcus Damon");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = vec![];
    c
}

/// A generic Tamer of a chosen color whose name is NOT "Marcus Damon".
fn make_tamer(id: &str, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = colors;
    c.traits = vec![];
    c
}

/// A simple Digimon filler with a given DP and color.
fn make_digimon(id: &str, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![color];
    c
}

fn agumon() -> digimon_dsl::compiled::CompiledCard {
    compiled(CARD_ID)
}

/// Standard runner: BT13-008 (Agumon) + a couple of Marcus Damon copies +
/// fillers registered, generous memory. `TOP-DGM` is a placeholder top card
/// so `place_stack` can host BT13-008 as a digivolution SOURCE (needed for
/// the inherited clause, which only fires from `card_sources` cards, not
/// from the top-of-stack card).
fn base_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-008 in embedded DSL pack")
        .add_card(make_marcus("MARCUS-Y", CardColor::Yellow))
        .add_card(make_marcus("MARCUS-R", CardColor::Red))
        .add_card(make_tamer("TAMER-G", vec![CardColor::Green]))
        .add_card(make_digimon("OPP-DGM-3000", 3000, CardColor::Blue))
        .add_card(make_digimon("OPP-DGM-4000", 4000, CardColor::Blue))
        .add_card(make_test_card("TOP-DGM", "TopDgm"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 4])
        .deck(1, &["FILLER"; 4])
        .memory(10)
}

/// Compute the [Main] field-effect activation action id for the permanent at
/// `field_index` (mirrors the P-224 idiom in
/// `tests/cards_behavioral/p/p_224.rs`). Driven through the real
/// `build_action_mask` + `Game::decode_action` path in callers.
fn field_main_action_id(field_index: usize) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_008_metadata_matches_printed() {
    let card = agumon();
    assert_eq!(card.card, "BT13-008");
    assert_eq!(card.name, "Agumon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.cost, Some(3));

    let colors: Vec<_> = card.color.iter().copied().collect();
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Red),
        "Agumon must be Red"
    );
    assert!(
        colors.contains(&digimon_dsl::compiled::CompiledColor::Yellow),
        "Agumon must be Yellow"
    );
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Dinosaur")),
        "Agumon must have the Dinosaur trait; got {:?}",
        card.traits
    );
}

/// Alt-path: `[Digivolve] [Koromon]: Cost 0`.
#[test]
fn bt13_008_has_koromon_alt_path_cost0() {
    let card = agumon();
    let koromon = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(0))
        })
        .expect("must have a Koromon cost-0 digivolve alt-path");
    assert_eq!(koromon.cost, Some(CompiledCost::Literal(0)));
}

/// Exactly two triggered clauses ship: the own [Main][OPT] treat-as clause
/// and the inherited on_suspend delete observer.
#[test]
fn bt13_008_has_exactly_two_triggered_clauses() {
    let card = agumon();
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        2,
        "Two triggered clauses ship: own [Main][OPT] + inherited on_suspend. Got: {:?}",
        triggered
            .iter()
            .map(|t| (t.scope, t.when.clone()))
            .collect::<Vec<_>>()
    );
}

/// Clause 1 (own): `MainOnField` timing, own (FaceUp) scope, [Once Per Turn].
#[test]
fn bt13_008_main_clause_shape() {
    let card = agumon();
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainOnField) => {
                Some(t)
            }
            _ => None,
        })
        .expect("own [Main] clause (MainOnField) must exist");
    assert_eq!(
        main.scope,
        CompiledScope::FaceUp,
        "own [Main] clause is printed as the card's own effect, not inherited"
    );
    assert!(main.once_per_turn, "[Once Per Turn] flag must be set");
}

/// Clause 2 (inherited): `OnSuspend` timing, Inherited scope, optional,
/// [Once Per Turn].
#[test]
fn bt13_008_inherited_on_suspend_clause_shape() {
    let card = agumon();
    let triggered = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("OnSuspend triggered clause must exist");
    assert_eq!(
        triggered.scope,
        CompiledScope::Inherited,
        "clause is printed as 'Inherited Effect' so scope must be Inherited"
    );
    assert!(
        triggered.optional,
        "card text 'you may delete' makes the prompt optional"
    );
    assert!(triggered.once_per_turn, "[Once Per Turn] flag must be set");
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1 [Main][OPT]: condition gating + mask visibility
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with a Marcus Damon on the field, the [Main] activation bit is
/// visible in the action mask (DCGO `CanUseCondition` >=1 Marcus Damon).
#[test]
fn bt13_008_main_action_visible_with_marcus_present() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let _marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "BT13-008 [Main] must be visible in the action mask when a Marcus Damon is present"
    );
}

/// NEGATIVE: with NO Marcus Damon on the field, the [Main] activation bit
/// must NOT be visible in the mask (DCGO `CanUseCondition` gates the whole
/// activation on >=1 own Marcus Damon).
#[test]
fn bt13_008_main_action_hidden_without_marcus() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    // No Marcus Damon on the field.

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "BT13-008 [Main] must be hidden from the action mask with no Marcus Damon present"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause 1 behavioral outcome
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: activating [Main] with exactly one Marcus Damon installs a
/// mandatory selection to pick it (DCGO `canNoSelect: false`, maxCount 1).
#[test]
fn bt13_008_main_installs_marcus_select() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let _marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    runner.game.decode_action(action, 0);

    let pending =
        runner.game.pending_selection.as_ref().expect(
            "a mandatory selection must install: pick the Marcus Damon to treat as a Digimon",
        );
    assert_eq!(
        pending.selecting_player, 0,
        "you select your own Marcus Damon"
    );
    assert!(
        !pending.is_optional,
        "the outer select is mandatory once the [Main] activates (DCGO canNoSelect: false)"
    );
    assert!(
        !pending.valid_action_ids.is_empty(),
        "the Marcus Damon must be a valid target"
    );
}

/// POSITIVE: after picking the Marcus Damon, it is treated as a 3000 DP
/// Digimon and can't digivolve, for the turn.
#[test]
fn bt13_008_main_treats_marcus_as_3000_digimon_cannot_digivolve() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));

    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must not start treated as a Digimon"
    );
    assert!(
        !runner
            .modifiers()
            .has(marcus, ModifierType::CannotDigivolve),
        "Marcus must not start with CannotDigivolve"
    );

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    runner.game.decode_action(action, 0);

    let pick = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, pick)
        .expect("Marcus selection resolves");
    let _ = runner.auto_resolve();

    assert!(
        runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must be treated as a Digimon"
    );
    assert_eq!(
        runner.effective_dp(marcus),
        Some(3000),
        "Marcus must be treated as a 3000 DP Digimon"
    );
    assert!(
        runner
            .modifiers()
            .has(marcus, ModifierType::CannotDigivolve),
        "Marcus must gain CannotDigivolve"
    );
}

/// The treat-as grants are for-the-turn — they revert after end_turn.
#[test]
fn bt13_008_main_grants_expire_at_end_of_turn() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    runner.game.decode_action(action, 0);
    let pick = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, pick).expect("resolves");
    let _ = runner.auto_resolve();

    assert!(runner.modifiers().has(marcus, ModifierType::TreatAsDigimon));
    assert!(runner
        .modifiers()
        .has(marcus, ModifierType::CannotDigivolve));

    runner.game.end_turn(); // P0 -> P1
    runner.game.end_turn(); // P1 -> P0

    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "TreatAsDigimon must expire at end of turn"
    );
    assert!(
        !runner
            .modifiers()
            .has(marcus, ModifierType::CannotDigivolve),
        "CannotDigivolve must expire at end of turn"
    );
}

/// Multi-target select: with TWO Marcus Damons on the field, both are legal
/// picks — the choice of WHICH one is exposed to the player (no
/// auto-selection).
#[test]
fn bt13_008_main_offers_choice_between_two_marcus_damons() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let _marcus_y = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let _marcus_r = runner.place_on_field(0, "MARCUS-R", Some(0));

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    runner.game.decode_action(action, 0);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("selection must install with 2 Marcus Damons present");
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "both Marcus Damons must be selectable targets; got {:?}",
        pending.valid_action_ids
    );
}

/// A generic Tamer (not named "Marcus Damon") must never be selectable as
/// the treat-as target, even though it's an own Tamer on the field.
#[test]
fn bt13_008_main_does_not_offer_non_marcus_tamer() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let _marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let _other_tamer = runner.place_on_field(0, "TAMER-G", Some(0));

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    runner.game.decode_action(action, 0);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("selection must install (Marcus present)");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the Marcus Damon is a legal target, not the generic Tamer; got {:?}",
        pending.valid_action_ids
    );
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 4 — Clause 1 OPT lockout
// ════════════════════════════════════════════════════════════════════════════

/// OPT: after activating [Main] once this turn, the action must no longer
/// appear in the mask (second activation in the same turn is blocked).
#[test]
fn bt13_008_main_opt_lockout_blocks_second_activation_same_turn() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let _marcus_y = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let _marcus_r = runner.place_on_field(0, "MARCUS-R", Some(0));

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    runner.game.decode_action(action, 0);
    let pick = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, pick).expect("resolves");
    let _ = runner.auto_resolve();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "OPT must lock the second [Main] activation in the same turn"
    );
}

/// OPT lockout clears after end_turn cycles back to the controller.
#[test]
fn bt13_008_main_opt_lockout_clears_next_turn() {
    let mut runner = base_runner().start();
    let agumon = runner.place_on_field(0, CARD_ID, Some(0));
    let _marcus_y = runner.place_on_field(0, "MARCUS-Y", Some(0));

    runner.game.enter_main_phase();
    let action = field_main_action_id(agumon.index as usize);
    runner.game.decode_action(action, 0);
    let pick = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, pick).expect("resolves");
    let _ = runner.auto_resolve();

    runner.game.end_turn(); // P0 -> P1
    runner.game.end_turn(); // P1 -> P0
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "OPT lockout must clear on a new turn"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 5 — Clause 2 (inherited [Your Turn][OPT] on red/yellow Tamer
// suspend -> may delete opp Digimon DP<=3000): condition gating
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE condition gate: an own red Tamer suspending on the controller's
/// turn installs the optional delete prompt.
#[test]
fn bt13_008_on_own_red_tamer_suspend_installs_delete_prompt() {
    let mut runner = base_runner().start();
    // Agumon must be a digivolution SOURCE (not the top card) for its
    // inherited effect to be active.
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    assert_eq!(runner.game.turn_player(), 0, "test assumes P0's turn");

    runner.game.suspend(red_tamer);

    let pending = runner.game.pending_selection.as_ref().expect(
        "BT13-008 inherited clause must install a pending selection \
         when an own red/yellow Tamer suspends on the controller's turn",
    );
    assert!(
        pending.is_optional,
        "delete prompt is printed as 'you may delete', so optional"
    );
}

/// POSITIVE condition gate: an own yellow Tamer suspending also installs the
/// prompt (the printed text covers red OR yellow).
#[test]
fn bt13_008_on_own_yellow_tamer_suspend_installs_delete_prompt() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let yellow_tamer = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(yellow_tamer);

    assert!(
        runner.game.pending_selection.is_some(),
        "an own yellow Tamer suspending must also install the delete prompt"
    );
}

/// NEGATIVE condition gate (color): a green own Tamer suspending must NOT
/// install the prompt.
#[test]
fn bt13_008_does_not_fire_for_own_green_tamer() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let green_tamer = runner.place_on_field(0, "TAMER-G", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(green_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "color-restricted clause must not fire for a green own Tamer suspend"
    );
}

/// NEGATIVE condition gate (kind): an own Digimon suspending must NOT
/// install the prompt. The printed text reads "one of your red or yellow
/// **Tamers**" — Digimon suspends are out of scope.
#[test]
fn bt13_008_does_not_fire_when_own_digimon_suspends() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let ally_dgm = runner.place_on_field(0, "OPP-DGM-4000", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(ally_dgm);

    assert!(
        runner.game.pending_selection.is_none(),
        "an own Digimon suspending must NOT trigger the inherited Tamer-suspend clause"
    );
}

/// NEGATIVE condition gate (owner): the OPPONENT'S red/yellow Tamer
/// suspending must NOT install the prompt.
#[test]
fn bt13_008_does_not_fire_when_opponent_tamer_suspends() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let opp_tamer = runner.place_on_field(1, "MARCUS-R", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(opp_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "opponent's Tamer suspending must NOT trigger your inherited clause"
    );
}

/// Active-when gate: it must be the CONTROLLER'S turn ([Your Turn] scoping).
#[test]
fn bt13_008_does_not_fire_on_opponents_turn() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.turn_player_idx = 1;

    runner.game.suspend(red_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "[Your Turn] active_when must gate the clause off on the opponent's turn"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 6 — Clause 2 behavioral outcomes + DP filter
// ════════════════════════════════════════════════════════════════════════════

/// Behavioral: accepting the prompt + picking the legal target deletes the
/// opponent's <=3000 DP Digimon.
#[test]
fn bt13_008_accept_delete_removes_opponent_digimon() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    let opp_before = runner.battle_area_size(1);
    assert_eq!(opp_before, 1);

    runner.game.suspend(red_tamer);

    let (player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("delete prompt must be installed");
        (
            pending.selecting_player,
            *pending
                .valid_action_ids
                .first()
                .expect("at least one legal target"),
        )
    };
    runner
        .game
        .resolve_selection(player, action_id)
        .expect("delete selection resolves");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the opponent Digimon must be deleted"
    );
}

/// Behavioral: declining the prompt leaves the opponent's Digimon in play.
#[test]
fn bt13_008_decline_leaves_opponent_digimon() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    let _opp = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(red_tamer);

    let player = runner
        .game
        .pending_selection
        .as_ref()
        .expect("optional prompt installed")
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline resolves");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "declining the optional prompt must leave the opp Digimon in play"
    );
}

/// DP filter: a 4000 DP opponent Digimon must NOT be an eligible target;
/// only the <=3000 DP one is selectable.
#[test]
fn bt13_008_dp_lte_filter_excludes_high_dp_targets() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    let _low = runner.place_on_field(1, "OPP-DGM-3000", Some(0));
    let _high = runner.place_on_field(1, "OPP-DGM-4000", Some(0));

    runner.game.suspend(red_tamer);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("delete prompt installed");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "dp_lte: 3000 must filter out the 4000 DP opp Digimon; got {:?}",
        pending.valid_action_ids
    );
}

/// Activation feasibility gate: when the opponent has NO Digimon at all, the
/// optional clause must not install a useless prompt.
#[test]
fn bt13_008_does_not_install_prompt_when_opponent_has_no_digimon() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red_tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    // No opponent Digimon.

    runner.game.suspend(red_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "with no eligible targets the optional delete prompt should not install"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// SECTION 7 — Clause 2 OPT lockout
// ════════════════════════════════════════════════════════════════════════════

/// OPT: two own red/yellow Tamer suspends in the same turn must produce only
/// ONE optional delete prompt.
#[test]
fn bt13_008_inherited_opt_lockout_blocks_second_activation_in_same_turn() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red = runner.place_on_field(0, "MARCUS-R", Some(0));
    let yel = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let _o1 = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(red);
    assert!(
        runner.game.pending_selection.is_some(),
        "first own-Tamer suspend must install the prompt"
    );
    let player = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline resolves");

    runner.game.suspend(yel);
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT must lock the second activation in the same turn"
    );
}

/// OPT lockout clears after end_turn cycles back to the controller.
#[test]
fn bt13_008_inherited_opt_lockout_clears_next_turn() {
    let mut runner = base_runner().start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "TOP-DGM"]);
    let red = runner.place_on_field(0, "MARCUS-R", Some(0));
    let _o1 = runner.place_on_field(1, "OPP-DGM-3000", Some(0));

    runner.game.suspend(red);
    let player = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline resolves");

    runner.game.end_turn(); // P0 -> P1
    runner.game.end_turn(); // P1 -> P0

    // Re-suspend for a fresh delete-prompt opportunity next turn.
    runner.game.players[0].battle_area[red.index as usize].is_suspended = false;
    runner.game.suspend(red);

    assert!(
        runner.game.pending_selection.is_some(),
        "OPT lockout must clear next turn — observer fires again"
    );
}
