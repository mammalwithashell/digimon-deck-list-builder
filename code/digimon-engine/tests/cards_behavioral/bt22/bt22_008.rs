//! BT22-008 Agumon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Reptile, CS. Attribute: Vaccine.
//! Evo: Lv.2 Red / cost 0; bonus Koromon-OR-(Lv.2 CS) / cost 0.
//!
//! # Card text (cards.json)
//!
//! [On Play] You may return 1 Digimon card with [Greymon], [Garurumon] or
//!     [Omnimon] in its name from your trash to the hand.
//!
//! Inherited:
//!   [End of Your Turn] This Digimon and any of your other Digimon may DNA
//!     digivolve into a Digimon card in the hand.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/Red/BT22_008.cs`
//!
//! Notable DCGO contracts:
//!   - [On Play] outer activate: `SetUpActivateClass(..., -1, isOptional: true, ...)`
//!     and inner `SelectCardEffect.SetUp(canNoSelect: () => true, maxCount: 1, ...)`.
//!     Both layers are opt-in — printed "you may" + "1 card" (not "must
//!     return"). Player may decline both at the trigger and at the prompt.
//!   - [On Play] `CanActivateCondition` short-circuits when no matching
//!     trash card exists (`HasMatchConditionOwnersCardInTrash`); the
//!     pending selection should not install in that case.
//!   - End-of-Your-Turn inherited clause: `SetUpActivateClass(..., isOptional: true, ...)`
//!     consistent with the printed "may DNA digivolve". Implemented as an
//!     `alt_path_registration` declarative (scope: inherited,
//!     trigger: end_of_your_turn, registers: dna_digivolve) per the
//!     RESOLVED-2026-05-02 vocab note in qa/dsl-vocab-gaps.md.
//!   - Self-digivolution-requirement static effect (`EffectTiming.None`):
//!     `permanentCondition = (TopCard.EqualsCardName("Koromon")) ||
//!                           (TopCard.IsLevel2 && TopCard.HasCSTraits)`,
//!     `digivolutionCost: 0`, `ignoreDigivolutionRequirement: false`.
//!     This is NOT redundant with the standard Lv.2 Red cost 0 path
//!     (unlike BT17-007's Koromon block) because it admits any-color
//!     Koromon AND any-color Lv.2-CS cards, widening beyond the printed
//!     Lv.2 Red restriction. We author it as a second `digivolve`
//!     alt-path with an `any_of: [name_is: Koromon, all_of: [level_eq: 2,
//!     trait_has: CS]]` predicate.
//!
//! # Sister card
//!
//! BT17-007 (Agumon, Tai Kamiya). The trash-return body shape and
//! inherited DNA-digivolve registration are identical to BT17-007.
//! Differences:
//!   (a) BT22-008 fires on `on_play` with no Tamer gate; BT17-007 fires
//!       on `start_of_your_main_phase` gated by a Tai-Kamiya tamer;
//!   (b) BT22-008 has a bonus Koromon/Lv.2-CS digivolve alt-path;
//!       BT17-007 has only the standard Lv.2 Red path.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//!   - §5 Structural: two digivolve alt-paths (standard + bonus widened);
//!     on-play triggered clause; inherited `alt_path_registration` for
//!     end-of-your-turn DNA digivolve.
//!   - §5 Condition gating: empty trash → no fire; non-matching trash →
//!     no fire; matching trash → fires.
//!   - §5 Behavioral integrated: resolving the install moves the chosen
//!     Greymon/Garurumon/Omnimon-named card from trash to hand.
//!   - §5 Faithfulness gate: outer trigger AND inner pick are both
//!     optional (printed "you may"; DCGO `isOptional: true` +
//!     `canNoSelect: true`). PASS must be a legal action when the
//!     selection installs.
//!   - OPT: clause has no [Once Per Turn] — out of scope.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT22-008";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn agumon_card_data() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A generic Digimon card with the given id and name; used to seed trash
/// for "return to hand" tests.
fn make_trash_target(card_id: &str, card_name: &str) -> CardData {
    let mut card = make_test_card(card_id, card_name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

/// Push a card with the given `card_id` onto a player's trash zone.
/// `card_id` must already be registered in the runner's card_data store.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            data_idx, player, next,
        ));
}

/// Return the card_ids in a player's hand, in order.
fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Return the card_ids in a player's trash, in order.
fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

// ─── Common runner builders ──────────────────────────────────────────────────

/// Build a runner with Agumon registered and card_data present for the test
/// targets we care about. Both players get a 5-card filler deck so
/// `enter_main_phase` does not deck-out.
fn runner_with_targets() -> DebugRunner {
    let filler = make_test_card("DECK-PAD", "Filler");
    let greymon = make_trash_target("TRASH-GREYMON", "Greymon");
    let garurumon = make_trash_target("TRASH-GARURUMON", "Garurumon");
    let omnimon = make_trash_target("TRASH-OMNIMON", "Omnimon");
    let other = make_trash_target("TRASH-OTHER", "RandomDigimon");

    DebugRunner::builder()
        .add_card(agumon_card_data())
        .add_card(filler)
        .add_card(greymon)
        .add_card(garurumon)
        .add_card(omnimon)
        .add_card(other)
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(3)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt22_008_yaml_compiles_in_embedded_pack() {
    // Smoke: the embedded DSL pack ships BT22-008 with the printed metadata.
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Reptile")),
        "BT22-008 must carry the Reptile trait; got traits={:?}",
        card.traits
    );
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("CS")),
        "BT22-008 must carry the CS trait; got traits={:?}",
        card.traits
    );
}

#[test]
fn bt22_008_has_two_digivolve_alt_paths() {
    // Card prints two digivolve paths:
    //   (1) standard Lv.2 Red / cost 0,
    //   (2) bonus xros_req Koromon-OR-(Lv.2 CS) / cost 0.
    // DCGO encodes (2) as `AddSelfDigivolutionRequirementStaticEffect`
    // — NOT redundant with (1) because (2) admits any-color Koromon
    // AND any-color Lv.2-CS, both of which exceed the standard
    // Lv.2 Red restriction.
    let card = compiled(CARD_ID);
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        2,
        "BT22-008 must expose two digivolve alt-paths (standard Lv.2 Red \
         + bonus Koromon/Lv.2-CS); got {}",
        digivolve_paths.len()
    );
    for p in &digivolve_paths {
        assert_eq!(
            p.cost,
            Some(digimon_dsl::compiled::CompiledCost::Literal(0)),
            "Both digivolve alt-paths must be cost 0"
        );
    }
}

#[test]
fn bt22_008_has_on_play_triggered_clause() {
    let card = compiled(CARD_ID);
    let clause = card.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::OnPlay) {
                return Some(t);
            }
        }
        None
    });
    assert!(
        clause.is_some(),
        "BT22-008 must have a triggered clause at OnPlay"
    );
    let clause = clause.unwrap();
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "On-play clause is face-up only"
    );
    assert!(
        !clause.once_per_turn,
        "On-play clause has no [Once Per Turn]"
    );
    assert!(
        clause.optional,
        "Printed text says 'you may'; outer trigger is opt-in (DCGO isOptional: true)"
    );
}

#[test]
fn bt22_008_has_inherited_dna_digivolve_alt_path_registration() {
    // RESOLVED 2026-05-02 (qa/dsl-vocab-gaps.md BT22-008/BT22-017 entry):
    // inherited end-of-your-turn DNA digivolve is authored as an
    // `alt_path_registration` declarative clause with
    // `kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn`.
    let card = compiled(CARD_ID);
    let registration = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration {
            scope,
            trigger,
            registers,
            ..
        }) if *scope == CompiledScope::Inherited
            && *trigger == CompiledTiming::EndOfYourTurn
            && registers.kind == CompiledAltPathKind::DnaDigivolve =>
        {
            Some(())
        }
        _ => None,
    });
    assert!(
        registration.is_some(),
        "BT22-008 must register an inherited end-of-your-turn DNA digivolve alt-path"
    );
}

#[test]
fn bt22_008_clause_count_matches_card_text() {
    // Card prints exactly two effect clauses:
    //   (1) the [On Play] return-from-trash trigger
    //   (2) the inherited [End of Your Turn] DNA digivolve registration.
    // No other effects, no aura, no cost reduction.
    let card = compiled(CARD_ID);
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let alt_path_regs = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration { .. })
            )
        })
        .count();
    assert_eq!(triggered, 1, "Exactly one triggered clause expected");
    assert_eq!(
        alt_path_regs, 1,
        "Exactly one alt_path_registration clause expected"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating ([On Play])
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt22_008_on_play_no_fire_when_trash_has_no_matching_card() {
    let mut runner = runner_with_targets();
    // Seed trash with a non-matching Digimon only.
    push_to_trash(&mut runner, 0, "TRASH-OTHER");

    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, perm.index as usize);

    // Whether the engine installs an empty/auto-passed selection or skips
    // the trigger entirely is implementation-detail — what we care about
    // is that no card moves out of trash and that no selection is left
    // hanging that requires an out-of-band PASS.
    let _ = runner.auto_resolve();
    assert_eq!(
        hand_ids(&runner, 0),
        Vec::<String>::new(),
        "no matching trash card → nothing returned to hand"
    );
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["TRASH-OTHER".to_string()],
        "non-matching trash card stays put"
    );
}

#[test]
fn bt22_008_on_play_no_fire_with_empty_trash() {
    let mut runner = runner_with_targets();
    // Trash is empty.

    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, perm.index as usize);

    let _ = runner.auto_resolve();
    assert_eq!(
        hand_ids(&runner, 0),
        Vec::<String>::new(),
        "empty trash → nothing returned to hand"
    );
}

#[test]
fn bt22_008_on_play_fires_with_matching_trash() {
    let mut runner = runner_with_targets();
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");

    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, perm.index as usize);

    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "matching trash → trigger must install a Trash selection"
    );
    assert!(
        matches!(kind, Some(SelectionKind::Trash)),
        "selection must target the trash zone; got {:?}",
        kind
    );
}

#[test]
fn bt22_008_on_play_no_fire_with_only_opponent_matching_trash() {
    // Faithfulness pin: the printed text is "from YOUR trash". If only the
    // opponent has a Greymon in trash, no selection should install (DCGO
    // `HasMatchConditionOwnersCardInTrash` is owner-scoped).
    let mut runner = runner_with_targets();
    push_to_trash(&mut runner, 1, "TRASH-GREYMON");

    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, perm.index as usize);

    let _ = runner.auto_resolve();
    assert_eq!(
        hand_ids(&runner, 0),
        Vec::<String>::new(),
        "opponent's matching trash card must not be returnable to controller's hand"
    );
    assert_eq!(
        trash_ids(&runner, 1),
        vec!["TRASH-GREYMON".to_string()],
        "opponent's trash must be untouched"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral integrated
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt22_008_returns_selected_greymon_from_trash_to_hand() {
    let mut runner = runner_with_targets();
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");
    push_to_trash(&mut runner, 0, "TRASH-OTHER");

    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, perm.index as usize);

    // First non-PASS eligible action should select the matching Greymon.
    let action = {
        let pending = runner
            .pending_selection()
            .expect("Trash selection installed");
        let target = pending
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .expect("at least one non-PASS target action expected");
        target
    };
    runner
        .game
        .resolve_selection(0, action)
        .expect("selection resolves");

    let _ = runner.auto_resolve();

    // Hand must now contain the Greymon; trash must no longer.
    assert!(
        hand_ids(&runner, 0).contains(&"TRASH-GREYMON".to_string()),
        "Greymon must be returned to hand after resolving the selection; hand={:?}",
        hand_ids(&runner, 0)
    );
    assert!(
        !trash_ids(&runner, 0).contains(&"TRASH-GREYMON".to_string()),
        "Greymon must leave trash; trash={:?}",
        trash_ids(&runner, 0)
    );
}

#[test]
fn bt22_008_select_trash_filter_admits_garurumon_and_omnimon_too() {
    // The filter is `name_contains: Greymon | Garurumon | Omnimon`. We seed
    // all three plus a non-matching card and verify that exactly the three
    // matching cards are targetable (PASS may also be a legal action since
    // both layers are optional — see the faithfulness gate test below).
    let mut runner = runner_with_targets();
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");
    push_to_trash(&mut runner, 0, "TRASH-GARURUMON");
    push_to_trash(&mut runner, 0, "TRASH-OMNIMON");
    push_to_trash(&mut runner, 0, "TRASH-OTHER");

    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, perm.index as usize);

    let pending = runner
        .pending_selection()
        .expect("Trash selection installed");
    let target_actions: Vec<_> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .collect();
    assert_eq!(
        target_actions.len(),
        3,
        "exactly 3 matching-name trash cards should be targetable; got {:?}",
        pending.valid_action_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Faithfulness gate (printed text vs runtime behavior)
// ═══════════════════════════════════════════════════════════════════════════════

/// Printed text reads "You MAY return 1 card" (note the "may"); DCGO uses
/// `isOptional: true` on the outer ActivateClass AND
/// `SelectCardEffect.SetUp(canNoSelect: () => true, ...)` on the inner pick.
/// Both layers are opt-in: the player may decline at the outer trigger
/// (skip the prompt entirely) AND may decline at the inner pick (PASS must
/// remain a legal action even with eligible targets present).
///
/// This is the inverse of the BT17-007 faithfulness pin (which is mandatory
/// because that card's printed text has no "may").
#[test]
fn bt22_008_pending_selection_is_optional_when_target_exists() {
    let mut runner = runner_with_targets();
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");

    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, perm.index as usize);

    let pending = runner
        .pending_selection()
        .expect("Trash selection must be installed when matching trash card exists");
    // The convention (mirroring sister BT17-007's mandatory test) is that
    // `valid_action_ids` for a Trash selection holds only the target action
    // IDs (TRASH_EFFECT_START + i). PASS is never pushed into that vec —
    // its availability is gated through `is_optional` and applied by the
    // action-mask layer (`action::mask` adds `mask[PASS]=1.0` iff
    // `sel.is_optional`). So the faithfulness pin here is `is_optional`.
    assert!(
        pending.is_optional,
        "printed 'you may' + DCGO canNoSelect:true → inner pick MUST be \
         declinable (action mask exposes PASS via sel.is_optional); \
         pending_is_optional={}",
        pending.is_optional
    );
    // Cross-check: at least one matching target is present (otherwise the
    // whole pending wouldn't have installed). This is the inverse pin to
    // BT17-007's mandatory case (which asserts !contains(&PASS) — same
    // convention, opposite optionality).
    let target_count = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .count();
    assert!(
        target_count >= 1,
        "Optional pick must still surface eligible targets; \
         valid_action_ids={:?}",
        pending.valid_action_ids
    );
}
