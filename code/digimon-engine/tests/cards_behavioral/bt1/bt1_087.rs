//! BT1-087 T.K. Takaishi — Tamer, Yellow, Cost 4.
//!
//! # Card text (data/card_bundles/BT1-087.md — official Bandai DB, verbatim)
//!
//! ```text
//! [Start of Your Turn] If you have 2 or less memory, set your memory to 3.
//! [On Play] Look at your security stack, then reveal 1 card in it and add it
//!   to your hand. If that card is yellow, trigger <Recovery +1 (Deck)>.
//!   (Place the top card of your deck on top of your security stack.) Then
//!   shuffle your security stack.
//! ```
//!
//! Inherited/Security: [Security] Play this card without paying its memory cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT1/Yellow/BT1_087.cs`
//!
//! Clause-by-clause DCGO walkthrough (BT1_087.cs):
//! - **Clause 1** — `EffectTiming.OnStartTurn` -> `SetMemoryTo3TamerEffect`.
//!   The canonical "[Start of Your Turn] if memory <= 2, set it to 3" ramp
//!   idiom (sister BT1-085 Tai Kamiya).
//! - **Clause 2** — `EffectTiming.OnEnterFieldAnyone` -> `ActivateClass` with
//!   `CanActivateCondition = SecurityCards.Count >= 1`. Inside the coroutine:
//!   `SelectCardEffect(root: Security, maxCount: 1, canNoSelect: () => false,
//!   canTargetCondition: (_) => true, mode: AddHand)` — mandatory, UNFILTERED
//!   pick (any card in security qualifies). After the pick:
//!   `if (selectedCard.HasCardColor(Yellow)) -> IRecovery(owner, 1)`. Then
//!   `ShuffledDeckCards(SecurityCards)` runs as the final statement of the
//!   same guarded branch (only reached at all when security was non-empty).
//! - **Clause 3 (Security)** — `EffectTiming.SecuritySkill` ->
//!   `PlaySelfTamerSecurityEffect(card)`.
//!
//! # YAML clause layout
//!
//! | # | Clause                                              | Kind / when                | Shape |
//! |---|------------------------------------------------------|----------------------------|-------|
//! | 1 | [Start of Your Turn] if mem<=2 set to 3             | triggered, StartOfYourTurn | condition: { memory_lte: 2 }; process: [set_memory: 3] |
//! | 2 | [On Play] reveal 1 security (unfiltered), add to hand; if yellow Recovery +1; shuffle | triggered, OnPlay | search_own_security_stack { filter: {}, optional: false, bind_as: revealed, on_select: [add_to_hand_from_security, if binding_card_color yellow -> recover, shuffle_security] } |
//! | 3 | [Security] Play self without paying                 | triggered, OnSecurity      | process: [play_from_security] |
//!
//! # G-DSL-BINDING-CARD-COLOR — RESOLVED 2026-07-03
//! Clause 2's "If that card is yellow, trigger Recovery +1" branch was
//! previously BLOCKED — there was no predicate leaf to test a bound card's
//! printed COLOR inside a subject-less `if:` (the reveal search itself must
//! stay unfiltered per DCGO's `canTargetCondition: (_) => true`; only the
//! Recovery TRIGGER is color-gated). `binding_card_color: { binding,
//! color_is }` (sibling of `binding_card_kind`) now exists — covered by
//! `code/digimon-engine/tests/dsl/predicate_leaves_ii.rs` (Leaf 1), whose own
//! driver comment names this exact card. Clause 2 is authored below using it.
//!
//! # Patterns this test covers
//! - Start-of-turn memory ramp (memory_lte gate + set_memory), sister of BT1-085
//! - On Play: mandatory UNFILTERED security reveal -> add to hand -> color-gated
//!   Recovery +1 (Deck) -> shuffle security, all nested inside one `on_select`
//! - Empty-security no-op (mandatory search finds nothing to pick)
//! - [Security] play-self-without-cost Tamer idiom (structural, per BT1-085 precedent)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::action::space::SEL_MY_SECURITY_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT1-087";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn fire_timing(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn make_yellow_sec(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c
}

fn make_blue_sec(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c
}

fn hand_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn security_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .security
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT1-087 must load from the embedded DSL pack")
        .add_card(make_yellow_sec("SEC-YELLOW"))
        .add_card(make_blue_sec("SEC-BLUE"))
        .add_card(make_test_card("DECK-TOP", "Deck Top Card"))
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt1_087_compiles() {
    let runner = base().memory(0).start();
    assert!(
        runner.compiled_card(CARD_ID).is_some(),
        "BT1-087 must compile from the embedded DSL pack"
    );
}

#[test]
fn bt1_087_metadata_is_yellow_tamer_cost_four() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    assert_eq!(card.cost, Some(4), "BT1-087 prints Cost 4");
    assert!(
        matches!(card.kind, digimon_dsl::compiled::CompiledCardKind::Tamer),
        "BT1-087 must be a Tamer"
    );
    assert_eq!(card.level, None, "Tamers carry no printed level");
    assert_eq!(card.dp, None, "Tamers carry no printed DP");
    assert!(
        card.color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Yellow)),
        "BT1-087 must include Yellow color"
    );
}

#[test]
fn bt1_087_has_exactly_three_clauses() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    assert_eq!(
        card.effects.len(),
        3,
        "BT1-087 must have 3 clauses (SoT memory + On Play + security); got {:?}",
        card.effects
    );
}

#[test]
fn bt1_087_clause1_is_start_of_your_turn_triggered() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourTurn))
        .expect("StartOfYourTurn clause must exist");

    assert!(
        !clause.optional,
        "the memory ramp has no \"you may\" — mandatory when condition holds"
    );
    assert!(clause.condition.is_some(), "must gate on memory_lte: 2");
}

#[test]
fn bt1_087_clause2_is_on_play_triggered_and_mandatory() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OnPlay clause must exist");

    assert!(
        !clause.optional,
        "DCGO canNoSelect: () => false — the reveal pick itself is mandatory once security is non-empty"
    );
}

#[test]
fn bt1_087_clause3_on_security_plays_self_without_cost() {
    let runner = base().memory(0).start();
    let card = runner.compiled_card(CARD_ID).unwrap();

    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert!(
        !security.optional,
        "[Security] play_from_security is mandatory per general_rule.pdf Security Effect timing"
    );
    assert!(
        security
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromSecurity)),
        "on_security clause must include play_from_security; got {:?}",
        security.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2 — Clause 1 condition gating: [Start of Your Turn] memory ramp
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt1_087_start_of_turn_sets_memory_to_three_when_two_or_less() {
    let mut runner = base().memory(2).start();
    let tk = runner.place_on_field(0, CARD_ID, Some(0));

    fire_timing(&mut runner, EffectTiming::StartOfYourTurn, tk);

    assert_eq!(
        runner.memory(),
        3,
        "memory 2 (<= 2) must be set to 3; got {}",
        runner.memory()
    );
}

#[test]
fn bt1_087_start_of_turn_raises_zero_memory_to_three() {
    let mut runner = base().memory(0).start();
    let tk = runner.place_on_field(0, CARD_ID, Some(0));

    fire_timing(&mut runner, EffectTiming::StartOfYourTurn, tk);

    assert_eq!(runner.memory(), 3, "memory 0 (<= 2) must be set to 3");
}

#[test]
fn bt1_087_start_of_turn_does_not_change_memory_above_two() {
    let mut runner = base().memory(4).start();
    let tk = runner.place_on_field(0, CARD_ID, Some(0));

    fire_timing(&mut runner, EffectTiming::StartOfYourTurn, tk);

    assert_eq!(
        runner.memory(),
        4,
        "memory 4 (> 2) fails the memory_lte:2 gate — must stay 4, not drop to 3"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3 — Clause 2 behavioral: [On Play] reveal / add / recover / shuffle
// ═══════════════════════════════════════════════════════════════════════════════

/// Security has exactly one (yellow) card: the mandatory pick installs a
/// Security-kind selection over that single slot.
#[test]
fn bt1_087_on_play_installs_mandatory_security_selection_when_non_empty() {
    let mut runner = base()
        .memory(10)
        .security(0, &["SEC-YELLOW"])
        .start();
    let tk = runner.place_on_field(0, CARD_ID, None);

    fire_timing(&mut runner, EffectTiming::OnPlay, tk);

    assert!(
        !runner.pending_is_optional(),
        "the reveal pick must be mandatory (DCGO canNoSelect: () => false)"
    );
    let view = runner
        .pending_selection_view()
        .expect("a Security selection must install when security is non-empty");
    assert!(view.valid_action_ids.contains(&SEL_MY_SECURITY_START));
}

/// Picking the (only) yellow security card adds it to hand and triggers
/// Recovery +1 (Deck): the deck's top card lands on top of security, so the
/// security count is restored to its pre-pick size.
#[test]
fn bt1_087_on_play_yellow_pick_adds_to_hand_and_triggers_recovery() {
    let mut runner = base()
        .memory(10)
        .security(0, &["SEC-YELLOW"])
        .deck(0, &["DECK-TOP"])
        .start();
    let tk = runner.place_on_field(0, CARD_ID, None);

    let security_before = runner.security_count(0);
    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.deck_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, tk);
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("pick the sole security card");
    let _ = runner.auto_resolve();

    assert!(
        hand_contains(&runner, 0, "SEC-YELLOW"),
        "the revealed yellow card must be added to hand"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "exactly one card added to hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Recovery +1 (Deck) must move exactly 1 card off the top of the deck"
    );
    assert_eq!(
        runner.security_count(0),
        security_before,
        "Recovery +1 restores the security count 1-for-1 after the yellow pick is removed"
    );
    assert_eq!(
        security_ids(&runner, 0),
        vec!["DECK-TOP".to_string()],
        "the recovered deck-top card must now be the (sole) security card"
    );
}

/// Picking a non-yellow security card adds it to hand but must NOT trigger
/// Recovery — the security count simply drops by 1 with no replacement, even
/// though a deck card is available to recover.
#[test]
fn bt1_087_on_play_non_yellow_pick_adds_to_hand_without_recovery() {
    let mut runner = base()
        .memory(10)
        .security(0, &["SEC-BLUE"])
        .deck(0, &["DECK-TOP"])
        .start();
    let tk = runner.place_on_field(0, CARD_ID, None);

    let security_before = runner.security_count(0);
    let deck_before = runner.deck_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, tk);
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("pick the sole security card");
    let _ = runner.auto_resolve();

    assert!(
        hand_contains(&runner, 0, "SEC-BLUE"),
        "the revealed non-yellow card must still be added to hand (the search itself is unfiltered)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "a non-yellow pick must NOT trigger Recovery +1 — the deck must be untouched"
    );
    assert_eq!(
        runner.security_count(0),
        security_before - 1,
        "without Recovery, security count simply drops by 1 (no replacement)"
    );
}

/// The search itself is UNFILTERED: with only non-yellow cards in security,
/// the mandatory pick must still install and succeed (DCGO
/// `canTargetCondition: (_) => true` — no color/trait restriction on which
/// card may be revealed; only the downstream Recovery trigger is color-gated).
#[test]
fn bt1_087_on_play_search_is_unfiltered_even_with_zero_yellow_cards() {
    let mut runner = base()
        .add_card(make_blue_sec("SEC-BLUE-2"))
        .memory(10)
        .security(0, &["SEC-BLUE", "SEC-BLUE-2"])
        .start();
    let tk = runner.place_on_field(0, CARD_ID, None);

    fire_timing(&mut runner, EffectTiming::OnPlay, tk);

    let view = runner
        .pending_selection_view()
        .expect("a mandatory Security selection must install even with zero yellow candidates");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both non-yellow security cards must be legal picks (unfiltered search)"
    );
}

/// Resolution completes cleanly (shuffle runs, no dangling selection) for a
/// multi-card security stack regardless of which slot is picked.
#[test]
fn bt1_087_on_play_resolves_fully_leaving_no_pending_selection() {
    let mut runner = base()
        .add_card(make_test_card("SEC-FILL", "SEC-FILL"))
        .memory(10)
        .security(0, &["SEC-YELLOW", "SEC-BLUE", "SEC-FILL"])
        .start();
    let tk = runner.place_on_field(0, CARD_ID, None);

    fire_timing(&mut runner, EffectTiming::OnPlay, tk);
    runner
        .execute_action(0, SEL_MY_SECURITY_START + 1) // pick the middle (blue) slot
        .expect("pick a security card");
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "the whole On Play clause (reveal -> add -> conditional recovery -> shuffle) must fully resolve"
    );
    assert_eq!(
        runner.security_count(0),
        2,
        "one of the 3 security cards was added to hand and not replaced (non-yellow pick)"
    );
}

/// Empty security: the mandatory search finds nothing, `on_no_match` (absent)
/// no-ops, and the clause ends without touching hand/deck/security at all —
/// no phantom shuffle side effects, no dangling selection.
#[test]
fn bt1_087_on_play_no_op_when_security_is_empty() {
    let mut runner = base().memory(10).start();
    let tk = runner.place_on_field(0, CARD_ID, None);

    assert_eq!(runner.security_count(0), 0);
    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.deck_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, tk);

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when security is empty (has_match precheck fails)"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "with empty security, no card can be revealed or added to hand"
    );
    assert_eq!(runner.deck_size(0), deck_before, "Recovery must not fire");
    assert_eq!(runner.security_count(0), 0, "security must remain empty");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4 — Cost/zone-bookkeeping firing on the On Play reveal-and-add:
// the picked security card must be fully materialized and cleanly removed
// from the security zone's face-up bookkeeping (`Player::face_up_security`)
// once it lands in hand — not merely absent from the security Vec while a
// stale face-up-card_index entry lingers behind.
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt1_087_on_play_add_to_hand_clears_face_up_security_bookkeeping() {
    let mut runner = base().memory(10).security(0, &["SEC-BLUE"]).start();
    let tk = runner.place_on_field(0, CARD_ID, None);
    let picked_card_index = runner.game.players[0].security[0].card_index;

    fire_timing(&mut runner, EffectTiming::OnPlay, tk);
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("pick the sole security card");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&picked_card_index),
        "the picked card's face_up_security bookkeeping must be cleared once \
         it moves out of the security zone into hand"
    );
    assert!(
        hand_contains(&runner, 0, "SEC-BLUE"),
        "sanity: the picked card actually landed in hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5 — OPT enforcement
// ═══════════════════════════════════════════════════════════════════════════════
//
// No clause on BT1-087 is [Once Per Turn]. Skipped intentionally (matches the
// sibling BT1-085's own Section 5 disposition).
