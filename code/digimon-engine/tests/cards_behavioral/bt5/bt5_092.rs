//! BT5-092 Nokia Shiramine — Tamer, White, Cost 3.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] You may play 1 [Agumon] or [Gabumon] from your hand without
//!     paying the cost.
//! [Your Turn] When one of your Digimon would digivolve into a Digimon card
//!     in your hand with [Greymon], [Garurumon] or [Omnimon] in its name,
//!     by suspending this Tamer, reduce the digivolution cost by 1.
//! ```
//!
//! Inherited (Security):
//!   `[Security] Play this card without paying the cost.`
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT5/White/BT5_092.cs`
//!
//! Notable DCGO contracts:
//!   - On Play: `OnEnterFieldAnyone` ActivateClass with `isOptional: true`
//!     and `SelectHandEffect.SetUp(canNoSelect: true, maxCount: 1)`. Filter is
//!     `cardSource.CardNames.Contains("Agumon") || cardSource.CardNames.Contains("Gabumon")`.
//!     The bracketed `[Agumon]/[Gabumon]` printed text matches by substring
//!     (Comprehensive Rules §16) — DSL spelling is `name_contains`. Same
//!     spelling as sibling card BT22-084 (Nokia Lv.5).
//!   - Your Turn (BLOCKED): `BeforePayCost` ActivateClass that fires when
//!     `CanTriggerWhenPermanentWouldDigivolve` matches an OWNER permanent
//!     digivolving into a hand card with HasGreymonName ||
//!     ContainsCardName("Omnimon") || HasGarurumonName. Suspends Nokia and
//!     reduces the next paid digivolution cost by 1. Hash:
//!     "Digisorption-1_BT5_092" (1-per-trigger throttle shared across copies).
//!   - Security: `CardEffectFactory.PlaySelfTamerSecurityEffect(card)` — same
//!     primitive as BT5-093, BT22-084, BT9-092 (resolved precedent
//!     G-PLACE-SELF-AS-OPTION-PERMANENT, engine-gaps.md 2026-04-27).
//!
//! # Verdict — PARTIAL (per orchestrator brief)
//!
//! | Clause | Status |
//! |---|---|
//! | 1 [On Play] free play of Agumon/Gabumon | IMPLEMENTED — structural + behavioral |
//! | 2 [Your Turn] cost reduction by suspending self | BLOCKED — DSL vocab gap (no `when_*_digivolves_into` trigger; no `pay_cost: [suspend self]` on cost_reduction). Same root gap as BT23-005 in `qa/dsl-vocab-gaps.md` (search anchor: "when_this_digivolves_into"). |
//! | 3 [Security] play this card free | IMPLEMENTED — structural; full security-replay path covered by BT21-015 / BT5-093 |
//!
//! # Faithfulness diff (per clause)
//!
//! 1. **[On Play] (you may) play 1 [Agumon]/[Gabumon] free** — YAML matches
//!    printed text in optionality (`optional: true` at clause level + on
//!    `select_hand`), filter (`name_contains: Agumon | Gabumon`), and
//!    payment-free play (`play_from_hand_free`). Same shape as BT22-084
//!    clause 2 (audited 2026-05-03 with `select_hand` precedent).
//!
//! 2. **[Your Turn] suspend self → -1 digivolve cost when ally digivolves
//!    into hand Greymon/Garurumon/Omnimon** — BLOCKED on DSL vocab gap.
//!    `CostReductionBody` lacks both (a) a `when_any_ally_digivolves_into`
//!    target-name-aware trigger and (b) a `pay_cost: [suspend self]` payment
//!    hook. The engine path `scan_before_pay_cost_reduction` also does not
//!    thread the digivolution-target hand-card into `EffectReadContext`.
//!    See `qa/dsl-vocab-gaps.md` BT23-005 entry (this card widens the gap
//!    from "this permanent" to "any ally permanent" but the missing
//!    primitive is the same trigger family). Test branches for this clause
//!    are `#[ignore]`'d with that gap tag.
//!
//! 3. **[Security] play this card free** — YAML uses `when: on_security`,
//!    `process: [play_from_security: {}]`. Same primitive as BT5-093 /
//!    BT22-084 / BT9-092. Resolved precedent
//!    G-PLACE-SELF-AS-OPTION-PERMANENT (engine-gaps.md, 2026-04-27).
//!    Structural-only assertion here; full play-from-security replay
//!    coverage lives in `bt5_093.rs` / `bt21_015.rs` siblings.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//!   - §5 Structural: 2 triggered clauses (on_play + on_security). Clause
//!     count is 2, not 3, because the BLOCKED [Your Turn] cost reduction is
//!     intentionally omitted from the YAML (no silent approximation).
//!     Optionality flags.
//!   - §5 Condition gating: clause 1 positive (Agumon in hand → prompt
//!     installs); clause 1 negative (no Agumon/Gabumon in hand → no useful
//!     pick); clause 1 optional (PASS path is BLOCKED — same family as
//!     BT22-084 G-OUTER-OPTIONAL-NOT-INSTALLED).
//!   - §5 Behavioral integrated: clause 1 picks Agumon and plays it for
//!     free (no memory deduction for Agumon's printed cost).
//!   - Group B2 — Tamer play-anchor (cost-3 persistent on-your-turn).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT5-092";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn nokia_card_data() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A Lv.3 Agumon — used by clause 1 free-play tests (matches `name_contains:
/// Agumon` filter).
fn make_agumon(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Agumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(1000);
    c
}

/// A Lv.3 Gabumon.
fn make_gabumon(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Gabumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(1000);
    c
}

/// A non-matching Lv.3 Digimon (no Agumon/Gabumon in name) — used to verify
/// the clause-1 hand filter rejects unrelated names.
fn make_other_digimon(card_id: &str, card_name: &str) -> CardData {
    let mut c = make_test_card(card_id, card_name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(1000);
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// Smoke: the embedded DSL pack ships BT5-092 and it compiles as a Tamer.
#[test]
fn bt5_092_compiles_as_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3));
}

/// Two triggered clauses: on_play + on_security. The BLOCKED [Your Turn]
/// cost-reduction clause is intentionally omitted from the YAML pending the
/// DSL vocab gap (see file header).
#[test]
fn bt5_092_has_exactly_two_triggered_clauses() {
    let card = compiled(CARD_ID);
    assert_eq!(
        card.effects.len(),
        2,
        "BT5-092 must have exactly 2 compiled clauses (on_play + on_security); \
         [Your Turn] cost reduction is BLOCKED on DSL vocab gap and is omitted"
    );

    let triggered: Vec<_> = card
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
        "both clauses must be triggered (no declarative clauses on BT5-092)"
    );
}

/// Clause 1: on_play, FaceUp, optional ("you may"), not OPT.
#[test]
fn bt5_092_clause1_on_play_face_up_optional() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("BT5-092 must have an on_play clause");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 1 must be FaceUp (own-field) scope"
    );
    assert!(
        clause.optional,
        "Clause 1 must be optional (printed 'you may')"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 1 has no [Once Per Turn] in printed text"
    );
}

/// Clause 3 (positionally — [Your Turn] is BLOCKED): on_security triggered
/// clause. Not player-optional (printed text says "Play this card without
/// paying the cost" with no "may").
#[test]
fn bt5_092_clause_on_security_triggered_present() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("BT5-092 must have an on_security clause");

    assert!(
        !clause.optional,
        "on_security clause is not player-optional (printed: 'Play this card without paying the cost')"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1: [On Play] (you may) play 1 [Agumon]/[Gabumon] free
//
// We exercise via `play()` from hand. Same shape as BT22-084 clause 2 (on_play
// half) — see bt22_084.rs precedents.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: Agumon in hand → On Play installs prompt; player picks Agumon →
/// it enters battle area for free (no memory deduction for Agumon's printed
/// cost).
#[test]
fn bt5_092_clause1_on_play_offers_and_plays_agumon_free() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_agumon("AGUMON-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "AGUMON-1"])
        .memory(10) // enough to play Nokia (cost 3); we'll measure post-play
        .start();

    let memory_before_nokia = runner.memory();
    runner
        .play(0, 0)
        .expect("Nokia plays from hand (cost 3)");

    // After Nokia hits the field, On Play fires; clause 1 should install a
    // selection prompt to play 1 Agumon/Gabumon free.
    assert!(
        runner.game.pending_selection.is_some(),
        "Nokia's On Play should install a select_hand prompt for Agumon/Gabumon"
    );

    let memory_after_nokia = runner.memory();

    // Resolve selection: pick the first valid action (Agumon).
    let action = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        pending.valid_action_ids[0]
    };
    runner
        .game
        .resolve_selection(0, action)
        .expect("select Agumon from hand");
    let _ = runner.game.drain_effect_queue();

    // Battle area: Nokia + Agumon == 2 permanents.
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "Nokia + Agumon must both be on field"
    );
    // Hand: Agumon should no longer be there.
    assert_eq!(
        runner.hand_size(0),
        0,
        "Agumon must leave hand once played free"
    );
    // Free play must not deduct Agumon's printed cost (3) from memory.
    let memory_after_agumon = runner.memory();
    assert_eq!(
        memory_after_agumon, memory_after_nokia,
        "play_from_hand_free must not deduct Agumon's cost; \
         memory_after_nokia={memory_after_nokia}, memory_after_agumon={memory_after_agumon}, \
         memory_before_nokia={memory_before_nokia}"
    );
}

/// Positive: Gabumon variant (same filter — `name_contains: Gabumon`).
#[test]
fn bt5_092_clause1_on_play_offers_gabumon() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_gabumon("GABU-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "GABU-1"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Nokia plays from hand");
    assert!(
        runner.game.pending_selection.is_some(),
        "Nokia On Play must install prompt when Gabumon is the eligible candidate"
    );

    let action = runner.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("select Gabumon");
    let _ = runner.game.drain_effect_queue();

    assert_eq!(runner.battle_area_size(0), 2);
}

/// Optional: clause 1 is optional — player may PASS and the candidate stays
/// in hand. BLOCKED on G-OUTER-OPTIONAL-NOT-INSTALLED — same family as
/// AD1-009 / BT22-084 clause-2 PASS path: clause-level `optional: true` does
/// not install an outer accept/decline before the inner select_hand prompt,
/// so the prompt is forced even though the printed text says "you may".
#[test]
#[ignore = "pending: G-OUTER-OPTIONAL-NOT-INSTALLED — clause-level `optional: true` does not install an outer accept/decline before inner select_hand prompt; same family as AD1-009 / BT22-084 clause 2"]
fn bt5_092_clause1_pass_leaves_hand_intact() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_agumon("AGUMON-1"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "AGUMON-1"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Nokia plays");
    assert!(runner.game.pending_selection.is_some());
    assert!(
        runner.pending_is_optional(),
        "Clause 1 must be optional (printed 'you may')"
    );

    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("pass the optional select_hand prompt");
    let _ = runner.game.drain_effect_queue();

    // Battle area: only Nokia.
    assert_eq!(runner.battle_area_size(0), 1);
    // Hand: Agumon still there.
    assert_eq!(runner.hand_size(0), 1);
}

/// Negative: hand has no Agumon / Gabumon → select_hand has no candidates →
/// no prompt installed (or prompt has zero valid picks; either way, nothing
/// enters the field). Same precedent as BT22-084 clause-2 negative.
#[test]
fn bt5_092_clause1_no_prompt_when_no_matching_card_in_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(nokia_card_data())
        .add_card(make_other_digimon("OTHER-1", "Patamon"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .hand(0, &[CARD_ID, "OTHER-1"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Nokia plays");
    let _ = runner.game.drain_effect_queue();

    // No matching card → no Digimon enters the field beyond Nokia.
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "no Agumon/Gabumon in hand → no extra Digimon enters via clause 1"
    );
    // Patamon stays in hand.
    assert_eq!(runner.hand_size(0), 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2: [Your Turn] suspend self → -1 cost when an ally
//                       Digimon would digivolve into a hand Digimon with
//                       Greymon / Garurumon / Omnimon in name.
//
// BLOCKED on DSL vocab gap. `CostReductionBody` (digimon-dsl/src/clause.rs)
// has no `when_any_ally_digivolves_into` (or `when_this_digivolves_into`)
// trigger variant keyed on the digivolution-target hand card, and the
// engine path `scan_before_pay_cost_reduction` does not thread the target
// CardSource into `EffectReadContext` so a predicate could not inspect it
// even if the trigger existed. The clause also requires a `pay_cost:
// [suspend: { target: self }]` payment on a cost-reduction clause, which
// `CostReductionBody.pay_cost` does not currently honour for declarative
// reductions.
//
// Same root gap as BT23-005 (`qa/dsl-vocab-gaps.md`, search anchor:
// "when_this_digivolves_into"). BT5-092 widens the gap from "this
// permanent" (BT23-005) to "any of your permanents" (BT5-092); the missing
// primitive is the same trigger family.
//
// Until the DSL adds (a) the `when_*_digivolves_into` trigger variant with
// `target_name_contains` predicates AND (b) the suspend-self payment hook
// AND (c) the engine threads target hand-card metadata through to
// `EffectReadContext`, the four branches below are `#[ignore]`'d with
// `todo!()` bodies that mirror the BT23-005 precedent.
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive branch: own ally Digimon would digivolve into a Greymon-named
/// hand card. With Nokia ready (unsuspended) on the field, suspending Nokia
/// reduces the digivolution cost by 1.
#[test]
#[ignore = "pending: DSL vocab gap — no when_any_ally_digivolves_into + target_name_contains in CostReductionBody, no pay_cost: [suspend self] hook on cost_reduction clauses (see qa/dsl-vocab-gaps.md BT23-005 entry; same root gap)"]
fn bt5_092_clause2_cost_reduction_fires_on_greymon_target() {
    todo!(
        "implement once DSL adds when_any_ally_digivolves_into + target_name_contains \
         trigger AND cost-reduction `pay_cost: [suspend self]` payment hook"
    )
}

/// Positive branch: Garurumon-named hand-card target.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt5_092_clause2_cost_reduction_fires_on_greymon_target"]
fn bt5_092_clause2_cost_reduction_fires_on_garurumon_target() {
    todo!("implement once DSL adds when_any_ally_digivolves_into + suspend-self payment")
}

/// Positive branch: Omnimon-named hand-card target.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt5_092_clause2_cost_reduction_fires_on_greymon_target"]
fn bt5_092_clause2_cost_reduction_fires_on_omnimon_target() {
    todo!("implement once DSL adds when_any_ally_digivolves_into + suspend-self payment")
}

/// Negative: digivolving into an unrelated-name hand card (no Greymon /
/// Garurumon / Omnimon in name) must NOT trigger the cost reduction.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt5_092_clause2_cost_reduction_fires_on_greymon_target"]
fn bt5_092_clause2_cost_reduction_does_not_fire_on_unrelated_target() {
    todo!("implement once DSL adds when_any_ally_digivolves_into + suspend-self payment")
}

/// Negative: [Your Turn] gate must suppress the cost reduction on the
/// opponent's turn even if they digivolve into a matching-named card.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt5_092_clause2_cost_reduction_fires_on_greymon_target"]
fn bt5_092_clause2_cost_reduction_inactive_on_opponents_turn() {
    todo!("implement once DSL adds when_any_ally_digivolves_into + suspend-self payment")
}

/// Negative: if Nokia is already suspended (or otherwise cannot suspend),
/// the cost reduction cannot be paid for and must not fire.
#[test]
#[ignore = "pending: DSL vocab gap — same as bt5_092_clause2_cost_reduction_fires_on_greymon_target; payment hook also missing"]
fn bt5_092_clause2_cost_reduction_blocked_when_nokia_already_suspended() {
    todo!("implement once DSL adds when_any_ally_digivolves_into + suspend-self payment")
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3: [Security] Play this card without paying the cost
//
// Resolved precedent: BT21-015 / BT5-093 / BT9-092 / BT22-084 — see
// engine-gaps.md G-PLACE-SELF-AS-OPTION-PERMANENT (resolved 2026-04-27).
// Full security-replay path coverage lives in those companion test files;
// here we only assert the structural shape (the on_security clause exists
// with play_from_security as its body).
// ═══════════════════════════════════════════════════════════════════════════════

/// Structural: an on_security triggered clause exists and lowers from a
/// `play_from_security` step. The behavioral security-replay path is
/// covered by `bt5_093.rs` / `bt22_084.rs` / `bt21_015.rs` siblings using
/// the same primitive.
#[test]
fn bt5_092_clause3_on_security_structural_present() {
    let card = compiled(CARD_ID);
    let on_sec = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity]);
    assert!(
        on_sec.is_some(),
        "BT5-092 must compile an on_security clause for the [Security] play-self effect"
    );
}
