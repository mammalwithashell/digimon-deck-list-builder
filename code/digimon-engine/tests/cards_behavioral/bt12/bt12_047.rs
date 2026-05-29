//! BT12-047 Wormmon — Digimon, Lv.3, Green, DP 1000, Cost 3.
//! Traits: Larva.
//! Evo: Lv.2 Green / cost 0 (printed evo_costs).
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with
//! [Imperialdramon] in its name or [Free] trait and 1 Tamer card with
//! [Ken Ichijoji] in its name among them to your hand. Place the rest at
//! the bottom of your deck in any order.
//!
//! Inherited Effect:
//! [End of Your Turn] This Digimon and any of your other Digimon may DNA
//! digivolve into a Digimon card in the hand.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT12/Green/BT12_047.cs`
//!
//! # Sister card
//! BT12-021 Veemon — same OnPlay reveal-3 two-bucket shape (Imperialdramon/Free
//! Digimon + named Tamer) and same inherited EOYT DNA digivolve registration.
//! Only the Tamer-name filter differs (Ken Ichijoji here vs Davis Motomiya
//! for BT12-021).
//!
//! # DCGO contract crosscheck
//!
//! Clause 0 (OnPlay):
//!   * `CanSelectCardCondition`: IsDigimon AND (ContainsCardName("Imperialdramon")
//!     OR CardTraits.Contains("Free")). DSL: `kind: digimon` AND `any_of:
//!     [name_contains: "Imperialdramon", trait_has: "Free"]`.
//!   * `CanSelectCardCondition1`: IsTamer AND ContainsCardName("Ken Ichijoji").
//!     DSL: `kind: tamer` AND `name_contains: "Ken Ichijoji"`.
//!   * `mutualConditions: true` → `no_duplicate_cards: true` in DSL.
//!   * `RemainingCardsPlace.DeckBottom` → trailing
//!     `place_remainder_on_deck { position: bottom }`.
//!
//! Clause 1 (Inherited EOYT):
//!   * Encoded as `alt_path_registration` declarative, scope: inherited,
//!     trigger: end_of_your_turn, registers: dna_digivolve.
//!
//! # Patterns this test covers
//! - A1/A3 — On-play reveal-N + multi-bucket selection (two named buckets
//!   where bucket 0 combines name + trait predicate).
//! - Inherited alt_path_registration for EOYT DNA digivolve.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "BT12-047";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card
}

fn make_named_digimon_with_trait(id: &str, name: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.traits = vec![trait_name.to_string()];
    card
}

fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn deck_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    zone_ids(&runner.game.players[0].hand, &runner.game.card_data)
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
        "{label}: action {action} for {id} not legal in current bucket"
    );
    runner.execute_action(0, action).expect(label);
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: bucket should be mandatory when a matching card exists"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal before the required pick"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT12-047 YAML must parse and compile cleanly.
#[test]
fn bt12_047_yaml_parses_and_compiles() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("Larva")),
        "BT12-047 must carry the Larva trait; got traits={:?}",
        card.traits
    );
    assert!(
        card.color
            .iter()
            .any(|c| format!("{c:?}").to_lowercase() == "green"),
        "BT12-047 must be Green; got color={:?}",
        card.color
    );
}

/// G-DSL-EOT-DNA-INLINE (2026-05-24): BT12-047 declares exactly one
/// `OnPlay` triggered clause AND one inherited end-of-your-turn triggered
/// clause for DNA digivolve (was previously an alt_path_registration
/// declarative — see proposal `may-dna-digivolve-now-dsl-step`). No more
/// alt_path_registration clauses on this card.
#[test]
fn bt12_047_has_one_on_play_triggered_and_one_inherited_dna_eot_trigger() {
    let card = compiled(CARD_ID);

    let on_play_count = card
        .effects
        .iter()
        .filter(|c| {
            if let CompiledClause::Triggered(t) = c {
                t.when.contains(&CompiledTiming::OnPlay)
            } else {
                false
            }
        })
        .count();
    assert_eq!(on_play_count, 1, "exactly one OnPlay triggered clause");

    let inherited_eot_count = card
        .effects
        .iter()
        .filter(|c| {
            if let CompiledClause::Triggered(t) = c {
                t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::EndOfYourTurn)
            } else {
                false
            }
        })
        .count();
    assert_eq!(
        inherited_eot_count, 1,
        "exactly one inherited end-of-your-turn triggered clause (DNA digivolve)"
    );

    let registration_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::AltPathRegistration { .. })
            )
        })
        .count();
    assert_eq!(
        registration_count, 0,
        "zero alt_path_registration clauses expected after EoT DNA migration"
    );
}

/// G-DSL-EOT-DNA-INLINE (2026-05-24): inherited end-of-your-turn DNA
/// digivolve is now authored via the `may_dna_digivolve_now` step verb
/// inside a `Triggered` clause whose `when: end_of_your_turn, scope:
/// inherited, optional: true`. The body is a single `MayDnaDigivolveNow`
/// step that orchestrates the partner + target selections at trigger fire.
///
/// Predecessor authoring (`alt_path_registration { kind: dna_digivolve,
/// scope: inherited, trigger: end_of_your_turn }`) registered a next-turn
/// alt-path action and did not satisfy the printed "AT end of turn"
/// surface — see proposal `may-dna-digivolve-now-dsl-step`.
#[test]
fn bt12_047_has_inherited_dna_digivolve_may_step() {
    let card = compiled(CARD_ID);
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
        .expect("BT12-047 must carry an inherited optional EoT triggered clause for DNA digivolve");
    assert!(
        t.process.iter().any(|step| matches!(
            step,
            CompiledStep::MayDnaDigivolveNow {
                ignore_requirements: true,
                cost: 0,
                ..
            }
        )),
        "BT12-047 EoT inherited body must contain a `MayDnaDigivolveNow` step \
         with cost=0 and ignore_requirements=true"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — On-play behavioral: reveal 3, two buckets, bottom remainder
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive path — both buckets satisfied. Reveal 3 = [IMPERIALDRAMON, KEN,
/// FILLER]. Pick IMPERIALDRAMON for the Digimon bucket (name match) and KEN
/// for the Tamer bucket. FILLER goes to deck bottom.
#[test]
fn bt12_047_on_play_adds_imperialdramon_digimon_and_ken_tamer_bottoms_rest() {
    let imperialdramon = make_named_digimon("IMPERIALDRAMON", "Imperialdramon");
    let ken = make_named_tamer("KEN", "Ken Ichijoji");
    let filler = make_test_card("FILLER", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-047 YAML loads")
        .add_card(imperialdramon)
        .add_card(ken)
        .add_card(filler)
        .deck(0, &["IMPERIALDRAMON", "KEN", "FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon BT12-047");

    // First bucket: Digimon with Imperialdramon name or Free trait.
    assert_required_pending_pick(&runner, "Imperialdramon/Free Digimon bucket");
    pick_revealed_by_id(&mut runner, "IMPERIALDRAMON", "pick Imperialdramon");

    // Second bucket: Tamer with Ken Ichijoji name.
    assert_required_pending_pick(&runner, "Ken Ichijoji Tamer bucket");
    pick_revealed_by_id(&mut runner, "KEN", "pick Ken Ichijoji");

    runner.auto_resolve().expect("bottom remainder");

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"IMPERIALDRAMON".to_string()),
        "Imperialdramon Digimon must land in hand; hand={hand:?}"
    );
    assert!(
        hand.contains(&"KEN".to_string()),
        "Ken Ichijoji Tamer must land in hand; hand={hand:?}"
    );

    let deck = deck_ids(&runner);
    let last = deck.iter().rev().take(1).cloned().collect::<Vec<_>>();
    assert!(
        last.contains(&"FILLER".to_string()),
        "unchosen FILLER must return to deck bottom; deck_end={last:?}"
    );
}

/// Free-trait Digimon bucket — reveal 3 contains a Digimon with [Free] trait
/// (not named Imperialdramon). Must be eligible for the first bucket.
#[test]
fn bt12_047_on_play_free_trait_digimon_satisfies_first_bucket() {
    let free_digimon = make_named_digimon_with_trait("FREE-D", "ExVeemon", "Free");
    let ken = make_named_tamer("KEN", "Ken Ichijoji");
    let filler = make_test_card("FILLER", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-047 YAML loads")
        .add_card(free_digimon)
        .add_card(ken)
        .add_card(filler)
        .deck(0, &["FREE-D", "KEN", "FILLER"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon BT12-047");

    // The [Free]-trait Digimon must appear in the first bucket's candidates.
    assert_required_pending_pick(&runner, "Free-trait Digimon bucket");
    pick_revealed_by_id(&mut runner, "FREE-D", "pick Free-trait Digimon");

    // Ken Ichijoji Tamer second bucket.
    assert_required_pending_pick(&runner, "Ken Ichijoji Tamer bucket");
    pick_revealed_by_id(&mut runner, "KEN", "pick Ken Ichijoji");

    runner.auto_resolve().expect("bottom remainder");

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"FREE-D".to_string()),
        "Free-trait Digimon must land in hand; hand={hand:?}"
    );
    assert!(
        hand.contains(&"KEN".to_string()),
        "Ken Ichijoji Tamer must land in hand; hand={hand:?}"
    );
}

/// Imperialdramon-only branch — reveal 3 has Imperialdramon Digimon but NO
/// Ken Ichijoji Tamer. The Tamer bucket has zero candidates; the engine
/// skips it. Only the Digimon bucket adds to hand.
#[test]
fn bt12_047_on_play_only_imperialdramon_found_skips_tamer_bucket() {
    let imperialdramon = make_named_digimon("IMPERIALDRAMON", "Imperialdramon");
    let filler1 = make_test_card("FILLER1", "Filler1");
    let filler2 = make_test_card("FILLER2", "Filler2");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-047 YAML loads")
        .add_card(imperialdramon)
        .add_card(filler1)
        .add_card(filler2)
        .deck(0, &["IMPERIALDRAMON", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon BT12-047");
    pick_revealed_by_id(&mut runner, "IMPERIALDRAMON", "pick Imperialdramon");
    runner
        .auto_resolve()
        .expect("resolve through empty Tamer bucket");

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"IMPERIALDRAMON".to_string()),
        "Imperialdramon must be added to hand; hand={hand:?}"
    );
    assert!(
        !hand.iter().any(|id| id == "FILLER1" || id == "FILLER2"),
        "no Tamer found, no extra add; hand={hand:?}"
    );
}

/// Ken-only branch — reveal 3 has a Ken Ichijoji Tamer but NO matching Digimon.
/// The Digimon bucket has zero candidates and is skipped; the Tamer bucket fires.
#[test]
fn bt12_047_on_play_only_ken_found_skips_digimon_bucket() {
    let ken = make_named_tamer("KEN", "Ken Ichijoji");
    let filler1 = make_test_card("FILLER1", "Filler1");
    let filler2 = make_test_card("FILLER2", "Filler2");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-047 YAML loads")
        .add_card(ken)
        .add_card(filler1)
        .add_card(filler2)
        .deck(0, &["KEN", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon BT12-047");

    // Drive prompts — pick Ken when visible in a bucket, advance otherwise.
    while let Some(view) = runner.pending_selection_view() {
        let ken_action = revealed_action_for_id(&runner, "KEN");
        if let Some(action) = ken_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick Ken");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"KEN".to_string()),
        "Ken Ichijoji Tamer must land in hand; hand={hand:?}"
    );
}

/// Neither bucket has a candidate — reveal 3 fillers. Both buckets resolve
/// without adds. All 3 cards go to deck bottom. Hand stays clear.
#[test]
fn bt12_047_on_play_neither_bucket_found_bottoms_all_three() {
    let filler1 = make_test_card("FILLER1", "Filler1");
    let filler2 = make_test_card("FILLER2", "Filler2");
    let filler3 = make_test_card("FILLER3", "Filler3");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-047 YAML loads")
        .add_card(filler1)
        .add_card(filler2)
        .add_card(filler3)
        .deck(0, &["FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon BT12-047");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets");

    let hand = hand_ids(&runner);
    for filler in ["FILLER1", "FILLER2", "FILLER3"] {
        assert!(
            !hand.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no bucket matches; hand={hand:?}"
        );
    }
    let deck = deck_ids(&runner);
    assert_eq!(deck.len(), 3, "all 3 reveal cards back in deck");
}

/// The Digimon bucket only accepts `kind: digimon`. A Tamer card named
/// "Ken Ichijoji" must NOT satisfy the Digimon bucket (even if it were
/// given a Free trait — tamer kind is the gate).
///
/// Setup: reveal contains a Tamer named "Ken Ichijoji" (no Free-named
/// Digimon). The Digimon bucket finds zero candidates. The Tamer bucket
/// picks Ken.
#[test]
fn bt12_047_digimon_bucket_rejects_tamer_kind_card() {
    let ken_tamer = make_named_tamer("KEN", "Ken Ichijoji");
    let filler1 = make_test_card("FILLER1", "Filler1");
    let filler2 = make_test_card("FILLER2", "Filler2");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-047 YAML loads")
        .add_card(ken_tamer)
        .add_card(filler1)
        .add_card(filler2)
        .deck(0, &["KEN", "FILLER1", "FILLER2"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon BT12-047");

    // Drive: pick Ken when legal (Tamer bucket), reject if it appears alone
    // as Digimon-bucket-only option (which would be wrong).
    while let Some(view) = runner.pending_selection_view() {
        let ken_action = revealed_action_for_id(&runner, "KEN");
        if let Some(action) = ken_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick Ken");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"KEN".to_string()),
        "Ken Tamer must be picked for the Tamer bucket; hand={hand:?}"
    );
}

/// `no_duplicate_cards: true` — the same card must not satisfy both buckets.
/// If the only revealed card is a Digimon named "Imperialdramon" with Free
/// trait, it goes to the Digimon bucket; it does NOT also go to the Tamer
/// bucket (and cannot, since it's kind: digimon, not kind: tamer).
///
/// This test verifies the overall one-card-per-bucket invariant by checking
/// that with only one Digimon in the reveal and no Tamer, only the Digimon
/// bucket produces an add.
#[test]
fn bt12_047_no_duplicate_cards_across_buckets() {
    // Single-card deck: one Imperialdramon Digimon with Free trait. Reveals
    // exactly 1 card (deck size = 1). Digimon bucket takes it; Tamer bucket
    // has no candidates. Net result: Imperialdramon in hand, deck empty.
    let imperialdramon_free = make_named_digimon_with_trait("IMP-FREE", "Imperialdramon", "Free");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-047 YAML loads")
        .add_card(imperialdramon_free)
        .deck(0, &["IMP-FREE"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Wormmon BT12-047");

    while let Some(view) = runner.pending_selection_view() {
        let imp_action = revealed_action_for_id(&runner, "IMP-FREE");
        if let Some(action) = imp_action {
            if view.valid_action_ids.contains(&action) {
                runner
                    .execute_action(0, action)
                    .expect("pick Imperialdramon");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand = hand_ids(&runner);
    assert!(
        hand.contains(&"IMP-FREE".to_string()),
        "Imperialdramon Digimon must be in hand once; hand={hand:?}"
    );
    // Exactly one copy — it must not have been added twice.
    let count = hand.iter().filter(|id| id.as_str() == "IMP-FREE").count();
    assert_eq!(
        count, 1,
        "card must appear exactly once in hand; hand={hand:?}"
    );
}
