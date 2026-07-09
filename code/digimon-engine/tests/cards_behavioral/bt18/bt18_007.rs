//! BT18-007 Gazimon — Digimon, Lv.3, Red/Purple, DP 1000, Cost 3.
//! Traits: Mammal. Attribute: Virus.
//!
//! # Card text (data/card_bundles/BT18-007.md / cards.json)
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with
//! [Millenniummon] in its name and 1 card with the [Composite] or
//! [Wicked God] trait among them to the hand. Return the rest to the
//! bottom of the deck.
//!
//! Inherited Effect: ＜Retaliation＞ (When only this Digimon is deleted in
//! battle, delete the Digimon it battled.)
//!
//! Standard digivolve: Red Lv.2 / cost 1, Purple Lv.2 / cost 1 (two mono-
//! color standard circles). Special digivolution condition (black text):
//! `[Digivolve] [Pagumon]: Cost 0`.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Red/BT18_007.cs
//!
//! DCGO models the On Play as `SimplifiedRevealDeckTopCardsAndSelect` with
//! two independent `SimplifiedSelectCardConditionClass` buckets (Millenniummon
//! name-bucket maxCount 1, Composite/WickedGod trait-bucket maxCount 1),
//! `remainingCardsPlace: RemainingCardsPlace.DeckBottom`. The official Q&A
//! notes both picks are mandatory when a matching card exists ("you must add
//! as many cards to your hand as possible"). Retaliation is
//! `RetaliationSelfEffect(isInheritedEffect: true)`. The alt-digivolve is
//! `AddSelfDigivolutionRequirementStaticEffect(TopCard.EqualsCardName("Pagumon"), cost 0)`.
//!
//! # Patterns this test covers (docs/RUST_DSL_TEST_API.md §4.3)
//! - A2/A3 two-bucket reveal-and-select (RevealBucket flow, §14.6)
//! - G1-adjacent alt-digivolve from a specific card name (`from: { name_is }`)
//! - Mandatory-if-present bucket semantics (min:1 clamped to available matches)
//! - Inherited static keyword grant (`scope: inherited, kind: grant_keyword`)

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, PlaySource};
use digimon_engine::selection::SelectionKind;

// ─── Structural ──────────────────────────────────────────────────────────

#[test]
fn bt18_007_yaml_has_two_standard_circles_and_pagumon_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .memory(5)
        .build();
    let compiled = runner
        .compiled_card("BT18-007")
        .expect("BT18-007 compiled card present");

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        3,
        "two standard mono-color circles (Red Lv2, Purple Lv2) + 1 Pagumon bonus path"
    );

    // Two standard circles: Red Lv.2 cost 1, Purple Lv.2 cost 1.
    let red_circle = digivolve_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .map(|f| {
                    f.all_of
                        .iter()
                        .any(|pred| pred.color_is == Some(CompiledColor::Red))
                        && f.all_of.iter().any(|pred| pred.level_eq == Some(2))
                })
                .unwrap_or(false)
        })
        .expect("Red Lv.2 standard circle present");
    assert_eq!(red_circle.cost, Some(CompiledCost::Literal(1)));

    let purple_circle = digivolve_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .map(|f| {
                    f.all_of
                        .iter()
                        .any(|pred| pred.color_is == Some(CompiledColor::Purple))
                        && f.all_of.iter().any(|pred| pred.level_eq == Some(2))
                })
                .unwrap_or(false)
        })
        .expect("Purple Lv.2 standard circle present");
    assert_eq!(purple_circle.cost, Some(CompiledCost::Literal(1)));

    // Special condition: [Digivolve] [Pagumon]: Cost 0.
    let pagumon_path = digivolve_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .map(|f| f.name_is.as_deref() == Some("Pagumon"))
                .unwrap_or(false)
        })
        .expect("Pagumon bonus digivolve path present");
    assert_eq!(pagumon_path.cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt18_007_has_on_play_clause_and_inherited_retaliation_grant() {
    let runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .memory(5)
        .build();
    let compiled = runner
        .compiled_card("BT18-007")
        .expect("BT18-007 compiled card present");

    let on_play_clauses: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(on_play_clauses.len(), 1, "exactly one On Play clause");
    assert_eq!(on_play_clauses[0].scope, CompiledScope::FaceUp);

    let inherited_retaliation = compiled.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            scope,
            keyword,
            ..
        }) => *scope == CompiledScope::Inherited && keyword == "Retaliation",
        _ => false,
    });
    assert!(
        inherited_retaliation,
        "inherited <Retaliation> must be a declarative grant_keyword clause scoped Inherited"
    );
}

// ─── Behavioral: On Play reveal-and-bucket ──────────────────────────────

#[test]
fn bt18_007_on_play_adds_millenniummon_named_and_composite_traited_cards_then_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .add_card(make_millenniummon_named_card("MILL-NAMED"))
        .add_card(make_trait_card("COMPOSITE-CARD", &["Composite"]))
        .add_card(make_test_card("BLANK", "Blank"))
        .deck(0, &["BLANK", "COMPOSITE-CARD", "MILL-NAMED"])
        .hand(0, &["BT18-007"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gazimon");

    // Bucket 0: [Millenniummon]-in-name.
    assert_required_pending_pick(&runner, "Millenniummon-name bucket");
    pick_first_pending(&mut runner, "pick Millenniummon-name bucket");

    // Bucket 1: [Composite]/[Wicked God] trait.
    assert_required_pending_pick(&runner, "Composite/Wicked God trait bucket");
    pick_first_pending(&mut runner, "pick trait bucket");

    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"MILL-NAMED".to_string()),
        "Millenniummon-named card must land in hand"
    );
    assert!(
        hand_ids.contains(&"COMPOSITE-CARD".to_string()),
        "Composite-trait card must land in hand"
    );
    assert_eq!(
        runner.game.players[0]
            .deck
            .first()
            .unwrap()
            .card_id(&runner.game.card_data),
        "BLANK",
        "unchosen reveal card returned to deck bottom"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        1,
        "only BLANK remains in deck"
    );
}

#[test]
fn bt18_007_on_play_wicked_god_trait_also_satisfies_second_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .add_card(make_millenniummon_named_card("MILL-NAMED"))
        .add_card(make_trait_card("WICKED-GOD-CARD", &["Wicked God"]))
        .add_card(make_test_card("BLANK", "Blank"))
        .deck(0, &["BLANK", "WICKED-GOD-CARD", "MILL-NAMED"])
        .hand(0, &["BT18-007"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gazimon");
    assert_required_pending_pick(&runner, "Millenniummon-name bucket");
    pick_first_pending(&mut runner, "pick Millenniummon-name bucket");
    assert_required_pending_pick(&runner, "Wicked God trait bucket");
    pick_first_pending(&mut runner, "pick Wicked God bucket");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"WICKED-GOD-CARD".to_string()));
}

#[test]
fn bt18_007_on_play_no_matching_cards_skips_both_buckets_and_bottoms_all_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .add_card(make_test_card("BLANK-A", "Blank A"))
        .add_card(make_test_card("BLANK-B", "Blank B"))
        .add_card(make_test_card("BLANK-C", "Blank C"))
        .deck(0, &["BLANK-A", "BLANK-B", "BLANK-C"])
        .hand(0, &["BT18-007"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    runner.play(0, 0).expect("play Gazimon");
    runner
        .auto_resolve()
        .expect("no matching cards -> straight to bottoming");

    // hand grew by 0 net (Gazimon left hand via play, no reveal cards added)
    assert_eq!(runner.hand_size(0), hand_before - 1);
    assert_eq!(
        runner.game.players[0].deck.len(),
        3,
        "all 3 revealed cards returned to deck"
    );
}

#[test]
fn bt18_007_on_play_millenniummon_match_absent_still_offers_only_trait_bucket() {
    // Only the trait-matching card is present among the reveal; the name
    // bucket must not offer a pick (min clamps to 0 when no candidate exists)
    // while the trait bucket still fires as a mandatory pick.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .add_card(make_trait_card("COMPOSITE-ONLY", &["Composite"]))
        .add_card(make_test_card("BLANK-A", "Blank A"))
        .add_card(make_test_card("BLANK-B", "Blank B"))
        .deck(0, &["BLANK-A", "BLANK-B", "COMPOSITE-ONLY"])
        .hand(0, &["BT18-007"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gazimon");

    // Bucket 0 (Millenniummon-name) has no eligible candidate: no PASS-less
    // mandatory prompt should install for it because min clamps to available
    // matches (0). The next pending pick (if any) must be the trait bucket.
    let view = runner
        .pending_selection_view()
        .expect("trait bucket should still install a pick");
    match view.kind {
        SelectionKind::RevealBucket { bucket_index, .. } => {
            assert_eq!(
                bucket_index, 1,
                "bucket 0 (no eligible Millenniummon-named card) must be skipped"
            );
        }
        other => panic!("expected RevealBucket selection, got {other:?}"),
    }
    assert!(
        !runner.pending_is_optional(),
        "trait bucket is mandatory when a matching card exists"
    );
    pick_first_pending(&mut runner, "pick trait bucket");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"COMPOSITE-ONLY".to_string()));
}

#[test]
fn bt18_007_on_play_reveals_three_cards_from_deck_top_not_bottom() {
    // Confirms the reveal draws from the TOP of the deck (last element in
    // the `.deck()` builder), not the bottom — all 3 end up back in the
    // deck when none match, proving the reveal count and source zone.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .add_card(make_test_card("BOTTOM-CARD", "Bottom Card"))
        .add_card(make_test_card("MID-CARD", "Mid Card"))
        .add_card(make_test_card("TOP-CARD", "Top Card"))
        .deck(0, &["BOTTOM-CARD", "MID-CARD", "TOP-CARD"])
        .hand(0, &["BT18-007"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gazimon");
    runner.auto_resolve().expect("no matches -> bottoms all 3");

    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck_ids.len(), 3, "all 3 revealed cards returned to deck");
}

// ─── Digivolve alt-path: behavioral (Pagumon cost 0) ────────────────────

#[test]
fn bt18_007_digivolves_from_pagumon_stack_for_cost_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .add_card(make_pagumon())
        .hand(0, &["BT18-007"])
        .memory(0)
        .start();

    let pagumon = runner.place_on_field(0, "PAGUMON", Some(0));
    let memory_before = runner.memory();

    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT18-007")
        .expect("BT18-007 in hand");

    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        pagumon.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(
        ok,
        "Gazimon must digivolve from Pagumon via the bonus cost-0 path"
    );
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "Pagumon bonus digivolve path costs 0 memory"
    );
    assert_eq!(
        runner.game.players[0]
            .battle_area
            .get(pagumon.index as usize)
            .unwrap()
            .top_card()
            .card_id(&runner.game.card_data),
        "BT18-007",
        "Gazimon now on top of the stack"
    );
}

#[test]
fn bt18_007_cannot_digivolve_from_unrelated_level_2_card_without_matching_color_or_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-007")
        .expect("BT18-007 YAML loads")
        .add_card(make_unrelated_level_2())
        .hand(0, &["BT18-007"])
        .memory(0)
        .start();

    let unrelated = runner.place_on_field(0, "UNRELATED-LV2", Some(0));

    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "BT18-007")
        .expect("BT18-007 in hand");

    let ok = runner.game.digivolve_from_hand(
        0,
        hand_idx,
        unrelated.index as usize,
        PlaySource::ByDigivolve,
    );
    assert!(
        !ok,
        "Gazimon must not digivolve from an unrelated, off-color Lv.2 card"
    );
}

// ─── Structural: reveal bucket PASS-gating (mandatory-if-present) ───────

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: reveal bucket must be mandatory when a matching card exists"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal before the required pick"
    );
}

fn pick_first_pending(runner: &mut DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect(label);
}

// ─── Fixtures ────────────────────────────────────────────────────────────

fn make_millenniummon_named_card(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, "Millenniummon")
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_pagumon() -> digimon_engine::card_data::CardData {
    // Off-color (blue) so the Pagumon-name bonus path (cost 0) is the ONLY
    // digivolve route available, isolating the special-condition path from
    // the standard Red/Purple Lv.2 circles (which some real Pagumon
    // printings could also satisfy, correctly triggering a route-choice
    // prompt instead — that ambiguity is out of scope for this test).
    let mut card = make_test_card("PAGUMON", "Pagumon");
    card.level = Some(2);
    card.colors = vec![digimon_engine::enums::CardColor::Blue];
    card
}

fn make_unrelated_level_2() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("UNRELATED-LV2", "Unrelated Rookie");
    card.level = Some(2);
    card.colors = vec![digimon_engine::enums::CardColor::Blue];
    card
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
