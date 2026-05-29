//! BT22-017 Gabumon — Digimon, Lv.3, Blue, DP 1000, Cost 3.
//! Traits: Reptile, CS.
//! Evo: Lv.2 Blue / cost 0.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Omnimon]
//! in its text and 1 card with the [CS] trait among them to the hand.
//! Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//!   [End of Your Turn] This Digimon and any of your other Digimon may DNA
//!   digivolve into a Digimon card in the hand.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/Blue/BT22_017.cs`
//!
//! Notable DCGO contracts:
//!   - [On Play] outer activate: `SetUpActivateClass(..., -1, false, ...)`
//!     — printed text has no "you may"; reveal-and-add is mandatory if the
//!     activate condition holds.
//!   - Bucket 1: `SelectOmnimonInText(source) => source.HasText("Omnimon")`
//!     — DCGO scans the candidate's printed effect text for "Omnimon".
//!   - Bucket 2: `SelectCS(source) => source.HasCSTraits` — `CS` trait scan.
//!   - `SimplifiedRevealDeckTopCardsAndSelect` with `RemainingCardsPlace.DeckBottom`
//!     and the standard `mutualConditions` — encoded as
//!     `select_reveal_buckets` with `no_duplicate_cards: true` plus a trailing
//!     `place_remainder_on_deck { position: bottom }` step.
//!   - Inherited [End of Your Turn] DNA digivolve registration —
//!     `SetUpActivateClass(..., -1, true, ...)` + `SetIsInheritedEffect(true)`.
//!     Encoded via `alt_path_registration` declarative (RESOLVED 2026-05-02
//!     vocab note in qa/dsl-vocab-gaps.md, BT22-008/BT22-017 entry).
//!
//! # Sister card
//!
//! BT12-059 (Agumon — reveal 4, two buckets: Greymon/Omnimon Digimon +
//! Tai Kamiya Tamer). BT22-017 differs by:
//!   (a) reveal count 3 instead of 4;
//!   (b) bucket 1 is `[Omnimon] in TEXT` (`effect_text_contains`) instead
//!       of a name scan;
//!   (c) bucket 2 is `[CS] trait` instead of a name+kind filter;
//!   (d) inherited clause is the BT22-008-style DNA digivolve registration
//!       instead of BT12-059's +1000 DP aura.
//!
//! BT22-008 is the BT22 sibling that uses the same inherited DNA
//! registration shape. Tests for the inherited registration mirror
//! BT22-008's `bt22_008_has_inherited_dna_digivolve_alt_path_registration`.
//!
//! # G-DSL-PREDICATE-TEXT-CONTAINS — `effect_text_contains` (RESOLVED)
//!
//! Bucket 1's printed filter is "1 card with [Omnimon] in its TEXT" — i.e.
//! a substring scan against the candidate card's printed `effect_text`
//! (and inherited / security text). RESOLVED 2026-05-19: the DSL gained an
//! `effect_text_contains` predicate leaf — a case-insensitive substring
//! scan against the candidate card's concatenated printed text
//! (`effect_text` + `inherited_text` + `security_text`). Bucket 1 now uses
//! `effect_text_contains: "Omnimon"` directly, matching DCGO
//! `source.HasText("Omnimon")` faithfully. Cards that mention "[Omnimon]"
//! only in their effect text — without carrying Omnimon in their name — are
//! now correctly admitted (see
//! `bt22_017_on_play_bucket1_admits_card_with_omnimon_only_in_text`).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4 / §5)
//!   - §5 Structural: standard digivolve alt-path; on-play triggered clause;
//!     inherited `alt_path_registration` for end-of-your-turn DNA digivolve.
//!   - §5 Condition gating: empty deck reveal (engine truncates), zero
//!     bucket-1 candidates, zero bucket-2 candidates, both buckets satisfied.
//!   - §5 Behavioral integrated: resolving the install moves the chosen
//!     [Omnimon]-text card and CS-trait card from reveal to hand; remainder
//!     to deck bottom.
//!   - §5 Faithfulness gate: outer trigger is MANDATORY (no "you may" in
//!     printed text; DCGO `isOptional: false`).
//!   - §5 Faithfulness: bucket 1 admits a card mentioning [Omnimon] only in
//!     effect text (`effect_text_contains` predicate).
//!
//! # `cards.rs` and `mod.rs` policy
//! This test file is registered in `tests/cards_behavioral/bt22/mod.rs`
//! (already present). No `cards.rs` / `trackers` mutation required —
//! BT22-017 is a pure DSL-only card; no hand-written `CardEffect` exists
//! for it. The `dsl_card_data` helper resolves the card via the embedded
//! pack built from `cards/bt22/BT22-017.yaml`.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

const CARD_ID: &str = "BT22-017";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn gabumon_card_data() -> CardData {
    dsl_card_data::card_data_from_compiled(CARD_ID)
}

/// Build a Digimon test card with a specific id, name, and trait list. Used
/// to seed deck / reveal candidates for the two buckets.
fn make_named_digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Build a Tamer test card with a specific id, name, and trait list. Used
/// for the `[CS]` trait bucket — Tamers can carry traits too.
fn make_named_tamer(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Build an Omnimon-style Digimon fixture for the bucket-1 ("[Omnimon] in
/// its text") candidate. Bucket 1 uses the `effect_text_contains: "Omnimon"`
/// predicate, so a legitimate bucket-1 target must carry "Omnimon" in its
/// printed effect text — real Omnimon cards print their own name in body
/// text (`[Omnimon] gains …` / DNA-digivolve clauses). The fixture name
/// also contains "Omnimon" so it doubles as a realistic Omnimon card.
fn make_omnimon_card(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_named_digimon(id, "Omnimon", traits);
    card.effect_text =
        "[When Digivolving] [Omnimon] gains <Security Attack +1> for the turn.".to_string();
    card
}

/// Convert a zone of `CardSource`s into a Vec of card_id strings, in order.
fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

/// Resolve a revealed-card action id by candidate card_id. Returns None if
/// the named card is not currently in the reveal overlay.
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

/// Pick a revealed card by card_id. Asserts the action is currently legal in
/// the active bucket; panics on mismatch.
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
fn bt22_017_yaml_parses_and_compiles() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/bt22/BT22-017.yaml"))
        .expect("BT22-017 YAML parses");
    let _compiled = compile(&spec).expect("BT22-017 YAML compiles");
}

#[test]
fn bt22_017_yaml_compiles_in_embedded_pack() {
    // Smoke: the embedded DSL pack ships BT22-017 with the printed metadata.
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Gabumon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("Reptile")),
        "BT22-017 must carry the Reptile trait; got traits={:?}",
        card.traits
    );
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("CS")),
        "BT22-017 must carry the CS trait; got traits={:?}",
        card.traits
    );
}

#[test]
fn bt22_017_has_standard_lv2_blue_digivolve_alt_path_at_cost_zero() {
    // Printed evo_costs entry is Lv.2 Blue / cost 0.
    let card = dsl_card_data::compiled(CARD_ID);
    let standard = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        standard.is_some(),
        "BT22-017 must declare its standard Lv.2 Blue / cost 0 digivolve path"
    );
}

#[test]
fn bt22_017_has_on_play_triggered_clause_mandatory() {
    let card = dsl_card_data::compiled(CARD_ID);
    // Filter by OnPlay so we ignore the inherited EOT DNA-digivolve triggered
    // clause (G-DSL-EOT-DNA-INLINE, 2026-05-24).
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
         (DCGO isOptional: false at BT22_017.cs line 43)"
    );
    assert!(
        !clause.once_per_turn,
        "On-play clause has no [Once Per Turn]"
    );
}

#[test]
fn bt22_017_has_inherited_dna_digivolve_may_step() {
    // G-DSL-EOT-DNA-INLINE (2026-05-24): inherited end-of-your-turn DNA
    // digivolve is now authored via the `may_dna_digivolve_now` step verb
    // inside a `Triggered` clause whose `when: end_of_your_turn, scope:
    // inherited, optional: true`. The body is a single `MayDnaDigivolveNow`
    // step that orchestrates the partner + target selections at trigger fire.
    //
    // Predecessor authoring (`alt_path_registration { kind: dna_digivolve,
    // scope: inherited, trigger: end_of_your_turn }`) registered a next-turn
    // alt-path action and did not satisfy the printed "AT end of turn"
    // surface — see proposal `may-dna-digivolve-now-dsl-step`.
    let card = dsl_card_data::compiled(CARD_ID);
    let eot_dna_clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::EndOfYourTurn)
                && t.optional =>
        {
            Some(t)
        }
        _ => None,
    });
    let t = eot_dna_clause
        .expect("BT22-017 must carry an inherited optional EoT triggered clause for DNA digivolve");
    assert!(
        t.process.iter().any(|step| matches!(
            step,
            CompiledStep::MayDnaDigivolveNow {
                ignore_requirements: true,
                cost: 0,
                ..
            }
        )),
        "BT22-017 EoT inherited body must contain a `MayDnaDigivolveNow` step \
         with cost=0 and ignore_requirements=true"
    );
}

#[test]
fn bt22_017_clause_count_matches_card_text() {
    // G-DSL-EOT-DNA-INLINE (2026-05-24): card now prints two triggered
    // clauses — (1) the [On Play] reveal-3 + 2-bucket trigger and (2) the
    // inherited [End of Your Turn] DNA digivolve trigger (was previously
    // an alt_path_registration declarative — see proposal
    // `may-dna-digivolve-now-dsl-step`). No more alt_path_registration
    // clauses on this card.
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
    assert_eq!(triggered, 2, "Exactly two triggered clauses expected");
    assert_eq!(
        alt_path_regs, 0,
        "Zero alt_path_registration clauses expected after EoT DNA migration"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — On-play behavioral: reveal 3, two buckets, bottom remainder
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive path — both buckets satisfied. Reveal 3 = [OMNIMON, CS-CARD,
/// FILLER]. Pick OMNIMON for bucket 1 and CS-CARD for bucket 2. The FILLER
/// goes to deck bottom.
#[test]
fn bt22_017_on_play_adds_one_omnimon_named_and_one_cs_trait_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-017")
        .expect("BT22-017 YAML loads")
        .add_card(make_omnimon_card("OMNIMON", &["Holy Warrior"]))
        .add_card(make_named_digimon("CS-CARD", "Greymon", &["Reptile", "CS"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["OMNIMON", "CS-CARD", "FILLER"])
        .hand(0, &["BT22-017"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon BT22-017");

    // Bucket 1: [Omnimon] in effect text.
    pick_revealed_by_id(&mut runner, "OMNIMON", "pick Omnimon (effect-text)");

    // Bucket 2: [CS] trait.
    pick_revealed_by_id(&mut runner, "CS-CARD", "pick CS-trait card");

    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"OMNIMON".to_string()),
        "Omnimon-named card must land in hand; hand={:?}",
        hand_ids
    );
    assert!(
        hand_ids.contains(&"CS-CARD".to_string()),
        "CS-trait card must land in hand; hand={:?}",
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
        "unchosen reveal card returns to deck bottom; deck end was {:?}",
        deck_ids
    );
}

/// Bucket-1 only — reveal contains an Omnimon-named card but NO CS-trait
/// card. The [CS] bucket has zero candidates and is skipped; the Omnimon
/// bucket fires.
#[test]
fn bt22_017_on_play_only_omnimon_found_skips_cs_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-017")
        .expect("BT22-017 YAML loads")
        .add_card(make_omnimon_card("OMNIMON", &["Holy Warrior"]))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["OMNIMON", "FILLER1", "FILLER2"])
        .hand(0, &["BT22-017"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon BT22-017");
    pick_revealed_by_id(&mut runner, "OMNIMON", "pick Omnimon");
    runner
        .auto_resolve()
        .expect("resolve through empty CS bucket");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"OMNIMON".to_string()),
        "Omnimon must be added to hand; hand={:?}",
        hand_ids
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "no CS-trait candidate → no extra add; hand={:?}",
        hand_ids
    );
}

/// Bucket-2 only — reveal contains a CS-trait card but NO Omnimon-named
/// card. The Omnimon bucket has zero candidates and is skipped.
#[test]
fn bt22_017_on_play_only_cs_trait_found_skips_omnimon_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-017")
        .expect("BT22-017 YAML loads")
        .add_card(make_named_digimon("CS-CARD", "Greymon", &["Reptile", "CS"]))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["CS-CARD", "FILLER1", "FILLER2"])
        .hand(0, &["BT22-017"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon BT22-017");

    // Walk prompts: pick CS-CARD when offered, otherwise advance with first
    // legal action (PASS-equivalent for empty bucket / bottom-placement order).
    while let Some(view) = runner.pending_selection_view() {
        if let Some(action) = revealed_action_for_id(&runner, "CS-CARD") {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick CS card");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"CS-CARD".to_string()),
        "CS-trait card must land in hand; hand={:?}",
        hand_ids
    );
}

/// Neither bucket has a candidate — reveal 3 fillers, both buckets skip,
/// all 3 cards hit deck bottom.
#[test]
fn bt22_017_on_play_neither_bucket_found_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-017")
        .expect("BT22-017 YAML loads")
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["BT22-017"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon BT22-017");
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
// Section 3 — `no_duplicate_cards` and bucket-overlap semantics
// ═══════════════════════════════════════════════════════════════════════════════

/// A single card can satisfy BOTH bucket predicates simultaneously: an
/// Omnimon-named CS-trait Digimon (e.g. BT22-015 itself). With
/// `no_duplicate_cards: true` (DCGO `mutualConditions`), a card consumed by
/// one bucket must NOT be available to the other. The test below seeds a
/// dual-eligible card plus a CS-only card; picking the dual-eligible for
/// bucket 1 must leave bucket 2 with only the CS-only card available.
#[test]
fn bt22_017_no_duplicate_cards_prevents_double_consumption() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-017")
        .expect("BT22-017 YAML loads")
        // Dual-eligible: Omnimon-named AND CS-trait.
        .add_card(make_named_digimon(
            "DUAL",
            "Omnimon",
            &["Royal Knight", "CS"],
        ))
        // CS-only: not Omnimon-named, has CS trait.
        .add_card(make_named_digimon("CS-ONLY", "Greymon", &["Reptile", "CS"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["DUAL", "CS-ONLY", "FILLER"])
        .hand(0, &["BT22-017"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon BT22-017");

    // Pick the dual-eligible for bucket 1 (Omnimon).
    pick_revealed_by_id(&mut runner, "DUAL", "pick dual for Omnimon bucket");

    // Bucket 2 must now offer ONLY the CS-only card — not the already-consumed
    // DUAL card.
    let view = runner
        .pending_selection_view()
        .expect("CS bucket should be active");
    let dual_action = revealed_action_for_id(&runner, "DUAL");
    let cs_only_action = revealed_action_for_id(&runner, "CS-ONLY");
    if let Some(dual) = dual_action {
        assert!(
            !view.valid_action_ids.contains(&dual),
            "no_duplicate_cards: DUAL must NOT be re-selectable after \
             being consumed by bucket 1; valid_action_ids={:?}",
            view.valid_action_ids
        );
    }
    if let Some(cs) = cs_only_action {
        assert!(
            view.valid_action_ids.contains(&cs),
            "CS-ONLY must remain selectable for bucket 2; \
             valid_action_ids={:?}",
            view.valid_action_ids
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Faithfulness: bucket 1 scans printed effect text
//                                              (G-DSL-PREDICATE-TEXT-CONTAINS)
// ═══════════════════════════════════════════════════════════════════════════════

/// Faithfulness test: bucket 1's printed filter is "[Omnimon] in its TEXT".
/// The YAML uses the `effect_text_contains: "Omnimon"` predicate
/// (G-DSL-PREDICATE-TEXT-CONTAINS, RESOLVED 2026-05-19), which scans the
/// candidate card's printed effect / inherited / security text. A card that
/// mentions `[Omnimon]` only in its effect_text WITHOUT carrying "Omnimon"
/// in its card_name must still be selectable for bucket 1 per the printed
/// text.
///
/// This test seeds a Tamer named "Yamato Ishida" with `effect_text` =
/// "Add 1 [Omnimon] from your trash to your hand." (a printed text
/// containing the marker "[Omnimon]" but a name without "Omnimon"). The
/// bucket 1 prompt MUST offer this Tamer as a target.
#[test]
fn bt22_017_on_play_bucket1_admits_card_with_omnimon_only_in_text() {
    let mut yamato = make_named_tamer("YAMATO", "Yamato Ishida", &[]);
    yamato.effect_text = "Add 1 [Omnimon] from your trash to your hand.".to_string();

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-017")
        .expect("BT22-017 YAML loads")
        .add_card(yamato)
        .add_card(make_named_digimon("CS-CARD", "Greymon", &["Reptile", "CS"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["YAMATO", "CS-CARD", "FILLER"])
        .hand(0, &["BT22-017"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon BT22-017");

    // Per the printed semantic, YAMATO must be a legal target for bucket 1
    // because its effect_text contains "[Omnimon]" — the
    // `effect_text_contains: "Omnimon"` predicate scans printed text.
    let view = runner
        .pending_selection_view()
        .expect("bucket 1 selection installed");
    let yamato_action =
        revealed_action_for_id(&runner, "YAMATO").expect("YAMATO is in the reveal overlay");
    assert!(
        view.valid_action_ids.contains(&yamato_action),
        "Faithfulness: bucket 1 must admit YAMATO (effect_text contains [Omnimon]); \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Faithfulness gate (printed text vs runtime behavior)
// ═══════════════════════════════════════════════════════════════════════════════

/// Printed text reads "Reveal the top 3 cards of your deck. Add 1 ... and 1 ..."
/// — there is NO "you may" gate at the trigger level. DCGO confirms
/// `isOptional: false` on the outer ActivateClass at line 43.
///
/// The active bucket selection (which ALWAYS installs once the trigger
/// fires) MUST not be declinable at the trigger level. This is the inverse
/// of the BT22-008 faithfulness pin (which is OPTIONAL because that card's
/// printed text reads "you may").
#[test]
fn bt22_017_on_play_trigger_is_mandatory_no_outer_optional() {
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
         (DCGO BT22_017.cs line 43 isOptional: false). Setting optional: true \
         on this clause would let the player decline the entire reveal-and-add \
         flow, which the printed text forbids."
    );
}

/// Cross-check: BT22-017's `[On Play]` and the inherited `alt_path_registration`
/// must be the ONLY effects on the card. No `static`, no `aura`, no `cost
/// reduction`, no extra triggers. Mirrors the `bt22_008_clause_count_matches_card_text`
/// pin.
#[test]
fn bt22_017_no_extra_effects_beyond_two_clauses() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(
        card.effects.len(),
        2,
        "BT22-017 prints exactly 2 effect clauses; got {} clauses: {:?}",
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

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — PASS legality on the bucket prompts (no over-permissive opt-out)
// ═══════════════════════════════════════════════════════════════════════════════

/// Once a bucket has a legal target, PASS must NOT be a legal action on
/// that bucket — the printed text says "Add 1 card with [X]" (mandatory
/// add when a candidate exists). Mirrors BT12-059's
/// `assert_required_pending_pick` convention.
#[test]
fn bt22_017_bucket_pick_is_mandatory_when_candidate_exists() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-017")
        .expect("BT22-017 YAML loads")
        .add_card(make_omnimon_card("OMNIMON", &["Holy Warrior"]))
        .add_card(make_named_digimon("CS-CARD", "Greymon", &["Reptile", "CS"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["OMNIMON", "CS-CARD", "FILLER"])
        .hand(0, &["BT22-017"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gabumon BT22-017");

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
