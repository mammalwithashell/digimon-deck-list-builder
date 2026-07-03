//! BT21-087 Zenith — Tamer, Black, Cost 4.
//!
//! # Card text (data/cards.json / official bundle — verbatim)
//!
//! ```text
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [On Play] Reveal the top 3 cards of your deck. Among them, play 1
//! [Vemmon] without paying the cost or add 1 card with [Vemmon] in its
//! text to the hand. Trash the rest.
//! ```
//!
//! Inherited (Security):
//!   Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_087.cs`
//!
//! Notable DCGO contracts:
//!   - `[Start of Your Turn]` → `CardEffectFactory.SetMemoryTo3TamerEffect(card)`
//!     — unconditional call; the "if memory <= 2" gate lives inside the
//!     factory. Not player-optional, no `[Once Per Turn]`.
//!   - `[On Play]` → `ActivateClass` with `SetUpActivateClass(canUseCondition,
//!     canActivateCondition, -1, false, ...)` — outer trigger is MANDATORY
//!     (no "you may" in printed text; `isOptional: false`). Body:
//!     `CardEffectCommons.SimplifiedRevealDeckTopCardsAndSelect` with
//!     `revealCount: 3`, a SINGLE `SimplifiedSelectCardConditionClass` bucket
//!     (`CanSelectCardCondition: cardSource.HasText("Vemmon")`, `mode:
//!     Custom`, `maxCount: 1`), `remainingCardsPlace: RemainingCardsPlace.Trash`,
//!     `canNoSelect: true` (the pick itself is optional — "Among them" implies
//!     zero eligible candidates is a legal outcome, and DCGO's `canNoSelect`
//!     flag confirms the single pick may be declined even when a candidate
//!     exists).
//!     After the (optional) pick resolves:
//!       - `selectedCard == null` → nothing further happens (no play, no add).
//!       - `selectedCard.EqualsCardName("Vemmon")` (EXACT name match) → a
//!         binary "Play" / "Add to your hand" choice
//!         (`GManager.userSelectionManager.SetBoolSelection`) is offered to
//!         the controller. `Play` branch → `CardEffectCommons
//!         .PlayPermanentCards(payCost: false, ...)`. `Add to hand` branch →
//!         `CardEffectCommons.AddThisCardToHand(...)`.
//!       - otherwise (a card with "[Vemmon]" in its text but NOT itself named
//!         "Vemmon") → straight to `CardEffectCommons.AddThisCardToHand(...)`
//!         — no play option, no player choice.
//!     The remaining (unselected) revealed cards go to Trash
//!     (`RemainingCardsPlace.Trash`), regardless of what happened to the pick.
//!   - `[Security]` → `CardEffectFactory.PlaySelfTamerSecurityEffect(card)` —
//!     plays this card from security without paying cost. Standard
//!     `play_from_security` pattern (sister cards: BT22-084, BT22-094,
//!     BT18-087, BT21-015, BT9-092).
//!
//! # DSL modeling notes
//!
//! **In-text filter (`[Vemmon] in its text`):** DCGO's `HasText("Vemmon")`
//! scans a broader surface than the DSL's `effect_text_contains` (card name,
//! dual/effect/inherited/security text, attribute, type/traits, and
//! DNA-jogress condition messages — see `CardSource.HasText`,
//! `DCGO/Assets/Scripts/Script/CardSource.cs:2120`). Every "Vemmon"-lineage
//! card in the real pool (BT11-061, BT11-065, BT11-070, BT18-060, BT21-056,
//! the "Vemmon" printings themselves) carries "Vemmon" in EITHER its printed
//! name OR its printed effect/inherited/security text — none rely on the
//! trait/type/attribute-only surface. The YAML therefore uses
//! `any_of: [name_contains: "Vemmon", effect_text_contains: "Vemmon"]`,
//! faithfully covering the full realized candidate pool without needing the
//! trait/attribute branches of `HasText` (no such card exists to exercise
//! them). This is the "any_of workaround + fidelity comment" flagged by the
//! pre-flight audit brief.
//!
//! **Single unified pick → post-pick name branch (`G-DSL-BINDING-CARD-NAME-IS`):**
//! DCGO uses ONE `SimplifiedSelectCardConditionClass` bucket (not two), so the
//! player picks ANY ONE eligible card from a single combined pool, and ONLY
//! AFTER the pick is made does `EqualsCardName("Vemmon")` decide whether a
//! play/add choice or a forced add follows. This is NOT the "two filtered
//! buckets + `no_duplicate_cards`" idiom used by BT22-017 / EX5-015 / BT20-100
//! (which presents two SEQUENTIAL prompts, changing the player's decision
//! order) — that shape would incorrectly force a player to decide on an
//! exact-name Vemmon candidate BEFORE ever seeing a text-only candidate as an
//! alternative, when DCGO offers both side-by-side in one prompt. Testing the
//! ALREADY-PICKED card's exact name required a new DSL predicate leaf,
//! `binding_card_name_is: { binding, name }` — the exact sibling of the
//! existing `binding_card_kind` leaf (used by LM-020 Quantumon), just testing
//! printed name instead of printed kind. Added in this change
//! (`code/digimon-dsl/src/predicate.rs`, `compiled.rs`, `compile.rs`;
//! evaluator in `code/digimon-engine/src/dsl_cards/predicate.rs`). Unlike
//! `binding_card_kind` (which only reads a `Card`-typed binding), the new leaf
//! also accepts a singleton `CardList` binding — the shape
//! `select_reveal_buckets` always produces, even for a `min:0,max:1` bucket.
//!
//! **"Nothing picked" no-op:** `min: 0` on the bucket lets the pick resolve
//! with zero cards (DCGO `canNoSelect: true`). `binding_card_name_is` fails
//! closed on an empty/absent binding, so the play/add branch never installs
//! with no pick; the fallback "not exactly Vemmon" branch is guarded by
//! `binding_count_eq: { binding: pick, n: 1 }` so it, too, only fires when a
//! card was actually picked (not on the empty-pick no-op).
//!
//! **Trash the rest:** `RemainingCardsPlace.Trash` — modeled via `per_selected`
//! over the `revealed` binding + `trash_from_reveal` (BT24-059 / EX8-050
//! idiom), which naturally trashes every revealed card NOT consumed by the
//! bucket pick (the picked card is removed from `revealed`'s candidate pool
//! by the reveal-bucket state machine once selected).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//!   - B1 Start-of-main/turn tamer (memory swing)
//!   - A1/A2 Searching + two-pass reveal (reveal-3, single bucket, branch on
//!     picked card identity)
//!   - E1 Branch-choice OnPlay (play vs add, gated on exact name)
//!   - F9-adjacent / G4-adjacent: Security effect play-self-free
//!   - §5 Structural: 3 triggered clauses (start_of_your_turn, on_play,
//!     on_security); condition gating; mandatory vs optional flags.
//!   - §5 Condition gating: clause 1 fires at memory<=2 / not above;
//!     clause 2's bucket pick is declinable (min:0) even with a candidate
//!     present (`canNoSelect: true`).
//!   - §5 Behavioral integrated: reveal-3 → pick exact "Vemmon" → branch
//!     play-free vs add-to-hand; reveal-3 → pick a Vemmon-text (non-exact)
//!     card → forced add-to-hand (no choice surfaces); reveal-3 → decline
//!     the pick → no-op; trash-the-rest in every branch.
//!   - §5 Faithfulness gate: outer On Play trigger is MANDATORY (reveal
//!     always fires); the INNER pick is optional (`canNoSelect: true`); the
//!     play-vs-add choice ONLY appears for an exact-name "Vemmon" pick.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

const CARD_ID: &str = "BT21-087";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// Build a Digimon fixture named exactly "Vemmon" (level 3, cost 3, black —
/// matches the real BT11-061 / BT18-060 / BT21-056 printings). Its own
/// `effect_text` mentions "Vemmon" (the real printings all self-reference),
/// so it satisfies the `effect_text_contains: "Vemmon"` arm too (redundant
/// with the `name_contains` arm — both are faithful to DCGO `HasText`).
fn make_exact_vemmon(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(2000);
    c.effect_text =
        "[On Play] Reveal the top 3 cards of your deck. Place 1 [Vemmon] among them.".to_string();
    c
}

/// Build a Digimon fixture with "[Vemmon]" in its printed effect text but a
/// DIFFERENT card name (mirrors the real BT18-060 / BT21-056 / BT11-065
/// pattern: e.g. "Snatchmon" prints "... cards from your trash may also be
/// treated as [Vemmon] ..."). Must be admitted by the reveal bucket's filter
/// but must NOT trigger the play/add choice — only a forced add-to-hand.
fn make_vemmon_text_card(card_id: &str, card_name: &str) -> CardData {
    let mut c = make_test_card(card_id, card_name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.play_cost = 6;
    c.dp = Some(5000);
    c.effect_text = "While you have no Digimon other than [Vemmon], cards may be treated as [Vemmon].".to_string();
    c
}

/// A Digimon fixture whose name AND text carry no "Vemmon" reference at all —
/// used as a non-candidate filler.
fn make_unrelated_digimon(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Patamon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.play_cost = 3;
    c.dp = Some(1000);
    c
}

fn zenith_card_data() -> CardData {
    dsl_card_data::card_data_from_compiled(CARD_ID)
}

/// Resolve a revealed-card action id by candidate card_id. `None` if the
/// named card is not currently in the reveal overlay.
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

/// Pick a revealed card by card_id. Asserts the action is currently legal;
/// panics on mismatch.
fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: revealed card {id} not present"));
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: action {action} for {id} not legal in current selection; \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect(label);
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_087_compiles_as_tamer() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Tamer,
        "BT21-087 Zenith is a Tamer"
    );
    assert_eq!(card.cost, Some(4));
}

#[test]
fn bt21_087_has_exactly_three_triggered_clauses() {
    let card = dsl_card_data::compiled(CARD_ID);
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
        3,
        "BT21-087 prints exactly 3 triggered clauses (Start of Your Turn, On Play, Security); \
         got {} clauses",
        triggered.len()
    );
}

#[test]
fn bt21_087_clause1_start_of_your_turn_not_optional() {
    let card = dsl_card_data::compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::StartOfYourTurn])
        .expect("BT21-087 must have a start_of_your_turn clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        !clause.optional,
        "Clause 1 (set memory to 3) is not player-optional"
    );
    assert!(!clause.once_per_turn);
}

#[test]
fn bt21_087_clause2_on_play_is_mandatory_outer_trigger() {
    let card = dsl_card_data::compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("BT21-087 must have an on_play clause");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        !clause.optional,
        "Printed text has no 'you may' at the trigger level — the reveal-3 \
         always fires when Zenith is played (DCGO isOptional: false)"
    );
    assert!(!clause.once_per_turn);
}

#[test]
fn bt21_087_clause3_on_security_not_optional() {
    let card = dsl_card_data::compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("BT21-087 must have an on_security clause");

    assert!(
        !clause.optional,
        "[Security] play-self is not player-optional (DCGO PlaySelfTamerSecurityEffect, \
         no canNoSelect)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1: [Start of Your Turn] set memory to 3 if memory <= 2
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_087_clause1_fires_when_memory_lte_2() {
    let mut runner = DebugRunner::builder()
        .add_card(zenith_card_data())
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(2)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 2;

    runner.end_turn(); // P1's turn
    runner.end_turn(); // back to P0: begin_turn fires StartOfYourTurn → 2 <= 2 → set 3

    assert_eq!(
        runner.memory(),
        3,
        "memory must be set to 3 when it was 2 at start of P0's turn"
    );
}

#[test]
fn bt21_087_clause1_fires_when_memory_is_zero() {
    let mut runner = DebugRunner::builder()
        .add_card(zenith_card_data())
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 0;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 3);
}

#[test]
fn bt21_087_clause1_does_not_fire_when_memory_above_2() {
    let mut runner = DebugRunner::builder()
        .add_card(zenith_card_data())
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 5;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        5,
        "memory must remain 5 when condition fails (5 > 2)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2: [On Play] reveal 3, pick 1 [Vemmon]-text card
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive path — exact-name "Vemmon" is revealed. Picking it must offer a
/// binary Play/Add-to-hand choice (NOT a forced add). Branch 0 = Play.
#[test]
fn bt21_087_on_play_pick_exact_vemmon_offers_play_branch() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_exact_vemmon("VEMMON-EXACT"))
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .deck(0, &["VEMMON-EXACT", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");

    // Bucket prompt: pick the exact-name Vemmon.
    pick_revealed_by_id(&mut runner, "VEMMON-EXACT", "pick exact Vemmon");

    // A binary Play/Add-to-hand EffectChoice must now be pending.
    let kind = runner.pending_kind().expect("play/add choice must install");
    assert_eq!(
        kind,
        SelectionKind::EffectChoice,
        "picking the EXACT-name Vemmon must offer a play-or-add EffectChoice"
    );

    // Branch 0 = Play (without paying the cost).
    runner.execute_branch(0).expect("select Play branch");
    runner.auto_resolve().expect("resolve remainder (trash rest)");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "Zenith + the played Vemmon must both be on P0's battle area"
    );
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"VEMMON-EXACT".to_string()),
        "played Vemmon must NOT be in hand; hand={:?}",
        hand_ids
    );
}

/// Same setup, but branch 1 = Add to hand (decline the free play).
#[test]
fn bt21_087_on_play_pick_exact_vemmon_add_to_hand_branch() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_exact_vemmon("VEMMON-EXACT"))
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .deck(0, &["VEMMON-EXACT", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");
    pick_revealed_by_id(&mut runner, "VEMMON-EXACT", "pick exact Vemmon");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "play/add choice must install"
    );

    // Branch 1 = Add to hand.
    runner.execute_branch(1).expect("select Add-to-hand branch");
    runner.auto_resolve().expect("resolve remainder (trash rest)");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "only Zenith should be on the battle area — Vemmon was added to hand, not played"
    );
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"VEMMON-EXACT".to_string()),
        "Vemmon must be in hand after the Add-to-hand branch; hand={:?}",
        hand_ids
    );
}

/// Negative path — a [Vemmon]-TEXT card that is NOT itself named "Vemmon" is
/// picked. No play/add choice must appear — it goes straight to hand.
#[test]
fn bt21_087_on_play_pick_vemmon_text_card_forces_add_no_choice() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_vemmon_text_card("SNATCHMON-LIKE", "Snatchmon"))
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .deck(0, &["SNATCHMON-LIKE", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");
    pick_revealed_by_id(&mut runner, "SNATCHMON-LIKE", "pick Vemmon-text (non-exact) card");

    // No further pending selection — the add-to-hand is forced/automatic
    // once the pick's identity is resolved (no branch choice for a
    // non-exact-name pick). `auto_resolve` should complete the trash-rest
    // tail without hitting any EffectChoice.
    if let Some(kind) = runner.pending_kind() {
        assert_ne!(
            kind,
            SelectionKind::EffectChoice,
            "a non-exact-name Vemmon-text pick must NOT offer a play/add choice"
        );
    }
    runner.auto_resolve().expect("resolve remainder (trash rest)");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "only Zenith should be on the battle area — the non-exact pick is never playable"
    );
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"SNATCHMON-LIKE".to_string()),
        "the non-exact Vemmon-text card must be added to hand automatically; hand={:?}",
        hand_ids
    );
}

/// Neither candidate exists in the reveal — the bucket has zero eligible
/// cards and resolves with no pick. No further selection installs; all 3
/// revealed cards trash.
#[test]
fn bt21_087_on_play_no_vemmon_candidate_bottoms_nothing_trashes_all() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .add_card(make_unrelated_digimon("FILLER3"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");
    runner
        .auto_resolve()
        .expect("resolve through the empty bucket + trash-rest tail");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "no Vemmon candidate present → nothing extra enters the field"
    );
    assert_eq!(runner.hand_size(0), 0, "no card added to hand");
    assert_eq!(
        runner.trash_size(0),
        3,
        "all 3 revealed cards must be trashed when nothing is picked"
    );
}

/// A candidate DOES exist, but the player declines the pick (`canNoSelect:
/// true` — the bucket is optional even with a legal target). No play/add
/// choice appears; the declined candidate trashes along with the rest.
#[test]
fn bt21_087_on_play_decline_pick_with_candidate_present_is_legal() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_exact_vemmon("VEMMON-EXACT"))
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .deck(0, &["VEMMON-EXACT", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");

    let view = runner
        .pending_selection_view()
        .expect("bucket prompt installed");
    assert!(
        !view.valid_action_ids.is_empty(),
        "a Vemmon candidate exists — the bucket must offer at least one pick action"
    );
    assert!(
        runner.pending_is_optional(),
        "the bucket pick must be declinable even though a Vemmon candidate exists \
         (DCGO canNoSelect: true). PASS legality is gated purely by \
         `is_optional` (not membership in `valid_action_ids` — see \
         `resolve_generic_selection`), so this flag IS the faithfulness check."
    );

    runner.execute_action(0, PASS).expect("decline the pick");
    runner
        .auto_resolve()
        .expect("resolve trash-rest tail after declining");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "declining the pick must not play anything"
    );
    assert_eq!(runner.hand_size(0), 0, "declining the pick adds nothing to hand");
    assert_eq!(
        runner.trash_size(0),
        3,
        "all 3 revealed cards (including the declined Vemmon) must be trashed"
    );
}

/// Faithfulness: the reveal-bucket filter admits a card whose [Vemmon] match
/// comes from EFFECT TEXT only (not its name) — mirrors real BT18-060 /
/// BT21-056 printings whose own text says "[Vemmon]" without the bracket
/// literally matching their OWN name substring differently from other
/// Vemmon-lineage cards. This test targets a distinctly-named card (not
/// "Vemmon") whose ONLY match surface is effect_text.
#[test]
fn bt21_087_bucket_admits_card_with_vemmon_only_in_effect_text() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_vemmon_text_card("DESTROMON-LIKE", "Destromon"))
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .deck(0, &["DESTROMON-LIKE", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");

    let view = runner
        .pending_selection_view()
        .expect("bucket prompt installed");
    let action = revealed_action_for_id(&runner, "DESTROMON-LIKE")
        .expect("DESTROMON-LIKE is in the reveal overlay");
    assert!(
        view.valid_action_ids.contains(&action),
        "bucket must admit a card whose ONLY [Vemmon] match surface is effect_text; \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
}

/// Faithfulness: a card with NEITHER "Vemmon" in its name NOR in its text
/// must NOT be a legal pick.
#[test]
fn bt21_087_bucket_rejects_unrelated_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_unrelated_digimon("PATAMON-1"))
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .deck(0, &["PATAMON-1", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");

    // Either no selection installs at all (zero-candidate bucket auto-advances)
    // or, if a selection is pending for some other reason, PATAMON-1 must not
    // be a legal target.
    if let Some(view) = runner.pending_selection_view() {
        let action = revealed_action_for_id(&runner, "PATAMON-1");
        if let Some(action) = action {
            assert!(
                !view.valid_action_ids.contains(&action),
                "unrelated card (no [Vemmon] in name or text) must never be a legal pick"
            );
        }
    }
}

/// Event-log / cost-firing check: the reveal step itself has no side-effect
/// cost to assert (no security/hand trash as a COST), but we assert the
/// remainder correctly lands in the trash zone (not deck bottom/top) —
/// distinguishing BT21-087 from sibling reveal cards that bottom the rest.
#[test]
fn bt21_087_on_play_remainder_lands_in_trash_not_deck() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_exact_vemmon("VEMMON-EXACT"))
        .add_card(make_unrelated_digimon("FILLER1"))
        .add_card(make_unrelated_digimon("FILLER2"))
        .deck(0, &["VEMMON-EXACT", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Zenith");
    pick_revealed_by_id(&mut runner, "VEMMON-EXACT", "pick exact Vemmon");
    runner.execute_branch(1).expect("Add-to-hand branch"); // keep Vemmon out of battle area
    runner.auto_resolve().expect("resolve trash-rest tail");

    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"FILLER1".to_string()) && trash_ids.contains(&"FILLER2".to_string()),
        "unpicked reveal cards must land in trash (RemainingCardsPlace.Trash); trash={:?}",
        trash_ids
    );
    assert!(
        !trash_ids.contains(&"VEMMON-EXACT".to_string()),
        "the picked (and hand-added) Vemmon must NOT be trashed; trash={:?}",
        trash_ids
    );
    // Deck must be fully drained by the reveal (3 cards revealed, 3-card deck).
    assert_eq!(
        runner.game.players[0].deck.len(),
        0,
        "no revealed cards return to the deck (Trash, not DeckBottom/Top)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3: [Security] Play this card without paying the cost
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_087_security_effect_plays_zenith_onto_field_after_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML loads")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .deck(0, &["ATTACKER"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    // Put Zenith at the top of P1's security stack.
    {
        let zenith_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == CARD_ID)
            .expect("BT21-087 in card_data");
        let next = runner.game.next_card_index();
        let zenith_src = CardSource::new(zenith_idx, 1, next);
        runner.game.players[1].security.push(zenith_src);
    }

    assert_eq!(runner.security_count(1), 1, "pre: P1 has exactly 1 security (Zenith)");

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "the security card (Zenith) must be removed by the attack"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "Zenith must be played onto P1's battle area by its [Security] effect \
         (played for free — no memory payment required)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT: none of Zenith's clauses print [Once Per Turn]
// ═══════════════════════════════════════════════════════════════════════════════

/// Faithfulness: printed text carries NO `[Once Per Turn]` tag on any
/// clause. Structural cross-check (integration coverage of "no OPT lockout"
/// is implicit: On Play only fires once per play anyway; Start of Your Turn
/// fires once per turn boundary by definition).
#[test]
fn bt21_087_no_clause_is_marked_once_per_turn() {
    let card = dsl_card_data::compiled(CARD_ID);
    for clause in &card.effects {
        if let CompiledClause::Triggered(t) = clause {
            assert!(
                !t.once_per_turn,
                "BT21-087 prints no [Once Per Turn] tag on any clause; \
                 got once_per_turn=true on a clause with when={:?}",
                t.when
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — No extra effects beyond the 3 printed clauses
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_087_no_extra_effects_beyond_three_clauses() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(
        card.effects.len(),
        3,
        "BT21-087 prints exactly 3 effect clauses; got {} clauses: {:?}",
        card.effects.len(),
        card.effects
            .iter()
            .map(|c| match c {
                CompiledClause::Triggered(t) => format!("Triggered({:?})", t.when),
                CompiledClause::Declarative(_) => "Declarative".to_string(),
            })
            .collect::<Vec<_>>()
    );
}

/// Structural cross-check: the On Play clause's process must contain a
/// `SelectRevealBuckets` step with exactly ONE bucket (the unified pool),
/// NOT two — pinning the single-bucket architectural decision documented in
/// the header (vs. the two-bucket `no_duplicate_cards` idiom used elsewhere).
#[test]
fn bt21_087_on_play_uses_single_reveal_bucket_not_two() {
    let card = dsl_card_data::compiled(CARD_ID);
    let on_play = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnPlay] => Some(t),
            _ => None,
        })
        .next()
        .expect("on_play clause present");

    let bucket_step = on_play
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::SelectRevealBuckets { buckets, .. } => Some(buckets),
            _ => None,
        })
        .expect("on_play process must contain a SelectRevealBuckets step");

    assert_eq!(
        bucket_step.len(),
        1,
        "BT21-087 must use ONE unified reveal bucket (single \
         SimplifiedSelectCardConditionClass in DCGO), not two filtered buckets — \
         the play-vs-add branch is decided AFTER the pick, not by which bucket \
         admitted it"
    );
}
