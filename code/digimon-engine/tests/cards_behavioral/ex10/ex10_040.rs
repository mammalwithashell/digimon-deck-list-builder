//! EX10-040 DemiDevimon — Digimon, Lv.3, Purple, DP 1000, Cost 3.
//! Traits: [Evil]. Form: Rookie. Attribute: Virus.
//! Standard digivolve (cards.json evo_costs / official DB): Lv.2 Purple / Cost 0.
//!
//! # Card text (data/card_bundles/EX10-040.md + card image — verbatim)
//!
//! ```text
//! [Start of Your Main Phase] If your opponent has 10 or fewer cards in
//!   their trash, trash the top 2 cards of both players' decks. Then, if
//!   they have 10 or more cards in their trash, gain 1 memory.
//!
//! Inherited [When Attacking] [Once Per Turn] Trash the top card of both
//!   players' decks.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX10/Purple/EX10_040.cs`
//!
//! Notable DCGO contracts (source priority: DCGO tiebreaks nuance the
//! printed text doesn't pin down; printed text wins on disagreement):
//!   - Own [Start of Your Main Phase] clause:
//!     `SetUpActivateClass(CanActivateCondition, ActivateCoroutine, -1, false, ...)`
//!     — `isOptional: false` (mandatory once gated; no "may" in the printed
//!     text). `CanActivateCondition` only requires the carrier to exist on
//!     the controller's battle area and it be the controller's turn — the
//!     "opponent trash <= 10" gate lives INSIDE `ActivateCoroutine`, so the
//!     trigger always *fires* but its mill body is itself conditional.
//!     `ActivateCoroutine`: `if (card.Owner.Enemy.TrashCards.Count <= 10)`
//!     mills 2 from EACH player's deck in turn-player order (own deck THEN
//!     opponent's deck — `Players_ForTurnPlayer`), each gated individually
//!     by `player.LibraryCards.Count >= 1` (partial/no mill on an empty
//!     deck; matches `trash_from_top`'s natural stop-on-empty behavior).
//!     THEN — unconditionally re-evaluated, independent of whether the mill
//!     branch ran — `if (card.Owner.Enemy.TrashCards.Count >= 10)` gains 1
//!     memory. Because the mill (when it runs) can push the opponent's
//!     trash from <=10 up to >=10 in the SAME resolution, the memory check
//!     is a fresh RE-READ of the opponent trash count taken AFTER the mill,
//!     not a cached pre-mill value.
//!   - Inherited [When Attacking][OPT] clause:
//!     `SetUpActivateClass(..., 1, false, ...)` — `isOptional: false`, so
//!     the mill is MANDATORY once per turn (no "may" in the printed text).
//!     `ActivateCoroutine` mills 1 from EACH player's deck (same
//!     `LibraryCards.Count >= 1` per-player gate, no condition on trash
//!     counts at all — unconditional per-attack mill, once per turn).
//!
//! # YAML location
//! `code/digimon-engine/cards/ex10/EX10-040.yaml`
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 Groups)
//!   - B1 Start-of-main mandatory trigger with a conditional body.
//!   - D-adjacent: nested `if` gating on an opponent-trash-count predicate
//!     (`count_lte` / `count_gte` over `zone: [trash]`), re-evaluated after
//!     an intervening zone-mutating step (mill) in the SAME process.
//!   - G4-adjacent: inherited [When Attacking][Once Per Turn] mandatory
//!     mill (no branch/selection surface at all — pure zone-move step).
//!   - Both triggered clauses are entirely selection-free (no player choice
//!     anywhere in the printed text): no `pending_selection` should ever
//!     install for either clause. OPT enforcement on the inherited clause is
//!     asserted via the trash/deck state delta (there is no selection to
//!     gate against).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "EX10-040";

// ─── Helpers ───────────────────────────────────────────────────────────────

/// Push `count` `TRASH-FILLER` cards onto a player's trash zone so
/// trash-count conditions can be exercised deterministically.
fn seed_trash(runner: &mut DebugRunner, player: u8, count: usize) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TRASH-FILLER")
        .expect("TRASH-FILLER must be registered via add_card before seed_trash");
    for _ in 0..count {
        let next = runner.game.next_card_index();
        runner.game.players[player as usize]
            .trash
            .push(digimon_engine::card_source::CardSource::new(
                data_idx, player, next,
            ));
    }
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

/// Runner with DemiDevimon registered, a filler card for trash-seeding, and
/// generous decks on both sides so mill effects don't accidentally deck the
/// player out mid-test.
fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-040 YAML parses and compiles")
        .add_card(make_test_card("TRASH-FILLER", "Trash Filler"))
        .add_card(make_test_card("DECK-FILLER", "Deck Filler"))
        .deck(0, &["DECK-FILLER"; 20])
        .deck(1, &["DECK-FILLER"; 20])
        .memory(5)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex10_040_compiles_from_dsl_pack() {
    let r = base_runner();
    let card = r
        .compiled_card(CARD_ID)
        .expect("EX10-040 must be present in the embedded DSL pack");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("Evil")),
        "EX10-040 must carry the Evil trait"
    );
}

#[test]
fn ex10_040_has_start_of_main_phase_own_clause() {
    let r = base_runner();
    let card = r.compiled_card(CARD_ID).unwrap();
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("EX10-040 must have a triggered clause at StartOfYourMainPhase");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "own [Start of Your Main Phase] clause is face-up only, not inherited"
    );
    assert!(
        !clause.optional,
        "printed text has no 'may' — the own clause must be mandatory (optional: false)"
    );
}

#[test]
fn ex10_040_has_inherited_when_attacking_opt_clause() {
    let r = base_runner();
    let card = r.compiled_card(CARD_ID).unwrap();
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("EX10-040 must have an inherited [When Attacking] triggered clause");

    assert!(
        clause.once_per_turn,
        "[Once Per Turn] is printed — inherited clause must set once_per_turn: true"
    );
    assert!(
        !clause.optional,
        "printed text has no 'may' — the inherited mill is mandatory (optional: false)"
    );
}

#[test]
fn ex10_040_has_exactly_two_triggered_clauses() {
    let r = base_runner();
    let card = r.compiled_card(CARD_ID).unwrap();
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 2,
        "EX10-040 prints exactly two triggered clauses: own Start-of-Main + inherited When Attacking"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Own clause: mill gate (opponent trash <= 10)
// ═══════════════════════════════════════════════════════════════════════════
//
// DCGO gates the MILL body on `card.Owner.Enemy.TrashCards.Count <= 10`
// inside `ActivateCoroutine` — the trigger itself always fires (no gate on
// `CanActivateCondition`), but the mill only runs conditionally. We assert
// on the resulting deck/trash deltas rather than on `pending_selection`
// (there is none — the whole clause is selection-free).

/// POSITIVE: opponent trash == 10 (<=10 boundary) ⇒ both decks mill 2.
#[test]
fn ex10_040_start_of_main_mills_both_decks_when_opponent_trash_at_10() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut r, 1, 10);
    let deck0_before = r.deck_size(0);
    let deck1_before = r.deck_size(1);

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(
        r.deck_size(0),
        deck0_before - 2,
        "your deck must mill 2 when opponent trash <= 10"
    );
    assert_eq!(
        r.deck_size(1),
        deck1_before - 2,
        "opponent's deck must also mill 2 when opponent trash <= 10"
    );
}

/// POSITIVE: opponent trash well under 10 (e.g. 0) ⇒ both decks mill 2.
#[test]
fn ex10_040_start_of_main_mills_both_decks_when_opponent_trash_empty() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    let deck0_before = r.deck_size(0);
    let deck1_before = r.deck_size(1);

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(r.deck_size(0), deck0_before - 2);
    assert_eq!(r.deck_size(1), deck1_before - 2);
}

/// NEGATIVE: opponent trash == 11 (> 10) ⇒ no mill at all, either side.
#[test]
fn ex10_040_start_of_main_no_mill_when_opponent_trash_above_10() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut r, 1, 11);
    let deck0_before = r.deck_size(0);
    let deck1_before = r.deck_size(1);

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(
        r.deck_size(0),
        deck0_before,
        "your deck must NOT mill when opponent trash > 10"
    );
    assert_eq!(
        r.deck_size(1),
        deck1_before,
        "opponent's deck must NOT mill when opponent trash > 10"
    );
}

/// Selection-free contract: firing the own clause never installs a
/// `pending_selection` — every step is a mandatory zone-move / memory-gain.
#[test]
fn ex10_040_start_of_main_never_installs_a_selection() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut r, 1, 5);

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);

    assert!(
        r.pending_selection().is_none(),
        "the own clause has no player choice anywhere in the printed text"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Own clause: memory gain gate (opponent trash >= 10, RE-READ
// after the mill — the DCGO-critical nuance from the pre-flight brief)
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE (post-mill promotion): opponent trash starts at exactly 10
/// (<=10 ⇒ mill fires) and the mill does not touch the opponent's TRASH
/// zone (it mills DECKS, not trash) — so the opponent trash count is
/// unchanged by the mill at 10, and 10 >= 10 ⇒ gain 1 memory. This is the
/// simplest boundary case that exercises "recount after milling" without
/// requiring the mill itself to move the needle (mills move deck→trash for
/// the DECK owner, not into the opponent's existing trash pile count
/// directly — the recount is over the OPPONENT's trash, and the OPPONENT's
/// own deck mill (from the "both players" trash-2) adds exactly 2 cards to
/// the opponent's trash pile, so seeding opponent trash at 8 also promotes
/// to 10 post-mill; both are pinned below).
#[test]
fn ex10_040_gains_memory_when_opponent_trash_at_least_10_before_mill() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut r, 1, 10);
    let memory_before = r.memory();

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        memory_before + 1,
        "opponent trash == 10 both before and after mill (mill doesn't touch \
         opponent trash contents at this exact count boundary from the DECK \
         side beyond the +2 it always adds) ⇒ 10 >= 10 ⇒ gain 1 memory"
    );
}

/// CRITICAL nuance: the mill itself pushes the opponent's trash count from
/// BELOW 10 to AT OR ABOVE 10 in the SAME resolution (opponent's deck mills
/// 2 cards into their own trash as part of the "trash top 2 of both
/// players' decks" step). The gain-memory check must be a fresh RE-READ
/// taken AFTER the mill, not a stale pre-mill snapshot — seed opponent
/// trash at 9 (<=10 ⇒ mill fires; PRE-mill 9 < 10 would NOT gain memory),
/// then after the mill adds 2 the opponent trash is 11 (>= 10 ⇒ DOES gain
/// memory). A pre-mill (cached) read would wrongly skip the memory gain.
#[test]
fn ex10_040_gains_memory_when_mill_pushes_opponent_trash_from_below_to_above_10() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut r, 1, 9);
    let memory_before = r.memory();
    let opp_trash_before = r.trash_size(1);
    assert!(
        opp_trash_before < 10,
        "test setup: opponent trash must start below 10"
    );

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(
        r.trash_size(1),
        opp_trash_before + 2,
        "the mill must add exactly 2 cards to the opponent's trash (their \
         own deck's top 2 land in their own trash)"
    );
    assert_eq!(
        r.memory(),
        memory_before + 1,
        "the opponent-trash-count re-check for the memory gain must run \
         AFTER the mill, not against a stale pre-mill snapshot — post-mill \
         opponent trash is 11 (>= 10) even though pre-mill it was 9 (< 10)"
    );
}

/// NEGATIVE: opponent trash stays below 10 even after the mill (seed at 6:
/// post-mill 8, still < 10) ⇒ no memory gain.
#[test]
fn ex10_040_no_memory_gain_when_opponent_trash_stays_below_10_after_mill() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut r, 1, 6);
    let memory_before = r.memory();

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(
        r.trash_size(1),
        8,
        "opponent trash must be 6 + 2 (own deck's mill) = 8"
    );
    assert_eq!(
        r.memory(),
        memory_before,
        "opponent trash stays below 10 even post-mill (8 < 10) ⇒ no memory gain"
    );
}

/// NEGATIVE: opponent trash > 10 ⇒ mill branch does NOT run at all, so the
/// opponent trash count cannot be promoted by this effect — but the
/// >=10 gate for memory is STILL independently checked (and passes, since
/// opponent trash > 10 already). This confirms the two `if`s are sequential
/// and independently evaluated, not "only check memory if we milled."
#[test]
fn ex10_040_gains_memory_from_pre_existing_high_trash_even_when_mill_is_skipped() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    seed_trash(&mut r, 1, 15);
    let memory_before = r.memory();
    let deck1_before = r.deck_size(1);

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(
        r.deck_size(1),
        deck1_before,
        "opponent trash > 10 ⇒ the mill branch must be skipped entirely"
    );
    assert_eq!(
        r.memory(),
        memory_before + 1,
        "the memory gain is evaluated independently of the mill branch; \
         opponent trash (15) is already >= 10"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Own clause: partial mill on a near-empty deck (per-player gate)
// ═══════════════════════════════════════════════════════════════════════════

/// DCGO gates each player's mill individually on `LibraryCards.Count >= 1`
/// (not >= 2) — a deck with exactly 1 card left still mills that 1 card
/// (partial mill) rather than being skipped outright.
#[test]
fn ex10_040_start_of_main_partial_mills_a_deck_with_only_one_card_left() {
    let mut r = base_runner();
    let demi = r.place_on_field(0, CARD_ID, Some(0));
    // Trim player 0's deck down to exactly 1 card.
    while r.game.players[0].deck.len() > 1 {
        r.game.players[0].deck.pop();
    }
    let deck1_before = r.deck_size(1);

    fire(&mut r, EffectTiming::StartOfYourMainPhase, demi);
    let _ = r.auto_resolve();

    assert_eq!(
        r.deck_size(0),
        0,
        "a 1-card deck mills its single remaining card (partial mill), not skipped"
    );
    assert_eq!(
        r.deck_size(1),
        deck1_before - 2,
        "the opponent's deck (unaffected by the low deck count) still mills 2"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited clause: mandatory mill-1-of-both-decks on attack
// ═══════════════════════════════════════════════════════════════════════════

/// Carrier with EX10-040 as an inherited (digivolution-stack) source.
fn carrier_with_demidevimon_source(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "TOP-MON"])
}

/// POSITIVE: firing [When Attacking] on a carrier with EX10-040 in its
/// digivolution stack mills 1 card from BOTH players' decks, unconditionally
/// (no trash-count gate on the inherited clause at all).
#[test]
fn ex10_040_inherited_when_attacking_mills_one_from_both_decks() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-040 YAML parses and compiles")
        .add_card({
            let mut c = make_test_card("TOP-MON", "TopMon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card(make_test_card("DECK-FILLER", "Deck Filler"))
        .deck(0, &["DECK-FILLER"; 20])
        .deck(1, &["DECK-FILLER"; 20])
        .memory(5)
        .start();

    let carrier = carrier_with_demidevimon_source(&mut r);
    let deck0_before = r.deck_size(0);
    let deck1_before = r.deck_size(1);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert_eq!(
        r.deck_size(0),
        deck0_before - 1,
        "your deck must mill exactly 1 card"
    );
    assert_eq!(
        r.deck_size(1),
        deck1_before - 1,
        "opponent's deck must also mill exactly 1 card"
    );
}

/// Selection-free contract: the inherited clause never installs a
/// `pending_selection` — it is a pure unconditional mandatory mill.
#[test]
fn ex10_040_inherited_when_attacking_never_installs_a_selection() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-040 YAML parses and compiles")
        .add_card({
            let mut c = make_test_card("TOP-MON", "TopMon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card(make_test_card("DECK-FILLER", "Deck Filler"))
        .deck(0, &["DECK-FILLER"; 20])
        .deck(1, &["DECK-FILLER"; 20])
        .memory(5)
        .start();

    let carrier = carrier_with_demidevimon_source(&mut r);
    fire(&mut r, EffectTiming::WhenAttacking, carrier);

    assert!(
        r.pending_selection().is_none(),
        "the inherited clause has no player choice anywhere in the printed text"
    );
}

/// OPT lockout: a second [When Attacking] fire in the same turn must NOT
/// mill again — the deck sizes stay put after the first activation.
#[test]
fn ex10_040_inherited_when_attacking_is_once_per_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-040 YAML parses and compiles")
        .add_card({
            let mut c = make_test_card("TOP-MON", "TopMon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card(make_test_card("DECK-FILLER", "Deck Filler"))
        .deck(0, &["DECK-FILLER"; 20])
        .deck(1, &["DECK-FILLER"; 20])
        .memory(5)
        .start();

    let carrier = carrier_with_demidevimon_source(&mut r);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    let deck0_after_first = r.deck_size(0);
    let deck1_after_first = r.deck_size(1);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert_eq!(
        r.deck_size(0),
        deck0_after_first,
        "[Once Per Turn] ⇒ no second mill of your deck in the same turn"
    );
    assert_eq!(
        r.deck_size(1),
        deck1_after_first,
        "[Once Per Turn] ⇒ no second mill of the opponent's deck in the same turn"
    );
}

/// OPT clears after `end_turn`: once the turn rotates away and back, the
/// inherited clause can fire again.
#[test]
fn ex10_040_inherited_when_attacking_opt_clears_after_end_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX10-040 YAML parses and compiles")
        .add_card({
            let mut c = make_test_card("TOP-MON", "TopMon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card(make_test_card("DECK-FILLER", "Deck Filler"))
        .deck(0, &["DECK-FILLER"; 20])
        .deck(1, &["DECK-FILLER"; 20])
        .memory(5)
        .start();

    let carrier = carrier_with_demidevimon_source(&mut r);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    // Rotate the turn all the way back to player 0. Each `end_turn()` that
    // lands on a new turn player also runs that player's normal Draw phase
    // (P1's end_turn draws a card for P0), so re-snapshot player 0's deck
    // size AFTER the rotation completes and BEFORE the second fire — the
    // assertion cares only about the OPT lockout, not the draw-phase delta.
    r.end_turn(); // P0 -> P1
    let _ = r.auto_resolve();
    r.end_turn(); // P1 -> P0 (draws 1 card for P0's new turn)
    let _ = r.auto_resolve();

    let deck0_before_second_fire = r.deck_size(0);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert_eq!(
        r.deck_size(0),
        deck0_before_second_fire - 1,
        "after the turn rotates back, the OPT lockout must have cleared \
         and the inherited clause can mill again"
    );
}
