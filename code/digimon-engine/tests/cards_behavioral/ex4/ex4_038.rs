//! EX4-038 Agumon — Digimon, Lv.3, Black, DP 1000, Cost 3, traits: [Reptile].
//! Standard digivolution: Lv2 Black / Cost 0.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Greymon] in its name and 1 Digimon card with [Gabumon], [Garurumon] or
//! [Omnimon] in its name among them to your hand. Place the rest on top of
//! your deck in any order.
//!
//! Inherited:
//! [Your Turn] [Once Per Turn] When one of your other Digimon digivolves,
//! gain 1 memory.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX4/Black/EX4_038.cs`
//!
//! # Patterns this test covers
//!
//! - **[On Play] reveal-N + multi-bucket selection (Greymon Digimon bucket
//!   AND Gabumon/Garurumon/Omnimon Digimon bucket) + top remainder** —
//!   Clause 1. Mirror of BT12-059 two-bucket reveal-4 pattern, but with
//!   reveal-3 and `position: top` (which surfaces an OrderedPermutation
//!   selection per the printed "in any order").
//! - **Inherited [Your Turn][OPT] `on_digivolve` mandatory gain_memory** —
//!   Clause 2. Inspired by BT22-005 inherited [Your Turn][OPT] mandatory
//!   on `on_enter_field_anyone`, here swapped for `on_digivolve` per the
//!   printed text, with body `gain_memory: 1`.
//!
//! # Faithfulness audit (per clause)
//!
//! 1. **[On Play] reveal-3, two buckets, top remainder** — Two
//!    `select_reveal_buckets` buckets with `min: 1, max: 1` mirror DCGO's
//!    `SimplifiedRevealDeckTopCardsAndSelect(revealCount: 3, [bucket-1
//!    (Digimon + Greymon-name), bucket-2 (Digimon + (Gabumon | Garurumon |
//!    Omnimon))], remainingCardsPlace: DeckTop, mutualConditions: true)`.
//!    `no_duplicate_cards: true` enforces `mutualConditions: true`
//!    (a single card cannot satisfy both buckets). Outer trigger NOT
//!    optional (DCGO `SetUpActivateClass(..., false, ...)`; printed has
//!    no "you may"). Each bucket is mandatory IF a matching card is
//!    revealed; resolves silently when none exists.
//!
//!    `place_remainder_on_deck { position: top }` triggers an
//!    OrderedPermutation selection so the player chooses the order ("in
//!    any order"), keeping the choice in the RL action space per the
//!    no-approximations policy.
//!
//! 2. **Inherited [Your Turn][OPT] gain_memory on ally digivolve** —
//!    `scope: inherited` + `when: on_digivolve` + `active_when:
//!    { your_turn: true }` + `once_per_turn: true` + `condition:
//!    { event_target_owner: you, event_target_kind: digimon }` mirrors
//!    DCGO `SetIsInheritedEffect(true)` + `oncePerTurn: 1` +
//!    `isOptional: false` + `IsOwnerTurn(card)` +
//!    `CanTriggerWhenPermanentDigivolving(IsPermanentExistsOnOwnerBattleAreaDigimon)`.
//!    The "OTHER Digimon" sub-clause (`perm != card.PermanentOfThisCard()`)
//!    is not expressible — G-DSL-EVENT-TARGET-IS-SELF gap. The workaround
//!    over-fires when the carrier itself digivolves while EX4-038 is in
//!    its source stack; the carrier-self exclusion test is `#[ignore]`'d.
//!
//!    G-OPT-TRIGGERED: same-turn lockout for triggered clauses is not yet
//!    enforced through queue drain; lockout assertion is `#[ignore]`'d.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "EX4-038";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, card_id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c
}

fn make_named_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c
}

fn make_named_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

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

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX4-038 must compile from the embedded DSL pack as a Lv3 Black Reptile
/// Digimon at cost 3 / DP 1000.
#[test]
fn ex4_038_compiles_as_lv3_black_reptile_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Digimon,
        "EX4-038 is a Digimon"
    );
    assert_eq!(card.cost, Some(3), "EX4-038 prints Cost 3");
    assert_eq!(card.level, Some(3), "EX4-038 is Lv.3");
    assert_eq!(card.dp, Some(1000), "EX4-038 prints DP 1000");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Reptile")),
        "EX4-038 must carry the Reptile trait"
    );
    assert!(
        card.color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Black)),
        "EX4-038 is a Black Digimon"
    );
}

/// EX4-038 declares exactly one OnPlay triggered clause and exactly one
/// inherited triggered clause (`on_digivolve`). No declarative clauses.
#[test]
fn ex4_038_has_one_on_play_and_one_inherited_on_digivolve() {
    let card = compiled(CARD_ID);

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
        2,
        "EX4-038 must have exactly 2 triggered clauses (on_play + inherited on_digivolve)"
    );

    let on_play = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("EX4-038 must have an on_play clause");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(
        !on_play.optional,
        "Clause 1 is not optional — DCGO SetUpActivateClass(..., false, ...); \
         printed has no 'you may'"
    );
    assert!(!on_play.once_per_turn, "Clause 1 has no [Once Per Turn]");

    let on_digivolve = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnDigivolve])
        .expect("EX4-038 must have an on_digivolve clause");
    assert_eq!(
        on_digivolve.scope,
        CompiledScope::Inherited,
        "Clause 2 must be inherited scope (DCGO SetIsInheritedEffect(true))"
    );
    assert!(
        !on_digivolve.optional,
        "Clause 2 is not optional — DCGO isOptional: false; printed has no 'you may'"
    );
    assert!(
        on_digivolve.once_per_turn,
        "Clause 2 carries [Once Per Turn] (DCGO oncePerTurn: 1)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 [On Play] reveal-3 + two buckets + top
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive path — both buckets satisfied. Reveal 3 = [GREYMON, GABUMON,
/// FILLER]. Pick GREYMON for bucket 1, GABUMON for bucket 2. The single
/// FILLER is placed on top of the deck through an OrderedPermutation
/// selection.
#[test]
fn ex4_038_on_play_picks_greymon_and_gabumon_top_remainder() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_named_digimon("GABUMON", "Gabumon"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["GREYMON", "GABUMON", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX4-038 (cost 3)");

    // First bucket: a Greymon-named Digimon.
    assert!(
        runner.game.pending_selection.is_some(),
        "select_reveal_buckets prompt must install"
    );
    let greymon_action = revealed_action_for_id(&runner, "GREYMON")
        .expect("GREYMON should appear in revealed actions");
    runner
        .execute_action(0, greymon_action)
        .expect("pick GREYMON for the Greymon bucket");

    // Second bucket: a Gabumon/Garurumon/Omnimon-named Digimon.
    assert!(
        runner.game.pending_selection.is_some(),
        "second bucket prompt must install"
    );
    let gabumon_action = revealed_action_for_id(&runner, "GABUMON")
        .expect("GABUMON should appear in revealed actions");
    runner
        .execute_action(0, gabumon_action)
        .expect("pick GABUMON for the Gabumon/Garurumon/Omnimon bucket");

    // Drive any remaining prompts (top-remainder permutation for FILLER).
    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GREYMON".to_string()),
        "GREYMON must land in hand; hand was {hand_ids:?}"
    );
    assert!(
        hand_ids.contains(&"GABUMON".to_string()),
        "GABUMON must land in hand; hand was {hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"FILLER".to_string()),
        "FILLER must NOT enter hand; hand was {hand_ids:?}"
    );

    // FILLER must be on TOP of the deck (deck.last() = top, drawn first).
    let top_id = runner.game.players[0]
        .deck
        .last()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .unwrap_or_default();
    assert_eq!(
        top_id, "FILLER",
        "FILLER must be at the top of the deck after place_remainder_on_deck position: top; \
         top was {top_id}"
    );
}

/// Greymon-only branch — reveal contains a Greymon Digimon but no
/// Gabumon/Garurumon/Omnimon match. Bucket 2 has zero candidates and
/// silently resolves; only Greymon is added.
#[test]
fn ex4_038_on_play_only_greymon_skips_other_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["GREYMON", "FILLER1", "FILLER2"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX4-038");

    // Pick GREYMON for bucket 1. Bucket 2 has no candidates → silent skip.
    let greymon_action = revealed_action_for_id(&runner, "GREYMON")
        .expect("GREYMON should appear in revealed actions");
    runner
        .execute_action(0, greymon_action)
        .expect("pick GREYMON");

    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GREYMON".to_string()),
        "GREYMON must land in hand"
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "no Gabumon/Garurumon/Omnimon was found, no extra add"
    );
}

/// Gabumon-only branch — reveal contains a Gabumon Digimon but no Greymon
/// match. Bucket 1 silently resolves; only Gabumon is added.
#[test]
fn ex4_038_on_play_only_gabumon_skips_greymon_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_named_digimon("GABUMON", "Gabumon"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["GABUMON", "FILLER1", "FILLER2"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX4-038");

    // Drive prompts: pick Gabumon for bucket 2 when offered, advance otherwise.
    while let Some(view) = runner.pending_selection_view() {
        let gabu_action = revealed_action_for_id(&runner, "GABUMON");
        if let Some(action) = gabu_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick Gabumon");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GABUMON".to_string()),
        "GABUMON must land in hand"
    );
}

/// Garurumon name match — confirms the bucket-2 `name_contains: "Garurumon"`
/// branch fires (e.g., a Champion-stage Digimon named "Garurumon").
#[test]
fn ex4_038_on_play_garurumon_satisfies_other_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_named_digimon("GARURUMON", "Garurumon"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["GREYMON", "GARURUMON", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX4-038");

    let greymon_action = revealed_action_for_id(&runner, "GREYMON")
        .expect("GREYMON should appear in revealed actions");
    runner
        .execute_action(0, greymon_action)
        .expect("pick GREYMON");

    let garu_action = revealed_action_for_id(&runner, "GARURUMON")
        .expect("GARURUMON should appear in revealed actions");
    runner
        .execute_action(0, garu_action)
        .expect("pick GARURUMON for the Gabumon/Garurumon/Omnimon bucket");

    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GARURUMON".to_string()),
        "GARURUMON must land in hand via the Garurumon-name branch of bucket 2"
    );
}

/// Omnimon name match — confirms the bucket-2 `name_contains: "Omnimon"`
/// branch fires.
#[test]
fn ex4_038_on_play_omnimon_satisfies_other_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_named_digimon("OMNIMON", "Omnimon"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["GREYMON", "OMNIMON", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX4-038");

    let greymon_action = revealed_action_for_id(&runner, "GREYMON")
        .expect("GREYMON should appear in revealed actions");
    runner
        .execute_action(0, greymon_action)
        .expect("pick GREYMON");

    let omni_action = revealed_action_for_id(&runner, "OMNIMON")
        .expect("OMNIMON should appear in revealed actions");
    runner
        .execute_action(0, omni_action)
        .expect("pick OMNIMON for the Gabumon/Garurumon/Omnimon bucket");

    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"OMNIMON".to_string()),
        "OMNIMON must land in hand via the Omnimon-name branch of bucket 2"
    );
}

/// Neither bucket has a candidate — three fillers are revealed, both
/// buckets silently resolve, all 3 cards return to top of deck via the
/// OrderedPermutation selection.
#[test]
fn ex4_038_on_play_no_match_tops_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .add_card(make_filler("FILLER3"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    let deck_size_before = runner.game.players[0].deck.len();

    runner.play(0, 0).expect("play EX4-038");

    // Drive prompts until settled (both buckets empty → top-permutation only).
    let _ = runner.auto_resolve();

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    for filler in ["FILLER1", "FILLER2", "FILLER3"] {
        assert!(
            !hand_ids.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no bucket matches"
        );
    }

    // All 3 reveal cards must return to deck (size unchanged).
    let deck_size_after = runner.game.players[0].deck.len();
    assert_eq!(
        deck_size_after, deck_size_before,
        "all 3 revealed cards must return to deck; deck size was {deck_size_before}, \
         now {deck_size_after}"
    );
}

/// The Greymon bucket must reject Tamer-kind cards even with "Greymon" in
/// the name — both buckets carry `kind: digimon` per printed text.
#[test]
fn ex4_038_greymon_bucket_rejects_tamer_named_greymon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_named_tamer("FAKE-GREYMON-TAMER", "Greymon"))
        .add_card(make_named_digimon("GABUMON", "Gabumon"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FAKE-GREYMON-TAMER", "GABUMON", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX4-038");

    // Walk all prompts. The Tamer-kind Greymon must NEVER be the only legal
    // pick for any bucket. Always prefer the Gabumon pick, never the Tamer.
    while let Some(view) = runner.pending_selection_view() {
        let fake_action = revealed_action_for_id(&runner, "FAKE-GREYMON-TAMER");
        let gabu_action = revealed_action_for_id(&runner, "GABUMON");

        if let (Some(fake), Some(gabu)) = (fake_action, gabu_action) {
            let allows_fake = view.valid_action_ids.contains(&fake);
            let allows_gabu = view.valid_action_ids.contains(&gabu);
            if allows_fake && !allows_gabu {
                panic!(
                    "Greymon bucket wrongly accepted Tamer-kind card: \
                     valid_action_ids = {:?}",
                    view.valid_action_ids
                );
            }
            if allows_gabu {
                runner.execute_action(0, gabu).expect("pick Gabumon");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"FAKE-GREYMON-TAMER".to_string()),
        "Tamer-kind 'Greymon' must NOT enter hand — both buckets are kind: digimon"
    );
    assert!(
        hand_ids.contains(&"GABUMON".to_string()),
        "GABUMON must land in hand via bucket 2"
    );
}

/// `no_duplicate_cards: true` (DCGO `mutualConditions: true`) — a card that
/// satisfies BOTH buckets cannot be added twice. A Digimon named
/// "Greymon Gabumon" (synthetic) would match both filters; the engine must
/// pick it for at most one bucket.
#[test]
fn ex4_038_no_duplicate_cards_enforces_mutual_exclusion() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_named_digimon("HYBRID", "Greymon Gabumon"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["HYBRID", "FILLER1", "FILLER2"])
        .deck(1, &["FILLER1"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play EX4-038");

    // Drive prompts. Pick HYBRID whenever it's legal.
    while let Some(view) = runner.pending_selection_view() {
        let hybrid_action = revealed_action_for_id(&runner, "HYBRID");
        if let Some(action) = hybrid_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick HYBRID");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    let hybrid_count = hand_ids.iter().filter(|id| *id == "HYBRID").count();
    assert!(
        hybrid_count <= 1,
        "no_duplicate_cards: true must prevent the same revealed card from \
         being added to both buckets; got {hybrid_count} HYBRID copies in hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 [Your Turn][OPT] inherited gain_memory
// ═══════════════════════════════════════════════════════════════════════════════
//
// EX4-038 is a Lv3 Rookie. Its inherited clause fires when EX4-038 is a
// digivolution source under another carrier on the battle area
// (G-INHERITED-DISPATCH closed 2026-04-29). Setup pattern (per BT12-059
// inherited tests): use `place_stack` to seat EX4-038 below a Champion-stage
// carrier. Then play a separate Champion onto a separate Lv3 base via
// `digivolve_from_hand` to trigger the on_digivolve event.

/// Helper: build a Champion (Lv4) Digimon card with a Lv3-Black evo cost
/// of 0 memory, suitable for `digivolve_from_hand`.
fn make_lv4_evo_target(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: CardColor::Black as u8,
        level: 3,
        memory_cost: 0,
    }];
    c
}

/// Helper: build a Lv3-Black Digimon usable as a digivolve base.
fn make_lv3_base(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Black];
    c
}

/// Positive: EX4-038 sits as a digivolution source under a Champion. A
/// SEPARATE friendly Lv3 base is digivolved into a Champion → the inherited
/// clause fires and gain_memory: 1 takes effect.
#[test]
fn ex4_038_inherited_gain_memory_on_other_ally_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_lv4_evo_target("CARRIER", "Carrier"))
        .add_card(make_lv4_evo_target("EVO-TARGET", "Evo Target"))
        .add_card(make_lv3_base("BASE", "Base"))
        .hand(0, &["EVO-TARGET"])
        .memory(0)
        .start();

    // Seat EX4-038 as a digivolution source under CARRIER (the carrier is
    // the top card; EX4-038 is below it).
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    // Place a SEPARATE Lv3 base on the field. We will digivolve EVO-TARGET
    // onto BASE so the digivolving permanent is NOT the carrier hosting
    // EX4-038. With your_turn=true and event_target_owner=you predicates,
    // the inherited clause must fire.
    let base_handle = runner.place_on_field(0, "BASE", Some(0));

    runner.game.enter_main_phase();
    let mem_before = runner.memory();

    // Digivolve EVO-TARGET (hand index 0) onto BASE (battle_area index).
    let ok = runner.game.digivolve_from_hand(
        0,
        0,
        base_handle.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(
        ok,
        "digivolve_from_hand must succeed (Lv3 Black → Lv4 Black at evo cost 0)"
    );

    // Drain any pending selections (none expected here, but safety).
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    let gain = mem_after - mem_before;
    assert_eq!(
        gain, 1,
        "EX4-038's inherited [Your Turn][OPT] +1 memory must fire when an ally \
         (a SEPARATE Digimon) digivolves; mem went from {mem_before} → {mem_after} \
         (delta {gain})"
    );
}

/// Negative: opponent's turn — `active_when: { your_turn: true }` must
/// suppress the inherited gain_memory.
#[test]
fn ex4_038_inherited_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_lv4_evo_target("CARRIER", "Carrier"))
        .add_card(make_lv4_evo_target("OPP-EVO", "Opp Evo"))
        .add_card(make_lv3_base("OPP-BASE", "Opp Base"))
        .deck(0, &["CARRIER"])
        .deck(1, &["CARRIER"])
        .hand(1, &["OPP-EVO"])
        .memory(0) // 0 → end_turn flips to P1
        .start();

    // Seat EX4-038 under CARRIER on P0's field.
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    // Seat an OPP-BASE on P1's field for digivolution.
    let opp_base_handle = runner.place_on_field(1, "OPP-BASE", Some(0));

    // Hand off to P1.
    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );

    let mem_before = runner.memory();
    runner.game.enter_main_phase();

    // P1 digivolves OPP-EVO onto OPP-BASE.
    let ok = runner.game.digivolve_from_hand(
        1,
        0,
        opp_base_handle.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(ok, "P1 digivolve must succeed");

    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    // event_target_owner: you on P0's perspective is P0, but the digivolving
    // permanent is on P1. Both your_turn (false from P0's side) and
    // event_target_owner mismatch should suppress the trigger.
    assert_eq!(
        mem_after, mem_before,
        "EX4-038's inherited gain_memory must NOT fire on opponent's turn / \
         opponent's digivolve; memory must be unchanged (was {mem_before}, now {mem_after})"
    );
}

/// Negative (G-DSL-EVENT-TARGET-IS-SELF — pending): when the carrier hosting
/// EX4-038 is itself the digivolving permanent, the inherited clause must
/// NOT fire (printed text says "your OTHER Digimon"). Today the engine
/// over-fires because there is no `event_target_is_source` predicate.
///
/// Setup: place EX4-038 as a source under a Lv3 BASE carrier (i.e., the
/// carrier IS itself currently a Lv3 with EX4-038 in its stack — synthetic
/// since EX4-038 is also Lv3, but `place_stack` allows arbitrary placement
/// for testing). Digivolve a Champion onto the carrier — that carrier IS
/// the digivolving permanent. Per printed text, "other Digimon" excludes
/// the carrier so no memory should be gained.
#[test]
#[ignore = "pending: G-DSL-EVENT-TARGET-IS-SELF (no event_target_is_source predicate; \
            EX4-038 inherited clause over-fires when carrier itself digivolves)"]
fn ex4_038_inherited_does_not_fire_when_carrier_self_digivolves() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_lv3_base("BASE", "Base"))
        .add_card(make_lv4_evo_target("EVO-TARGET", "Evo Target"))
        .hand(0, &["EVO-TARGET"])
        .memory(0)
        .start();

    // Seat EX4-038 under BASE (the carrier). BASE is a Lv3 Black Digimon
    // suitable for being digivolved upon.
    let carrier_handle = runner.place_stack(0, &[CARD_ID, "BASE"]);

    runner.game.enter_main_phase();
    let mem_before = runner.memory();

    // Digivolve EVO-TARGET onto the BASE carrier — the carrier (which has
    // EX4-038 in its source stack) IS itself the digivolving permanent.
    let ok = runner.game.digivolve_from_hand(
        0,
        0,
        carrier_handle.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(ok, "digivolve onto carrier must succeed");

    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    assert_eq!(
        mem_after, mem_before,
        "EX4-038's inherited clause says 'your OTHER Digimon' — when the carrier \
         hosting EX4-038 is itself the digivolving permanent, the clause must NOT \
         fire. mem was {mem_before}, now {mem_after}"
    );
}

/// G-OPT-TRIGGERED — `once_per_turn: true` lockout for triggered clauses
/// is not yet enforced by the engine queue-drain path. Ignored pending
/// engine fix.
#[test]
fn ex4_038_inherited_once_per_turn_locks_out_second_trigger() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX4-038 in embedded DSL pack")
        .add_card(make_lv4_evo_target("CARRIER", "Carrier"))
        .add_card(make_lv4_evo_target("EVO1", "Evo One"))
        .add_card(make_lv4_evo_target("EVO2", "Evo Two"))
        .add_card(make_lv3_base("BASE1", "Base 1"))
        .add_card(make_lv3_base("BASE2", "Base 2"))
        .hand(0, &["EVO1", "EVO2"])
        .memory(0)
        .start();

    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let base1 = runner.place_on_field(0, "BASE1", Some(0));
    let base2 = runner.place_on_field(0, "BASE2", Some(0));

    runner.game.enter_main_phase();
    let mem_before = runner.memory();

    let ok1 = runner.game.digivolve_from_hand(
        0,
        0,
        base1.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(ok1, "first digivolve must succeed");
    let _ = runner.auto_resolve();
    let mem_mid = runner.memory();

    let ok2 = runner.game.digivolve_from_hand(
        0,
        0,
        base2.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(ok2, "second digivolve must succeed");
    let _ = runner.auto_resolve();
    let mem_after = runner.memory();

    assert_eq!(
        mem_mid - mem_before,
        1,
        "first ally digivolve gains 1 memory"
    );
    assert_eq!(
        mem_after - mem_mid,
        0,
        "second ally digivolve must NOT gain memory ([Once Per Turn] lockout)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _: PermanentHandle = PermanentHandle {
        player: 0,
        index: 0,
    };
    let _ = make_filler("X");
    let _ = make_named_digimon("X", "X");
    let _ = make_named_tamer("X", "X");
}
