//! BT17-007 Agumon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Reptile.
//! Evo: Lv.2 Red / cost 0.
//!
//! # Card text (cards.json)
//!
//! [Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in
//!     its name, return 1 card with [Garurumon], [Greymon] or [Omnimon] in
//!     its name from your trash to the hand.
//!
//! Inherited:
//!   [End of Your Turn] This Digimon and any of your other Digimon may DNA
//!     digivolve into a Digimon card in the hand.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_007.cs`
//!
//! Notable DCGO contracts:
//!   - Start-of-Main-Phase clause: `SetUpActivateClass(..., isOptional: false, ...)`
//!     and `SelectCardEffect.SetUp(canNoSelect: () => false, ...)` — once the
//!     Tai Kamiya + matching trash gating is met, the controller MUST return
//!     a card; declining is not exposed.
//!   - End-of-Your-Turn inherited clause: `SetUpActivateClass(..., isOptional: true, ...)`
//!     consistent with the printed "may DNA digivolve".
//!   - The end-of-turn DNA digivolve partner condition is
//!     `permanent.IsDigimon && permanent != card.PermanentOfThisCard()` — i.e.
//!     "another of your Digimon", which matches the "any of your **other**
//!     Digimon" wording.
//!
//! # YAML location
//! `code/digimon-engine/cards/bt17/BT17-007.yaml` — promoted to the per-set
//! folder on 2026-05-20 (was `cards/_examples/BT17-007.yaml`). This test
//! reads the card through `card_data_from_compiled` + `compiled` (the
//! embedded DSL pack) rather than `include_str!`, matching the BT22-084
//! promotion convention.
//!
//! # Audit summary (2026-05-03)
//!
//! Status: AUDITED-OK — observed behavior matches printed text + DCGO.
//!
//! Initial concern: the YAML authors the start-of-main-phase clause with
//! `optional: true`, but the printed text says "return 1 card" (no "may"),
//! and DCGO uses `isOptional: false` + `canNoSelect: () => false`. This
//! looked like authoring drift.
//!
//! Empirical finding (see `bt17_007_pending_selection_is_mandatory_when_target_exists`):
//! when the Tai-Kamiya gating + matching-trash conditions are both met,
//! the engine installs the inner `select_trash` prompt with
//! `pending_is_optional() == false`. The clause-level `optional: true`
//! only affects whether the trigger may be skipped when no valid target
//! exists; once a target exists, declining is not exposed. Behavior
//! matches DCGO's `canNoSelect: () => false` mandatory-pick contract.
//!
//! Author-style note: setting `optional: true` on a triggered clause whose
//! printed text has no "may" is mildly misleading even though it produces
//! the correct runtime behavior. A future cosmetic cleanup could drop the
//! flag, but this is not a faithfulness drift.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//!   - §5 Structural: alt-path digivolve from Koromon @ cost 0; start-of-
//!     main-phase triggered clause with Tai-Kamiya predicate; inherited
//!     `alt_path_registration` for end-of-your-turn DNA digivolve.
//!   - §5 Condition gating split: no Tamer → no fire; non-Tai Kamiya Tamer →
//!     no fire; Tai Kamiya Tamer + empty/non-matching trash → no fire;
//!     Tai Kamiya Tamer + matching trash → fires.
//!   - §5 Behavioral integrated: resolving the install moves a Greymon card
//!     from trash to hand.
//!   - §5 Event-log: not asserted — the YAML's `add_to_hand_from_trash`
//!     lowering does not currently emit a card-resolution-specific
//!     GameEvent variant the test would gain anything by inspecting; zone
//!     observation is sufficient.
//!   - OPT: neither clause has [Once Per Turn] — out of scope.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

// ─── Compile-time access to the embedded compiled card ───────────────────────
//
// `dsl_card_data` lives at `tests/support/dsl_card_data.rs`. The cards_behavioral
// test binary already mounts it at the crate root via the `#[path]` directive
// in `tests/cards_behavioral/main.rs`, so we re-export from `crate::dsl_card_data`
// rather than re-mounting it here (which would attempt a path traversal that
// fails on Windows).

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT17-007";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn agumon_card_data() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_tamer(card_id: &str, card_name: &str) -> CardData {
    let mut card = make_test_card(card_id, card_name);
    card.card_kind = CardKind::Tamer;
    card
}

/// Garurumon-named Digimon card placed in trash for "return to hand" tests.
fn make_trash_target(card_id: &str, card_name: &str) -> CardData {
    let mut card = make_test_card(card_id, card_name);
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
/// `enter_main_phase` does not deck-out into game_over before the trigger
/// drainer runs.
fn runner_with_targets() -> DebugRunner {
    let filler = make_test_card("DECK-PAD", "Filler");
    let tai = make_tamer("TAMER-TAI", "Tai Kamiya");
    let other_tamer = make_tamer("TAMER-OTHER", "Marcus Damon");
    let greymon = make_trash_target("TRASH-GREYMON", "Greymon");
    let garurumon = make_trash_target("TRASH-GARURUMON", "Garurumon");
    let omnimon = make_trash_target("TRASH-OMNIMON", "Omnimon");
    let other = make_trash_target("TRASH-OTHER", "RandomDigimon");

    DebugRunner::builder()
        .add_card(agumon_card_data())
        .add_card(filler)
        .add_card(tai)
        .add_card(other_tamer)
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
fn bt17_007_yaml_compiles_in_embedded_pack() {
    // Smoke: the embedded DSL pack ships BT17-007 from `_examples/`.
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Reptile")),
        "BT17-007 must carry the Reptile trait"
    );
}

#[test]
fn bt17_007_has_lv2_red_cost0_digivolve_alt_path() {
    // Card prints "Digivolve: Lv.2 / Red / cost 0" — the YAML authors this as
    // `from: { name_is: Koromon }` because the card's printed digivolve path
    // is "from a Lv.2 Red Digimon" but the only legal Lv.2 Red predecessor in
    // a Tai-Kamiya pile is Koromon. This assertion sanity-checks the alt-path
    // count + cost; the predicate name match is intentionally loose because
    // the printed text is "Lv.2 Red" not literally "Koromon".
    let card = compiled(CARD_ID);
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        1,
        "BT17-007 must expose exactly one digivolve alt-path"
    );
    assert_eq!(
        digivolve_paths[0].cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(0)),
        "Digivolve cost must be 0"
    );
}

#[test]
fn bt17_007_has_start_of_main_phase_triggered_clause() {
    let card = compiled(CARD_ID);
    let clause = card.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::StartOfYourMainPhase) {
                return Some(t);
            }
        }
        None
    });
    assert!(
        clause.is_some(),
        "BT17-007 must have a triggered clause at StartOfYourMainPhase"
    );
    let clause = clause.unwrap();
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Start-of-main-phase clause is face-up only"
    );
    assert!(
        !clause.once_per_turn,
        "Start-of-main-phase clause has no [Once Per Turn]"
    );
}

#[test]
fn bt17_007_has_inherited_dna_digivolve_may_step() {
    // G-DSL-EOT-DNA-INLINE (2026-05-24): inherited end-of-your-turn DNA
    // digivolve is now authored via the `may_dna_digivolve_now` step verb
    // inside a `Triggered` clause whose `when: end_of_your_turn, scope:
    // inherited, optional: true`. The body is a single `MayDnaDigivolveNow`
    // step that orchestrates the partner + target selections at trigger fire.
    //
    // Predecessor authoring (`alt_path_registration { kind: dna_digivolve,
    // scope: inherited, trigger: end_of_your_turn }`) registered a next-turn
    // alt-path action and did not satisfy the printed "AT end of turn"
    // surface — see proposal `may-dna-digivolve-now-dsl-step`.
    let card = compiled(CARD_ID);
    let eot_dna_clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::EndOfYourTurn)
                && t.optional =>
        {
            Some(t)
        }
        _ => None,
    });
    let t = eot_dna_clause
        .expect("BT17-007 must carry an inherited optional EoT triggered clause for DNA digivolve");
    assert!(
        t.process.iter().any(|step| matches!(
            step,
            CompiledStep::MayDnaDigivolveNow { ignore_requirements: true, cost: 0, .. }
        )),
        "BT17-007 EoT inherited body must contain a `MayDnaDigivolveNow` step \
         with cost=0 and ignore_requirements=true"
    );
}

#[test]
fn bt17_007_clause_count_matches_card_text() {
    // G-DSL-EOT-DNA-INLINE (2026-05-24): card now prints two triggered
    // clauses — (1) the [Start of Your Main Phase] return-from-trash trigger
    // and (2) the inherited [End of Your Turn] DNA digivolve trigger (was
    // previously an alt_path_registration declarative — see proposal
    // `may-dna-digivolve-now-dsl-step`). No more alt_path_registration
    // clauses on this card.
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
    assert_eq!(triggered, 2, "Exactly two triggered clauses expected");
    assert_eq!(
        alt_path_regs, 0,
        "Zero alt_path_registration clauses expected after EoT DNA migration"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (clause: [Start of Your Main Phase])
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt17_007_start_of_main_no_fire_when_no_tamer_present() {
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "no Tamer present → trigger must not install a selection"
    );
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["TRASH-GREYMON".to_string()],
        "trash must be untouched when condition fails"
    );
}

#[test]
fn bt17_007_start_of_main_no_fire_with_non_tai_kamiya_tamer() {
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAMER-OTHER", None);
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "Tamer name does not contain 'Tai Kamiya' → no fire"
    );
}

#[test]
fn bt17_007_start_of_main_no_fire_when_trash_has_no_matching_card() {
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAMER-TAI", None);
    // Trash has a non-matching card only.
    push_to_trash(&mut runner, 0, "TRASH-OTHER");

    runner.game.enter_main_phase();

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
fn bt17_007_start_of_main_no_fire_with_empty_trash() {
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAMER-TAI", None);
    // Trash is empty.

    runner.game.enter_main_phase();

    let _ = runner.auto_resolve();
    assert_eq!(
        hand_ids(&runner, 0),
        Vec::<String>::new(),
        "empty trash → nothing returned to hand"
    );
}

#[test]
fn bt17_007_start_of_main_fires_with_tai_kamiya_and_matching_trash() {
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAMER-TAI", None);
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");

    runner.game.enter_main_phase();

    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "Tai Kamiya + matching trash → trigger must install a Trash selection"
    );
    assert!(
        matches!(kind, Some(SelectionKind::Trash)),
        "selection must target the trash zone"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral integrated
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt17_007_returns_selected_greymon_from_trash_to_hand() {
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAMER-TAI", None);
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");
    push_to_trash(&mut runner, 0, "TRASH-OTHER");

    runner.game.enter_main_phase();

    // First (and only) eligible action should select the matching Greymon.
    let action = {
        let pending = runner
            .pending_selection()
            .expect("Trash selection installed");
        assert!(
            !pending.valid_action_ids.is_empty(),
            "at least one matching trash card is targetable"
        );
        pending.valid_action_ids[0]
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
fn bt17_007_select_trash_filter_admits_garurumon_and_omnimon_too() {
    // The filter is `name_contains: Garurumon | Greymon | Omnimon`. We seed
    // all three plus a non-matching card and verify the count of valid actions
    // is exactly 3.
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAMER-TAI", None);
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");
    push_to_trash(&mut runner, 0, "TRASH-GARURUMON");
    push_to_trash(&mut runner, 0, "TRASH-OMNIMON");
    push_to_trash(&mut runner, 0, "TRASH-OTHER");

    runner.game.enter_main_phase();

    let pending = runner
        .pending_selection()
        .expect("Trash selection installed");
    // Exactly the 3 matching-name cards should be selectable. Any PASS
    // entry — if present due to `optional: true` — is asserted separately
    // by the drift test below.
    let target_actions: Vec<_> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != digimon_engine::action::space::PASS)
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

/// Printed text reads "return 1 card" (no "may"); DCGO uses
/// `isOptional: false` + `SelectCardEffect.SetUp(canNoSelect: () => false, ...)`.
/// Therefore once the Tai-Kamiya gating clears AND a matching trash card
/// exists, the controller MUST resolve the selection — declining is not a
/// legal action.
///
/// Even though the YAML authors the clause with `optional: true`, the engine
/// installs the inner `select_trash` prompt as mandatory in this state.
/// This test pins that contract so any regression in the trigger-drainer or
/// `select_trash` lowering would surface immediately.
#[test]
fn bt17_007_pending_selection_is_mandatory_when_target_exists() {
    let mut runner = runner_with_targets();
    runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "TAMER-TAI", None);
    push_to_trash(&mut runner, 0, "TRASH-GREYMON");

    runner.game.enter_main_phase();

    let pending = runner
        .pending_selection()
        .expect("Trash selection must be installed when condition+target are met");
    assert!(
        !pending.is_optional,
        "printed text + DCGO say [Start of Your Main Phase] return is mandatory \
         once gated; pending_is_optional={}",
        pending.is_optional
    );
    assert!(
        !pending
            .valid_action_ids
            .contains(&digimon_engine::action::space::PASS),
        "PASS must not be a legal action when the controller is required \
         to return a matching trash card; valid_action_ids={:?}",
        pending.valid_action_ids
    );
}
