//! EX9-058 Gazimon — Digimon, Lv.3, Purple, DP 1000, Cost 3.
//! Traits: Mammal, DM, Ver.5. Attribute: Virus.
//!
//! # Card text (code/digimon-engine/cards/ex9/EX9-058.json)
//!
//! Standard digivolve: Purple Lv.2 / cost 0.
//! Alt path (xros_req): "[Digivolve] Lv.2 w/[DM] trait: Cost 0"
//!
//! Effect:
//!   [On Play] Reveal the top 3 cards of your deck. Among them, add 1 card
//!   with the [DM] trait to the hand and place 1 card with the [Ver.5]
//!   trait face down as the bottom digivolution card of any of your Digimon
//!   with the [DM] trait. Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//!   <Retaliation> (When this Digimon is deleted after losing a battle,
//!   delete the Digimon it was battling.)
//!
//! Official Q&A (data/card_bundles/EX9-058.md):
//!   No, you can't. You must first choose a card to add to the hand, add
//!   it, then choose from the remaining revealed cards to place in
//!   digivolution cards. (I.e. the DM-to-hand pick and the Ver.5-to-source
//!   pick cannot be the same physical card — sequential consumption.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX9/Purple/EX9_058.cs
//!
//! DCGO shape: `SimplifiedRevealDeckTopCardsAndSelect(revealCount: 3, ...)`
//! with two `SimplifiedSelectCardConditionClass` entries — bucket 1
//! (`HadDMTrait`, mode `AddHand`, maxCount 1) and bucket 2 (`HasVer5Trait`,
//! mode `Custom`, maxCount 1, captures `selectedCard`). Both picks are
//! mandatory-if-a-candidate-exists (DCGO `canNoSelect: false` default) and
//! natural-fizzle if zero candidates match (no forced selection). AFTER
//! both picks resolve, if the Ver.5 pick succeeded AND
//! `HasMatchConditionOwnersPermanent(CanSelectPermanentCondition)` (a own
//! battle-area Digimon with the [DM] trait exists), a `SelectPermanentEffect`
//! opens (`canNoSelect: false`) and the Ver.5 card is placed face-down as
//! that chosen permanent's bottom digivolution card via
//! `AddDigivolutionCardsBottom(isFacedown: true)`. If no matching permanent
//! exists, the Ver.5 pick is simply never placed as a digivolution source.
//!
//! Zone-mechanics note: DCGO's "reveal" only flags library cards
//! `IsBeingRevealed = true` — it does not physically remove them from the
//! library; the actual zone-move happens only in the explicit AddHand /
//! AddDigivolutionCardsBottom / ReturnToLibraryBottom calls. If the Ver.5
//! pick's placement never fires (no valid [DM] target), DCGO's bookkeeping
//! list `remainingCards` excludes it too (it was already spliced into
//! `chosenCards`), so it receives no explicit zone-move at all in DCGO —
//! it stays wherever it physically already was in the library. This
//! engine's `reveal_top_deck` DOES pop cards into a distinct
//! `revealed_cards` pool, so an equivalent "no explicit move" outcome
//! would leave the card outside every zone, which the engine (correctly,
//! per the no-approximations policy — a card may never vanish from all
//! zones) does not allow: `place_remainder_on_deck` sweeps every card
//! still resident in `revealed_cards`, including a picked-but-never-routed
//! one, to the deck bottom. This is the faithful engine-side rendering of
//! the printed text's "Return the rest to the bottom of the deck" for this
//! edge case, and is additionally moot in real play — see the "Ver.5 pick
//! with no valid permanent" test below, which also notes the scenario is
//! unreachable via a genuine play of this specific card.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - A2 two-pass reveal (reveal, then route to hand or to bottom source)
//! - A6 trash-to-digi-stack-style placement (`place_as_bottom_source`,
//!   face-down variant — via explicit `select_reveal` + `select_own_permanent`
//!   + `place_as_bottom_source`, NOT `choose_from_reveal`'s `bottom_source_of`
//!   destination, because that destination hardcodes `face_down: false` and
//!   this card requires face-down placement)
//! - E1-adjacent: sequential mandatory picks, no player-facing "choose one"
//!   branch (both actions always happen when candidates exist)
//! - G2/digivolve: standard Purple Lv.2/cost 0 circle + alt-digivolve
//!   Lv.2 w/[DM] trait / cost 0
//! - G-select_own_permanent-then-bottom_source_of-of-CHOSEN-permanent: the
//!   untested variant flagged by the pre-flight audit brief — this is NOT
//!   BT25-078/P-167's `bottom_source_of: { target: this }` (self); it is
//!   `place_as_bottom_source: { target: <select_own_permanent bind_as> }`
//!   (an explicitly CHOSEN permanent, which may or may not be the carrier
//!   itself). Section 5 below is the explicit coverage for that binding
//!   path (a `select_own_permanent`-bound name flowing into
//!   `place_as_bottom_source`'s `target:` field).
//! - F5-adjacent (none — no OnDeletion clause)
//! - Inherited <Retaliation> keyword grant

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

fn gazimon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 must be present in the embedded DSL pack")
        .memory(10)
        .build()
}

fn compiled_on_play(runner: &DebugRunner) -> CompiledTriggeredClause {
    runner
        .compiled_card("EX9-058")
        .expect("EX9-058 compiled card must be available")
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => {
                Some(t.clone())
            }
            _ => None,
        })
        .expect("EX9-058 must have an [On Play] triggered clause")
}

/// Pull just the [On Play] clause's `process` steps, for clause-isolated
/// invocation via `run_steps` (mirrors BT25-078's `shared_process` idiom).
fn on_play_process(runner: &DebugRunner) -> Vec<CompiledStep> {
    compiled_on_play(runner).process
}

/// Push cards directly onto a player's deck TOP (last id in `ids` ends up
/// as the very top card), bypassing the builder's `.deck()` (which only
/// applies at `.start()`/`.build()` time). Used for clause-isolated tests
/// that build the runner via `.build()` and then manually seed the deck
/// before invoking the process closures directly.
fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

/// Build a [DM]-trait card eligible for the hand-add bucket.
fn make_dm_card(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.traits = vec!["DM".to_string()];
    card
}

/// Build a [Ver.5]-trait card eligible for the bottom-source bucket.
fn make_ver5_card(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.traits = vec!["Ver.5".to_string()];
    card
}

/// Build a card carrying BOTH [DM] and [Ver.5] traits, to exercise the
/// no-double-consumption edge (Official Q&A: must pick two DIFFERENT cards).
fn make_dm_ver5_card(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.traits = vec!["DM".to_string(), "Ver.5".to_string()];
    card
}

/// A [DM]-trait Digimon fixture to place on the battle area as a valid
/// bottom-source target.
fn make_dm_target(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.traits = vec!["DM".to_string()];
    card
}

// ═══════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex9_058_yaml_loads_and_compiles_from_embedded_pack() {
    let runner = gazimon_runner();
    let compiled = runner
        .compiled_card("EX9-058")
        .expect("EX9-058 compiled card must be available");

    assert_eq!(compiled.card, "EX9-058");
    assert_eq!(compiled.name, "Gazimon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert_eq!(compiled.cost, Some(3));
    assert!(compiled.color.contains(&CompiledColor::Purple));
    for t in ["Mammal", "DM", "Ver.5"] {
        assert!(
            compiled.traits.iter().any(|x| x == t),
            "EX9-058 must carry trait {t}; got {:?}",
            compiled.traits
        );
    }
}

#[test]
fn ex9_058_declares_standard_purple_lv2_cost_zero_circle() {
    let runner = gazimon_runner();
    let compiled = runner.compiled_card("EX9-058").expect("compiled");

    let standard = compiled.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .map(|f| f.level_eq == Some(2) && f.color_is == Some(CompiledColor::Purple))
                .unwrap_or(false)
    });
    assert!(
        standard.is_some(),
        "EX9-058 must declare the printed standard Purple Lv.2 / cost 0 circle"
    );
}

#[test]
fn ex9_058_declares_alt_digivolve_lv2_dm_trait_cost_zero() {
    let runner = gazimon_runner();
    let compiled = runner.compiled_card("EX9-058").expect("compiled");

    let dm_alt = compiled.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .map(|f| f.level_eq == Some(2) && f.trait_has.as_deref() == Some("DM"))
                .unwrap_or(false)
    });
    assert!(
        dm_alt.is_some(),
        "EX9-058 must declare '[Digivolve] Lv.2 w/[DM] trait: Cost 0'"
    );
}

#[test]
fn ex9_058_has_exactly_two_digivolve_alt_paths() {
    let runner = gazimon_runner();
    let compiled = runner.compiled_card("EX9-058").expect("compiled");
    let count = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(
        count, 2,
        "EX9-058 prints exactly the standard Purple circle + the DM-trait alt \
         (no other digivolve alt-paths); got {count}"
    );
}

#[test]
fn ex9_058_on_play_clause_is_mandatory_and_not_opt() {
    let runner = gazimon_runner();
    let clause = compiled_on_play(&runner);

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        !clause.optional,
        "Printed text has no 'you may' at the outer trigger — DCGO \
         SetUpActivateClass(..., isOptional: false)"
    );
    assert!(
        !clause.once_per_turn,
        "No [Once Per Turn] printed on the [On Play] clause"
    );
}

#[test]
fn ex9_058_on_play_process_reveals_three_and_places_remainder_at_bottom() {
    let runner = gazimon_runner();
    let clause = compiled_on_play(&runner);

    let has_reveal_three = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. }));
    assert!(has_reveal_three, "must reveal top 3 cards");

    let has_place_remainder = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlaceRemainderOnDeck { .. }));
    assert!(has_place_remainder, "rest returns to deck bottom");
}

#[test]
fn ex9_058_on_play_process_selects_reveal_twice_and_places_source_face_down() {
    let runner = gazimon_runner();
    let clause = compiled_on_play(&runner);

    let select_reveal_count = clause
        .process
        .iter()
        .filter(|s| matches!(s, CompiledStep::SelectReveal { .. }))
        .count();
    assert_eq!(
        select_reveal_count, 2,
        "two sequential reveal picks: [DM]-to-hand, then [Ver.5]-to-source"
    );

    let has_add_to_hand = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddToHandFromReveal { .. }));
    assert!(has_add_to_hand, "the DM pick routes to hand");

    let has_select_own_permanent = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectOwnPermanent { .. }));
    assert!(
        has_select_own_permanent,
        "must select which [DM] Digimon receives the bottom source"
    );

    let place_source = clause.process.iter().find_map(|s| match s {
        CompiledStep::PlaceAsBottomSource {
            face_down, target, ..
        } => Some((*face_down, target.clone())),
        _ => None,
    });
    let (face_down, target) =
        place_source.expect("must place the Ver.5 pick as a bottom digivolution source");
    assert!(face_down, "printed text requires FACE DOWN placement");
    assert!(
        !matches!(target, digimon_dsl::compiled::CompiledBindingRef::Source),
        "target must be the CHOSEN permanent (select_own_permanent binding), \
         not `this`/`self` — the printed text says 'any of your Digimon with \
         the [DM] trait', not 'this Digimon'"
    );
}

#[test]
fn ex9_058_has_inherited_retaliation() {
    let runner = gazimon_runner();
    let compiled = runner.compiled_card("EX9-058").expect("compiled");
    let retaliation = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Retaliation")
        )
    });
    assert!(retaliation, "EX9-058 must grant inherited <Retaliation>");
}

// ═══════════════════════════════════════════════════════════════════════
// Section 2 — Positive/negative condition gating: DM-to-hand bucket
// ═══════════════════════════════════════════════════════════════════════

/// Positive: a [DM]-trait card is among the top 3 → it must be offered
/// (and, being the only candidate, is a mandatory pick).
#[test]
fn ex9_058_dm_bucket_offers_dm_trait_candidate() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_dm_card("DM-A", "DM Card"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .deck(0, &["DM-A", "FILLER-1", "FILLER-2"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gazimon EX9-058");

    let view = runner
        .pending_selection_view()
        .expect("DM-bucket reveal-pick prompt installed");
    assert_eq!(view.kind, SelectionKind::Reveal);
    assert!(
        !runner.pending_is_optional(),
        "DM pick is mandatory when a candidate exists"
    );
}

/// Negative: no [DM]-trait card in the top 3 → the DM bucket naturally
/// fizzles (no forced pick), and the flow proceeds straight to the Ver.5
/// pick (or, if none there either, to bottoming the remainder).
#[test]
fn ex9_058_dm_bucket_fizzles_when_no_dm_trait_candidate() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .add_card(make_test_card("FILLER-3", "Filler 3"))
        .deck(0, &["FILLER-1", "FILLER-2", "FILLER-3"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gazimon EX9-058");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets and remainder placement");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    for id in ["FILLER-1", "FILLER-2", "FILLER-3"] {
        assert!(
            !hand_ids.contains(&id),
            "no DM-trait candidate → nothing added to hand; hand={hand_ids:?}"
        );
    }
    assert_eq!(
        runner.game.players[0].deck.len(),
        3,
        "all 3 revealed fillers return to deck bottom"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 3 — Positive/negative condition gating: Ver.5-to-source bucket
// ═══════════════════════════════════════════════════════════════════════

/// Positive: a [Ver.5]-trait card is among the top 3 AND a [DM] Digimon is
/// on the field → the Ver.5 pick installs after the DM pick resolves.
#[test]
fn ex9_058_ver5_bucket_offers_ver5_trait_candidate_when_dm_target_exists() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_ver5_card("V5-A", "Ver5 Card"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .add_card(make_dm_target("DM-TARGET", "DM Target"))
        .deck(0, &["V5-A", "FILLER-1", "FILLER-2"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    // A [DM] Digimon already on the field to serve as the placement target.
    runner.place_on_field(0, "DM-TARGET", Some(0));

    runner.play(0, 0).expect("play Gazimon EX9-058");
    // No DM-trait candidate among V5-A/FILLER-1/FILLER-2 → DM bucket
    // fizzles automatically; the next prompt is the Ver.5 pick.
    let view = runner
        .pending_selection_view()
        .expect("Ver.5-bucket reveal-pick prompt installed");
    assert_eq!(view.kind, SelectionKind::Reveal);
    assert!(
        !runner.pending_is_optional(),
        "Ver.5 pick is mandatory when a candidate exists"
    );
}

/// Negative: no [Ver.5]-trait card in the top 3 → the Ver.5 bucket
/// naturally fizzles even though a [DM] target Digimon exists on the field.
#[test]
fn ex9_058_ver5_bucket_fizzles_when_no_ver5_trait_candidate() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .add_card(make_test_card("FILLER-3", "Filler 3"))
        .add_card(make_dm_target("DM-TARGET", "DM Target"))
        .deck(0, &["FILLER-1", "FILLER-2", "FILLER-3"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    runner.place_on_field(0, "DM-TARGET", Some(0));
    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(0)
        .expect("DM target placed")
        .card_sources
        .len();

    runner.play(0, 0).expect("play Gazimon EX9-058");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets and remainder placement");

    let target_perm = runner
        .game
        .player(0)
        .battle_area
        .get(0)
        .expect("DM target still present");
    assert_eq!(
        target_perm.card_sources.len(),
        sources_before,
        "no Ver.5 candidate → nothing placed under the DM target"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral outcome: both buckets satisfied, placement chosen
// ═══════════════════════════════════════════════════════════════════════

/// Full happy path: reveal has a [DM] card and a distinct [Ver.5] card;
/// a [DM] Digimon is on the field. Drives the whole prompt chain:
/// DM pick -> Ver.5 pick -> select the [DM] target permanent -> placement.
#[test]
fn ex9_058_full_flow_adds_dm_to_hand_and_places_ver5_face_down_under_chosen_dm() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_dm_card("DM-A", "DM Card"))
        .add_card(make_ver5_card("V5-A", "Ver5 Card"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_dm_target("DM-TARGET", "DM Target"))
        .deck(0, &["DM-A", "V5-A", "FILLER-1"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    let target_handle = runner.place_on_field(0, "DM-TARGET", Some(0));
    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(target_handle.index as usize)
        .expect("DM target placed")
        .card_sources
        .len();

    runner.play(0, 0).expect("play Gazimon EX9-058");

    // Prompt 1: DM-to-hand pick.
    let view = runner
        .pending_selection_view()
        .expect("DM pick prompt installed");
    assert_eq!(view.kind, SelectionKind::Reveal);
    let dm_action = view.valid_action_ids[0];
    runner.execute_action(0, dm_action).expect("pick DM-A");

    // Prompt 2: Ver.5-to-source pick.
    let view2 = runner
        .pending_selection_view()
        .expect("Ver.5 pick prompt installed");
    assert_eq!(view2.kind, SelectionKind::Reveal);
    let v5_action = view2.valid_action_ids[0];
    runner.execute_action(0, v5_action).expect("pick V5-A");

    // Prompt 3: choose the [DM] Digimon to receive the bottom source.
    // The played Gazimon carries [DM] too, so it is ALSO a legal target
    // alongside DM-TARGET — assert both are offered, then pick DM-TARGET's
    // own-field action id explicitly.
    let view3 = runner
        .pending_selection_view()
        .expect("permanent-select prompt installed");
    assert_eq!(view3.kind, SelectionKind::OwnField);
    assert_eq!(
        view3.valid_action_ids.len(),
        2,
        "DM-TARGET and the played Gazimon (self-[DM]) are both legal; got {:?}",
        view3.valid_action_ids
    );
    let perm_action = digimon_engine::action::space::encode_attack(0, target_handle.index as u16);
    assert!(
        view3.valid_action_ids.contains(&perm_action),
        "DM-TARGET's own-field action id must be legal; valid_action_ids={:?}",
        view3.valid_action_ids
    );
    runner
        .execute_action(0, perm_action)
        .expect("choose DM-TARGET as the placement site");

    runner.auto_resolve().expect("resolve remainder placement");

    // DM-A landed in hand.
    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"DM-A"),
        "[DM] card must be added to hand; hand={hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"V5-A"),
        "[Ver.5] card must NOT be added to hand; hand={hand_ids:?}"
    );

    // V5-A is the new bottom digivolution source of DM-TARGET, face down.
    let target_perm = runner
        .game
        .player(0)
        .battle_area
        .get(target_handle.index as usize)
        .expect("DM target present");
    assert_eq!(
        target_perm.card_sources.len(),
        sources_before + 1,
        "exactly one card placed under DM-TARGET"
    );
    let bottom_source = &target_perm.card_sources[0];
    assert_eq!(
        bottom_source.card_id(&runner.game.card_data),
        "V5-A",
        "V5-A must be the BOTTOM digivolution source"
    );
    assert!(
        bottom_source.face_down,
        "printed text requires the placed card to be FACE DOWN"
    );

    // FILLER-1 returns to deck bottom (the only remaining unpicked card).
    let deck_ids: Vec<&str> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        deck_ids.contains(&"FILLER-1"),
        "unpicked filler returns to the deck; deck={deck_ids:?}"
    );
}

/// Ver.5 pick succeeds but NO [DM] Digimon exists on the field — DCGO's
/// `HasMatchConditionOwnersPermanent` gate fails, so the permanent-select
/// prompt never installs and the picked Ver.5 card is simply never placed
/// as a digivolution source. Per the engine's `place_as_bottom_source`
/// contract, an unresolved `target` binding makes the placement step a
/// silent no-op — the picked card is left exactly where `select_reveal`
/// found it: still in `ctx.game.revealed_cards`. It is therefore correctly
/// swept up by `place_remainder_on_deck` at the end of the clause, exactly
/// like every other un-routed revealed card — matching the printed text's
/// "Return the rest to the bottom of the deck" literally (a card whose
/// placement fizzled for lack of a valid target IS "the rest"; the engine
/// never leaves a picked-but-unrouted card floating outside every zone).
///
/// NOTE: this scenario cannot be reached by literally *playing* EX9-058
/// from hand, because EX9-058 itself carries the [DM] trait — once played,
/// EX9-058's own battle-area permanent always satisfies
/// `CanSelectPermanentCondition`, so a normal play always has >=1 legal
/// target (itself). To exercise the true "zero [DM] permanents anywhere on
/// the field" branch, this test invokes the compiled [On Play] process
/// directly (clause-isolated, per RUST_DSL_TEST_API.md §7) against a
/// carrier permanent that does NOT carry [DM] — proving the gate is a real
/// board-state check, not a tautology that always passes because the
/// source card carries the trait itself.
#[test]
fn ex9_058_ver5_pick_with_no_dm_target_never_installs_permanent_prompt() {
    let holder = make_test_card("EX9058-NODM-HOLDER", "Non-DM Holder");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_ver5_card("V5-A", "Ver5 Card"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .add_card(holder)
        .build();

    // A battle-area permanent that does NOT carry [DM] — the only
    // permanent on the field, so `select_own_permanent`'s filter has zero
    // candidates.
    let holder_perm = runner.place_on_field(0, "EX9058-NODM-HOLDER", None);
    let src = runner.top_card(holder_perm);
    stack_deck_top(&mut runner, &["FILLER-2", "FILLER-1", "V5-A"]);

    let process = on_play_process(&runner);
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, Some(holder_perm), 0);
        run_steps(&process, &mut ctx, &mut Bindings::new());
    }

    // DM bucket fizzles (no DM candidate among V5-A/FILLER-1/FILLER-2);
    // Ver.5 prompt installs directly.
    let view = runner
        .pending_selection_view()
        .expect("Ver.5 pick prompt installed");
    assert_eq!(view.kind, SelectionKind::Reveal);
    let v5_action = view.valid_action_ids[0];
    runner.execute_action(0, v5_action).expect("pick V5-A");

    // No [DM] Digimon on the field -> select_own_permanent has zero
    // candidates -> silent no-op, no PendingSelection installs for it.
    // The very next prompt (if any) must NOT be an OwnField permanent pick.
    if let Some(sel) = runner.pending_selection() {
        assert_ne!(
            sel.kind,
            SelectionKind::OwnField,
            "no [DM] Digimon exists on field -> the permanent-select prompt \
             must not install (DCGO HasMatchConditionOwnersPermanent gate)"
        );
    }
    runner.auto_resolve().expect("resolve remainder placement");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        !hand_ids.contains(&"V5-A"),
        "V5-A must not land in hand; hand={hand_ids:?}"
    );

    let deck_ids: Vec<&str> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        deck_ids.contains(&"V5-A"),
        "V5-A was picked but never routed anywhere (no valid [DM] target) \
         -> it is swept into the deck-bottom remainder along with every \
         other un-routed revealed card; deck={deck_ids:?}"
    );

    // The non-DM holder must remain untouched (no card placed under it).
    let holder_after = runner
        .game
        .player(0)
        .battle_area
        .get(holder_perm.index as usize)
        .expect("holder still present");
    assert_eq!(
        holder_after.card_sources.len(),
        1,
        "no card is placed under the non-DM holder"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 5 — select_own_permanent -> place_as_bottom_source(target=CHOSEN)
// ═══════════════════════════════════════════════════════════════════════

/// Explicit coverage for the pre-flight-flagged "untested variant": the
/// `select_own_permanent`-bound permanent (NOT `this`/`self`) must be the
/// one that actually receives the placed card. Because EX9-058 itself
/// carries the [DM] trait, the played Gazimon permanent is ALSO a legal
/// target alongside DM-CHOSEN (two legal targets total) — this test picks
/// the NON-carrier target explicitly, so an unconditional "place under the
/// carrier / source permanent" bug would be observably wrong (it would
/// place under Gazimon instead of DM-CHOSEN).
#[test]
fn ex9_058_places_ver5_under_the_selected_permanent_not_a_fixed_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_ver5_card("V5-A", "Ver5 Card"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .add_card(make_dm_target("DM-CHOSEN", "DM Chosen"))
        .deck(0, &["V5-A", "FILLER-1", "FILLER-2"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    // A second [DM] Digimon target on the field, distinct from the Gazimon
    // carrier that will be played from hand.
    let chosen_handle = runner.place_on_field(0, "DM-CHOSEN", Some(0));

    let gazimon_handle = runner
        .play(0, 0)
        .map(|idx| digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: idx as u8,
        })
        .expect("play Gazimon EX9-058");

    let view = runner
        .pending_selection_view()
        .expect("Ver.5 pick prompt installed");
    let v5_action = view.valid_action_ids[0];
    runner.execute_action(0, v5_action).expect("pick V5-A");

    let perm_view = runner
        .pending_selection_view()
        .expect("permanent-select prompt installed for the Ver.5 placement");
    assert_eq!(perm_view.kind, SelectionKind::OwnField);
    // EX9-058 itself carries [DM] too, so both it and DM-CHOSEN are legal.
    assert_eq!(
        perm_view.valid_action_ids.len(),
        2,
        "both the played Gazimon (self-[DM]) and DM-CHOSEN are legal targets; \
         got {:?}",
        perm_view.valid_action_ids
    );

    // Resolve the action id that targets DM-CHOSEN specifically (not the
    // Gazimon carrier) via the OwnField encoding convention
    // (`encode_attack(0, battle_area_index)`, per `install_field_selection`).
    let chosen_action = digimon_engine::action::space::encode_attack(0, chosen_handle.index as u16);
    assert!(
        perm_view.valid_action_ids.contains(&chosen_action),
        "DM-CHOSEN's own-field action id must be legal; valid_action_ids={:?}",
        perm_view.valid_action_ids
    );
    runner
        .execute_action(0, chosen_action)
        .expect("choose DM-CHOSEN explicitly");
    runner.auto_resolve().expect("resolve remainder");

    let chosen_perm = runner
        .game
        .player(0)
        .battle_area
        .get(chosen_handle.index as usize)
        .expect("DM-CHOSEN present");
    let source_ids: Vec<&str> = chosen_perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        source_ids.contains(&"V5-A"),
        "V5-A must be placed under the SELECTED permanent (DM-CHOSEN); \
         sources={source_ids:?}"
    );

    // The Gazimon carrier itself must NOT have received the card.
    let gazimon_perm = runner
        .game
        .player(0)
        .battle_area
        .get(gazimon_handle.index as usize)
        .expect("Gazimon present");
    let gazimon_source_ids: Vec<&str> = gazimon_perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        !gazimon_source_ids.contains(&"V5-A"),
        "V5-A must NOT be placed under the carrier when DM-CHOSEN was \
         explicitly selected; gazimon sources={gazimon_source_ids:?}"
    );
}

/// When TWO extra [DM] Digimon are on the field (in addition to the played
/// Gazimon carrier, which also carries [DM]), all three must be offered as
/// legal targets — the choice is a genuine player decision (no
/// auto-selection), and picking a non-first, non-carrier target must place
/// the card under THAT permanent specifically.
#[test]
fn ex9_058_offers_all_eligible_dm_targets_and_places_under_the_one_chosen() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_ver5_card("V5-A", "Ver5 Card"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .add_card(make_dm_target("DM-FIRST", "DM First"))
        .add_card(make_dm_target("DM-SECOND", "DM Second"))
        .deck(0, &["V5-A", "FILLER-1", "FILLER-2"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    let first_handle = runner.place_on_field(0, "DM-FIRST", Some(0));
    let second_handle = runner.place_on_field(0, "DM-SECOND", Some(0));

    runner.play(0, 0).expect("play Gazimon EX9-058");

    let view = runner
        .pending_selection_view()
        .expect("Ver.5 pick prompt installed");
    let v5_action = view.valid_action_ids[0];
    runner.execute_action(0, v5_action).expect("pick V5-A");

    let perm_view = runner
        .pending_selection_view()
        .expect("permanent-select prompt installed");
    assert_eq!(perm_view.kind, SelectionKind::OwnField);
    // Gazimon (self-[DM]) + DM-FIRST + DM-SECOND = 3 legal targets.
    assert_eq!(
        perm_view.valid_action_ids.len(),
        3,
        "the played Gazimon plus both extra [DM] Digimon must all be \
         offered as legal targets; got {:?}",
        perm_view.valid_action_ids
    );

    // Choose DM-SECOND explicitly via its own-field action id — proves the
    // placement follows the actual selection, not a fixed first/carrier
    // default.
    let second_action = digimon_engine::action::space::encode_attack(0, second_handle.index as u16);
    assert!(
        perm_view.valid_action_ids.contains(&second_action),
        "DM-SECOND's own-field action id must be legal; valid_action_ids={:?}",
        perm_view.valid_action_ids
    );
    runner
        .execute_action(0, second_action)
        .expect("choose DM-SECOND explicitly");
    runner.auto_resolve().expect("resolve remainder");

    let first_perm = runner
        .game
        .player(0)
        .battle_area
        .get(first_handle.index as usize)
        .expect("DM-FIRST present");
    let second_perm = runner
        .game
        .player(0)
        .battle_area
        .get(second_handle.index as usize)
        .expect("DM-SECOND present");

    let first_has_v5 = first_perm
        .card_sources
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "V5-A");
    let second_has_v5 = second_perm
        .card_sources
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "V5-A");

    assert!(
        !first_has_v5,
        "V5-A must NOT be placed under DM-FIRST (it was not selected)"
    );
    assert!(
        second_has_v5,
        "V5-A must be placed under DM-SECOND (the explicitly selected target)"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 6 — Official Q&A: DM-hand pick and Ver.5-source pick cannot be
//   the same physical card (sequential no-duplicate-consumption)
// ═══════════════════════════════════════════════════════════════════════

/// A single card carries BOTH [DM] and [Ver.5]. Per the official Q&A,
/// once it is chosen for the hand-add, it is removed from the reveal
/// pool and can no longer be offered for the bottom-source pick.
#[test]
fn ex9_058_dual_trait_card_consumed_by_dm_pick_is_unavailable_for_ver5_pick() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_dm_ver5_card("DUAL", "Dual Trait"))
        .add_card(make_ver5_card("V5-ONLY", "Ver5 Only"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_dm_target("DM-TARGET", "DM Target"))
        .deck(0, &["DUAL", "V5-ONLY", "FILLER-1"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    runner.place_on_field(0, "DM-TARGET", Some(0));
    runner.play(0, 0).expect("play Gazimon EX9-058");

    // DM bucket: DUAL is the only [DM]-trait candidate.
    let view = runner
        .pending_selection_view()
        .expect("DM pick prompt installed");
    assert_eq!(view.kind, SelectionKind::Reveal);
    // DUAL must be a legal (in fact the only) DM-bucket candidate.
    let dual_action = view.valid_action_ids[0];
    runner
        .execute_action(0, dual_action)
        .expect("pick DUAL for hand");

    // Ver.5 bucket must now offer ONLY V5-ONLY, not DUAL again.
    let view2 = runner
        .pending_selection_view()
        .expect("Ver.5 pick prompt installed");
    assert_eq!(view2.kind, SelectionKind::Reveal);
    assert_eq!(
        view2.valid_action_ids.len(),
        1,
        "only V5-ONLY remains eligible for the Ver.5 bucket \
         (DUAL was already consumed by the DM-to-hand pick); \
         valid_action_ids={:?}",
        view2.valid_action_ids
    );

    let v5_action = view2.valid_action_ids[0];
    runner.execute_action(0, v5_action).expect("pick V5-ONLY");
    runner
        .auto_resolve()
        .expect("resolve permanent select + remainder");

    let hand_ids: Vec<&str> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(
        hand_ids.contains(&"DUAL"),
        "DUAL added to hand; hand={hand_ids:?}"
    );
    assert!(
        !hand_ids.contains(&"V5-ONLY"),
        "V5-ONLY must not be added to hand; hand={hand_ids:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 7 — Inherited <Retaliation> visible under a carrier
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex9_058_inherited_retaliation_is_visible_under_a_carrier() {
    let mut carrier = make_test_card("EX9058-CARRIER", "CarrierLv4");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 must be present in the embedded DSL pack")
        .add_card(carrier)
        .memory(10)
        .build();

    let carrier_handle = runner.place_stack(0, &["EX9-058", "EX9058-CARRIER"]);

    assert!(
        runner
            .game
            .has_keyword(carrier_handle, Keyword::Retaliation),
        "a carrier with EX9-058 as a digivolution source must inherit Retaliation"
    );
}

#[test]
fn ex9_058_inherited_retaliation_is_absent_without_source() {
    let mut carrier = make_test_card("EX9058-CARRIER2", "CarrierLv4b");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 must be present in the embedded DSL pack")
        .add_card(carrier)
        .memory(10)
        .build();

    let carrier_handle = runner.place_on_field(0, "EX9058-CARRIER2", Some(0));

    assert!(
        !runner
            .game
            .has_keyword(carrier_handle, Keyword::Retaliation),
        "a carrier without EX9-058 in sources must not gain Retaliation"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Section 8 — PASS legality: no auto-decline surfaced on mandatory picks
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ex9_058_dm_pick_does_not_expose_pass_when_a_candidate_exists() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-058")
        .expect("EX9-058 loads")
        .add_card(make_dm_card("DM-A", "DM Card"))
        .add_card(make_test_card("FILLER-1", "Filler 1"))
        .add_card(make_test_card("FILLER-2", "Filler 2"))
        .deck(0, &["DM-A", "FILLER-1", "FILLER-2"])
        .hand(0, &["EX9-058"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Gazimon EX9-058");

    let view = runner
        .pending_selection_view()
        .expect("DM pick prompt installed");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must NOT be legal on the DM pick when a candidate exists \
         (printed 'add 1 card', no 'may'); valid_action_ids={:?}",
        view.valid_action_ids
    );
}
