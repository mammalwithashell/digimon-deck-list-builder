//! P-118 Wormmon — Digimon, Lv.3, Green, DP 1000, Cost 3. Traits: Larva.
//! Digivolve: Green Lv.2 / cost 0, Blue Lv.2 / cost 0 (official Bandai DB).
//!
//! # Card text (official — data/card_bundles/P-118.md)
//!
//! ```text
//! [On Play] Reveal the top 3 cards of your deck. Add 1 green or blue card
//! with 2 or more colors and 1 Tamer card with [Ken Ichijoji] in its name
//! among them to the hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect:
//! [End of Your Turn] This Digimon and another of your Digimon may DNA
//! digivolve into a Digimon card in your hand.
//! ```
//!
//! Official Q&A: even if only one of the applicable cards is revealed, you can
//! still add it to your hand.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/P/Green/P_118.cs`
//!   * OnPlay — `SimplifiedRevealDeckTopCardsAndSelect(3)` with two AddHand
//!     conditions (max 1 each): (a) `CardColors.Count >= 2 && (Blue || Green)`,
//!     (b) `IsTamer && ContainsCardName("Ken Ichijoji")`; rest → deck bottom.
//!   * OnEndTurn (inherited, optional) — `DNADigivolvePermanentsIntoHandOrTrashCard
//!     (payCost: true, isHand: true, permanentConditions: [this])`: the hand
//!     card must satisfy its printed DNA requirements for {this, partner} and
//!     its printed DNA cost is paid.
//!
//! # Patterns this test covers
//! - Reveal-N + two named buckets (`select_reveal_buckets`), each skipped when
//!   no candidate exists (Q&A), remainder to deck bottom.
//! - Card-subject `self_color_count_gte` + color any_of filter (any card kind).
//! - Inherited optional EoT `may_dna_digivolve_now` (requirements honoured,
//!   printed DNA cost paid), decline path, requirement-miss negative.
//! - Dual-colour Lv.2 digivolve circles (structural).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_dna_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-118";
const YAML: &str = include_str!("../../../cards/p/P-118.yaml");

// ─── Factories / helpers ─────────────────────────────────────────────────────

fn card(id: &str, name: &str, kind: CardKind, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    c.colors = colors.to_vec();
    c
}

fn lv_digimon(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = card(id, id, CardKind::Digimon, &[color]);
    c.level = Some(level);
    c.dp = Some(3000);
    c
}

fn ids(cards: &[digimon_engine::card_source::CardSource], runner: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    ids(&runner.game.players[0].hand, runner)
}

fn deck_ids(runner: &DebugRunner) -> Vec<String> {
    ids(&runner.game.players[0].deck, runner)
}

fn revealed_action(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(i, c)| {
            (c.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + i as u16)
        })
}

/// Assert the current bucket offers EXACTLY `expected` revealed cards and is
/// mandatory (a match exists), then pick `pick`.
fn pick_bucket(runner: &mut DebugRunner, expected: &[&str], pick: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional() && !view.valid_action_ids.contains(&PASS),
        "{label}: bucket must be mandatory when a matching card is revealed"
    );
    let mut want: Vec<u16> = expected
        .iter()
        .map(|id| revealed_action(runner, id).unwrap_or_else(|| panic!("{id} not revealed")))
        .collect();
    want.sort();
    let mut got = view.valid_action_ids.clone();
    got.sort();
    assert_eq!(got, want, "{label}: candidate set mismatch");
    let action = revealed_action(runner, pick).unwrap();
    runner.execute_action(0, action).expect(label);
}

fn on_play_runner(extra: Vec<CardData>, deck: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-118 YAML parses and compiles");
    for c in extra {
        b = b.add_card(c);
    }
    b.deck(0, deck).hand(0, &[CARD_ID]).memory(10).start()
}

fn gb_digimon(id: &str) -> CardData {
    card(
        id,
        id,
        CardKind::Digimon,
        &[CardColor::Green, CardColor::Blue],
    )
}

fn ken(id: &str) -> CardData {
    card(id, "Ken Ichijoji", CardKind::Tamer, &[CardColor::Green])
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn p_118_yaml_compiles_with_printed_stats() {
    let runner = on_play_runner(vec![], &[]);
    let c = runner.compiled_card(CARD_ID).expect("compiled P-118");
    assert_eq!(c.level, Some(3));
    assert_eq!(c.cost, Some(3));
    assert_eq!(c.dp, Some(1000));
    assert_eq!(c.color, vec![CompiledColor::Green]);
    assert!(c.traits.iter().any(|t| t == "Larva"));
    assert_eq!(c.effects.len(), 2, "On Play + inherited EoT clauses");
}

/// Official DB: green Lv.2 / 0 AND blue Lv.2 / 0 digivolve circles.
#[test]
fn p_118_has_green_and_blue_lv2_digivolve_costs() {
    let runner = on_play_runner(vec![], &[]);
    let data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .expect("P-118 card data");
    let mut circles: Vec<(u8, u8, u16)> = data
        .evo_costs
        .iter()
        .map(|e| (e.card_color, e.level, e.memory_cost))
        .collect();
    circles.sort();
    let mut want = vec![
        (CardColor::Green as u8, 2, 0),
        (CardColor::Blue as u8, 2, 0),
    ];
    want.sort();
    assert_eq!(circles, want);
}

#[test]
fn p_118_clause_shapes() {
    let runner = on_play_runner(vec![], &[]);
    let c = runner.compiled_card(CARD_ID).unwrap();
    let on_play = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("On Play clause");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(on_play
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectRevealBuckets { .. })));

    let eot = c
        .effects
        .iter()
        .find_map(|cl| match cl {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EoT clause");
    assert_eq!(eot.scope, CompiledScope::Inherited);
    assert!(eot.optional, "printed 'may DNA digivolve'");
    assert!(eot.process.iter().any(|s| matches!(
        s,
        CompiledStep::MayDnaDigivolveNow {
            ignore_requirements: false,
            ..
        }
    )));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] reveal 3, two buckets
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn p_118_on_play_adds_multicolor_and_ken_bottoms_rest() {
    let mut runner = on_play_runner(
        vec![
            gb_digimon("GB"),
            ken("KEN"),
            make_test_card("FILL", "Filler"),
        ],
        &["GB", "KEN", "FILL"],
    );
    runner.play(0, 0).expect("play P-118");

    pick_bucket(&mut runner, &["GB"], "GB", "multicolor bucket");
    pick_bucket(&mut runner, &["KEN"], "KEN", "Ken bucket");
    runner.auto_resolve().expect("finish");

    let hand = hand_ids(&runner);
    assert!(hand.contains(&"GB".to_string()) && hand.contains(&"KEN".to_string()));
    assert_eq!(deck_ids(&runner), vec!["FILL".to_string()]);
    assert!(runner.game.revealed_cards.is_empty());
}

/// Filter: needs 2+ colors AND green or blue. Mono-blue, mono-green and a
/// red/yellow dual are rejected; a green/blue (or blue/purple) card of ANY kind
/// qualifies (printed "card", not "Digimon card").
#[test]
fn p_118_multicolor_bucket_filter_positive_and_negative() {
    let mut runner = on_play_runner(
        vec![
            card(
                "BP-OPT",
                "BluePurpleOption",
                CardKind::Option,
                &[CardColor::Blue, CardColor::Purple],
            ),
            lv_digimon("MONO-BLUE", 3, CardColor::Blue),
            card(
                "RY",
                "RedYellow",
                CardKind::Digimon,
                &[CardColor::Red, CardColor::Yellow],
            ),
        ],
        &["BP-OPT", "MONO-BLUE", "RY"],
    );
    runner.play(0, 0).expect("play P-118");

    pick_bucket(&mut runner, &["BP-OPT"], "BP-OPT", "multicolor bucket");
    runner
        .auto_resolve()
        .expect("Ken bucket skipped, remainder bottom");

    let hand = hand_ids(&runner);
    assert!(hand.contains(&"BP-OPT".to_string()));
    assert!(!hand.contains(&"MONO-BLUE".to_string()));
    assert!(!hand.contains(&"RY".to_string()));
    assert_eq!(
        deck_ids(&runner).len(),
        2,
        "two unchosen cards returned to deck"
    );
}

/// Q&A: only the Ken Tamer is revealed → it alone is added. A non-Ken Tamer
/// and a Ken-named non-Tamer are not eligible for the Tamer bucket.
#[test]
fn p_118_on_play_only_ken_found() {
    let mut runner = on_play_runner(
        vec![
            ken("KEN"),
            card(
                "OTHER-TAMER",
                "Davis Motomiya",
                CardKind::Tamer,
                &[CardColor::Blue],
            ),
            card(
                "KEN-DIGI",
                "Ken Ichijoji Digimon",
                CardKind::Digimon,
                &[CardColor::Green],
            ),
        ],
        &["KEN", "OTHER-TAMER", "KEN-DIGI"],
    );
    runner.play(0, 0).expect("play P-118");

    pick_bucket(&mut runner, &["KEN"], "KEN", "Ken bucket");
    runner.auto_resolve().expect("finish");

    let hand = hand_ids(&runner);
    assert!(hand.contains(&"KEN".to_string()));
    assert!(!hand.contains(&"OTHER-TAMER".to_string()));
    assert!(!hand.contains(&"KEN-DIGI".to_string()));
    assert_eq!(deck_ids(&runner).len(), 2);
}

/// Q&A: only the multicolor card is revealed → it alone is added.
#[test]
fn p_118_on_play_only_multicolor_found() {
    let mut runner = on_play_runner(
        vec![
            gb_digimon("GB"),
            make_test_card("F1", "Filler1"),
            make_test_card("F2", "Filler2"),
        ],
        &["F1", "GB", "F2"],
    );
    runner.play(0, 0).expect("play P-118");
    pick_bucket(&mut runner, &["GB"], "GB", "multicolor bucket");
    runner.auto_resolve().expect("finish");
    assert!(hand_ids(&runner).contains(&"GB".to_string()));
    assert_eq!(deck_ids(&runner).len(), 2);
}

#[test]
fn p_118_on_play_no_match_bottoms_all() {
    let mut runner = on_play_runner(
        vec![
            make_test_card("F1", "Filler1"),
            make_test_card("F2", "Filler2"),
            make_test_card("F3", "Filler3"),
        ],
        &["F1", "F2", "F3"],
    );
    runner.play(0, 0).expect("play P-118");
    runner.auto_resolve().expect("finish");
    assert!(hand_ids(&runner).is_empty());
    assert_eq!(deck_ids(&runner).len(), 3);
    assert!(runner.game.revealed_cards.is_empty());
}

/// A card qualifying for both buckets (green/blue Ken Tamer) can be added only
/// once; the other bucket must take a different card.
#[test]
fn p_118_no_duplicate_card_across_buckets() {
    let mut runner = on_play_runner(
        vec![
            card(
                "GB-KEN",
                "Ken Ichijoji",
                CardKind::Tamer,
                &[CardColor::Green, CardColor::Blue],
            ),
            gb_digimon("GB"),
            make_test_card("F1", "Filler1"),
        ],
        &["GB-KEN", "GB", "F1"],
    );
    runner.play(0, 0).expect("play P-118");
    pick_bucket(&mut runner, &["GB-KEN", "GB"], "GB", "multicolor bucket");
    pick_bucket(&mut runner, &["GB-KEN"], "GB-KEN", "Ken bucket");
    runner.auto_resolve().expect("finish");
    let hand = hand_ids(&runner);
    assert!(hand.contains(&"GB".to_string()) && hand.contains(&"GB-KEN".to_string()));
    assert_eq!(deck_ids(&runner), vec!["F1".to_string()]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited [End of Your Turn] DNA digivolve
// ═══════════════════════════════════════════════════════════════════════════════

/// Runner with P-118 as a digivolution SOURCE under CARRIER (Lv.4), a Lv.4
/// partner on field, and `hand_target` in hand. Memory 3.
fn eot_runner(hand_target: CardData) -> (DebugRunner, String) {
    let target_id = hand_target.card_id.clone();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-118 YAML")
        .add_card(lv_digimon("CARRIER", 4, CardColor::Green))
        .add_card(lv_digimon("PARTNER", 4, CardColor::Blue))
        .add_card(hand_target)
        .add_card(make_test_card("PAD", "Pad"))
        .hand(0, &[target_id.as_str()])
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(3)
        .start();
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.place_on_field(0, "PARTNER", Some(0));
    (runner, target_id)
}

fn dna_target(id: &str, req_level: u8, cost: i16) -> CardData {
    let mut t = make_test_dna_card(id, id, req_level, req_level, cost);
    t.card_kind = CardKind::Digimon;
    t.colors = vec![CardColor::Green];
    t.level = Some(5);
    t.dp = Some(9000);
    t
}

fn top_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Accept path: the optional trigger surfaces, partner pick offers only the
/// OTHER Digimon (never this carrier), the hand pick offers the legal DNA
/// target, and the merge pays the printed DNA cost (2).
#[test]
fn p_118_eot_dna_digivolves_and_pays_printed_dna_cost() {
    let (mut runner, target) = eot_runner(dna_target("DNA-RESULT", 4, 2));
    runner.game.fire_end_of_your_turn(0);

    let mut saw_partner_pick = false;
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        if view.kind == SelectionKind::Hand {
            // The hand holds only the DNA target, so it is the sole candidate.
            let non_pass: Vec<u16> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|a| *a != PASS)
                .collect();
            assert_eq!(non_pass.len(), 1, "the legal DNA target is offered");
            let action = non_pass[0];
            runner
                .execute_action(view.selecting_player, action)
                .unwrap();
            break;
        }
        if view.kind == SelectionKind::OwnField {
            saw_partner_pick = true;
            assert_eq!(
                view.valid_action_ids.iter().filter(|a| **a != PASS).count(),
                1,
                "only the other Digimon (PARTNER) is a DNA partner — never this Digimon"
            );
        }
        let a = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .expect("non-PASS action");
        runner.execute_action(view.selecting_player, a).unwrap();
    }
    let _ = runner.auto_resolve();

    assert!(
        saw_partner_pick,
        "the partner choice must be exposed to the player"
    );
    let tops = top_ids(&runner);
    assert_eq!(
        tops,
        vec![target.clone()],
        "both materials merged under the DNA result; field={tops:?}"
    );
    let stack = ids(&runner.game.players[0].battle_area[0].card_sources, &runner);
    for m in ["CARRIER", "PARTNER", CARD_ID] {
        assert!(
            stack.contains(&m.to_string()),
            "{m} must be in the DNA stack; {stack:?}"
        );
    }
    assert_eq!(runner.memory(), 1, "printed DNA cost 2 paid from 3 memory");
    assert!(!hand_ids(&runner).contains(&target));
}

/// Decline path: PASS at the first (optional) prompt leaves the board intact.
#[test]
fn p_118_eot_dna_decline_changes_nothing() {
    let (mut runner, target) = eot_runner(dna_target("DNA-RESULT", 4, 2));
    runner.game.fire_end_of_your_turn(0);
    let view = runner
        .pending_selection_view()
        .expect("the optional DNA effect must surface a prompt");
    assert!(
        runner.pending_is_optional() || view.valid_action_ids.contains(&PASS),
        "printed 'may' → declinable"
    );
    let mut guard = 0;
    while runner.pending_selection_view().is_some() && guard < 6 {
        let v = runner.pending_selection_view().unwrap();
        runner
            .execute_action(v.selecting_player, PASS)
            .expect("decline");
        guard += 1;
    }
    let _ = runner.auto_resolve();
    let mut tops = top_ids(&runner);
    tops.sort();
    assert_eq!(tops, vec!["CARRIER".to_string(), "PARTNER".to_string()]);
    assert!(hand_ids(&runner).contains(&target));
    assert_eq!(runner.memory(), 3);
}

/// Requirements are NOT ignored: a hand Digimon whose DNA requirements (Lv.6 +
/// Lv.6) the {Lv.4, Lv.4} pair cannot meet never becomes the merge result.
#[test]
fn p_118_eot_dna_requires_printed_dna_requirements() {
    let (mut runner, target) = eot_runner(dna_target("DNA-BAD", 6, 0));
    runner.game.fire_end_of_your_turn(0);
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 8, "selection loop did not terminate");
        if v.kind == SelectionKind::Hand {
            assert!(
                v.valid_action_ids.iter().all(|a| *a == PASS),
                "requirement-breaking hand card must not be offered: {v:?}"
            );
            runner
                .execute_action(v.selecting_player, PASS)
                .expect("pass");
            continue;
        }
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .unwrap_or(PASS);
        runner
            .execute_action(v.selecting_player, a)
            .expect("advance");
    }
    let _ = runner.auto_resolve();
    let mut tops = top_ids(&runner);
    tops.sort();
    assert_eq!(tops, vec!["CARRIER".to_string(), "PARTNER".to_string()]);
    assert!(hand_ids(&runner).contains(&target));
}

/// No other Digimon → no DNA partner → the effect does nothing (DCGO
/// CanActivateCondition requires another own Digimon).
#[test]
fn p_118_eot_dna_no_partner_no_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-118 YAML")
        .add_card(lv_digimon("CARRIER", 4, CardColor::Green))
        .add_card(dna_target("DNA-RESULT", 4, 2))
        .add_card(make_test_card("PAD", "Pad"))
        .hand(0, &["DNA-RESULT"])
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(3)
        .start();
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.fire_end_of_your_turn(0);
    while let Some(v) = runner.pending_selection_view() {
        assert_ne!(
            v.kind,
            SelectionKind::Hand,
            "no hand pick without a partner"
        );
        assert_ne!(
            v.kind,
            SelectionKind::OwnField,
            "no partner pick without a partner"
        );
        runner
            .execute_action(v.selecting_player, PASS)
            .expect("pass");
    }
    assert_eq!(top_ids(&runner), vec!["CARRIER".to_string()]);
    assert!(hand_ids(&runner).contains(&"DNA-RESULT".to_string()));
}

/// The EoT clause is inherited-only: P-118 as the TOP card does not offer it.
#[test]
fn p_118_eot_dna_not_active_as_top_card() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-118 YAML")
        .add_card(lv_digimon("PARTNER", 4, CardColor::Blue))
        .add_card(dna_target("DNA-RESULT", 3, 0))
        .add_card(make_test_card("PAD", "Pad"))
        .hand(0, &["DNA-RESULT"])
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(3)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "PARTNER", Some(0));
    runner.game.fire_end_of_your_turn(0);
    assert!(
        runner.pending_selection_view().is_none(),
        "inherited effect must not fire from the top card"
    );
    assert!(hand_ids(&runner).contains(&"DNA-RESULT".to_string()));
}
