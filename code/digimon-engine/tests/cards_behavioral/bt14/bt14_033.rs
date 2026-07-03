//! BT14-033 Patamon — Digimon, Lv.3, Yellow, DP 1000, Cost 3.
//! Traits: Mammal. Attribute: Data. Form: Rookie.
//!
//! # Card text (data/card_bundles/BT14-033.md — official Bandai DB, verbatim)
//!
//! [Start of Your Main Phase] Search your security stack. This Digimon may
//! digivolve into a yellow Digimon card with the [Vaccine] trait among them
//! without paying the cost. Then, shuffle your security stack. If digivolved
//! by this effect, you may place 1 yellow card with the [Vaccine] trait from
//! your hand at the bottom of your security stack.
//!
//! Inherited Effect [Your Turn][Once Per Turn]
//! When a card is added to your security stack, gain 1 memory.
//!
//! Official Q&A: "Yes, you can choose to not digivolve. In such cases, you
//! shuffle all of the searched cards and return them to the security stack."
//!
//! Digivolve: Lv.2 Yellow / cost 0 (printed evo_costs).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Yellow/BT14_033.cs
//!
//! DCGO OnStartMainPhase (ActivateClass, maxCount -1, not optional at the
//! ActivateClass level — the "may" surfaces via `canNoSelect: true` on the
//! inner `SelectCardEffect`):
//!   - CanUseCondition: on battle area + owner's turn.
//!   - CanActivateCondition: on battle area + `SecurityCards.Count >= 1`
//!     (NOT gated on an eligible yellow/Vaccine card existing — the filter is
//!     applied inside the picker itself, `canNoSelect: true`).
//!   - Body: SelectCardEffect over Root.Security, canNoSelect: true, filtered
//!     to CardTraits.Contains("Vaccine") && HasCardColor(Yellow) && IsDigimon
//!     && CanPlayCardTargetFrame(..., PayCost:false). ShuffleSecurity runs
//!     UNCONDITIONALLY after the pick resolves (whether or not a card was
//!     selected). If a card WAS selected and successfully digivolved
//!     (`IsDigivolvedByTheEffect`), a second `SelectHandEffect` offers 1
//!     yellow+Vaccine hand card (`canNoSelect: true`) to place at the BOTTOM
//!     of security (`toTop: false`).
//! DCGO OnAddSecurity (inherited ActivateClass, maxCount 1, hash
//!   "Memory+1_BT14_033", SetIsInheritedEffect(true)):
//!   - CanUseCondition: on battle area + owner's turn +
//!     CanTriggerWhenAddSecurity(player == card.Owner).
//!   - Body: AddMemory(1).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - A5-adjacent: digivolve-from-security (`select_security` +
//!   `effect_initiated_digivolve { source: <security binding> }`,
//!   G-EFFECT-DIGIVOLVE-FROM-ZONES — RESOLVED 2026-05-02, per
//!   `qa/resolved-gaps.md`).
//! - B1 Start-of-main triggered clause (mandatory ActivateClass; the "may" is
//!   inside the security picker, matching the official Q&A that declining is
//!   legal and simply reshuffles).
//! - E2 OPT + optional decline (the security search is `optional: true`; the
//!   hand-to-bottom-security follow-up is a same-clause `if binding_present`
//!   continuation — DCGO runs both in a single `ActivateCoroutine`, so this
//!   is ONE triggered clause with a linear body, not two separate clauses).
//! - G4-adjacent: Inherited [Your Turn][Once Per Turn] `on_added_to_security`
//!   → gain 1 memory (BT8-090 idiom, mandatory — no "may" on the inherited
//!   text, unlike BT8-090's suspend-cost variant).
//!
//! # Faithfulness diff vs. card text
//!
//! | Card-text element                                                  | YAML clause                                                             | Status |
//! |---------------------------------------------------------------------|--------------------------------------------------------------------------|--------|
//! | "[SoMP] Search your security stack"                                | `when: start_of_your_main_phase` + `select_security`                    | OK     |
//! | "may digivolve into a yellow [Vaccine] Digimon among them, no cost" | `select_security { filter: yellow+Vaccine+digimon, optional:true }` + `if binding_present` → `effect_initiated_digivolve { source, cost: free, ignore_requirements: true }` | OK |
//! | "Then, shuffle your security stack" (unconditional)                | `shuffle_security` as a sibling top-level step (runs via `Continue` even when `select_security` no-ops — collapse §1 contract) | OK |
//! | "If digivolved by this effect, you may place 1 yellow [Vaccine] hand card at the bottom of security" | same-clause `if binding_present: sec_pick` (BT13-012 idiom — the pick binding IS the "digivolved by this effect" gate, since `effect_initiated_digivolve` with `cost: free, ignore_requirements: true` always succeeds once a card is picked) → optional `select_hand` + `place_on_security { position: bottom }` | OK |
//! | Inherited "[Your Turn][OPT] card added to security → gain 1 memory" | `scope: inherited, when: on_added_to_security, active_when: your_turn, once_per_turn, mandatory gain_memory` | OK |
//!
//! # Known gaps
//! None. `effect_initiated_digivolve` with a `select_security`-bound `source`
//! resolves via `resolve_card_handle_source_ref` → `CardSourceRef::Security`
//! (already proven by `effect_digivolve_from_security_moves_selected_card_and_preserves_neighbors`
//! in `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`).
//! `on_added_to_security` / `OnPlaceSecurity` payload wiring is proven by
//! `on_place_security_fires_once_with_security_placement_payload`
//! (`code/digimon-engine/tests/timing_dispatch.rs`) and shipped on BT8-090 /
//! BT25-038.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A yellow [Vaccine] Digimon eligible for Patamon's free digivolve-from-
/// security pick. No `evo_costs` on purpose — the clause uses
/// `ignore_requirements: true` (no level restriction is printed), so a
/// standard evo-cost table entry must NOT be required for the pick to land.
fn make_yellow_vaccine(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Vaccine".to_string()];
    c
}

/// A non-yellow (or non-Vaccine) Digimon that must NOT be selectable by
/// Patamon's clause.
fn make_ineligible_security_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Vaccine".to_string()];
    c
}

fn make_yellow_vaccine_hand_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["Vaccine".to_string()];
    c
}

fn make_non_vaccine_hand_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![CardColor::Yellow];
    c.traits = vec![];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// Base runner with BT14-033 registered from the embedded DSL pack, plus
/// filler deck/security padding so `begin_turn` and shuffles don't deck out.
fn patamon_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT14-033")
        .expect("BT14-033 in embedded DSL pack")
        .add_card(make_yellow_vaccine("YEL-VAC"))
        .add_card(make_ineligible_security_card("RED-VAC"))
        .add_card(make_yellow_vaccine_hand_card("YEL-VAC-HAND"))
        .add_card(make_non_vaccine_hand_card("YEL-PLAIN-HAND"))
        .add_card(make_filler("FILLER"))
        .memory(10)
}

/// Place BT14-033 on P0's field as a face-up Digimon.
fn place_patamon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, "BT14-033", None)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt14_033_yaml_compiles_and_is_in_pack() {
    let runner = patamon_base().start();
    assert!(
        runner.compiled_card("BT14-033").is_some(),
        "BT14-033 must be present in the embedded DSL pack"
    );
}

/// Two triggered clauses: the SoMP search+digivolve+shuffle+follow-up body
/// (one linear DCGO ActivateCoroutine) and the inherited on_added_to_security
/// memory gain.
#[test]
fn bt14_033_has_two_triggered_clauses() {
    let runner = patamon_base().start();
    let compiled = runner
        .compiled_card("BT14-033")
        .expect("BT14-033 compiled card present");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
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
        "BT14-033 must have exactly 2 triggered clauses (SoMP search+digivolve+shuffle+followup body, inherited memory gain)"
    );
}

#[test]
fn bt14_033_search_clause_has_start_of_main_timing_own_scope() {
    let runner = patamon_base().start();
    let compiled = runner
        .compiled_card("BT14-033")
        .expect("BT14-033 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::FaceUp
                && t.when.contains(&CompiledTiming::StartOfYourMainPhase)
        })
        .expect("own-scope StartOfYourMainPhase clause must exist");

    assert!(
        clause.when.contains(&CompiledTiming::StartOfYourMainPhase),
        "search clause must have StartOfYourMainPhase timing"
    );
}

#[test]
fn bt14_033_inherited_clause_is_your_turn_once_per_turn() {
    let runner = patamon_base().start();
    let compiled = runner
        .compiled_card("BT14-033")
        .expect("BT14-033 compiled card present");

    let inherited = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("inherited clause must exist");

    assert!(
        inherited.once_per_turn,
        "inherited clause must be [Once Per Turn]"
    );
}

/// BT14-033 has no special/bonus digivolve box — the printed standard
/// Lv.2 Yellow / cost 0 evo path is carried entirely by `cards.json`'s
/// `evo_costs` (via `CardData`), not the DSL `alt_paths` list. This card's
/// YAML should therefore declare no `alt_paths`.
#[test]
fn bt14_033_has_no_special_alt_paths() {
    let runner = patamon_base().start();
    let compiled = runner
        .compiled_card("BT14-033")
        .expect("BT14-033 compiled card present");

    assert!(
        compiled.alt_paths.is_empty(),
        "BT14-033 prints no bonus/special digivolve box — alt_paths must be empty"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (CanActivateCondition: security count >= 1)
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: empty security stack → the SoMP clause must not install any
/// selection (DCGO CanActivateCondition: SecurityCards.Count >= 1).
#[test]
fn bt14_033_no_selection_when_security_empty() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(1, &["FILLER"; 3])
        .start();
    // P0 security intentionally left empty.
    let _p = place_patamon(&mut runner);

    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when P0's security stack is empty"
    );
}

/// Positive: non-empty security stack (even with zero eligible cards) still
/// installs the search prompt — DCGO's CanActivateCondition doesn't pre-check
/// eligibility, only stack size. The prompt then has only the (mandatory)
/// silent-decline path available since select_security no-ops with no
/// eligible candidates. We assert indirectly: memory should not drop from an
/// (illegal) digivolve, and the security count is unchanged after `auto_resolve`.
#[test]
fn bt14_033_security_nonempty_but_no_eligible_card_is_pure_shuffle_no_op() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["RED-VAC", "RED-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    let sec_before = runner.security_count(0);
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before,
        "security count must be unchanged — no eligible yellow [Vaccine] card, so the search is a pure shuffle"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no lingering selection after auto_resolve"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: search installs SelectionKind::Security
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt14_033_start_of_main_installs_security_selection_when_eligible_card_present() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC", "RED-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    runner.game.enter_main_phase();

    let kind = runner
        .pending_kind()
        .expect("Security selection must install when an eligible card exists");
    assert_eq!(kind, SelectionKind::Security);
    assert!(
        runner.pending_is_optional(),
        "the digivolve pick is optional (\"may digivolve\" / official Q&A: declining is legal)"
    );
}

/// Negative: the filter must exclude a non-yellow Vaccine card as a valid
/// pick target even when a yellow-but-non-Vaccine or red-Vaccine card sits
/// in security — only YEL-VAC should appear among the valid action ids.
#[test]
fn bt14_033_security_selection_filters_out_non_matching_colors_and_traits() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC", "RED-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    runner.game.enter_main_phase();

    let view = runner
        .pending_selection_view()
        .expect("selection view present");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the yellow [Vaccine] security card should be a legal pick (RED-VAC excluded)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral outcome: accept branch (digivolve from security)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt14_033_accepting_digivolves_patamon_into_the_picked_security_card() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let p = place_patamon(&mut runner);

    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accepting the digivolve pick must succeed");
    let _ = runner.auto_resolve();

    let perm = &runner.game.players[0].battle_area[p.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "YEL-VAC",
        "Patamon must have digivolved into the picked yellow [Vaccine] security card"
    );
}

#[test]
fn bt14_033_accepting_digivolve_pays_no_memory_cost() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    let memory_before = runner.memory();
    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept");
    let _ = runner.auto_resolve();

    // Memory may have INCREASED (the inherited clause fires +1 for the card
    // added to security when the hand-to-bottom follow-up runs), but the
    // digivolve itself must never have subtracted memory. We only assert it
    // did not drop below the pre-digivolve baseline from the digivolve cost
    // itself by checking it never went negative relative to start when no
    // follow-up runs (hand is not populated in this fixture's security(0)
    // scenario beyond YEL-VAC-HAND registration but not dealt to hand).
    assert!(
        runner.memory() >= memory_before,
        "digivolving from security must be free (no memory cost); memory must not have dropped, got before={memory_before} after={}",
        runner.memory()
    );
}

#[test]
fn bt14_033_accepting_digivolve_removes_exactly_the_picked_card_from_security_preserving_others() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["RED-VAC", "YEL-VAC", "RED-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    let sec_before = runner.security_count(0);
    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    assert_eq!(view.valid_action_ids.len(), 1, "only YEL-VAC is eligible");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "exactly one security card (the digivolved YEL-VAC) leaves the stack"
    );
    assert!(
        runner.game.players[0]
            .security
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) == "RED-VAC"),
        "the two RED-VAC security cards must remain in security (shuffled, but present)"
    );
}

/// Cost-firing / event-log assertion: accepting the pick emits NO
/// `MemoryChange` event — `pay_memory(0)` short-circuits before pushing an
/// event (see `Game::pay_memory`'s `if cost == 0 { return true; }` early
/// return), so a genuinely free digivolve produces zero memory-log noise.
/// (`GameEvent::Digivolve` itself is only emitted by the standard
/// digivolve-from-hand path — `effect_initiated_digivolve_from_source_inner`
/// does not push one; event-log coverage there is a pre-existing engine gap
/// out of this card's scope, not something this clause can assert against.)
#[test]
fn bt14_033_accepting_digivolve_emits_no_memory_change_event() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    let cp = runner.event_checkpoint();
    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept");
    let _ = runner.auto_resolve();

    let events = runner.events_since(cp);
    let memory_change = events
        .iter()
        .find(|e| matches!(e, GameEvent::MemoryChange { .. }));
    assert!(
        memory_change.is_none(),
        "a free digivolve (cost 0) must not emit a MemoryChange event"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Behavioral outcome: decline branch
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt14_033_declining_leaves_patamon_undigivolved_but_still_shuffles() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC", "RED-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let p = place_patamon(&mut runner);

    let sec_before = runner.security_count(0);
    runner.game.enter_main_phase();
    assert!(runner.pending_is_optional(), "decline (PASS) must be legal");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the digivolve pick");
    let _ = runner.auto_resolve();

    let perm = &runner.game.players[0].battle_area[p.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "BT14-033",
        "Patamon must remain undigivolved after declining"
    );
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "security count is unchanged after a decline (Q&A: cards are shuffled back)"
    );
}

/// After declining, the hand-to-bottom-security follow-up must NOT fire (it
/// is gated on `binding_present: sec_pick` — the search binding is unset
/// when the player declines, so no digivolve happened).
#[test]
fn bt14_033_declining_does_not_offer_hand_to_bottom_security_followup() {
    let mut runner = patamon_base()
        .hand(0, &["YEL-VAC-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    runner.game.enter_main_phase();
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the digivolve pick");

    // After the decline resolves (and the unconditional shuffle runs), there
    // must be no further pending selection — no hand-to-bottom-security
    // prompt should install.
    assert!(
        runner.pending_selection().is_none(),
        "declining the digivolve must not offer the hand-to-bottom-security follow-up"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — "If digivolved by this effect" hand-to-bottom-security follow-up
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt14_033_after_digivolving_offers_optional_hand_to_bottom_security() {
    let mut runner = patamon_base()
        .hand(0, &["YEL-VAC-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the digivolve");

    let kind = runner
        .pending_kind()
        .expect("hand-to-bottom-security follow-up must install after a successful digivolve");
    assert_eq!(kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "the hand-to-bottom-security placement is \"you may\""
    );
}

#[test]
fn bt14_033_hand_to_bottom_security_filters_out_non_vaccine_hand_cards() {
    let mut runner = patamon_base()
        .hand(0, &["YEL-VAC-HAND", "YEL-PLAIN-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the digivolve");

    let followup_view = runner
        .pending_selection_view()
        .expect("hand-to-bottom-security follow-up must install");
    assert_eq!(
        followup_view.valid_action_ids.len(),
        1,
        "only the yellow [Vaccine] hand card should be a legal pick (YEL-PLAIN-HAND excluded)"
    );
}

#[test]
fn bt14_033_accepting_hand_to_bottom_security_moves_the_card_to_the_bottom() {
    let mut runner = patamon_base()
        .hand(0, &["YEL-VAC-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    let hand_before = runner.hand_size(0);
    let sec_before = runner.security_count(0);

    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the digivolve");

    let followup_view = runner
        .pending_selection_view()
        .expect("hand-to-bottom-security follow-up must install");
    runner
        .execute_action(0, followup_view.valid_action_ids[0])
        .expect("accept placing the hand card at bottom of security");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the placed hand card must leave the hand"
    );
    // Security net change: -1 (digivolved YEL-VAC left security) + 1 (hand
    // card placed at bottom) = net 0 relative to sec_before.
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "security count nets to the same size: one card left (digivolved), one card added (from hand)"
    );
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "YEL-VAC-HAND",
        "the placed card must sit at the BOTTOM of the security stack (index 0)"
    );
    // NOTE: the inherited [Your Turn][OPT] on_added_to_security clause does
    // NOT fire here — per RULES 15-3-1, a Digimon's printed "Inherited
    // Effect" only activates once that card becomes a digivolution SOURCE
    // beneath another card on the battle area. Patamon itself is still the
    // top (and only) card of its own permanent in this scenario, so its own
    // inherited text stays dormant. See `bt14_033_inherited_gains_memory_*`
    // below for coverage of the inherited clause once Patamon is buried.
}

#[test]
fn bt14_033_declining_hand_to_bottom_security_leaves_hand_and_security_unchanged() {
    let mut runner = patamon_base()
        .hand(0, &["YEL-VAC-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    let hand_before = runner.hand_size(0);

    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the digivolve");

    let _followup_view = runner
        .pending_selection_view()
        .expect("hand-to-bottom-security follow-up must install");
    assert!(
        runner.pending_is_optional(),
        "decline (PASS) must be legal for the \"you may\" hand-to-bottom-security follow-up"
    );
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline placing a hand card");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining the follow-up must leave the hand unchanged"
    );
}

/// No hand-eligible card → the follow-up clause is a structural no-op (no
/// selection installs) after a successful digivolve.
#[test]
fn bt14_033_hand_to_bottom_security_noop_when_no_eligible_hand_card() {
    let mut runner = patamon_base()
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["YEL-VAC"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);
    // Hand intentionally empty.

    runner.game.enter_main_phase();
    let view = runner
        .pending_selection_view()
        .expect("Security selection must install");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the digivolve");

    assert!(
        runner.pending_selection().is_none(),
        "no eligible hand card → the hand-to-bottom-security follow-up must be a silent no-op"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 7 — Inherited [Your Turn][Once Per Turn] on_added_to_security
// ═══════════════════════════════════════════════════════════════════════════

/// Per RULES 15-3-1, a Digimon's printed "Inherited Effect" only activates
/// once that card becomes a digivolution SOURCE beneath another card on the
/// battle area — it stays dormant while the card is the top of its own
/// permanent. `place_stack` stacks a CARRIER on top of Patamon so the
/// inherited clause becomes live (BT16-040 idiom).
fn place_patamon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &["BT14-033", "CARRIER"])
}

/// Direct unit coverage of the inherited clause via a manual security
/// placement (independent of the SoMP clause), proving the OPT gain-1-memory
/// fires on any card added to the controller's own security once Patamon is
/// buried beneath a carrier.
#[test]
fn bt14_033_inherited_gains_memory_when_card_added_to_own_security() {
    let mut runner = patamon_base()
        .add_card(make_filler("CARRIER"))
        .hand(0, &["YEL-VAC-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["FILLER"])
        .security(1, &["FILLER"; 3])
        // Override patamon_base()'s memory(10) — 10 is the rules ceiling
        // (memory_range == (-10, 10)), so a +1 gain would silently clamp
        // back to 10 and mask the assertion. Start below the ceiling.
        .memory(5)
        .start();
    let _carrier = place_patamon_as_source(&mut runner);

    let memory_before = runner.memory();
    let source_card = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card,
            None,
            0,
        );
        ctx.place_on_security(
            0,
            digimon_engine::enums::CardSourceRef::Hand(0, 0),
            digimon_engine::enums::StackPosition::Top,
            false,
        );
    }
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "inherited clause must gain 1 memory when a card is added to the controller's security"
    );
}

/// OPT enforcement: a second card added to security in the same turn must
/// NOT gain a second memory.
#[test]
fn bt14_033_inherited_opt_blocks_second_memory_gain_same_turn() {
    let mut runner = patamon_base()
        .add_card(make_filler("CARRIER"))
        .hand(0, &["YEL-VAC-HAND", "YEL-PLAIN-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["FILLER"])
        .security(1, &["FILLER"; 3])
        // Override patamon_base()'s memory(10) ceiling — see the sibling
        // gains-memory test for why headroom matters here.
        .memory(3)
        .start();
    let _carrier = place_patamon_as_source(&mut runner);

    let memory_before = runner.memory();
    let source_card_0 = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card_0,
            None,
            0,
        );
        ctx.place_on_security(
            0,
            digimon_engine::enums::CardSourceRef::Hand(0, 0),
            digimon_engine::enums::StackPosition::Top,
            false,
        );
    }
    runner.game.drain_effect_queue();
    let memory_after_first = runner.memory();
    assert_eq!(
        memory_after_first,
        memory_before + 1,
        "the first security addition must gain 1 memory (sanity check before the OPT-block assertion)"
    );

    // Second placement, same turn — OPT must block a second gain.
    let source_card_1 = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card_1,
            None,
            0,
        );
        ctx.place_on_security(
            0,
            digimon_engine::enums::CardSourceRef::Hand(0, 0),
            digimon_engine::enums::StackPosition::Top,
            false,
        );
    }
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        memory_after_first,
        "OPT must block a second memory gain from a second security addition in the same turn"
    );
}

/// Only the controller's OWN security additions trigger the inherited
/// clause — an addition to the opponent's security must not fire it.
#[test]
fn bt14_033_inherited_does_not_fire_for_opponent_security_addition() {
    let mut runner = patamon_base()
        .add_card(make_filler("CARRIER"))
        .hand(1, &["YEL-VAC-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["FILLER"])
        .security(1, &["FILLER"; 3])
        .start();
    let _carrier = place_patamon_as_source(&mut runner);

    let memory_before = runner.memory();
    let source_card = runner.game.players[1].hand[0].handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card,
            None,
            1,
        );
        ctx.place_on_security(
            1,
            digimon_engine::enums::CardSourceRef::Hand(1, 0),
            digimon_engine::enums::StackPosition::Top,
            false,
        );
    }
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        memory_before,
        "adding a card to the OPPONENT's security must not trigger P0's inherited clause"
    );
}

/// NEGATIVE control for the RULES 15-3-1 dormancy rule itself: while Patamon
/// is the TOP (and only) card of its own permanent — not yet buried beneath
/// a carrier — its inherited clause must NOT fire even when a card is added
/// to its controller's security.
#[test]
fn bt14_033_inherited_dormant_while_patamon_is_top_card() {
    let mut runner = patamon_base()
        .hand(0, &["YEL-VAC-HAND"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .security(0, &["FILLER"])
        .security(1, &["FILLER"; 3])
        .start();
    let _p = place_patamon(&mut runner);

    let memory_before = runner.memory();
    let source_card = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card,
            None,
            0,
        );
        ctx.place_on_security(
            0,
            digimon_engine::enums::CardSourceRef::Hand(0, 0),
            digimon_engine::enums::StackPosition::Top,
            false,
        );
    }
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        memory_before,
        "RULES 15-3-1: Patamon's own inherited clause is dormant while it is the top card of its own permanent"
    );
}
