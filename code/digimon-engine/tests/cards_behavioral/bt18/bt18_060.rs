//! BT18-060 Vemmon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: Unknown/LIBERATOR. Form: Rookie. Attribute: Unknown.
//! Evo costs: Lv.2 Black / cost 0.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT18-060.md, verbatim)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Among them, add 1 card
//! with [Vemmon] in its text to the hand and place 1 [Vemmon] as any of
//! your Digimon's bottom digivolution card. Return the rest to the bottom
//! of the deck.
//!
//! Inherited Effect:
//! [Your Turn] [Once Per Turn] When this Digimon would digivolve into a
//! Digimon card with [Vemmon] in its text, reduce the digivolution cost
//! by 1.
//! ```
//!
//! Official Q&A: "Yes, you must add as many cards to your hand and place
//! as bottom digivolution cards as possible."
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT18/Black/BT18_060.cs`
//!
//! Notable DCGO contracts:
//!   - [On Play] outer activate: `SetUpActivateClass(..., -1, false, ...)`
//!     — mandatory, no "you may".
//!   - Bucket 1 (AddHand, maxCount 1): `cardSource.HasText("Vemmon")` —
//!     substring scan of printed text → `effect_text_contains: "Vemmon"`.
//!   - Bucket 2 (Custom, maxCount 1): `cardSource.CardNames.Contains
//!     ("Vemmon")` — EXACT name match → `name_is: Vemmon`. On pick, a
//!     SECOND selection (`canNoSelect: false` → mandatory) over
//!     `IsPermanentExistsOnOwnerBattleAreaDigimon` (ANY own Digimon, no
//!     self-exclusion) places the card via `AddDigivolutionCardsBottom`.
//!   - `SimplifiedRevealDeckTopCardsAndSelect` removes each bucket's picks
//!     from the shared pool before the next bucket evaluates candidates —
//!     cross-bucket exclusion → `no_duplicate_cards: true`.
//!   - `RemainingCardsPlace.DeckBottom` → `place_remainder_on_deck { position:
//!     bottom }`.
//!   - Inherited cost reducer: TWO DCGO constructs — `activateClass2`
//!     (`SetIsInheritedEffect(true)`, `SetIsBackgroundProcess(true)`,
//!     `MaxCountPerTurn 1`, holds the OPT counter) + `changeCostClass`
//!     (`EffectTiming.None`, `SetIsInheritedEffect(true)`,
//!     `CardSourceCondition => cardSource.HasText("Vemmon")`,
//!     `PermanentCondition => targetPermanent == card.PermanentOfThisCard()`).
//!     `card.PermanentOfThisCard()` does not distinguish top-card vs
//!     buried-source position — the reducer applies in BOTH cases. Modelled
//!     as two `kind: cost_reduction` clauses (`scope: face_up` for the
//!     top-card case, `scope: inherited` for the buried-source case) sharing
//!     an identical `active_when`/`amount`/`once_per_turn` body, since the
//!     DSL's `CostReduction` lowering treats `scope` as an exact
//!     top-vs-buried gate (no `Both` expansion the way `FloodGate` has).
//!
//! # Sister card
//! BT22-017 Gabumon (reveal-3 + two-bucket On Play with
//! `no_duplicate_cards`); BT21-055 Sunarizamon / ST23-02 Liollmon / P-117
//! Veemon (self `source_is_cost_target_permanent` digivolve-cost reducers —
//! but all THREE are printed under the card's main effect text, not a
//! separate "Inherited Effect" section, so unlike BT18-060 they are
//! face-up-only and never need the buried-source clause).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5 / §14.6)
//!   - A1/A2 searching rookie / two-pass reveal (reveal-3, bucketed add +
//!     place-as-bottom-source, remainder to deck bottom).
//!   - §5 Structural: standard digivolve alt-path; on-play triggered clause;
//!     two CostReduction declaratives (face_up + inherited).
//!   - §5 Condition gating: empty deck reveal truncation, zero bucket-1
//!     candidates, zero bucket-2 candidates, both buckets satisfied.
//!   - §14.6 RevealBucket flow: bucket_index advances 0 → 1 → terminal.
//!   - §5 Faithfulness gate: outer trigger MANDATORY (no "you may"); both
//!     buckets mandatory-when-candidate (official Q&A: "must add ... as
//!     many ... as possible").
//!   - Cost-reduction: positive/negative on text-match, [Your Turn] gate,
//!     [Once Per Turn] lockout, top-card AND buried-source cases.
//!
//! # `cards.rs` / `mod.rs` policy
//! BT18-060 is a pure DSL-only card; no hand-written `CardEffect`. This
//! test file must be registered in `tests/cards_behavioral/bt18/mod.rs`
//! (orchestrator-owned; not touched here per the batch-implement skill's
//! deliverable scope).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

const CARD_ID: &str = "BT18-060";

// ─── Card-data factories ─────────────────────────────────────────────────────

/// A Digimon fixture whose printed text mentions "Vemmon" but whose NAME is
/// something else — the bucket-1 ("[Vemmon] in its text") candidate.
fn make_vemmon_text_card(id: &str) -> CardData {
    let mut card = make_test_card(id, "Some Other Digimon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.colors = vec![CardColor::Black];
    card.effect_text = "[On Play] Add 1 card with [Vemmon] in its text from your trash to your hand.".to_string();
    card
}

/// A Digimon fixture literally named "Vemmon" — the bucket-2 (exact name)
/// candidate, eligible to be placed as a bottom digivolution card.
fn make_named_vemmon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Vemmon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.colors = vec![CardColor::Black];
    card
}

/// A Lv.4 black Digimon with "Vemmon" in its printed text and a base evo
/// cost of 1 from Lv.3 black — used for the inherited cost-reduction tests.
fn make_lv4_black_vemmon_text(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Black];
    c.effect_text = "[When Digivolving] [Vemmon] gains <Security Attack +1> for the turn.".to_string();
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 5, // Black = 5
        memory_cost: 1,
    }];
    c
}

/// A Lv.4 black Digimon with NO "Vemmon" text — the negative-case target.
fn make_lv4_black_no_vemmon_text(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 5,
        memory_cost: 1,
    }];
    c
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
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

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: revealed card {id} not present"));
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
fn bt18_060_yaml_parses_and_compiles() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/bt18/BT18-060.yaml"))
        .expect("BT18-060 YAML parses");
    let _compiled = compile(&spec).expect("BT18-060 YAML compiles");
}

#[test]
fn bt18_060_yaml_compiles_in_embedded_pack() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Vemmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("LIBERATOR")),
        "BT18-060 must carry the LIBERATOR trait; got traits={:?}",
        card.traits
    );
}

#[test]
fn bt18_060_has_standard_lv2_black_digivolve_alt_path_at_cost_zero() {
    let card = dsl_card_data::compiled(CARD_ID);
    let standard = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        standard.is_some(),
        "BT18-060 must declare its standard Lv.2 Black / cost 0 digivolve path"
    );
}

#[test]
fn bt18_060_has_on_play_triggered_clause_mandatory() {
    let card = dsl_card_data::compiled(CARD_ID);
    let triggered: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "exactly one OnPlay triggered clause (reveal-3 + two-bucket)"
    );
    let clause = triggered[0];
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "On-play clause is face-up only"
    );
    assert!(
        !clause.optional,
        "Printed text has NO 'you may' — outer trigger is mandatory \
         (DCGO isOptional: false)"
    );
    assert!(
        !clause.once_per_turn,
        "On-play clause has no [Once Per Turn]"
    );
}

#[test]
fn bt18_060_has_two_cost_reduction_clauses_face_up_and_inherited() {
    let card = dsl_card_data::compiled(CARD_ID);
    let cost_reductions: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope,
                once_per_turn,
                amount,
                ..
            }) => Some((*scope, *once_per_turn, *amount)),
            _ => None,
        })
        .collect();
    assert_eq!(
        cost_reductions.len(),
        2,
        "BT18-060 must declare exactly 2 CostReduction clauses (face_up top-card \
         case + inherited buried-source case); got {cost_reductions:?}"
    );
    assert!(
        cost_reductions
            .iter()
            .any(|(scope, _, _)| *scope == CompiledScope::FaceUp),
        "one CostReduction clause must be scope: face_up; got {cost_reductions:?}"
    );
    assert!(
        cost_reductions
            .iter()
            .any(|(scope, _, _)| *scope == CompiledScope::Inherited),
        "one CostReduction clause must be scope: inherited; got {cost_reductions:?}"
    );
    for (scope, once_per_turn, amount) in &cost_reductions {
        assert!(
            once_per_turn,
            "[Once Per Turn] must be set on the {scope:?} CostReduction clause"
        );
        assert_eq!(
            *amount,
            Some(1),
            "cost reduction amount must be 1 on the {scope:?} clause"
        );
    }
}

#[test]
fn bt18_060_clause_count_matches_card_text() {
    let card = dsl_card_data::compiled(CARD_ID);
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    let cost_reductions = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    assert_eq!(triggered, 1, "exactly one triggered clause ([On Play])");
    assert_eq!(
        cost_reductions, 2,
        "exactly two CostReduction declaratives (face_up + inherited)"
    );
    assert_eq!(
        card.effects.len(),
        3,
        "BT18-060 prints exactly 3 effect clauses; got {}",
        card.effects.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — On-play behavioral: reveal 3, two buckets, bottom remainder
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive path — both buckets satisfied. Reveal 3 = [TEXT-CARD, NAMED-VEMMON,
/// FILLER]. Pick TEXT-CARD for bucket 1 (hand), NAMED-VEMMON for bucket 2
/// (place as bottom digivolution card of the just-played BT18-060 itself,
/// since it is the only own Digimon on the field).
#[test]
fn bt18_060_on_play_adds_text_card_to_hand_and_places_named_vemmon_as_bottom_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(make_vemmon_text_card("TEXT-CARD"))
        .add_card(make_named_vemmon("NAMED-VEMMON"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["TEXT-CARD", "NAMED-VEMMON", "FILLER"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    let vemmon = runner.play(0, 0).expect("play Vemmon BT18-060");

    let sources_before = runner.game.players[0].battle_area[vemmon].card_sources.len();

    // Bucket 1: [Vemmon] in effect text.
    pick_revealed_by_id(&mut runner, "TEXT-CARD", "pick Vemmon-text card");

    // Bucket 2: exact name "Vemmon".
    pick_revealed_by_id(&mut runner, "NAMED-VEMMON", "pick named Vemmon");

    // Target selection: only own Digimon is the just-played Vemmon itself.
    let view = runner
        .pending_selection_view()
        .expect("target selection installed for bottom-source placement");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose target Digimon");

    runner.auto_resolve().expect("bottom remainder resolves");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"TEXT-CARD".to_string()),
        "Vemmon-text card must land in hand; hand={:?}",
        hand_ids
    );
    assert!(
        !hand_ids.contains(&"NAMED-VEMMON".to_string()),
        "named Vemmon must NOT land in hand (it was placed as a bottom source); hand={:?}",
        hand_ids
    );

    let sources_after = runner.game.players[0].battle_area[vemmon].card_sources.len();
    assert_eq!(
        sources_after,
        sources_before + 1,
        "named Vemmon must be added as a bottom digivolution source"
    );
    assert_eq!(
        runner.game.players[0].battle_area[vemmon].card_sources[0].card_id(&runner.game.card_data),
        "NAMED-VEMMON",
        "named Vemmon must land at index 0 (the BOTTOM of the digivolution stack)"
    );

    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let last = deck_ids.last().cloned().unwrap_or_default();
    assert_eq!(
        last, "FILLER",
        "unchosen reveal card returns to deck bottom; deck end was {:?}",
        deck_ids
    );
}

/// Bucket-1 only — reveal contains a Vemmon-text card but NO exact-name
/// Vemmon. Bucket 2 has zero candidates and is skipped (auto-finalizes
/// empty); bucket 1 fires.
#[test]
fn bt18_060_on_play_only_text_card_found_skips_name_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(make_vemmon_text_card("TEXT-CARD"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["TEXT-CARD", "FILLER1", "FILLER2"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Vemmon BT18-060");
    pick_revealed_by_id(&mut runner, "TEXT-CARD", "pick Vemmon-text card");
    runner
        .auto_resolve()
        .expect("resolve through empty name bucket");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"TEXT-CARD".to_string()),
        "Vemmon-text card must be added to hand; hand={:?}",
        hand_ids
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "no name-match candidate → no extra add; hand={:?}",
        hand_ids
    );
}

/// Bucket-2 only — reveal contains an exact-name Vemmon but NO Vemmon-text
/// card. Bucket 1 has zero candidates and is skipped.
#[test]
fn bt18_060_on_play_only_named_vemmon_found_skips_text_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(make_named_vemmon("NAMED-VEMMON"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["NAMED-VEMMON", "FILLER1", "FILLER2"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    let vemmon = runner.play(0, 0).expect("play Vemmon BT18-060");
    let sources_before = runner.game.players[0].battle_area[vemmon].card_sources.len();

    // Walk prompts: pick NAMED-VEMMON when offered as a reveal pick,
    // otherwise advance with the first legal action.
    loop {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        if let Some(action) = revealed_action_for_id(&runner, "NAMED-VEMMON") {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick named Vemmon");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let sources_after = runner.game.players[0].battle_area[vemmon].card_sources.len();
    assert_eq!(
        sources_after,
        sources_before + 1,
        "named Vemmon must be placed as a bottom digivolution source"
    );
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "no text-match candidate → no hand add; hand={:?}",
        hand_ids
    );
}

/// Neither bucket has a candidate — reveal 3 fillers, both buckets skip,
/// all 3 cards hit deck bottom.
#[test]
fn bt18_060_on_play_neither_bucket_found_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    let vemmon = runner.play(0, 0).expect("play Vemmon BT18-060");
    let sources_before = runner.game.players[0].battle_area[vemmon].card_sources.len();

    runner
        .auto_resolve()
        .expect("resolve through both empty buckets");

    let sources_after = runner.game.players[0].battle_area[vemmon].card_sources.len();
    assert_eq!(
        sources_after, sources_before,
        "no source placement when neither bucket has a candidate"
    );

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

/// Empty-deck reveal — deck has fewer than 3 cards. The engine truncates the
/// reveal to whatever is available; the effect resolves gracefully with no
/// panic and no phantom picks.
#[test]
fn bt18_060_on_play_reveal_truncates_when_deck_has_fewer_than_three_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(make_vemmon_text_card("TEXT-CARD"))
        .deck(0, &["TEXT-CARD"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Vemmon BT18-060");
    pick_revealed_by_id(&mut runner, "TEXT-CARD", "pick Vemmon-text card");
    runner
        .auto_resolve()
        .expect("resolve through short reveal without panic");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"TEXT-CARD".to_string()),
        "Vemmon-text card must still be added to hand from a truncated reveal; hand={:?}",
        hand_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — `no_duplicate_cards` cross-bucket exclusion
// ═══════════════════════════════════════════════════════════════════════════════

/// A single card can satisfy BOTH bucket predicates simultaneously: a card
/// literally named "Vemmon" whose own effect text also happens to mention
/// "Vemmon". With `no_duplicate_cards: true`, a card consumed by bucket 1
/// must NOT remain available to bucket 2.
#[test]
fn bt18_060_no_duplicate_cards_prevents_double_consumption() {
    let mut dual = make_named_vemmon("DUAL");
    dual.effect_text = "[On Play] Add 1 [Vemmon] from your trash to hand.".to_string();

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(dual)
        .add_card(make_named_vemmon("NAME-ONLY"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["DUAL", "NAME-ONLY", "FILLER"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Vemmon BT18-060");

    // Pick the dual-eligible card for bucket 1 (text bucket).
    pick_revealed_by_id(&mut runner, "DUAL", "pick dual for text bucket");

    // Bucket 2 must now offer ONLY NAME-ONLY — not the already-consumed DUAL.
    let view = runner
        .pending_selection_view()
        .expect("name bucket should be active");
    let dual_action = revealed_action_for_id(&runner, "DUAL");
    let name_only_action = revealed_action_for_id(&runner, "NAME-ONLY");
    if let Some(dual_action) = dual_action {
        assert!(
            !view.valid_action_ids.contains(&dual_action),
            "no_duplicate_cards: DUAL must NOT be re-selectable after being \
             consumed by bucket 1; valid_action_ids={:?}",
            view.valid_action_ids
        );
    }
    if let Some(name_only_action) = name_only_action {
        assert!(
            view.valid_action_ids.contains(&name_only_action),
            "NAME-ONLY must remain selectable for bucket 2; valid_action_ids={:?}",
            view.valid_action_ids
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Faithfulness: bucket 2 is EXACT name match, not text substring
// ═══════════════════════════════════════════════════════════════════════════════

/// A card whose printed text mentions "Vemmon" but whose NAME is NOT
/// "Vemmon" must be a legal bucket-1 target but NOT a legal bucket-2 target.
#[test]
fn bt18_060_bucket2_requires_exact_name_not_text_substring() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(make_vemmon_text_card("TEXT-CARD"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["TEXT-CARD", "FILLER1", "FILLER2"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    let vemmon = runner.play(0, 0).expect("play Vemmon BT18-060");
    let sources_before = runner.game.players[0].battle_area[vemmon].card_sources.len();

    // Bucket 1 (text bucket) must offer TEXT-CARD.
    let view = runner
        .pending_selection_view()
        .expect("bucket 1 selection installed");
    let text_action =
        revealed_action_for_id(&runner, "TEXT-CARD").expect("TEXT-CARD is in the reveal overlay");
    assert!(
        view.valid_action_ids.contains(&text_action),
        "bucket 1 must admit TEXT-CARD (effect_text contains 'Vemmon'); \
         valid_action_ids={:?}",
        view.valid_action_ids
    );

    // Decline bucket 1 is impossible (mandatory when a candidate exists) —
    // pick TEXT-CARD, then bucket 2 must have zero candidates (auto-skip,
    // no prompt naming TEXT-CARD as a placement target, and no target-Digimon
    // prompt installs at all since `per_selected` over an empty `vemmon_pick`
    // runs zero iterations).
    pick_revealed_by_id(&mut runner, "TEXT-CARD", "pick text card for bucket 1");
    if let Some(view2) = runner.pending_selection_view() {
        let text_action_2 = revealed_action_for_id(&runner, "TEXT-CARD");
        if let Some(a) = text_action_2 {
            assert!(
                !view2.valid_action_ids.contains(&a),
                "TEXT-CARD must not be offered again after being consumed by bucket 1"
            );
        }
    }
    runner
        .auto_resolve()
        .expect("resolve through the empty exact-name bucket and remainder");

    // TEXT-CARD must have landed in hand (bucket 1), NOT as a digivolution
    // source — its printed text mentions "Vemmon" but its NAME does not
    // equal "Vemmon", so bucket 2's exact-name filter must reject it.
    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"TEXT-CARD".to_string()),
        "TEXT-CARD must land in hand via bucket 1; hand={:?}",
        hand_ids
    );
    let sources_after = runner.game.players[0].battle_area[vemmon].card_sources.len();
    assert_eq!(
        sources_after, sources_before,
        "no card must be placed as a bottom digivolution source when bucket 2 \
         (exact name match) has zero candidates"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Faithfulness gate (printed text vs runtime behavior)
// ═══════════════════════════════════════════════════════════════════════════════

/// Printed text has no "you may" at the trigger level — the reveal-and-add
/// flow is mandatory once the On Play trigger fires.
#[test]
fn bt18_060_on_play_trigger_is_mandatory_no_outer_optional() {
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
         (DCGO isOptional: false). Setting optional: true would let the \
         player decline the entire reveal-and-add flow, which the printed \
         text forbids."
    );
}

/// Official Q&A: "you must add as many cards to your hand and place as
/// bottom digivolution cards as possible." Once a bucket has a legal
/// candidate, PASS must NOT be a legal action on that bucket.
#[test]
fn bt18_060_bucket_pick_is_mandatory_when_candidate_exists() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("BT18-060 YAML loads")
        .add_card(make_vemmon_text_card("TEXT-CARD"))
        .add_card(make_named_vemmon("NAMED-VEMMON"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["TEXT-CARD", "NAMED-VEMMON", "FILLER"])
        .hand(0, &["BT18-060"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Vemmon BT18-060");

    let view = runner
        .pending_selection_view()
        .expect("bucket 1 prompt installed");
    assert!(
        !runner.pending_is_optional(),
        "bucket 1 must be mandatory when a candidate exists (official Q&A: \
         'must add ... as many ... as possible'); pending_is_optional={}",
        runner.pending_is_optional()
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must NOT be legal on bucket 1 when a candidate exists; \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited [Your Turn][Once Per Turn] digivolve-cost reducer
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE (top-card case, face_up clause): BT18-060 on field (still its
/// permanent's own top card), Lv.4 Vemmon-text Digimon in hand with base evo
/// cost 1. After reduction, effective cost is 0 — memory unchanged.
#[test]
fn bt18_060_cost_reduction_fires_when_digivolving_self_into_vemmon_text() {
    let vemmon_target = make_lv4_black_vemmon_text("VEMMON-LV4");
    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(vemmon_target)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();
    runner.game.turn_count = 1; // [Your Turn]

    let bt18_060 = runner.place_on_field(0, "BT18-060", Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEMMON-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, bt18_060.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "BT18-060 must digivolve into VEMMON-LV4 (evo cost 1 - 1 reduction = 0)"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "digivolution cost must be 0 after BT18-060's own top-card reduction; \
         memory_before={memory_before}"
    );
}

/// NEGATIVE (no [Vemmon] in target text): BT18-060 on field, Lv.4 Digimon in
/// hand with NO "Vemmon" text. Cost reduction must NOT fire; memory
/// decreases by the full base evo cost.
#[test]
fn bt18_060_cost_reduction_does_not_fire_for_non_vemmon_text_target() {
    let plain_target = make_lv4_black_no_vemmon_text("PLAIN-LV4");
    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(plain_target)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let bt18_060 = runner.place_on_field(0, "BT18-060", Some(0));
    {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PLAIN-LV4")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let hand_idx = runner.game.player(0).hand.len() - 1;

    let memory_before = runner.game.memory;
    let _ =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, bt18_060.index as usize, PlaySource::ByHand);
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "cost reduction must NOT fire for a non-Vemmon-text target; base evo \
         cost 1 must be paid in full; memory_before={memory_before}"
    );
}

/// NEGATIVE ([Your Turn] gate): the reducer must not fire on the opponent's
/// turn. (Digivolving is a same-player action, but the `your_turn` gate
/// still exists in the printed text and DCGO's `IsOwnerTurn` check — assert
/// it structurally rather than via an illegal cross-turn digivolve attempt,
/// since the engine only allows digivolving on your own turn regardless.)
#[test]
fn bt18_060_cost_reduction_active_when_includes_your_turn_gate() {
    let card = dsl_card_data::compiled(CARD_ID);
    let mut checked_any = false;
    for clause in &card.effects {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            active_when: Some(pred),
            ..
        }) = clause
        {
            checked_any = true;
            let has_your_turn = pred.your_turn == Some(true)
                || pred.all_of.iter().any(|p| p.your_turn == Some(true));
            assert!(
                has_your_turn,
                "CostReduction active_when must gate on [Your Turn]; got {pred:?}"
            );
        }
    }
    assert!(
        checked_any,
        "expected at least one CostReduction clause with an active_when predicate"
    );
}

/// [Once Per Turn]: BT18-060 buried as a source under HOST-LV4. HOST-LV4
/// digivolves into a Vemmon-text Lv.5 card (reduction fires, cost 2 -> 1).
/// In the SAME turn, the resulting Lv.5 permanent digivolves AGAIN into a
/// second Vemmon-text Lv.6 card — BT18-060 is still buried under the same
/// permanent, but its [Once Per Turn] budget is already spent, so the
/// SECOND reduction must NOT fire (full base cost paid).
#[test]
fn bt18_060_cost_reduction_opt_blocks_second_activation_same_turn() {
    let mut host = make_test_card("HOST-LV4", "Host Lv4");
    host.card_kind = CardKind::Digimon;
    host.level = Some(4);
    host.dp = Some(3000);
    host.colors = vec![CardColor::Black];

    let mut lv5_target = make_test_card("VEMMON-LV5", "Vemmon Lv5 Target");
    lv5_target.card_kind = CardKind::Digimon;
    lv5_target.level = Some(5);
    lv5_target.dp = Some(6000);
    lv5_target.colors = vec![CardColor::Black];
    lv5_target.effect_text =
        "[When Digivolving] [Vemmon] gains <Security Attack +1> for the turn.".to_string();
    lv5_target.evo_costs = vec![EvoCost {
        level: 4,
        card_color: 5,
        memory_cost: 2,
    }];

    let mut lv6_target = make_test_card("VEMMON-LV6", "Vemmon Lv6 Target");
    lv6_target.card_kind = CardKind::Digimon;
    lv6_target.level = Some(6);
    lv6_target.dp = Some(9000);
    lv6_target.colors = vec![CardColor::Black];
    lv6_target.effect_text =
        "[When Digivolving] [Vemmon] gains <Security Attack +1> for the turn.".to_string();
    lv6_target.evo_costs = vec![EvoCost {
        level: 5,
        card_color: 5,
        memory_cost: 2,
    }];

    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(host)
        .add_card(lv5_target)
        .add_card(lv6_target)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let host_perm = runner.place_stack(0, &["BT18-060", "HOST-LV4"]);

    // First digivolve: HOST-LV4 -> VEMMON-LV5. Reduction fires (2 -> 1).
    let hand_idx_lv5 = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEMMON-LV5")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.player(0).hand.len() - 1
    };
    let memory_before_first = runner.game.memory;
    let digivolved_first = runner.game.digivolve_from_hand(
        0,
        hand_idx_lv5,
        host_perm.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_first, "first digivolve into VEMMON-LV5 must succeed");
    assert_eq!(
        runner.game.memory,
        memory_before_first - 1,
        "first digivolve this turn must be reduced by 1 (2 -> 1)"
    );

    // Second digivolve, SAME turn: VEMMON-LV5 (now top) -> VEMMON-LV6.
    // BT18-060 is still buried under the same permanent, but its [Once Per
    // Turn] budget for this turn is already spent — the reduction must NOT
    // fire a second time.
    let hand_idx_lv6 = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEMMON-LV6")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.player(0).hand.len() - 1
    };
    let memory_before_second = runner.game.memory;
    let digivolved_second = runner.game.digivolve_from_hand(
        0,
        hand_idx_lv6,
        host_perm.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved_second,
        "second digivolve into VEMMON-LV6 must succeed (just at full cost)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before_second - 2,
        "[Once Per Turn] must block the second reduction this turn; full \
         base cost 2 must be paid; memory_before_second={memory_before_second}"
    );
}

/// [Once Per Turn] clears after `end_turn`: the SAME two-digivolve sequence
/// as above, but the second digivolve happens on the controller's NEXT
/// turn. The reduction must fire again.
#[test]
fn bt18_060_cost_reduction_opt_clears_after_end_turn() {
    let mut host = make_test_card("HOST2-LV4", "Host2 Lv4");
    host.card_kind = CardKind::Digimon;
    host.level = Some(4);
    host.dp = Some(3000);
    host.colors = vec![CardColor::Black];

    let mut lv5_target = make_test_card("VEMMON2-LV5", "Vemmon2 Lv5 Target");
    lv5_target.card_kind = CardKind::Digimon;
    lv5_target.level = Some(5);
    lv5_target.dp = Some(6000);
    lv5_target.colors = vec![CardColor::Black];
    lv5_target.effect_text =
        "[When Digivolving] [Vemmon] gains <Security Attack +1> for the turn.".to_string();
    lv5_target.evo_costs = vec![EvoCost {
        level: 4,
        card_color: 5,
        memory_cost: 2,
    }];

    let mut lv6_target = make_test_card("VEMMON2-LV6", "Vemmon2 Lv6 Target");
    lv6_target.card_kind = CardKind::Digimon;
    lv6_target.level = Some(6);
    lv6_target.dp = Some(9000);
    lv6_target.colors = vec![CardColor::Black];
    lv6_target.effect_text =
        "[When Digivolving] [Vemmon] gains <Security Attack +1> for the turn.".to_string();
    lv6_target.evo_costs = vec![EvoCost {
        level: 5,
        card_color: 5,
        memory_cost: 2,
    }];

    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(host)
        .add_card(lv5_target)
        .add_card(lv6_target)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let host_perm = runner.place_stack(0, &["BT18-060", "HOST2-LV4"]);

    let hand_idx_lv5 = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEMMON2-LV5")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.player(0).hand.len() - 1
    };
    let digivolved_first = runner.game.digivolve_from_hand(
        0,
        hand_idx_lv5,
        host_perm.index as usize,
        PlaySource::ByHand,
    );
    assert!(digivolved_first, "first digivolve into VEMMON2-LV5 must succeed");

    // Cross a full round (opponent's turn, then back to player 0) so the
    // [Once Per Turn] budget resets.
    runner.end_turn();
    runner.end_turn();
    runner.game.turn_count = runner.game.turn_count.max(1);

    let hand_idx_lv6 = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEMMON2-LV6")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.player(0).hand.len() - 1
    };
    let memory_before_second = runner.game.memory;
    let digivolved_second = runner.game.digivolve_from_hand(
        0,
        hand_idx_lv6,
        host_perm.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved_second,
        "second digivolve into VEMMON2-LV6 must succeed"
    );
    assert_eq!(
        runner.game.memory,
        memory_before_second - 1,
        "after end_turn crosses a turn boundary, the [Once Per Turn] budget \
         resets and the reduction must fire again (2 -> 1); \
         memory_before_second={memory_before_second}"
    );
}

/// POSITIVE (buried-source case, inherited clause): BT18-060 is placed as a
/// bottom digivolution SOURCE under a stand-in top card. The stand-in then
/// digivolves further into a Vemmon-text Lv.5 card — the reduction must
/// still fire because BT18-060 (buried) is part of the same permanent.
#[test]
fn bt18_060_cost_reduction_fires_when_buried_source_and_host_digivolves_into_vemmon_text() {
    let mut host = make_test_card("HOST-LV4", "Host Lv4");
    host.card_kind = CardKind::Digimon;
    host.level = Some(4);
    host.dp = Some(3000);
    host.colors = vec![CardColor::Black];

    let mut lv5_target = make_test_card("VEMMON-LV5", "Vemmon Lv5 Target");
    lv5_target.card_kind = CardKind::Digimon;
    lv5_target.level = Some(5);
    lv5_target.dp = Some(6000);
    lv5_target.colors = vec![CardColor::Black];
    lv5_target.effect_text =
        "[When Digivolving] [Vemmon] gains <Security Attack +1> for the turn.".to_string();
    lv5_target.evo_costs = vec![EvoCost {
        level: 4,
        card_color: 5,
        memory_cost: 2,
    }];

    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(host)
        .add_card(lv5_target)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Build the stack: BT18-060 buried under HOST-LV4 (HOST-LV4 is the top).
    let host_perm = runner.place_stack(0, &["BT18-060", "HOST-LV4"]);
    assert_eq!(
        runner.game.players[0].battle_area[host_perm.index as usize]
            .card_sources
            .len(),
        2,
        "stack must have BT18-060 buried under HOST-LV4"
    );

    let hand_idx = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "VEMMON-LV5")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.player(0).hand.len() - 1
    };

    let memory_before = runner.game.memory;
    let digivolved = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        host_perm.index as usize,
        PlaySource::ByHand,
    );
    assert!(
        digivolved,
        "HOST-LV4 must digivolve into VEMMON-LV5 (evo cost 2 - 1 reduction = 1)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "digivolution cost must be reduced by exactly 1 (2 -> 1) via BT18-060's \
         buried-source (inherited) reducer; memory_before={memory_before}"
    );
}

/// NEGATIVE (buried-source case, no Vemmon text on target): same stack setup
/// as above, but the digivolve target has NO Vemmon text — the reduction
/// must not fire.
#[test]
fn bt18_060_cost_reduction_does_not_fire_buried_source_for_non_vemmon_target() {
    let mut host = make_test_card("HOST-LV4B", "Host Lv4 B");
    host.card_kind = CardKind::Digimon;
    host.level = Some(4);
    host.dp = Some(3000);
    host.colors = vec![CardColor::Black];

    let mut lv5_plain = make_test_card("PLAIN-LV5", "Plain Lv5 Target");
    lv5_plain.card_kind = CardKind::Digimon;
    lv5_plain.level = Some(5);
    lv5_plain.dp = Some(6000);
    lv5_plain.colors = vec![CardColor::Black];
    lv5_plain.evo_costs = vec![EvoCost {
        level: 4,
        card_color: 5,
        memory_cost: 2,
    }];

    let mut filler = make_test_card("FILL", "Filler");
    filler.colors = vec![CardColor::Black];

    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-060")
        .expect("parses")
        .add_card(host)
        .add_card(lv5_plain)
        .add_card(filler)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let host_perm = runner.place_stack(0, &["BT18-060", "HOST-LV4B"]);

    let hand_idx = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PLAIN-LV5")
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .hand
            .push(CardSource::new(data_idx, 0, card_index));
        runner.game.player(0).hand.len() - 1
    };

    let memory_before = runner.game.memory;
    let _ = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        host_perm.index as usize,
        PlaySource::ByHand,
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "no reduction for a non-Vemmon-text target even though BT18-060 is a \
         buried source of the digivolving permanent; full base cost 2 paid; \
         memory_before={memory_before}"
    );
}
