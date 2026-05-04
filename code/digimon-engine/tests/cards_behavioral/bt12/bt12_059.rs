//! BT12-059 Agumon — Digimon, Lv.3, Black/Red, DP 2000, Cost 3.
//! Traits: Reptile.
//! Evo: Lv2 Black / Cost 1; alt-path Koromon (any color/level) / Cost 0.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [On Play] Reveal the top 4 cards of your deck. Add 1 Digimon card with
//! [Greymon] or [Omnimon] in its name and 1 Tamer card with [Tai Kamiya]
//! in its name among them to your hand. Place the remaining cards at the
//! bottom of your deck in any order.
//!
//! Inherited Effect:
//! [All Turns] While this Digimon has [Greymon] or [Omnimon] in its name,
//! it gets +1000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Black/BT12_059.cs
//!
//! # Patterns this test covers
//! - A1/A3 — On-play reveal-N + multi-bucket selection (Greymon/Omnimon Digimon
//!   bucket and Tai Kamiya Tamer bucket) using `select_reveal_buckets`.
//! - D4 — Conditional inherited DP buff (carrier-name gated aura) via
//!   `scope: inherited`, `kind: aura`, `target: {}`, `dp_modifier: 1000`,
//!   gated by `self_digivolution_contains_name` (any_of Greymon / Omnimon).
//!
//! # Faithfulness notes (DCGO comparison)
//! - DCGO `CanSelectCardCondition`: Digimon AND (HasGreymonName OR
//!   ContainsCardName("Omnimon")). YAML: `kind: digimon` AND any_of
//!   name_contains Greymon/Omnimon.
//! - DCGO `CanSelectCardCondition1`: Tamer AND ContainsCardName("Tai Kamiya").
//!   YAML: `kind: tamer` AND name_contains "Tai Kamiya".
//! - DCGO `mutualConditions: true` and `RemainingCardsPlace.DeckBottom` —
//!   `select_reveal_buckets` with `no_duplicate_cards: true` plus a trailing
//!   `place_remainder_on_deck { position: bottom }` step matches.
//! - Aura: DCGO `Condition` checks `card.PermanentOfThisCard().TopCard.HasGreymonName
//!   || ContainsCardName("Omnimon")`. The closest DSL primitive is
//!   `self_digivolution_contains_name` which scans the carrier's full source
//!   stack. Because BT12-059 itself is "Agumon" (no Greymon/Omnimon), the
//!   only way the condition fires is if a Greymon-named or Omnimon-named card
//!   is stacked on top — exactly matching "TopCard.HasGreymonName" in
//!   practice for the Lv3 Agumon source slot.

use digimon_dsl::compiled::{
    CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT12-059 YAML must parse and compile cleanly.
#[test]
fn bt12_059_yaml_parses_and_compiles() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/bt12/BT12-059.yaml"))
        .expect("BT12-059 YAML parses");
    let _compiled = compile(&spec).expect("BT12-059 YAML compiles");
}

/// BT12-059 standard digivolution is Lv2 Black / Cost 1 (printed evo_costs).
/// The Koromon-name alt-path should be present at cost 0.
#[test]
fn bt12_059_has_koromon_named_alt_digivolve_path_at_cost_zero() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/bt12/BT12-059.yaml"))
        .expect("BT12-059 YAML parses");
    let compiled = compile(&spec).expect("BT12-059 YAML compiles");
    let koromon_alt = compiled.alt_paths.iter().any(|path| {
        let Some(from) = path.from.as_ref() else {
            return false;
        };
        let name_match = from.name_contains.as_deref() == Some("Koromon")
            || from
                .all_of
                .iter()
                .any(|p| p.name_contains.as_deref() == Some("Koromon"));
        name_match && path.cost == Some(CompiledCost::Literal(0))
    });
    assert!(
        koromon_alt,
        "BT12-059 must declare the Koromon-named alt-digivolve path at cost 0 \
         (DCGO AddSelfDigivolutionRequirementStaticEffect)"
    );
}

/// BT12-059 declares exactly one `OnPlay` triggered clause and exactly one
/// inherited Aura declarative clause.
#[test]
fn bt12_059_has_one_on_play_triggered_and_one_inherited_aura() {
    let runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 in embedded DSL pack")
        .memory(0)
        .start();
    let compiled = runner
        .compiled_card("BT12-059")
        .expect("BT12-059 compiled");

    let triggered: Vec<&CompiledTriggeredClause> = compiled
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
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnPlay]);
    assert_eq!(triggered[0].scope, CompiledScope::FaceUp);

    let auras: Vec<(_, _)> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                ..
            }) => Some((*scope, *dp_modifier)),
            _ => None,
        })
        .collect();
    assert_eq!(auras.len(), 1, "exactly one Aura declarative clause");
    assert_eq!(auras[0].0, CompiledScope::Inherited);
    assert_eq!(auras[0].1, Some(1000));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — On-play behavioral: reveal 4, two buckets, bottom remainder
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: build a Digimon test card with a custom name.
fn make_named_digimon(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card
}

/// Helper: build a Tamer test card with a custom name.
fn make_named_tamer(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn zone_ids(
    cards: &[CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
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

/// Positive path — both buckets satisfied. Reveal 4 = [GREYMON, TAI, FILLER1,
/// FILLER2]. Pick GREYMON for the Digimon bucket and TAI for the Tamer
/// bucket. The two FILLER cards go to deck bottom.
#[test]
fn bt12_059_on_play_adds_one_greymon_digimon_and_one_tai_tamer_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_named_tamer("TAI", "Tai Kamiya"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["GREYMON", "TAI", "FILLER1", "FILLER2"])
        .hand(0, &["BT12-059"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-059");

    // First bucket: a Digimon with Greymon/Omnimon in name.
    assert_required_pending_pick(&runner, "Greymon/Omnimon Digimon bucket");
    pick_revealed_by_id(&mut runner, "GREYMON", "pick Greymon");

    // Second bucket: a Tamer with Tai Kamiya in name.
    assert_required_pending_pick(&runner, "Tai Kamiya Tamer bucket");
    pick_revealed_by_id(&mut runner, "TAI", "pick Tai Kamiya");

    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GREYMON".to_string()),
        "Greymon Digimon must land in hand"
    );
    assert!(
        hand_ids.contains(&"TAI".to_string()),
        "Tai Kamiya Tamer must land in hand"
    );
    // Two filler cards should be at the bottom of the deck.
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let last_two: Vec<String> = deck_ids.iter().rev().take(2).cloned().collect();
    assert!(
        last_two.contains(&"FILLER1".to_string()) && last_two.contains(&"FILLER2".to_string()),
        "unchosen reveal cards return to deck bottom; deck end was {last_two:?}"
    );
}

/// Greymon-only branch — reveal 4 contains a Greymon Digimon but NO Tai Kamiya
/// Tamer. Only the Digimon bucket adds; the Tamer bucket has zero candidates
/// and the engine falls through to bottom-placement.
#[test]
fn bt12_059_on_play_only_greymon_found_skips_tamer_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_named_digimon("GREYMON", "Greymon"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .deck(0, &["GREYMON", "FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["BT12-059"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-059");
    pick_revealed_by_id(&mut runner, "GREYMON", "pick Greymon");
    runner
        .auto_resolve()
        .expect("resolve through empty Tamer bucket");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"GREYMON".to_string()),
        "Greymon must be added to hand"
    );
    assert!(
        !hand_ids.iter().any(|id| id == "FILLER1" || id == "FILLER2" || id == "FILLER3"),
        "no Tamer was found, no extra Tamer add"
    );
}

/// Tai-only branch — reveal 4 contains a Tai Kamiya Tamer but NO matching
/// Digimon. The Digimon bucket has zero candidates and is skipped; the Tamer
/// bucket fires.
#[test]
fn bt12_059_on_play_only_tai_kamiya_found_skips_digimon_bucket() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_named_tamer("TAI", "Tai Kamiya"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .deck(0, &["TAI", "FILLER1", "FILLER2", "FILLER3"])
        .hand(0, &["BT12-059"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-059");

    // Drive prompts: take the Tai action when it appears, otherwise advance
    // with the first legal action (PASS-equivalent for empty bucket /
    // bottom-placement order).
    while let Some(view) = runner.pending_selection_view() {
        let tai_action = revealed_action_for_id(&runner, "TAI");
        if let Some(action) = tai_action {
            if view.valid_action_ids.contains(&action) {
                runner.execute_action(0, action).expect("pick Tai");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"TAI".to_string()),
        "Tai Kamiya Tamer must land in hand"
    );
}

/// Neither bucket has a candidate — reveal 4 fillers, both buckets resolve
/// without adds, all 4 cards hit deck bottom and hand stays free of fillers.
#[test]
fn bt12_059_on_play_neither_bucket_found_bottoms_all_four() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .add_card(make_test_card("FILLER3", "Filler3"))
        .add_card(make_test_card("FILLER4", "Filler4"))
        .deck(0, &["FILLER1", "FILLER2", "FILLER3", "FILLER4"])
        .hand(0, &["BT12-059"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Agumon BT12-059");
    runner
        .auto_resolve()
        .expect("resolve through both empty buckets");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    for filler in ["FILLER1", "FILLER2", "FILLER3", "FILLER4"] {
        assert!(
            !hand_ids.contains(&filler.to_string()),
            "{filler} must NOT enter hand when no bucket matches"
        );
    }
    let deck_ids: Vec<String> = runner.game.players[0]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(deck_ids.len(), 4, "all 4 reveal cards back in deck");
}

/// The Digimon bucket only accepts cards of `kind: digimon`. A Tamer-kind card
/// named "Greymon" must NOT count toward the Digimon bucket.
///
/// Setup: reveal contains a Tamer-named-Greymon and the real Tai Kamiya
/// Tamer. The Digimon bucket should have no candidates (Tamer-Greymon is
/// excluded by the `kind: digimon` predicate). The Tamer bucket can accept
/// either Tamer (both have valid Tamer-bucket names, since Tamer-Greymon
/// happens to also include 'Tai Kamiya'? No — only TAI does), so Tai is the
/// unique pick.
#[test]
fn bt12_059_digimon_bucket_rejects_tamer_with_greymon_name() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_named_tamer("FAKE-GREYMON-TAMER", "Greymon"))
        .add_card(make_named_tamer("TAI", "Tai Kamiya"))
        .add_card(make_test_card("FILLER1", "Filler1"))
        .add_card(make_test_card("FILLER2", "Filler2"))
        .deck(0, &["FAKE-GREYMON-TAMER", "TAI", "FILLER1", "FILLER2"])
        .hand(0, &["BT12-059"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play BT12-059");

    // Walk the prompts. The fake Greymon Tamer must NEVER appear as a valid
    // action whose ONLY legal-pick interpretation would be "Digimon bucket
    // accepted a Tamer". We assert this by always picking Tai for the Tamer
    // bucket and never picking the fake Greymon for the Digimon bucket.
    while let Some(view) = runner.pending_selection_view() {
        let fake_action = revealed_action_for_id(&runner, "FAKE-GREYMON-TAMER");
        let tai_action = revealed_action_for_id(&runner, "TAI");
        if let (Some(fake), Some(tai)) = (fake_action, tai_action) {
            // The Tamer bucket allows either fake or tai (both are tamers);
            // the Digimon bucket should allow neither.
            let allows_tai = view.valid_action_ids.contains(&tai);
            let allows_fake = view.valid_action_ids.contains(&fake);
            if allows_fake && !allows_tai {
                panic!(
                    "Digimon bucket wrongly accepted Tamer-kind Greymon: \
                     valid_action_ids = {:?}",
                    view.valid_action_ids
                );
            }
            if allows_tai {
                runner.execute_action(0, tai).expect("pick Tai");
                continue;
            }
        }
        let next = view.valid_action_ids[0];
        runner.execute_action(0, next).expect("advance");
    }

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"TAI".to_string()),
        "Tai must be picked for the Tamer bucket"
    );
    // Fake Greymon Tamer is in the Tamer bucket's candidate set, but the
    // controller picked Tai. With `no_duplicate_cards: true` the Tamer
    // bucket adds exactly one card.
    assert!(
        !hand_ids.contains(&"FAKE-GREYMON-TAMER".to_string()),
        "Tamer bucket adds at most 1; we picked Tai, so the fake stays out"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited aura: +1000 DP while carrier has Greymon/Omnimon name
// ═══════════════════════════════════════════════════════════════════════════════

/// Stack BT12-059 under a "Greymon" carrier. The aura's
/// `self_digivolution_contains_name` condition must pass (the stack contains a
/// card named "Greymon"), so `source_dp_contribution` for BT12-059's source
/// slot is +1000.
#[test]
fn bt12_059_inherited_dp_active_when_carrier_name_contains_greymon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_named_digimon("GREYMON-CARRIER", "Greymon"))
        .memory(0)
        .start();

    let handle = runner.place_stack(0, &["BT12-059", "GREYMON-CARRIER"]);
    // The BT12-059 source is at index 0 (the base of the stack).
    let bt12_059_src_idx = 0;

    let contribution = runner
        .game
        .source_dp_contribution(handle, bt12_059_src_idx);
    assert_eq!(
        contribution, 1000,
        "BT12-059's inherited +1000 DP must contribute when the carrier's stack \
         contains a Greymon-named card; got {contribution}"
    );
}

/// Same setup but with an Omnimon carrier — the Omnimon-name branch of the
/// `any_of` must fire.
#[test]
fn bt12_059_inherited_dp_active_when_carrier_name_contains_omnimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_named_digimon("OMNIMON-CARRIER", "Omnimon"))
        .memory(0)
        .start();

    let handle = runner.place_stack(0, &["BT12-059", "OMNIMON-CARRIER"]);
    let bt12_059_src_idx = 0;

    let contribution = runner
        .game
        .source_dp_contribution(handle, bt12_059_src_idx);
    assert_eq!(
        contribution, 1000,
        "BT12-059's inherited +1000 DP must contribute when the carrier's stack \
         contains an Omnimon-named card; got {contribution}"
    );
}

/// Stack BT12-059 under a non-matching name ("Tyrannomon") — the aura must
/// NOT contribute.
#[test]
fn bt12_059_inherited_dp_inactive_when_carrier_name_unrelated() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .add_card(make_named_digimon("TYRANNO", "Tyrannomon"))
        .memory(0)
        .start();

    let handle = runner.place_stack(0, &["BT12-059", "TYRANNO"]);
    let bt12_059_src_idx = 0;

    let contribution = runner
        .game
        .source_dp_contribution(handle, bt12_059_src_idx);
    assert_eq!(
        contribution, 0,
        "BT12-059's inherited +1000 DP must NOT contribute when the carrier \
         has neither Greymon nor Omnimon in its name; got {contribution}"
    );
}

/// BT12-059 alone on the field (no top card stacked yet): the only card in
/// the stack is BT12-059 itself, named "Agumon". The `self_digivolution_contains_name`
/// scan finds neither Greymon nor Omnimon, so the aura does not contribute.
///
/// Note: `source_dp_contribution(_, 0)` queries the contribution from the
/// source at index 0 — which here is BT12-059 (the whole permanent). The
/// inherited aura must read 0 because no Greymon/Omnimon-named card is
/// stacked under or with BT12-059.
#[test]
fn bt12_059_inherited_dp_inactive_when_alone_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-059")
        .expect("BT12-059 YAML loads")
        .memory(0)
        .start();

    let handle: PermanentHandle = runner.place_on_field(0, "BT12-059", Some(0));
    let bt12_059_src_idx = 0;

    let contribution = runner
        .game
        .source_dp_contribution(handle, bt12_059_src_idx);
    assert_eq!(
        contribution, 0,
        "with no Greymon/Omnimon-named card in its own stack, the inherited \
         aura must not fire; got {contribution}"
    );
}
