//! BT22-094 Yuugo Kamishiro — Tamer, White, Cost 3, traits: [CS].
//!
//! # Card text (cards.json)
//!
//! **[On Play]** Reveal the top 3 cards of your deck. Add 1 card with the [CS]
//! trait among them to the hand. Return the rest to the bottom of the deck.
//!
//! **[Your Turn]** When any of your Digimon or Tamers with the [CS] trait
//! would be played, by returning this Tamer to the bottom of the deck, reduce
//! the play cost by 2.
//!
//! **Inherited (Security):** [Security] Play this card without paying the
//! cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/White/BT22_094.cs`
//!
//! # Patterns this test covers
//!
//! - **[On Play] reveal-N + single-bucket selection (CS trait) + bottom
//!   remainder** — Clause 1. Mirror of BT12-059 / BT24-031 multi-bucket
//!   pattern reduced to one bucket.
//! - **[Your Turn] cost reduction with `pay_cost`** — Clause 2. Mirrors
//!   EX8-074 (cost-reduction + pay_cost) and BT13-007 (cost-reduction +
//!   `optional` + `when_any_ally_played`), with the additional twist that
//!   the cost is "return THIS Tamer to deck bottom" via `return_to_deck:
//!   { target: source, position: bottom }` — same `target: source` shape
//!   as BT17-093 / BT24-082.
//! - **[Security] play self free** — Clause 3 (structural; behavioral
//!   replay coverage lives in BT21-015 / BT9-092 / BT5-093 sister tests
//!   for the `play_from_security` substrate).
//!
//! # Faithfulness audit (per clause)
//!
//! 1. **[On Play] reveal-3, add 1 [CS] card, bottom remainder** —
//!    `select_reveal_buckets` with one bucket `{ trait_has: CS, min: 1,
//!    max: 1 }` and trailing `place_remainder_on_deck { position: bottom }`
//!    matches DCGO `SimplifiedRevealDeckTopCardsAndSelect(revealCount: 3,
//!    [bucket(CS, AddHand, max 1)], remainingCardsPlace: DeckBottom)`.
//!    The DCGO outer trigger is NOT isOptional (no "you may"); the inner
//!    bucket is mandatory when a CS card is in the reveal and silently
//!    skipped when none is. The DSL `min: 1` semantics match.
//!
//! 2. **[Your Turn] -2 cost on ally CS Digimon/Tamer play, by returning
//!    self to deck bottom** — `kind: cost_reduction` declarative with
//!    `reduction_timing: before_pay_cost`, `active_when: { your_turn:
//!    true }` (mirrors DCGO `IsOwnerTurn(card)`), `optional: true`
//!    (mirrors DCGO `isOptional: true` on the outer ActivateClass; printed
//!    "by returning this Tamer ..." marks the cost as opt-in), `condition:
//!    any_permanent self-on-field` (mirrors DCGO `IsExistOnBattleArea(card)`),
//!    `when_any_ally_played: { all_of: [any_of: [kind: digimon, kind:
//!    tamer], trait_has: CS] }` (mirrors DCGO `(IsDigimon || IsTamer) &&
//!    HasCSTraits && Owner == card.Owner`), `amount: 2`, and `pay_cost:
//!    [return_to_deck { target: source, position: bottom }]`.
//!
//!    Owner-scoping on `when_any_ally_played` is implicit — the
//!    `cost_target_card` predicate-subject is set during the controller's
//!    own pay-cost flow (see `lower_cost_reduction.rs:116`,
//!    `effect_context/mod.rs:576`).
//!
//!    Once-per-turn is NOT marked: the cost (return self to deck bottom)
//!    self-limits re-entry — after the Tamer leaves the field, the
//!    `condition: any_permanent self-on-field` gate fails on subsequent
//!    cost-reduction scans. Matches DCGO behavior.
//!
//! 3. **[Security] play this card without paying the cost** —
//!    `when: on_security` + `process: [play_from_security: {}]`. Standard
//!    pattern (BT17-093, BT18-087, BT22-084, BT21-015, BT9-092, EX11-054).
//!    Mandatory (no [Security] effect is opt-out per RULES_CONTEXT.md §16).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "BT22-094";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A generic Digimon-kind card with the CS trait. Used to populate decks for
/// the on-play reveal test and as a play target for the cost-reduction test.
fn make_cs_digimon(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    c.traits = vec!["CS".to_string()];
    c
}

/// A Tamer-kind card with the CS trait. Used as a CS Tamer play target.
fn make_cs_tamer(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    c.traits = vec!["CS".to_string()];
    c
}

/// A Digimon WITHOUT the CS trait — for negative branch tests of the cost-
/// reduction `when_any_ally_played` predicate.
fn make_non_cs_digimon(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    c.traits = vec![]; // no CS
    c
}

/// An Option card with the CS trait — kind: option is neither Digimon nor
/// Tamer, so the cost-reduction predicate must reject it even with CS trait.
fn make_cs_option(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Option;
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    c.traits = vec!["CS".to_string()];
    c
}

/// Helper: walk pending selections, accepting the first non-PASS legal action,
/// until the engine settles. Mirrors the bt17_093 EOT-flow helper.
fn drive_accept(runner: &mut DebugRunner, player: u8) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(player, accept)
            .expect("execute pending selection");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT22-094 must compile from the embedded DSL pack as a Tamer with cost 3.
#[test]
fn bt22_094_compiles_as_tamer_cost_3() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3), "BT22-094 prints Cost 3");
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("CS")),
        "BT22-094 must carry the CS trait (printed type_eng = CS)"
    );
}

/// Three clauses total: 2 triggered (on_play, on_security) + 1 declarative
/// (cost_reduction).
#[test]
fn bt22_094_has_two_triggered_and_one_cost_reduction() {
    let card = compiled(CARD_ID);

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
        "BT22-094 must have exactly 2 triggered clauses (on_play + on_security)"
    );

    let cr_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    assert_eq!(
        cr_count, 1,
        "BT22-094 must have exactly 1 declarative cost_reduction clause"
    );
}

/// Clause 1: on_play, FaceUp, not optional, not OPT.
#[test]
fn bt22_094_clause1_on_play_face_up_mandatory() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("BT22-094 must have an on_play clause");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause 1 must be FaceUp (own-field) scope"
    );
    assert!(
        !clause.optional,
        "Clause 1 outer trigger is not optional — DCGO SetUpActivateClass(..., false, ...); \
         no 'you may' in printed text"
    );
    assert!(
        !clause.once_per_turn,
        "Clause 1 carries no [Once Per Turn] in printed text"
    );
}

/// Clause 2: cost_reduction with optional + active_when:your_turn + amount=2 +
/// when_any_ally_played present + non-empty pay_cost.
#[test]
fn bt22_094_clause2_cost_reduction_optional_your_turn_amount_2() {
    let card = compiled(CARD_ID);
    let cr = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope,
                active_when,
                optional,
                once_per_turn,
                amount,
                when_any_ally_played,
                pay_cost,
                ..
            }) => Some((
                *scope,
                active_when.clone(),
                *optional,
                *once_per_turn,
                *amount,
                when_any_ally_played.clone(),
                pay_cost.clone(),
            )),
            _ => None,
        })
        .expect("BT22-094 must have a cost_reduction clause");

    let (scope, active_when, optional, once_per_turn, amount, when_any_ally, pay_cost) = cr;

    assert_eq!(scope, CompiledScope::FaceUp, "cost_reduction is FaceUp scope");
    assert!(
        optional,
        "cost_reduction must be optional — DCGO isOptional: true; printed 'by returning ...' \
         is opt-in"
    );
    assert!(
        !once_per_turn,
        "cost_reduction is NOT marked OPT — printed text has no [Once Per Turn]; \
         the return-self cost self-limits re-entry"
    );
    assert_eq!(amount, Some(2), "amount must be 2 (printed -2 cost)");
    assert!(
        when_any_ally.is_some(),
        "when_any_ally_played predicate must be present"
    );
    assert!(
        active_when.is_some_and(|p| p.your_turn == Some(true)),
        "active_when must gate to your_turn=true"
    );
    assert!(
        !pay_cost.is_empty(),
        "pay_cost must be non-empty (return self to deck bottom)"
    );
}

/// Clause 3: on_security triggered, FaceUp, mandatory.
#[test]
fn bt22_094_clause3_on_security_face_up_mandatory() {
    let card = compiled(CARD_ID);
    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("BT22-094 must have an on_security clause");

    assert_eq!(security.scope, CompiledScope::FaceUp);
    assert!(
        !security.optional,
        "Security clause must be mandatory ([Security] effects are not opt-out per \
         RULES_CONTEXT.md §16; DCGO PlaySelfTamerSecurityEffect has no canNoSelect parameter)"
    );
    assert!(!security.once_per_turn, "Security clause has no OPT");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 [On Play] reveal-3 + add 1 CS + bottom rest
// ═══════════════════════════════════════════════════════════════════════════════

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

/// Positive: reveal contains [CS_DIGI, FILLER1, FILLER2]. Player must pick the
/// CS Digimon → it lands in hand; the two FILLERs go to deck bottom.
#[test]
fn bt22_094_on_play_picks_cs_card_and_bottoms_remainder() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-094 in embedded DSL pack")
        .add_card(make_cs_digimon("CS-DIGI", "CS Digimon", 5))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["CS-DIGI", "FILLER1", "FILLER2"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT22-094 (cost 3)");

    // The on_play reveal-bucket prompt must install. With one CS card among
    // the 3 revealed, the bucket is mandatory (min: 1).
    assert!(
        runner.game.pending_selection.is_some(),
        "select_reveal_buckets prompt must install for the CS bucket"
    );

    // Pick CS-DIGI for the CS bucket.
    let cs_action = revealed_action_for_id(&runner, "CS-DIGI")
        .expect("CS-DIGI should appear among the revealed actions");
    runner
        .execute_action(0, cs_action)
        .expect("pick CS-DIGI for the CS bucket");

    // Drive the remainder-placement order through to completion.
    let _ = runner.auto_resolve();

    // CS-DIGI must be in P0's hand.
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        hand_ids.contains(&"CS-DIGI".to_string()),
        "CS-DIGI must be added to hand; hand was {hand_ids:?}"
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "FILLERs must NOT enter hand; hand was {hand_ids:?}"
    );

    // The two FILLERs must be at the bottom of the deck (the last 2 entries).
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let last_two: Vec<&String> = deck_ids.iter().rev().take(2).collect();
    let last_two_set: Vec<String> = last_two.into_iter().cloned().collect();
    assert!(
        last_two_set.contains(&"FILLER1".to_string())
            && last_two_set.contains(&"FILLER2".to_string()),
        "FILLER1 and FILLER2 must be at the bottom of the deck; bottom-2 was {last_two_set:?}"
    );
}

/// Negative: no CS card in reveal — the CS bucket has zero candidates and
/// silently resolves; all 3 revealed cards are placed at deck bottom.
#[test]
fn bt22_094_on_play_no_cs_card_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-094 in embedded DSL pack")
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .add_card(make_filler("FILLER3"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT22-094");

    // Drive any prompts (e.g. remainder-placement order over 3 cards).
    let _ = runner.auto_resolve();

    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    for filler in ["FILLER1", "FILLER2", "FILLER3"] {
        assert!(
            !hand_ids.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no CS card is revealed; hand was {hand_ids:?}"
        );
    }

    let deck_size = runner.game.players[0].deck.len();
    assert_eq!(
        deck_size, 3,
        "all 3 revealed FILLERs must return to deck; deck size was {deck_size}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 [Your Turn] -2 cost on ally CS Digimon
//                          play, by returning self to deck bottom
// ═══════════════════════════════════════════════════════════════════════════════
//
// Strategy: place BT22-094 on P0's field, put a CS Digimon in P0's hand, and
// `play(0, hand_index)` it. The engine should detect the cost-reduction
// declarative effect during BeforePayCost scan; if the player accepts (i.e.
// the cost-reduction prompt installs as optional), the pay_cost step
// (return_to_deck of source) runs and the ally's effective cost is reduced
// by 2.
//
// Memory deduction is the cleanest assertion: with cost-5 ally + cost-2
// reduction, memory should drop by 3 instead of 5. We verify alongside the
// invariant that BT22-094 is no longer on the battle area (returned to deck
// bottom by pay_cost).

/// Positive: BT22-094 on field, CS Digimon (cost 5) played from hand → cost
/// reduction fires, BT22-094 returns to deck bottom, ally cost is 5-2=3.
///
/// Note on flow: when an `optional: true` cost_reduction matches, the engine
/// returns `PlayFromHandCostResult::Pending` (→ `play_from_hand` returns
/// `None`) and installs a `pending_selection` of kind EffectChoice with
/// `is_optional: true`, `valid_action_ids: [HAND_EFFECT_START]`. Accepting
/// (HAND_EFFECT_START) chains into the pay_cost steps; PASS-ing declines.
/// We therefore call `play()` without `.expect()` here and handle the
/// Pending state by accepting via `drive_accept`.
#[test]
fn bt22_094_cost_reduction_fires_on_ally_cs_digimon_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-094 in embedded DSL pack")
        .add_card(make_cs_digimon("CS-DIGI", "CS Digimon", 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["CS-DIGI"])
        .memory(10)
        .start();

    // Place BT22-094 on field (bypass on_play). Cost reduction is registered
    // as a BeforePayCost effect on the carrier permanent.
    runner.place_on_field(0, CARD_ID, Some(0));

    // Sanity: BT22-094 is on field and unsuspended.
    let yuugo_present_before = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(yuugo_present_before, "BT22-094 must be on P0's field before the play");

    let mem_before = runner.memory();
    let deck_size_before = runner.game.players[0].deck.len();

    // Play CS-DIGI from P0's hand. With an optional cost-reduction in scope,
    // play_from_hand returns Pending → None; we accept the prompt next.
    let _ = runner.play(0, 0);

    // A pending selection prompt for the optional cost reduction must install.
    assert!(
        runner.game.pending_selection.is_some(),
        "an optional cost-reduction prompt must install for the CS-DIGI play"
    );

    // The cost reduction is optional — accept the prompt (HAND_EFFECT_START)
    // to apply the reduction and trigger pay_cost. drive_accept picks the
    // first non-PASS option, which is the accept action.
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    let cost_paid = mem_before - mem_after;
    assert_eq!(
        cost_paid, 3,
        "CS-DIGI's effective cost must be 5 - 2 = 3 with BT22-094's reduction; got {cost_paid}"
    );

    // BT22-094 must no longer be on the field (returned to deck bottom by pay_cost).
    let yuugo_present_after = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !yuugo_present_after,
        "BT22-094 must be off-field after the cost (return-self-to-deck-bottom) is paid"
    );

    // BT22-094 must be at the bottom of the deck.
    let deck_size_after = runner.game.players[0].deck.len();
    assert_eq!(
        deck_size_after,
        deck_size_before + 1,
        "deck must grow by 1 (BT22-094 added to deck bottom); was {deck_size_before}, now {deck_size_after}"
    );
    // Engine convention: deck[0] = bottom of deck, deck.last() = top (drawn
    // first). `Bottom` placement uses `deck.insert(0, ...)`, so BT22-094 must
    // be at deck[0] after the cost is paid.
    let bottom_id = runner.game.players[0]
        .deck
        .first()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .unwrap_or_default();
    assert_eq!(
        bottom_id, CARD_ID,
        "the bottom (deck[0]) card must be BT22-094 after pay_cost; was {bottom_id}"
    );

    // The CS-DIGI ally must be on field (it was successfully played at the
    // reduced cost).
    let ally_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "CS-DIGI");
    assert!(ally_on_field, "CS-DIGI must be on field after the reduced-cost play");
}

/// Positive: also fires for an ally CS Tamer play (kind: tamer + trait: CS).
#[test]
fn bt22_094_cost_reduction_fires_on_ally_cs_tamer_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-094 in embedded DSL pack")
        .add_card(make_cs_tamer("CS-TAMER", "CS Tamer", 4))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["CS-TAMER"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0); // Pending → None when cost-reduction prompt installs
    assert!(
        runner.game.pending_selection.is_some(),
        "cost-reduction prompt must install for CS-TAMER play"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 2,
        "CS Tamer's effective cost must be 4 - 2 = 2; got {cost_paid}"
    );

    // BT22-094 returned to deck bottom.
    let yuugo_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(!yuugo_on_field, "BT22-094 must be off-field after pay_cost");
}

/// Negative: ally Digimon WITHOUT CS trait → cost reduction must NOT fire.
/// Full printed cost is paid and BT22-094 stays on the field.
#[test]
fn bt22_094_cost_reduction_does_not_fire_on_non_cs_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-094 in embedded DSL pack")
        .add_card(make_non_cs_digimon("NON-CS", "NonCS Digimon", 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["NON-CS"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    // For the no-fire case, the cost-reduction predicate (trait_has: CS) on
    // when_any_ally_played rejects NON-CS, so no Pending prompt installs and
    // play_from_hand returns Some(field_index) on success.
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "NON-CS Digimon play should succeed without any optional cost-reduction prompt"
    );
    // Defensive cleanup of any unexpected pending state.
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 5,
        "NON-CS Digimon must pay full printed cost 5 — predicate trait_has:CS \
         must reject it; got {cost_paid}"
    );

    // BT22-094 must still be on field — pay_cost was never run.
    let yuugo_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        yuugo_on_field,
        "BT22-094 must remain on field — cost reduction did not fire"
    );
}

/// Negative: opponent's turn — `active_when: { your_turn: true }` gate must
/// suppress the cost reduction. We exercise this by making it P1's turn,
/// having P1 play a CS Digimon, and asserting BT22-094 (P0's tamer) does
/// NOT fire its cost reduction.
///
/// Note: the predicate `cost_target_card` carries the card being played,
/// and `lower_cost_reduction.rs` evaluates the predicate against
/// `EffectReadContext` whose `player` is the OBSERVER (the controller of
/// BT22-094 = P0). On P1's turn, `your_turn` (P0's perspective) is FALSE,
/// blocking the reduction. Additionally, the opponent's CS digimon doesn't
/// match owner-scoping anyway (cost_target_card belongs to P1, but the
/// predicate doesn't gate on owner — the active_when:your_turn is the
/// effective gate here).
#[test]
fn bt22_094_cost_reduction_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-094 in embedded DSL pack")
        .add_card(make_cs_digimon("OPP-CS-DIGI", "Opp CS Digimon", 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .hand(1, &["OPP-CS-DIGI"])
        .memory(0) // 0 memory means P0 → end_turn flips to P1's turn
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    // Hand off to P1's turn.
    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );

    // P1 plays the opponent CS Digimon. P0's BT22-094 cost reduction must NOT
    // fire because active_when:your_turn (from P0's perspective) is false.
    let mem_before = runner.memory();
    let played = runner.play(1, 0);
    assert!(
        played.is_some(),
        "P1's CS-DIGI play should succeed with no cost-reduction prompt installed"
    );
    drive_accept(&mut runner, 1);
    let _ = runner.auto_resolve();

    // Cost paid by P1 is reflected in memory delta (absolute). With no
    // reduction, the full 5-cost ally play makes memory swing by 5.
    let mem_swing = (runner.memory() - mem_before).abs();
    assert_eq!(
        mem_swing, 5,
        "P1's CS-Digimon must pay full cost 5 — P0's BT22-094 cost reduction \
         must not fire on opponent's turn; mem swing was {mem_swing}"
    );

    // BT22-094 must remain on P0's field — pay_cost not run.
    let yuugo_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        yuugo_on_field,
        "BT22-094 must remain on P0's field — cost reduction did not fire on opponent's turn"
    );
}

/// Optional decline: cost reduction is `optional: true` (DCGO isOptional:true).
/// Player should be able to PASS the prompt and pay full cost — BT22-094
/// stays on field.
///
/// Implementation note: a single-trigger optional cost-reduction installs
/// the cost-reduction selection as an optional prompt. Sending PASS skips
/// the reduction and skips the pay_cost. This mirrors the BT13-007
/// optional Royal Knight reduction precedent.
#[test]
fn bt22_094_cost_reduction_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-094 in embedded DSL pack")
        .add_card(make_cs_digimon("CS-DIGI", "CS Digimon", 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["CS-DIGI"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0); // Pending → None when cost-reduction prompt installs

    // The optional cost-reduction prompt should install; we'll PASS to decline.
    assert!(
        runner.game.pending_selection.is_some(),
        "cost-reduction prompt must install (we'll decline below)"
    );

    // Decline every optional prompt by passing.
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids
                .iter()
                .copied()
                .find(|&id| id != PASS)
                .unwrap_or(view.valid_action_ids[0])
        };
        runner
            .execute_action(0, action)
            .expect("execute pending selection (declining where optional)");
    }
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 5,
        "with cost reduction declined, full 5-cost is paid; got {cost_paid}"
    );

    let yuugo_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        yuugo_on_field,
        "BT22-094 must remain on field when the player declines the cost reduction"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings across the test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_cs_option("X", "X", 0);
}
