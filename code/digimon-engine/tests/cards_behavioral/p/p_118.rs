//! P-118 Wormmon — Digimon, Lv.3, Green, DP 1000, Cost 3.
//! Traits: Larva. Attribute: Free. Evo costs: Lv.2 Green / cost 0.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 green or blue card
//! with 2 or more colors and 1 Tamer card with [Ken Ichijoji] in its name
//! among them to the hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//!   [End of Your Turn] This Digimon and any of your other Digimon may DNA
//!   digivolve into a Digimon card in the hand.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/P/Green/P_118.cs`
//!
//! Notable DCGO contracts:
//!   - [On Play] outer activate: `SetUpActivateClass(..., -1, false, ...)`
//!     — printed text has no "you may"; reveal-and-add is mandatory if the
//!     activate condition holds (deck non-empty).
//!   - Bucket 1: `CanSelectCardCondition`:
//!       `(CardColors.Count >= 2 && (Blue || Green))`
//!     DSL: `all_of: [self_color_count_gte: 2, any_of: [color_is: green, color_is: blue]]`
//!   - Bucket 2: `CanSelectCardCondition1`:
//!       `cardSource.IsTamer && cardSource.ContainsCardName("Ken Ichijoji")`
//!     DSL: `all_of: [kind: tamer, name_contains: "Ken Ichijoji"]`
//!   - `SimplifiedRevealDeckTopCardsAndSelect` with `RemainingCardsPlace.DeckBottom`
//!     and implicit `mutualConditions` — encoded as `select_reveal_buckets`
//!     with `no_duplicate_cards: true` plus trailing `place_remainder_on_deck { position: bottom }`.
//!   - Inherited [End of Your Turn] DNA digivolve: `SetUpActivateClass(..., -1, true, ...)`
//!     + `SetIsInheritedEffect(true)`. Encoded via `alt_path_registration`
//!     declarative (RESOLVED 2026-05-02 in qa/dsl-vocab-gaps.md, BT22-008/BT22-017 entry).
//!
//! # Sister card
//!
//! BT22-017 (Gabumon) — identical two-clause structure:
//! `[On Play] reveal-3 + 2-bucket` + `Inherited [EoT] DNA digivolve`.
//! Tests follow the BT22-017 test pattern (bt22_017.rs) with bucket-predicate
//! differences:
//!   - Bucket 1 is color+count (not effect_text_contains).
//!   - Bucket 2 is name_contains (not trait_has).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//!   - §5 Structural: standard digivolve alt-path; on-play triggered clause;
//!     inherited `alt_path_registration` for end-of-your-turn DNA digivolve.
//!   - §5 Behavioral integrated: positive (both buckets satisfied), partial
//!     (only one bucket), negative (neither bucket) + bottom-remainder.
//!   - §5 Faithfulness: outer trigger mandatory (no "you may"); bucket picks
//!     mandatory when candidate exists; no_duplicate_cards enforcement.
//!
//! # `mod.rs` policy
//! This file is registered in `tests/cards_behavioral/p/mod.rs` as `mod p_118;`.
//! No `cards.rs` / `trackers` mutation required — P-118 is a pure DSL-only card.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

const CARD_ID: &str = "P-118";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A two-color (green+blue) Lv.4 Digimon — satisfies bucket 1.
/// "green or blue card with 2 or more colors": has green AND blue (2 colors),
/// and contains both qualifying colors.
fn make_multicolor_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Green, CardColor::Blue];
    c
}

/// A single-color green Lv.4 Digimon — does NOT satisfy bucket 1 (only 1 color).
fn make_single_color_green_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Green];
    c
}

/// A Tamer named "Ken Ichijoji" — satisfies bucket 2.
fn make_ken_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Ken Ichijoji");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Green];
    c
}

/// A Tamer named "Tai Kamiya" — does NOT satisfy bucket 2 (wrong name).
fn make_other_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Tai Kamiya");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Red];
    c
}

/// A plain filler Digimon — satisfies neither bucket.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Filler");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c
}

/// Collect card_ids from a zone of CardSources.
fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

/// Resolve the reveal action ID for a card_id in the current reveal overlay.
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

/// Drive a reveal bucket pick by card_id; panics if not available.
fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: {id} not in reveal overlay"));
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: action {action} for {id} not legal in current bucket; \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect(label);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn p_118_yaml_parses_and_compiles() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/p/P-118.yaml"))
        .expect("P-118 YAML parses");
    let _compiled = compile(&spec).expect("P-118 YAML compiles");
}

#[test]
fn p_118_yaml_compiles_in_embedded_pack() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Wormmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("Larva")),
        "P-118 must carry the Larva trait; got traits={:?}",
        card.traits
    );
}

#[test]
fn p_118_has_standard_lv2_green_digivolve_alt_path_at_cost_zero() {
    let card = dsl_card_data::compiled(CARD_ID);
    let standard = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        standard.is_some(),
        "P-118 must declare its standard Lv.2 Green / cost 0 digivolve path"
    );
}

#[test]
fn p_118_has_on_play_triggered_clause_mandatory() {
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
        1,
        "exactly one triggered clause (OnPlay reveal)"
    );
    let clause = triggered[0];
    assert!(
        clause.when.contains(&CompiledTiming::OnPlay),
        "triggered clause must fire on OnPlay; got when={:?}",
        clause.when
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "On-play clause is face-up only"
    );
    assert!(
        !clause.optional,
        "Printed text has NO 'you may' — outer trigger is mandatory \
         (DCGO P_118.cs line 19 isOptional: false)"
    );
    assert!(!clause.once_per_turn, "On-play clause has no [Once Per Turn]");
}

#[test]
fn p_118_has_inherited_dna_digivolve_alt_path_registration() {
    // RESOLVED 2026-05-02 (qa/dsl-vocab-gaps.md BT22-008/BT22-017 entry):
    // inherited end-of-your-turn DNA digivolve is authored as an
    // `alt_path_registration` declarative clause with
    // `kind: dna_digivolve, scope: inherited, trigger: end_of_your_turn`.
    let card = dsl_card_data::compiled(CARD_ID);
    let registration = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration {
            scope,
            trigger,
            registers,
            ..
        }) if *scope == CompiledScope::Inherited
            && *trigger == CompiledTiming::EndOfYourTurn
            && registers.kind == CompiledAltPathKind::DnaDigivolve =>
        {
            Some(())
        }
        _ => None,
    });
    assert!(
        registration.is_some(),
        "P-118 must register an inherited end-of-your-turn DNA digivolve alt-path \
         (mirrors BT22-008 / BT22-017)"
    );
}

#[test]
fn p_118_clause_count_matches_card_text() {
    // Card prints exactly two effect clauses:
    //   (1) the [On Play] reveal-3 + 2-bucket triggered clause
    //   (2) the inherited [End of Your Turn] DNA digivolve alt_path_registration
    let card = dsl_card_data::compiled(CARD_ID);
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
    assert_eq!(triggered, 1, "Exactly one triggered clause expected");
    assert_eq!(
        alt_path_regs, 1,
        "Exactly one alt_path_registration clause expected"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Faithfulness gate: trigger is mandatory
// ═══════════════════════════════════════════════════════════════════════════════

/// Printed text reads "Reveal the top 3 cards... Add 1... and 1..." — no "you may"
/// at the trigger level. DCGO P_118.cs line 19 confirms `isOptional: false`.
#[test]
fn p_118_on_play_trigger_is_mandatory_no_outer_optional() {
    let card = dsl_card_data::compiled(CARD_ID);
    let triggered = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("OnPlay triggered clause present");
    assert!(
        !triggered.optional,
        "Printed text has no 'you may' — outer trigger MUST be mandatory \
         (DCGO P_118.cs line 19 isOptional: false)."
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral integrated tests
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive path — both buckets satisfied.
/// Reveal 3 = [MULTICOLOR, KEN, FILLER].
/// Pick MULTICOLOR for bucket 1 and KEN for bucket 2. FILLER goes to bottom.
#[test]
fn p_118_on_play_both_buckets_satisfied_both_added_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(make_multicolor_digimon("MULTICOLOR"))
        .add_card(make_ken_tamer("KEN"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["MULTICOLOR", "KEN", "FILLER"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");

    // Bucket 1: green or blue card with 2+ colors.
    pick_revealed_by_id(&mut runner, "MULTICOLOR", "pick multicolor card (bucket 1)");

    // Bucket 2: Tamer with [Ken Ichijoji] in name.
    pick_revealed_by_id(&mut runner, "KEN", "pick Ken Ichijoji Tamer (bucket 2)");

    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"MULTICOLOR".to_string()),
        "multicolor card must land in hand; hand={:?}",
        hand_ids
    );
    assert!(
        hand_ids.contains(&"KEN".to_string()),
        "Ken Ichijoji Tamer must land in hand; hand={:?}",
        hand_ids
    );

    // FILLER must end up at the bottom of the deck.
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let last = deck_ids.last().cloned().unwrap_or_default();
    assert_eq!(
        last, "FILLER",
        "unchosen reveal card returns to deck bottom; deck={:?}",
        deck_ids
    );
}

/// Partial path — only bucket 1 (multicolor) satisfied; no Ken Ichijoji Tamer in reveal.
/// Bucket 2 has zero candidates and is skipped by the engine.
#[test]
fn p_118_on_play_only_multicolor_found_skips_ken_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(make_multicolor_digimon("MULTICOLOR"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["MULTICOLOR", "FILLER1", "FILLER2"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");
    pick_revealed_by_id(&mut runner, "MULTICOLOR", "pick multicolor (bucket 1)");
    runner
        .auto_resolve()
        .expect("resolve through empty Ken bucket");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"MULTICOLOR".to_string()),
        "multicolor card must land in hand; hand={:?}",
        hand_ids
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "no Ken Ichijoji candidate → no extra add from bucket 2; hand={:?}",
        hand_ids
    );
}

/// Partial path — only bucket 2 (Ken Ichijoji Tamer) satisfied; no multicolor in reveal.
/// Bucket 1 has zero candidates and is skipped.
#[test]
fn p_118_on_play_only_ken_tamer_found_skips_multicolor_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(make_ken_tamer("KEN"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["KEN", "FILLER1", "FILLER2"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");

    // Walk prompts; pick KEN when offered.
    while let Some(view) = runner.pending_selection_view() {
        if let Some(action) = revealed_action_for_id(&runner, "KEN") {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick Ken");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance prompt");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"KEN".to_string()),
        "Ken Ichijoji Tamer must land in hand; hand={:?}",
        hand_ids
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "filler cards must not enter hand; hand={:?}",
        hand_ids
    );
}

/// Negative path — neither bucket has a candidate in the reveal.
/// Reveal 3 fillers; both buckets skip; all 3 cards go to deck bottom.
#[test]
fn p_118_on_play_neither_bucket_found_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .add_card(make_filler("FILLER3"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    for filler in ["FILLER1", "FILLER2", "FILLER3"] {
        assert!(
            !hand_ids.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no bucket matches; hand={:?}",
            hand_ids
        );
    }
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(deck_ids.len(), 3, "all 3 reveal cards back in deck");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Bucket-1 predicate faithfulness: 2+ colors AND (green OR blue)
// ═══════════════════════════════════════════════════════════════════════════════

/// A single-color green Digimon should NOT satisfy bucket 1 — only 1 color
/// (printed text requires "2 or more colors").
#[test]
fn p_118_bucket1_rejects_single_color_green_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(make_single_color_green_digimon("SINGLE_GREEN"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["SINGLE_GREEN", "FILLER1", "FILLER2"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");

    // Resolve through. The behavioral assertion is hand contents: a single-color
    // green Digimon must NOT end up in hand via bucket 1, because the printed
    // text requires "2 or more colors". The reveal action mask may still surface
    // every revealed card as a clickable slot (the engine treats an empty
    // bucket as auto-skip on commit rather than masking the actions out at
    // selection time); behavioral truth is what's added to hand.
    runner.auto_resolve().expect("resolve empty bucket 1");
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"SINGLE_GREEN".to_string()),
        "single-color green card must NOT enter hand via bucket 1; hand={:?}",
        hand_ids
    );
}

/// A non-Ken Tamer should NOT satisfy bucket 2.
#[test]
fn p_118_bucket2_rejects_tamer_without_ken_ichijoji_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(make_other_tamer("TAI"))
        .add_card(make_filler("FILLER1"))
        .add_card(make_filler("FILLER2"))
        .deck(0, &["TAI", "FILLER1", "FILLER2"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");
    runner.auto_resolve().expect("resolve through empty buckets");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"TAI".to_string()),
        "non-Ken Tamer must NOT enter hand via bucket 2; hand={:?}",
        hand_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — no_duplicate_cards: one card cannot satisfy both buckets
// ═══════════════════════════════════════════════════════════════════════════════

/// A Ken Ichijoji Tamer who also has 2+ colors (green+blue) could nominally
/// satisfy both buckets. With `no_duplicate_cards: true`, once consumed by
/// bucket 1 it must NOT appear in bucket 2.
#[test]
fn p_118_no_duplicate_cards_prevents_double_consumption() {
    // Make a dual-eligible card: Ken Ichijoji Tamer with 2 colors.
    let mut dual = make_ken_tamer("DUAL_KEN");
    dual.colors = vec![CardColor::Green, CardColor::Blue]; // 2 colors → satisfies bucket 1
    // card_kind is Tamer → satisfies bucket 2's `kind: tamer` + name_contains check

    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(dual)
        // A second Ken Ichijoji Tamer (also 1 color, so only bucket-2 eligible)
        .add_card(make_ken_tamer("KEN2"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["DUAL_KEN", "KEN2", "FILLER"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");

    // Pick the dual-eligible for bucket 1.
    pick_revealed_by_id(
        &mut runner,
        "DUAL_KEN",
        "pick dual-eligible for multicolor bucket 1",
    );

    // Bucket 2 must now NOT offer DUAL_KEN again.
    let view = runner
        .pending_selection_view()
        .expect("bucket 2 should be active");
    let dual_action = revealed_action_for_id(&runner, "DUAL_KEN");
    if let Some(dual) = dual_action {
        assert!(
            !view.valid_action_ids.contains(&dual),
            "no_duplicate_cards: DUAL_KEN must NOT be re-selectable after \
             being consumed by bucket 1; valid_action_ids={:?}",
            view.valid_action_ids
        );
    }
    // KEN2 must still be available for bucket 2.
    let ken2_action = revealed_action_for_id(&runner, "KEN2");
    if let Some(ken2) = ken2_action {
        assert!(
            view.valid_action_ids.contains(&ken2),
            "KEN2 must remain selectable for bucket 2; \
             valid_action_ids={:?}",
            view.valid_action_ids
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Bucket picks are mandatory when a candidate exists
// ═══════════════════════════════════════════════════════════════════════════════

/// Once a bucket has a legal target, PASS must NOT be legal on that bucket.
/// Printed text: "Add 1 green or blue card..." (mandatory when a candidate exists).
#[test]
fn p_118_bucket_pick_is_mandatory_when_candidate_exists() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-118")
        .expect("P-118 YAML loads")
        .add_card(make_multicolor_digimon("MULTICOLOR"))
        .add_card(make_ken_tamer("KEN"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["MULTICOLOR", "KEN", "FILLER"])
        .hand(0, &["P-118"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon P-118");

    let view = runner
        .pending_selection_view()
        .expect("bucket 1 prompt installed");
    assert!(
        !runner.pending_is_optional(),
        "bucket 1 must be mandatory when a candidate exists \
         (printed 'Add 1 card', no 'may'); pending_is_optional={}",
        runner.pending_is_optional()
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must NOT be legal on bucket 1 when a candidate exists; \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
}
