//! BT9-070 Gazimon (X Antibody) — Digimon, Lv.3, Purple, DP 3000, Cost 3.
//! Traits: Mammal / X Antibody. Form: Rookie. Attribute: Virus.
//! Standard digivolve (cards.json evo_costs): Lv.2 Purple / Cost 0.
//!
//! # Card text (card image BT9-070.webp — verbatim; matches cards.json)
//!
//! ```text
//! Digivolve: 0 from [Gazimon]
//! [When Digivolving] Trash the top 3 cards of your deck.
//! ```
//!
//! No inherited effect, no security effect (per-card JSON and card image both
//! confirm empty inherited/security text — the "Inherited Effect" frame label
//! on the card face carries no text on this printing).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT9/Purple/BT9_070.cs
//!
//! - `timing == EffectTiming.None`: `AddSelfDigivolutionRequirementStaticEffect`
//!   with `PermanentCondition = targetPermanent.TopCard.CardNames.Contains("Gazimon")`,
//!   `digivolutionCost: 0` → alt_path `{ kind: digivolve, from: { name_contains:
//!   "Gazimon" }, cost: 0 }`.
//! - `timing == EffectTiming.OnEnterFieldAnyone` (fires for BOTH On Play AND
//!   When Digivolving per DCGO's `CanTriggerWhenDigivolving` gate check inside
//!   `CanUseCondition`; `CanActivateCondition` additionally requires
//!   `Owner.LibraryCards.Count >= 1`): mandatory
//!   `IAddTrashCardsFromLibraryTop(3, card.Owner, ...)` → `trash_from_top: { of:
//!   you, count: 3 }`. No selection is exposed — DCGO trashes unconditionally
//!   off the top, no player choice.
//! - The printed text has ONLY `[When Digivolving]` (no `[On Play]` tag), so
//!   this spec fires solely on `when_digivolving`, matching the printed text
//!   over DCGO's broader `OnEnterFieldAnyone` internal wiring (DCGO's
//!   `CanUseCondition` gate — `CanTriggerWhenDigivolving` — restricts the
//!   *effective* firing window to When Digivolving only, so the two sources
//!   agree on final behavior).
//!
//! # Patterns this test covers
//! - Alt-path: `[Digivolve] 0 from [Gazimon]` name-gated cost-0 alt-digivolve.
//! - Simple mandatory `[When Digivolving]` mill-3 (no selection, no condition
//!   gate beyond DCGO's library-count guard, which the DSL step already
//!   no-ops on an empty deck).
//! - Cost-firing event log: `trash_from_top` must emit `GameEvent::Reveal`
//!   (`RevealZone::TrashFromDeckTop`) and `GameEvent::Trash` per card milled.
//! - Negative: `on_play` alone (not digivolving) must NOT fire the clause.
//! - Negative: an empty deck must not panic and must trash 0 cards (DCGO
//!   `Owner.LibraryCards.Count >= 1` guard mirrored by trash_from_top's
//!   natural `Ok(None) => break`).
//! - Integrated: `Game::digivolve_from_hand` (full production digivolve
//!   flow — memory paid, hand card removed, stacked on the base, triggers
//!   drained) exercises the clause end-to-end, both accept and reject paths.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT9-070";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A Lv.2 Purple "Gazimon"-named Digimon to use as the standard digivolve base.
fn gazimon_base(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Gazimon", 2);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.dp = Some(1000);
    c
}

/// A Lv.2 Purple Digimon whose name does NOT contain the substring "Gazimon"
/// — negative control for the alt-path name gate. (Careful: a name like
/// "NotGazimon" would still `.contains("Gazimon")` — must not share the
/// substring at all.)
fn non_gazimon_base(id: &str) -> CardData {
    let mut c = make_test_card_with_level(id, "Palmon", 2);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.dp = Some(1000);
    c
}

/// Standard runner: BT9-070 in the embedded DSL pack + a Gazimon-named base
/// in hand, ready to digivolve, with a non-trivial deck to mill from.
fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 must be in embedded DSL pack")
        .add_card(gazimon_base("GAZIMON-BASE"))
        .add_card(non_gazimon_base("NOT-GAZIMON-BASE"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 10])
        .memory(6)
        .start()
}

/// Manually enqueue+drain a triggered timing on a permanent — used for
/// clause-firing tests that don't need the full digivolve action flow.
fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt9_070_compiles_from_dsl_pack_with_correct_metadata() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("BT9-070 in pack");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(3000));
    assert!(
        card.traits.iter().any(|t| t == "Mammal"),
        "traits must include Mammal; got {:?}",
        card.traits
    );
    assert!(
        card.traits.iter().any(|t| t == "X Antibody"),
        "traits must include X Antibody; got {:?}",
        card.traits
    );
}

/// Alt-path: "Digivolve: 0 from [Gazimon]" — name-gated, cost 0.
#[test]
fn bt9_070_has_gazimon_name_gated_cost_0_alt_path() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("BT9-070 in pack");
    let alt = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.from
                .as_ref()
                .is_some_and(|f| f.name_contains.as_deref() == Some("Gazimon"))
    });
    assert!(
        alt.is_some(),
        "must have a Gazimon-name-gated Digivolve alt-path at cost 0; paths: {:?}",
        card.alt_paths
            .iter()
            .map(|p| (&p.kind, &p.cost))
            .collect::<Vec<_>>()
    );
}

/// Exactly one triggered clause: [When Digivolving] mill 3.
#[test]
fn bt9_070_has_exactly_one_triggered_clause() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("BT9-070 in pack");

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
        1,
        "BT9-070 has exactly one triggered clause (When Digivolving mill 3)"
    );
}

/// The triggered clause fires ONLY on WhenDigivolving (printed text has no
/// [On Play] tag), is mandatory (no "you may"), is own-scope (not inherited),
/// and has no [Once Per Turn] tag.
#[test]
fn bt9_070_clause_is_when_digivolving_only_mandatory_own_scope() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("BT9-070 in pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .next()
        .expect("the triggered clause must exist");

    assert_eq!(
        clause.when,
        vec![CompiledTiming::WhenDigivolving],
        "clause must fire on WhenDigivolving only — printed text has no [On Play]"
    );
    assert!(!clause.optional, "mandatory — no 'you may' in printed text");
    assert!(
        !clause.once_per_turn,
        "no [Once Per Turn] tag in printed text"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "own-scope effect, not inherited (this printing has no inherited text)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: [When Digivolving] mills the top 3 deck cards
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: firing WhenDigivolving mills exactly 3 cards from the top of the
/// deck into the trash, with no player selection involved (mandatory, no
/// choice).
#[test]
fn bt9_070_when_digivolving_mills_top_3_cards() {
    let mut r = base_runner();
    let handle = r.place_on_field(0, CARD_ID, Some(0));
    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);

    fire(&mut r, EffectTiming::WhenDigivolving, handle);

    assert!(
        r.game.pending_selection.is_none(),
        "trash_from_top exposes no player choice — mandatory, no selection"
    );
    assert_eq!(
        r.deck_size(0),
        deck_before - 3,
        "deck must shrink by exactly 3"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 3,
        "trash must grow by exactly 3"
    );
}

/// NEGATIVE: firing plain OnPlay (not digivolving) must NOT trigger the
/// clause — the printed text has no [On Play] tag.
#[test]
fn bt9_070_on_play_alone_does_not_mill() {
    let mut r = base_runner();
    let handle = r.place_on_field(0, CARD_ID, Some(0));
    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);

    fire(&mut r, EffectTiming::OnPlay, handle);

    assert_eq!(
        r.deck_size(0),
        deck_before,
        "OnPlay alone must not mill — no [On Play] tag on the printed clause"
    );
    assert_eq!(r.trash_size(0), trash_before, "trash must be unchanged");
}

/// NEGATIVE: an unrelated timing (WhenAttacking) must not fire the clause.
#[test]
fn bt9_070_when_attacking_does_not_mill() {
    let mut r = base_runner();
    let handle = r.place_on_field(0, CARD_ID, Some(0));
    let deck_before = r.deck_size(0);

    fire(&mut r, EffectTiming::WhenAttacking, handle);

    assert_eq!(
        r.deck_size(0),
        deck_before,
        "WhenAttacking must not fire the mill clause"
    );
    assert!(r.game.pending_selection.is_none());
}

/// EDGE: an empty deck must not panic; trash_from_top naturally stops at 0
/// cards available (DCGO's `Owner.LibraryCards.Count >= 1` guard is mirrored
/// by the engine's `Ok(None) => break`).
#[test]
fn bt9_070_when_digivolving_with_empty_deck_no_panic_mills_zero() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 must be in embedded DSL pack")
        .add_card(gazimon_base("GAZIMON-BASE-EMPTY"))
        .hand(0, &[CARD_ID])
        .memory(6)
        .start();

    let handle = r.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(r.deck_size(0), 0, "test setup: deck starts empty");
    let trash_before = r.trash_size(0);

    fire(&mut r, EffectTiming::WhenDigivolving, handle);

    assert_eq!(r.deck_size(0), 0, "deck stays empty, no panic");
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "nothing to mill from an empty deck"
    );
}

/// EDGE: a deck with fewer than 3 cards mills only what's available (deck
/// exhausts partway through, matching `trash_from_top`'s `Ok(None) => break`).
#[test]
fn bt9_070_when_digivolving_with_two_deck_cards_mills_only_two() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 must be in embedded DSL pack")
        .add_card(gazimon_base("GAZIMON-BASE-SHORT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 2])
        .memory(6)
        .start();

    let handle = r.place_on_field(0, CARD_ID, Some(0));
    let trash_before = r.trash_size(0);
    assert_eq!(r.deck_size(0), 2);

    fire(&mut r, EffectTiming::WhenDigivolving, handle);

    assert_eq!(r.deck_size(0), 0, "deck exhausts entirely");
    assert_eq!(
        r.trash_size(0),
        trash_before + 2,
        "only the 2 available cards are milled, no panic on running out"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Cost-firing event log: Reveal + Trash events for each milled card
// ═══════════════════════════════════════════════════════════════════════════

/// The mill must emit a `GameEvent::Reveal` (TrashFromDeckTop) AND a
/// `GameEvent::Trash` event for each of the 3 cards milled — the engine's
/// `trash_from_top` routes through the canonical trash path so downstream
/// observers (e.g. cards that react to `OnDiscard`/trash events) see it.
#[test]
fn bt9_070_mill_emits_reveal_and_trash_events_for_each_card() {
    let mut r = base_runner();
    let handle = r.place_on_field(0, CARD_ID, Some(0));

    let cp = r.event_checkpoint();
    fire(&mut r, EffectTiming::WhenDigivolving, handle);

    let events = r.events_since(cp);
    let reveal_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Reveal { .. }))
        .count();
    let trash_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .count();

    assert_eq!(
        reveal_count, 3,
        "one Reveal event must fire per milled card (3 total)"
    );
    assert_eq!(
        trash_count, 3,
        "one Trash event must fire per milled card (3 total)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Integrated: real digivolve action drives the clause
// ═══════════════════════════════════════════════════════════════════════════

/// INTEGRATED (positive): play a Gazimon-named base, drive the ACTUAL
/// digivolve action via `Game::digivolve_from_hand` (not `place_on_field`
/// followed by a manual `fire`), and confirm the mill fires end-to-end
/// through the full digivolution flow (memory paid, card removed from hand,
/// stacked on the base, `WhenDigivolving` triggers drained).
#[test]
fn bt9_070_real_digivolve_action_mills_top_3_via_full_flow() {
    let mut r = base_runner();

    // Player 0 plays the Gazimon-named base onto the field this turn.
    let base = r.place_on_field(0, "GAZIMON-BASE", Some(0));
    assert_eq!(base.index, 0);

    // BT9-070 sits in hand[0] (only card in hand).
    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);
    let field_before = r.battle_area_size(0);

    assert!(
        r.game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "digivolving Gazimon into Gazimon (X Antibody) at cost 0 must succeed"
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(0),
        field_before,
        "digivolving replaces the top card in place — field size unchanged"
    );
    // `Game::digivolve_from_hand` also draws 1 card as part of the standard
    // digivolution flow (separate from this card's own effect), so the deck
    // shrinks by 1 (draw) + 3 (mill) = 4 total.
    assert_eq!(
        r.deck_size(0),
        deck_before - 4,
        "the real digivolve action must draw 1 (standard digivolve draw) then \
         fire [When Digivolving] and mill 3 more (draw 1 + mill 3 = -4 total)"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 3,
        "3 cards land in the trash from the real digivolve flow (the drawn \
         card goes to hand, not trash)"
    );
}

/// INTEGRATED (negative): digivolving onto a base NOT named Gazimon must be
/// illegal via the cost-0 alt-path (the name gate blocks the action). This
/// base has no other digivolution route (no printed evo_costs on the
/// synthetic fixture, no alt-path match), so the attempt must fail outright.
#[test]
fn bt9_070_alt_path_rejects_non_gazimon_base() {
    let mut r = base_runner();

    // Player 0 plays the NON-Gazimon-named base onto the field this turn.
    let base = r.place_on_field(0, "NOT-GAZIMON-BASE", Some(0));
    assert_eq!(base.index, 0);
    let deck_before = r.deck_size(0);

    let succeeded =
        r.game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand);

    assert!(
        !succeeded,
        "the cost-0 alt-path must reject a base not named Gazimon"
    );
    assert_eq!(
        r.deck_size(0),
        deck_before,
        "a rejected digivolve must not mill any cards"
    );
}
