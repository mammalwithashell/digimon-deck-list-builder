//! EX7-016 Bulucomon — Digimon, Lv.3, Blue, DP 1000, Cost 3.
//! Traits: Mini Dragon. Attribute: Data. Rule box: "Trait: Has [Ice-Snow] Type."
//! Evo cost: from blue Lv.2 for 0 memory (cards.json evo_costs).
//!
//! # Card text (card image EX7-016.webp — authoritative; matches cards.json)
//!
//! **[On Play]** Reveal the top 3 cards of your deck. Add 1 card with
//! [Paledramon] or [Hexeblaumon] in its name and 1 card with the [Ice-Snow]
//! trait among them to the hand. Return the rest to the bottom of the deck.
//!
//! **Inherited:** [When Attacking] [Once Per Turn] Trash the top digivolution
//! card of 1 of your opponent's Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Blue/EX7_016.cs
//!   - On Play (OnEnterFieldAnyone): SimplifiedRevealDeckTopCardsAndSelect(
//!     revealCount: 3, two SimplifiedSelectCardConditionClass entries —
//!     [0] IsDigimon && (ContainsCardName "Paledramon" || "Hexeblaumon"),
//!     [1] ContainsTraits("Ice-Snow") — each AddHand maxCount 1,
//!     remainingCardsPlace: DeckBottom). canNoSelect defaults to FALSE: each
//!     add is MANDATORY when a matching card was revealed (no PASS). The
//!     selections run sequentially and a picked card is removed from the
//!     reveal pool before the second pick (no double-dipping).
//!   - When Attacking ESS (OnAllyAttack, SetHashString "WhenAttacking_EX7-016",
//!     maxCount 1 = [Once Per Turn]): if any opponent battle-area Digimon
//!     exists, SelectPermanentEffect (canNoSelect: false — MANDATORY) over
//!     opponent Digimon, then TrashDigivolutionCardsFromTopOrBottom(count 1,
//!     isFromTop: true) — trash the TOP digivolution card (the source
//!     immediately beneath the active top card).
//!
//! # Patterns this test covers
//! - A1/A3 searching rookie: reveal-3, two filtered adds (RevealBucket flow)
//! - G4-adjacent: inherited When Attacking + OPT
//! - Source-stack peel: trash the top digivolution card of a chosen opponent
//!   Digimon (`trash_top_stacked_sources`, count 1)

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A Digimon card whose NAME contains "Paledramon" (no Ice-Snow trait, so it
/// can only satisfy the name bucket).
fn paledramon_named(id: &str) -> CardData {
    let mut c = make_test_card(id, "Paledramon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c
}

/// A Digimon card whose NAME contains "Hexeblaumon" (no Ice-Snow trait).
fn hexeblaumon_named(id: &str) -> CardData {
    let mut c = make_test_card(id, "Hexeblaumon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c
}

/// A card with the [Ice-Snow] trait whose name matches neither filter.
fn ice_snow_card(id: &str) -> CardData {
    let mut c = make_test_card(id, "Frigimon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A card satisfying BOTH filters: named "Paledramon" AND [Ice-Snow] trait
/// (this is what real Paledramon printings look like — EX7-020 has Ice-Snow).
fn dual_match_card(id: &str) -> CardData {
    let mut c = make_test_card(id, "Paledramon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A card matching neither bucket.
fn junk(id: &str) -> CardData {
    let mut c = make_test_card(id, "Junkmon");
    c.traits = vec!["Machine".to_string()];
    c
}

/// High-DP carrier so the EX7-016 inherited effect can ride under a surviving
/// attacker across security checks.
fn strong_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(8000);
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("EX7-016")
        .expect("EX7-016 YAML loads from the embedded pack")
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

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .unwrap_or_else(|| panic!("card {id} not in reveal pool"))
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: {id} must be a legal pick; valid={:?}",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect(label);
}

fn assert_mandatory_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: the add is mandatory when a matching card was revealed (DCGO canNoSelect=false)"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be exposed before the required pick"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_016_compiles_with_printed_stats_and_lv2_blue_path() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card("EX7-016")
        .expect("EX7-016 in compiled cards");

    assert_eq!(compiled.card, "EX7-016");
    assert_eq!(compiled.name, "Bulucomon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Blue]);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    assert!(
        compiled.traits.iter().any(|t| t == "Mini Dragon"),
        "printed type [Mini Dragon] missing"
    );
    assert!(
        compiled.traits.iter().any(|t| t == "Ice-Snow"),
        "rule box 'Trait: Has [Ice-Snow] Type.' must surface as an Ice-Snow trait"
    );

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(2)
                        || from.all_of.iter().any(|p| p.level_eq == Some(2)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|p| p.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "Bulucomon must digivolve from a blue Lv.2 for cost 0"
    );
}

#[test]
fn ex7_016_clause_shape_on_play_mandatory_and_inherited_opt() {
    let runner = base_builder().start();
    let compiled = runner
        .compiled_card("EX7-016")
        .expect("EX7-016 in compiled cards");

    let triggered: Vec<_> = compiled
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
        "EX7-016 has exactly two triggered clauses (On Play + inherited When Attacking)"
    );

    let on_play = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("On Play clause exists");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(
        !on_play.optional,
        "the reveal/add has no 'you may' — mandatory (DCGO canNoSelect=false)"
    );
    assert!(!on_play.once_per_turn);

    let inherited = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenAttacking])
        .expect("inherited When Attacking clause exists");
    assert_eq!(inherited.scope, CompiledScope::Inherited);
    assert!(
        inherited.once_per_turn,
        "printed [Once Per Turn] must compile to once_per_turn"
    );
    assert!(
        !inherited.optional,
        "the source trash has no 'you may' — mandatory when a target exists"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — [On Play] reveal-3 / two filtered adds / rest to bottom
// ─────────────────────────────────────────────────────────────────────────────

/// Mixed reveal: one name-match, one Ice-Snow card, one junk → both buckets
/// prompt mandatorily with exactly the right candidates; the junk card is
/// returned to the bottom of the deck.
#[test]
fn ex7_016_on_play_mixed_reveal_adds_name_and_ice_snow_and_bottoms_rest() {
    let mut runner = base_builder()
        .add_card(paledramon_named("PALE"))
        .add_card(ice_snow_card("ICE"))
        .add_card(junk("JUNK"))
        .add_card(junk("D1"))
        .add_card(junk("D2"))
        // last element = deck top: reveal order PALE, ICE, JUNK.
        .deck(0, &["D1", "D2", "JUNK", "ICE", "PALE"])
        .hand(0, &["EX7-016"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Bulucomon");

    // Bucket 0 — name filter. Mandatory; only PALE is a candidate.
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::RevealBucket { bucket_index: 0, .. })
    ));
    assert_mandatory_pick(&runner, "name bucket");
    let view = runner.pending_selection_view().expect("name bucket view");
    assert_eq!(
        view.valid_action_ids,
        vec![revealed_action_for_id(&runner, "PALE")],
        "only the Paledramon-named card qualifies for the name bucket"
    );
    pick_revealed_by_id(&mut runner, "PALE", "pick Paledramon");

    // Bucket 1 — Ice-Snow trait filter. Mandatory; only ICE is a candidate.
    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::RevealBucket { bucket_index: 1, .. })
    ));
    assert_mandatory_pick(&runner, "Ice-Snow bucket");
    let view = runner
        .pending_selection_view()
        .expect("Ice-Snow bucket view");
    assert_eq!(
        view.valid_action_ids,
        vec![revealed_action_for_id(&runner, "ICE")],
        "only the Ice-Snow trait card qualifies for the trait bucket"
    );
    pick_revealed_by_id(&mut runner, "ICE", "pick Ice-Snow card");

    runner.auto_resolve().expect("finish on-play resolution");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"PALE".to_string()), "PALE added to hand");
    assert!(hand_ids.contains(&"ICE".to_string()), "ICE added to hand");
    assert!(!hand_ids.contains(&"JUNK".to_string()), "JUNK not added");

    // The rest (JUNK) returned to the BOTTOM of the deck (deck[0] = bottom).
    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck_ids.len(), 3, "deck = 2 fillers + 1 bottomed remainder");
    assert_eq!(
        deck_ids[0], "JUNK",
        "the unmatched revealed card must go to the deck bottom"
    );
    assert!(runner.game.revealed_cards.is_empty(), "reveal pool drained");
}

/// No matching cards: no selection installs, nothing is added to hand, all 3
/// revealed cards return to the bottom of the deck.
#[test]
fn ex7_016_on_play_no_matches_bottoms_all_three() {
    let mut runner = base_builder()
        .add_card(junk("J1"))
        .add_card(junk("J2"))
        .add_card(junk("J3"))
        .add_card(junk("D1"))
        .deck(0, &["D1", "J3", "J2", "J1"])
        .hand(0, &["EX7-016"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);

    runner.play(0, 0).expect("play Bulucomon");
    runner.auto_resolve().expect("resolve with no picks");

    assert!(
        runner.pending_selection().is_none(),
        "no bucket may prompt when nothing matches"
    );
    // Hand: -1 for playing Bulucomon, +0 adds.
    assert_eq!(runner.hand_size(0), hand_before - 1);

    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(deck_ids.len(), 4, "all 3 revealed cards returned to deck");
    let bottom3: Vec<&str> = deck_ids[..3].iter().map(|s| s.as_str()).collect();
    for id in ["J1", "J2", "J3"] {
        assert!(
            bottom3.contains(&id),
            "{id} must be among the 3 bottomed cards; bottom3={bottom3:?}"
        );
    }
    assert_eq!(deck_ids[3], "D1", "pre-existing deck card stays above the bottomed rest");
}

/// Name filter positive/negative: a Hexeblaumon-named card IS a name-bucket
/// candidate; an unrelated card is NOT.
#[test]
fn ex7_016_on_play_hexeblaumon_qualifies_for_name_bucket_and_junk_does_not() {
    let mut runner = base_builder()
        .add_card(hexeblaumon_named("HEXE"))
        .add_card(junk("J1"))
        .add_card(junk("J2"))
        .add_card(junk("D1"))
        .deck(0, &["D1", "J2", "J1", "HEXE"])
        .hand(0, &["EX7-016"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Bulucomon");

    assert!(matches!(
        runner.pending_kind(),
        Some(SelectionKind::RevealBucket { bucket_index: 0, .. })
    ));
    let view = runner.pending_selection_view().expect("name bucket view");
    assert_eq!(
        view.valid_action_ids,
        vec![revealed_action_for_id(&runner, "HEXE")],
        "Hexeblaumon qualifies; junk cards must not be candidates"
    );
    pick_revealed_by_id(&mut runner, "HEXE", "pick Hexeblaumon");
    runner.auto_resolve().expect("finish");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"HEXE".to_string()));
}

/// Ice-Snow-only reveal: the name bucket has no candidates and is skipped;
/// the trait bucket prompts directly.
#[test]
fn ex7_016_on_play_ice_snow_only_skips_name_bucket() {
    let mut runner = base_builder()
        .add_card(ice_snow_card("ICE"))
        .add_card(junk("J1"))
        .add_card(junk("J2"))
        .add_card(junk("D1"))
        .deck(0, &["D1", "J2", "J1", "ICE"])
        .hand(0, &["EX7-016"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Bulucomon");

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::RevealBucket { bucket_index: 1, .. })
        ),
        "with no name-bucket candidate, the Ice-Snow bucket prompts directly; got {:?}",
        runner.pending_kind()
    );
    pick_revealed_by_id(&mut runner, "ICE", "pick Ice-Snow card");
    runner.auto_resolve().expect("finish");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"ICE".to_string()));
}

/// A card satisfying BOTH filters can only be added once: after it is taken
/// for the name bucket it must not also be selectable for the trait bucket
/// (DCGO removes the picked card from the reveal pool between selections).
#[test]
fn ex7_016_on_play_dual_match_cannot_satisfy_both_buckets() {
    let mut runner = base_builder()
        .add_card(dual_match_card("PALE-ICE"))
        .add_card(ice_snow_card("ICE"))
        .add_card(junk("J1"))
        .add_card(junk("D1"))
        .deck(0, &["D1", "J1", "ICE", "PALE-ICE"])
        .hand(0, &["EX7-016"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Bulucomon");

    // Name bucket: PALE-ICE is the only name match.
    pick_revealed_by_id(&mut runner, "PALE-ICE", "pick dual-match for name bucket");

    // Trait bucket: PALE-ICE is gone from the pool — only ICE remains legal.
    let view = runner
        .pending_selection_view()
        .expect("Ice-Snow bucket view");
    assert_eq!(
        view.valid_action_ids,
        vec![revealed_action_for_id(&runner, "ICE")],
        "the already-picked dual-match card must not satisfy the second bucket"
    );
    pick_revealed_by_id(&mut runner, "ICE", "pick remaining Ice-Snow card");
    runner.auto_resolve().expect("finish");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(
        hand_ids.iter().filter(|id| *id == "PALE-ICE").count(),
        1,
        "the dual-match card is added exactly once"
    );
    assert!(hand_ids.contains(&"ICE".to_string()));
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Inherited [When Attacking][Once Per Turn] source peel
// ─────────────────────────────────────────────────────────────────────────────

/// Stack the carrier (with EX7-016 underneath) for player 0 and a 3-card
/// opponent stack; security for player 1 so the game continues.
fn inherited_setup() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = base_builder()
        .add_card(strong_carrier("CARRIER"))
        .add_card(junk("OPP-BOTTOM"))
        .add_card(junk("OPP-MID"))
        .add_card(junk("OPP-TOP"))
        .add_card(junk("SEC1"))
        .add_card(junk("SEC2"))
        .security(1, &["SEC1", "SEC2"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &["EX7-016", "CARRIER"]);
    // Opponent: top card OPP-TOP, digivolution cards OPP-BOTTOM (bottom) and
    // OPP-MID (top digivolution card).
    runner.place_stack(1, &["OPP-BOTTOM", "OPP-MID", "OPP-TOP"]);
    (runner, carrier)
}

/// POSITIVE: attacking with EX7-016 in the stack prompts a mandatory opponent
/// Digimon selection; resolving it trashes the TOP digivolution card
/// (OPP-MID), leaving the active top card and the bottom source in place.
#[test]
fn ex7_016_inherited_when_attacking_trashes_top_digivolution_card() {
    let (mut runner, carrier) = inherited_setup();

    runner.attack_player(carrier, 1, false);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "a mandatory opponent-Digimon selection must install"
    );
    let view = runner.pending_selection_view().expect("target prompt");
    assert!(
        !runner.pending_is_optional(),
        "the trash is not 'you may' — no PASS while a legal target exists"
    );
    assert!(!view.valid_action_ids.contains(&PASS));
    assert_eq!(view.valid_action_ids.len(), 1, "exactly one legal target");

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the stacked opponent Digimon");
    runner.auto_resolve().expect("finish attack");

    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-MID".to_string()),
        "the TOP digivolution card (immediately beneath the top card) is trashed; trash={trash_ids:?}"
    );
    assert!(
        !trash_ids.contains(&"OPP-BOTTOM".to_string()),
        "the bottom digivolution card stays"
    );
    assert!(
        !trash_ids.contains(&"OPP-TOP".to_string()),
        "the active top card stays"
    );
    let opp = &runner.game.players[1].battle_area[0];
    assert_eq!(opp.card_sources.len(), 2, "stack shrinks 3 -> 2");
    assert_eq!(
        opp.top_card().card_id(&runner.game.card_data),
        "OPP-TOP",
        "top card unchanged"
    );
}

/// The selection must only offer opponent Digimon that actually HAVE
/// digivolution cards: a bare (sourceless) Digimon is not a legal target.
#[test]
fn ex7_016_inherited_selection_excludes_sourceless_opponent_digimon() {
    let (mut runner, carrier) = inherited_setup();
    // Add a bare opponent Digimon with no digivolution cards.
    runner.place_on_field(1, "OPP-TOP", Some(0));

    runner.attack_player(carrier, 1, false);

    let view = runner.pending_selection_view().expect("target prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the stacked Digimon is targetable; the sourceless one is excluded"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the stacked opponent Digimon");
    runner.auto_resolve().expect("finish attack");

    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-MID".to_string()),
        "the trash came from the stacked Digimon"
    );
}

/// NEGATIVE: with no opponent Digimon holding digivolution cards, the clause
/// does not trigger at all (no prompt, no trash).
#[test]
fn ex7_016_inherited_no_trigger_without_stacked_opponent_digimon() {
    let mut runner = base_builder()
        .add_card(strong_carrier("CARRIER"))
        .add_card(junk("OPP-TOP"))
        .add_card(junk("SEC1"))
        .security(1, &["SEC1"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &["EX7-016", "CARRIER"]);
    // Bare opponent Digimon only — no digivolution cards anywhere.
    runner.place_on_field(1, "OPP-TOP", Some(0));

    runner.attack_player(carrier, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "no target selection may install when no opponent Digimon has digivolution cards"
    );
    let _ = runner.auto_resolve();
    // Only security-check fallout (SEC1) may reach the trash — the effect
    // itself must not have trashed anything.
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.iter().all(|id| id == "SEC1"),
        "the effect must not trash anything (only security fallout allowed); trash={trash_ids:?}"
    );
}

/// OPT: the second attack in the same turn must not re-trigger the inherited
/// effect even though a legal target (OPP-BOTTOM still stacked) remains.
#[test]
fn ex7_016_inherited_opt_blocks_second_attack_same_turn() {
    let (mut runner, carrier) = inherited_setup();

    // First attack: fires, trashes OPP-MID. (Security fallout — SEC cards —
    // also reaches the trash; assertions are membership-based.)
    runner.attack_player(carrier, 1, false);
    let view = runner.pending_selection_view().expect("first-attack prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("resolve first trash");
    runner.auto_resolve().expect("finish first attack");
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"OPP-MID".to_string()),
        "first attack must trash the top digivolution card; trash={trash_ids:?}"
    );
    assert!(
        !trash_ids.contains(&"OPP-BOTTOM".to_string()),
        "first attack trashes exactly one digivolution card"
    );

    if runner.game_over() {
        return; // security exhausted unexpectedly; OPT cannot be probed further
    }

    // Second attack same turn: the opponent stack still has OPP-BOTTOM
    // (stack size 2 >= 2), so a target exists — but OPT must lock the clause.
    let carrier_again = runner.perm_handle(0, 0);
    runner.attack_player(carrier_again, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must block the second activation in the same turn"
    );
    let _ = runner.auto_resolve();
    let trash_ids = zone_ids(&runner.game.players[1].trash, &runner.game.card_data);
    assert!(
        !trash_ids.contains(&"OPP-BOTTOM".to_string()),
        "no additional digivolution card may be trashed by the second attack; trash={trash_ids:?}"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        2,
        "the opponent stack must stay at 2 cards after the locked second attack"
    );
}
