//! BT12-092 Marcus Damon — Tamer, Yellow+Red, Cost 4.
//!
//! # Card text (BT12-092.json / card image — authoritative; both agree verbatim)
//!
//! ```text
//! [Start of Your Main Phase] If you have a Digimon with [Agumon] or [Greymon] in
//!   its name in play, by paying 1 memory, for the turn, this Tamer is also
//!   treated as a 3000 DP Digimon that can't digivolve.
//! [Your Turn] When this Tamer becomes suspended, 1 of your Digimon may digivolve
//!   into a yellow card with [Greymon] in its name in your hand without paying
//!   the cost.
//! ```
//!
//! ```text
//! Inherited:
//! Security Effect [Security] Play this card without paying the cost.
//! ```
//!
//! # Official Q&A (data/card_bundles/BT12-092.md)
//! "It causes that card to also be treated as a Digimon with X000 DP...if a
//! Tamer is also treated as a Digimon, it can attack like a standard Digimon,
//! and it will gain inherited effects from cards stacked under it. However,
//! even if a Tamer is played and then treated as a Digimon in the same turn,
//! it can't attack during that turn."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Yellow/BT12_092.cs
//!   - EffectTiming.OnStartMainPhase: CanUseCondition = own battle area + owner's
//!     turn. CanActivateCondition = HasMatchConditionOwnersPermanent(Agumon-name
//!     OR Greymon-name Digimon) && MaxMemoryCost >= 1. ActivateCoroutine:
//!     AddMemory(-1, ...) then BecomeDigimonThatCantDigivolve(DP: 3000,
//!     UntilEachTurnEnd). `SetUpActivateClass(..., -1, true, ...)` — isOptional
//!     true (the whole clause is a "you may activate" cost-gated ability).
//!   - EffectTiming.OnTappedAnyone: CanUseCondition = own battle area + owner's
//!     turn + CanTriggerWhenPermanentSuspends(permanent == this card's own
//!     permanent). CanActivateCondition = an own Digimon exists that can host a
//!     yellow-[Greymon]-named hand card via CanPlayCardTargetFrame.
//!     ActivateCoroutine: SelectPermanentEffect (canNoSelect: true — "may")
//!     then DigivolveIntoHandOrTrashCard(cardCondition: yellow + Greymon-name,
//!     payCost: false, isHand: true).
//!   - EffectTiming.SecuritySkill: PlaySelfTamerSecurityEffect (play self free).
//!
//! # Clause map
//! | Clause | Timing | Effect |
//! |--------|--------|--------|
//! | 1 | [Start of Your Main Phase] | If own Agumon/Greymon-named Digimon in play, you may pay 1 memory: for the turn, treat this Tamer as a 3000 DP Digimon that can't digivolve |
//! | 2 | [Your Turn] on self-suspend | 1 of your Digimon may digivolve into a yellow [Greymon]-named hand card, free, ignoring requirements |
//! | 3 | [Security] | play self without paying the cost |
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - B1-adjacent: start-of-main Tamer activated ability, memory-cost-gated
//! - G1-adjacent: TreatAsDigimon (Tamer treated as a 3000 DP Digimon for the turn)
//! - E2: OPT-adjacent optional activation with a mandatory-once-accepted cost
//!   (`optional: true` clause + non-selection first step -> outer accept/decline
//!   prompt, the BT22-041 Kentaurosmon idiom)
//! - G3: self-scoped OnSuspend observer (`event_permanent_is_source`)
//! - Chained `select_own_permanent -> select_hand -> effect_initiated_digivolve`
//!   with a SEPARATE selected-permanent target (G-EFFECT-INITIATED-DIGIVOLVE-
//!   FROM-HAND-WITH-PERMANENT-TARGET — RESOLVED 2026-05-17, Phase 2 Track F;
//!   see docs/RUST_ENGINE_GAPS.md and the BT22-013 / BT21-013 idiom)
//! - on_security play-self (tamer)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledPredicate, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT12-092";

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// A yellow Digimon with [Agumon] in its name (the Clause-1 condition enabler).
fn make_yellow_agumon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Agumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A yellow Digimon whose name does NOT contain Agumon or Greymon (negative
/// enabler check).
fn make_unrelated_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Renamon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A yellow Digimon that CAN host a digivolve (Clause-2 target base).
fn make_yellow_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A yellow Digimon with [Greymon] in its name — the free-digivolve target.
fn make_yellow_greymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "MetalGreymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A RED Digimon with [Greymon] in its name — fails the "yellow" filter.
fn make_red_greymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "WarGreymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Red];
    c
}

/// A yellow Digimon whose name does NOT contain Greymon — fails the name filter.
fn make_yellow_non_greymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Gabumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Yellow];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-092 in embedded DSL pack")
        .add_card(make_yellow_agumon("YAGU"))
        .add_card(make_unrelated_digimon("UNREL"))
        .add_card(make_yellow_base("YBASE"))
        .add_card(make_yellow_greymon("YGREY"))
        .add_card(make_red_greymon("RGREY"))
        .add_card(make_yellow_non_greymon("YGABU"))
        .add_card(make_filler("FILL"))
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_092_yaml_parses_and_compiles() {
    let _ = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
}

#[test]
fn bt12_092_is_tamer_cost_4_yellow_red() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Tamer
    );
    assert_eq!(compiled.cost, Some(4));
    assert!(compiled
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Yellow));
    assert!(compiled
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Red));
}

/// Clause 1 fires at [Start of Your Main Phase] and is optional (the whole
/// ability is a "by paying 1 memory" opt-in cost).
#[test]
fn bt12_092_clause1_is_start_of_main_and_optional() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("clause 1 (start_of_your_main_phase) must exist");
    assert!(clause1.optional, "'by paying 1 memory' is an opt-in cost");
    assert!(
        !clause1.once_per_turn,
        "printed text carries no [Once Per Turn] on clause 1"
    );
}

/// Clause 2 fires on_suspend, scoped to [Your Turn].
#[test]
fn bt12_092_clause2_is_on_suspend() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause2 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("clause 2 (on_suspend) must exist");
    assert!(clause2.when.contains(&CompiledTiming::OnSuspend));
}

/// Clause 2's condition gates on the suspending permanent being THIS card
/// (`event_permanent_is_source: true`) — not just any Tamer/Digimon suspending.
fn predicate_has_event_permanent_is_source(p: &CompiledPredicate) -> bool {
    p.event_permanent_is_source == Some(true)
        || p.all_of.iter().any(predicate_has_event_permanent_is_source)
        || p.any_of.iter().any(predicate_has_event_permanent_is_source)
}

#[test]
fn bt12_092_clause2_gates_on_event_permanent_is_source() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause2 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("clause 2 must exist");
    let cond = clause2
        .condition
        .as_ref()
        .expect("clause 2 must gate on the suspended permanent being this card");
    assert!(
        predicate_has_event_permanent_is_source(cond),
        "clause 2 must require event_permanent_is_source: true"
    );
}

/// Clause 3 fires at on_security.
#[test]
fn bt12_092_has_on_security_clause() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        compiled.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT12-092 must have an on_security clause"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1 [Start of Your Main Phase] behavioral
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE (condition): with a yellow [Agumon]-named Digimon on your field,
/// starting your main phase offers the optional "pay 1 memory" activation.
#[test]
fn bt12_092_start_main_offers_activation_when_agumon_present() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "YAGU", Some(0));

    // Non-zero memory: paying 1 must not itself cross memory negative and
    // end the turn (Game::check_turn_end), which would confound the assertion.
    runner.game.memory = 5;
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_some(),
        "the optional pay-1-memory activation must offer an accept/decline prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "'by paying 1 memory' is declinable ('you may')"
    );
}

/// NEGATIVE (condition): with NO Agumon/Greymon-named Digimon on the field,
/// no activation is offered at all.
#[test]
fn bt12_092_start_main_no_offer_without_agumon_or_greymon() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "UNREL", Some(0));

    runner.game.memory = 5;
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "no own Agumon/Greymon-named Digimon -> the ability must not be offered"
    );
    assert_eq!(runner.memory(), 5, "no memory spent when not offered");
}

/// ACCEPT: paying the 1 memory installs TreatAsDigimon (synth 3000 DP) +
/// CannotDigivolve on this Tamer, for the turn.
#[test]
fn bt12_092_accepting_pays_memory_and_treats_as_3000_dp_digimon() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "YAGU", Some(0));

    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must not start treated as a Digimon"
    );

    runner.game.memory = 5;
    runner.game.enter_main_phase();

    let memory_before = runner.memory();
    runner
        .accept_optional_trigger()
        .expect("accept the pay-1-memory activation");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before - 1,
        "accepting must pay exactly 1 memory"
    );
    assert!(
        runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must be treated as a Digimon after paying the cost"
    );
    assert_eq!(
        runner.effective_dp(marcus),
        Some(3000),
        "Marcus must be treated as a 3000 DP Digimon"
    );
    assert!(
        runner.modifiers().has(marcus, ModifierType::CannotDigivolve),
        "Marcus must gain CannotDigivolve"
    );
}

/// DECLINE: declining the activation leaves memory and the Tamer's identity
/// unchanged.
#[test]
fn bt12_092_declining_activation_spends_no_memory_and_grants_nothing() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "YAGU", Some(0));

    runner.game.memory = 5;
    runner.game.enter_main_phase();

    assert!(runner.pending_is_optional(), "decline must be legal");
    runner
        .decline_optional_trigger()
        .expect("decline the activation");
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 5, "declining must not spend memory");
    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "declining must not treat Marcus as a Digimon"
    );
}

/// The treated-Marcus grants are for the turn — they expire at end of turn.
#[test]
fn bt12_092_treat_as_digimon_grants_expire_at_end_of_turn() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let marcus = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "YAGU", Some(0));

    runner.game.memory = 5;
    runner.game.enter_main_phase();
    runner
        .accept_optional_trigger()
        .expect("accept the activation");
    let _ = runner.auto_resolve();

    assert!(runner.modifiers().has(marcus, ModifierType::TreatAsDigimon));

    runner.game.end_turn(); // P0's turn ends -> "for the turn" grants expire
    runner.game.end_turn(); // back to P0

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

/// [Agumon]-in-name Digimon (not just Greymon) also satisfies the condition.
#[test]
fn bt12_092_start_main_offers_activation_via_greymon_name_too() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    // YGREY carries "MetalGreymon" in its printed name (contains "Greymon").
    runner.place_on_field(0, "YGREY", Some(0));

    runner.game.memory = 5;
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_some(),
        "a Greymon-named Digimon must also satisfy the clause-1 condition"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2 [Your Turn] self-suspend behavioral
// ════════════════════════════════════════════════════════════════════════════

/// Helper: suspend the Tamer directly, firing its own OnSuspend clause.
fn fire_self_suspend(runner: &mut DebugRunner, tamer: PermanentHandle) {
    runner.game.suspend(tamer);
}

/// POSITIVE: when BT12-092 suspends on your turn, with an eligible own
/// Digimon base and a yellow [Greymon] card in hand, the optional
/// permanent-then-hand selection chain installs.
#[test]
fn bt12_092_self_suspend_offers_digivolve_selection() {
    let mut runner = builder()
        .hand(0, &["YGREY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let _base = runner.place_on_field(0, "YBASE", Some(0));

    fire_self_suspend(&mut runner, tamer);

    let pending = runner
        .pending_selection()
        .expect("a permanent-pick selection must install after self-suspend");
    assert_eq!(pending.selecting_player, 0);
}

/// ACCEPT: driving the chain to completion digivolves the base into the
/// yellow [Greymon] hand card, free (no memory spent), ignoring requirements.
#[test]
fn bt12_092_self_suspend_digivolve_into_yellow_greymon_free() {
    let mut runner = builder()
        .hand(0, &["YGREY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "YBASE", Some(0));

    let memory_before = runner.memory();
    fire_self_suspend(&mut runner, tamer);

    // Drive the permanent pick (select the base), then the hand pick. With
    // exactly one eligible own Digimon, `select_own_permanent` collapses to a
    // single-candidate confirm prompt (SelectionKind::Replacement shape);
    // drive its sole legal action.
    let view = runner
        .pending_selection_view()
        .expect("permanent-pick selection installs");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "with exactly one eligible own Digimon, the permanent pick has one legal action"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select the base to digivolve");
    let _ = runner.auto_resolve();

    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "YGREY",
        "YGREY must become the top card of the base stack after the free digivolve"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "YGREY"),
        "YGREY must leave the hand once it digivolves"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "the digivolve is free -- no memory cost"
    );
}

/// DECLINE: the digivolve chain is optional ("may digivolve") — declining
/// leaves the board unchanged.
#[test]
fn bt12_092_self_suspend_decline_leaves_board_unchanged() {
    let mut runner = builder()
        .hand(0, &["YGREY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "YBASE", Some(0));

    fire_self_suspend(&mut runner, tamer);

    assert!(
        runner.pending_is_optional(),
        "'may digivolve' -> PASS must be legal"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline the digivolve selection");
    let _ = runner.auto_resolve();

    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "YBASE",
        "declining must leave the base unchanged"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "YGREY"),
        "declining must leave YGREY in hand"
    );
}

/// NEGATIVE: no yellow [Greymon]-named card in hand -> after the permanent
/// pick, no digivolve happens (no eligible hand candidate). The RED Greymon
/// (RGREY) does not satisfy the "yellow" filter.
#[test]
fn bt12_092_self_suspend_no_yellow_greymon_in_hand_no_digivolve() {
    let mut runner = builder()
        .hand(0, &["RGREY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "YBASE", Some(0));

    fire_self_suspend(&mut runner, tamer);
    let _ = runner.auto_resolve();

    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "YBASE",
        "a red Greymon-named card must not satisfy the 'yellow' filter"
    );
}

/// NEGATIVE (name filter): a yellow card WITHOUT [Greymon] in its name does
/// not satisfy the hand filter either.
#[test]
fn bt12_092_self_suspend_yellow_non_greymon_in_hand_no_digivolve() {
    let mut runner = builder()
        .hand(0, &["YGABU"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "YBASE", Some(0));

    fire_self_suspend(&mut runner, tamer);
    let _ = runner.auto_resolve();

    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "YBASE",
        "a yellow non-Greymon-named card must not satisfy the name filter"
    );
}

/// NEGATIVE (self-scope): suspending a DIFFERENT own permanent must NOT fire
/// clause 2 — only THIS Tamer suspending triggers it.
#[test]
fn bt12_092_other_permanent_suspending_does_not_fire_clause2() {
    let mut runner = builder()
        .hand(0, &["YGREY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let _tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "YBASE", Some(0));

    // Suspend the base itself (NOT the Tamer).
    runner.game.suspend(base);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "suspending a different own permanent must not trigger clause 2"
    );
    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "YBASE",
        "no digivolve should occur"
    );
}

/// NEGATIVE (turn scope): the opponent suspending BT12-092 (e.g. via an
/// opponent effect while it is not the controller's turn) must not fire the
/// [Your Turn] clause. We approximate by suspending on the opponent's turn.
#[test]
fn bt12_092_suspend_on_opponents_turn_does_not_fire_clause2() {
    let mut runner = builder()
        .hand(0, &["YGREY"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let base = runner.place_on_field(0, "YBASE", Some(0));

    // Advance to the opponent's turn.
    runner.game.end_turn();

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] clause 2 must not fire on the opponent's turn"
    );
    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "YBASE",
        "no digivolve should occur outside the controller's turn"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3 [Security] behavioral
// ════════════════════════════════════════════════════════════════════════════

/// [Security] "Play this card without paying the cost." — driven through the
/// real combat / security-check path: BT12-092 sits in the defender's
/// security stack, an attacker checks it, and the security clause plays it
/// free onto the defender's field.
#[test]
fn bt12_092_security_plays_self_free() {
    let mut attacker = make_filler("BT12092-ATK");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = builder()
        .add_card(attacker.clone())
        .memory(10)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .security(1, &[CARD_ID])
        .start();

    let attacker_handle = runner.place_on_field(0, "BT12092-ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: BT12-092 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(
        runner.security_count(1),
        0,
        "BT12-092 must leave the defender's security stack after the security check"
    );

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "BT12-092 must be played onto the defender's field by the [Security] effect"
    );
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == CARD_ID),
        "BT12-092 must not fall through to the trash — the [Security] clause played it"
    );
}
