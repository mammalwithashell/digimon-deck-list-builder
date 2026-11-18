//! EX8-009 Guilmon (X Antibody) — Digimon, Lv.3, Red/Purple, DP 2000, Cost 3.
//! Traits: Dark Dragon, X Antibody.
//!
//! # Card text (card image + official Bandai DB bundle `data/card_bundles/EX8-009.md`)
//!
//! - Evo boxes: Lv.2 red/purple split circle, cost 1; "[Digivolve]
//!   [Gigimon]/[Guilmon]: Cost 0".
//! - [On Play] [When Digivolving] Reveal the top 3 cards of your deck. Add 1
//!   card with [Growlmon] or [Gallantmon] in its name and 1 [X Antibody] among
//!   them to the hand. Return the rest to the bottom of the deck.
//! - Inherited: [Your Turn] [Once Per Turn] When any of your opponent's
//!   Digimon are deleted, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX8/Red/EX8_009.cs — exact-name
//! (`EqualsCardName`) alt digivolve gate, `SimplifiedRevealDeckTopCardsAndSelect`
//! with two mandatory single-card buckets, inherited `OnDestroyedAnyone` OPT.
//!
//! # Replay regression (2026-11 DCGO PvP game)
//! A hatched [Gigimon] (EX13-001, cards.json `level: null`) digivolved into
//! this card from the BREEDING area through the cost-0 alt box.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "EX8-009";

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A hatched [Gigimon] exactly as cards.json carries EX13-001 (`level: null`).
fn gigimon_from_cards_json(id: &str) -> CardData {
    let json = format!(
        r#"{{ "{id}": {{
            "index": 1, "norm_id": 0.00005, "card_id": "{id}", "card_index": 1,
            "card_name_eng": "Gigimon", "card_name_jpn": "",
            "card_effect_class_name": "X", "play_cost": 0, "dp": null,
            "level": null, "card_kind": 3, "rarity": 1, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        }} }}"#
    );
    CardData::load_from_str(&json)
        .expect("egg row parses")
        .remove(id)
        .expect("egg row present")
}

/// A Lv.3 Digimon of the given exact name (battle-area digivolve base).
fn lv3_named(id: &str, name: &str, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = colors;
    c
}

fn growlmon_named(id: &str, name: &str, kind: CardKind) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = kind;
    c.level = if kind == CardKind::Digimon { Some(4) } else { None };
    c.dp = if kind == CardKind::Digimon { Some(5000) } else { None };
    c.colors = vec![CardColor::Red];
    c
}

fn x_antibody(id: &str) -> CardData {
    let mut c = make_test_card(id, "X Antibody");
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Red];
    c
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-009 YAML loads from the embedded DSL pack")
        .add_card(filler("PAD-1"))
        .add_card(filler("PAD-2"))
        .add_card(filler("PAD-3"))
        .add_card(growlmon_named("GROWL", "Growlmon", CardKind::Digimon))
        .add_card(growlmon_named("GALLANT", "Gallantmon", CardKind::Digimon))
        .add_card(growlmon_named("GROWL-OPT", "Mega Growlmon Charm", CardKind::Option))
        .add_card(x_antibody("XANTI"))
        .add_card(filler("FILL"))
}

/// Runner with EX8-009 in hand; `deck_top_down[0]` is the top card.
fn on_play_runner(deck_top_down: &[&str]) -> DebugRunner {
    let mut deck: Vec<&str> = vec!["PAD-1", "PAD-2", "PAD-3"];
    deck.extend(deck_top_down.iter().rev());
    let mut r = builder().hand(0, &[CARD_ID]).deck(0, &deck).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r
}

// ─── Structure ───────────────────────────────────────────────────────────────

#[test]
fn ex8_009_registers_split_circle_and_gigimon_guilmon_cost_0_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX8-009 YAML loads")
        .build();
    let card = runner.compiled_card(CARD_ID).expect("compiled card present");
    let mut costs: Vec<i32> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .map(|p| match p.cost.as_ref() {
            Some(CompiledCost::Literal(v)) => *v,
            other => panic!("literal costs expected, got {other:?}"),
        })
        .collect();
    costs.sort_unstable();
    assert_eq!(
        costs,
        vec![0, 1, 1],
        "red Lv.2 /1 + purple Lv.2 /1 split halves + [Gigimon]/[Guilmon] /0"
    );
}

// ─── [On Play] reveal ────────────────────────────────────────────────────────

/// [Growlmon]-named card + [X Antibody] + filler: both match, filler goes to
/// the deck bottom.
#[test]
fn ex8_009_on_play_adds_growlmon_named_and_x_antibody_rest_to_bottom() {
    let mut runner = on_play_runner(&["GROWL", "XANTI", "FILL"]);
    let deck_before = runner.game.players[0].deck.len();

    assert!(runner.game.play_from_hand(0, 0).is_some());
    runner.auto_resolve().expect("resolve both bucket picks");

    let hand = hand_ids(&runner);
    assert!(hand.contains(&"GROWL".to_string()), "Growlmon card to hand: {hand:?}");
    assert!(hand.contains(&"XANTI".to_string()), "[X Antibody] to hand: {hand:?}");
    assert!(!hand.contains(&"FILL".to_string()), "filler must stay: {hand:?}");
    assert_eq!(runner.game.players[0].deck.len(), deck_before - 2);
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "FILL",
        "the unpicked card returns to the BOTTOM of the deck"
    );
}

/// "[Growlmon] or [Gallantmon] in its name" — any card kind; Gallantmon
/// qualifies and an Option with Growlmon in its name qualifies too (DCGO has no
/// IsDigimon gate on this bucket).
#[test]
fn ex8_009_name_bucket_accepts_gallantmon_and_non_digimon_names() {
    let mut runner = on_play_runner(&["GALLANT", "XANTI", "GROWL-OPT"]);
    assert!(runner.game.play_from_hand(0, 0).is_some());

    // Bucket 0: both [Gallantmon] and the Growlmon-named Option are candidates.
    let first = runner.pending_selection_view().expect("bucket 0 pending");
    assert!(
        !first.valid_action_ids.contains(&PASS),
        "the pick is mandatory — no PASS (DCGO canNoSelect = false)"
    );
    assert_eq!(first.valid_action_ids.len(), 2, "GALLANT and GROWL-OPT");
}

/// A revealed card matching neither bucket is never added; with no [X Antibody]
/// revealed only the name bucket fires.
#[test]
fn ex8_009_unmatched_reveal_adds_only_the_matching_bucket() {
    let mut runner = on_play_runner(&["GROWL", "FILL", "FILL"]);
    assert!(runner.game.play_from_hand(0, 0).is_some());
    runner.auto_resolve().expect("resolve");
    let hand = hand_ids(&runner);
    assert_eq!(hand, vec!["GROWL".to_string()], "only the Growlmon card is added");
}

// ─── Alt digivolve [Gigimon]/[Guilmon]: Cost 0 ───────────────────────────────

fn breeding_runner(base: CardData) -> DebugRunner {
    let base_id = base.card_id.clone();
    let mut r = builder()
        .add_card(base)
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD-1", "PAD-2", "PAD-3", "GROWL", "XANTI", "FILL"])
        .memory(5)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_in_breeding(0, &base_id);
    r
}

/// A hatched [Gigimon] (cards.json `level: null` row) satisfies BOTH the
/// [Gigimon] cost-0 box and the red Lv.2 cost-1 circle -> a rule-17 cost
/// choice; picking cost 0 pays nothing. The digivolve happens in the BREEDING
/// area, where CR 4.3 3-4-7-4 stops the card's own [When Digivolving] effect
/// from triggering (DCGO `IsExistOnBattleAreaTrigger` agrees), so the reveal
/// does not run.
#[test]
fn ex8_009_breeding_gigimon_offers_cost_0_route_and_when_digivolving_does_not_fire() {
    let mut runner = breeding_runner(gigimon_from_cards_json("GIGI"));

    let mask = build_action_mask(&runner.game, 0);
    assert!(
        mask[encode_digivolve(0, BREEDING_TARGET) as usize] > 0.0,
        "the breeding digivolve must be offered over a hatched [Gigimon]"
    );

    let mem_before = runner.game.memory;
    let proceeded = runner
        .game
        .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
    assert!(
        !proceeded,
        "two routes (cost 0 and cost 1) -> a cost choice pauses the digivolve"
    );
    let view = runner.pending_selection_view().expect("cost choice pending");
    let choices = view.effect_choices.as_ref().expect("effect_choices");
    let cost0 = choices
        .iter()
        .find(|c| c.label.to_lowercase().contains("cost 0"))
        .expect("cost-0 option")
        .action_id;
    assert!(
        choices
            .iter()
            .any(|c| c.label.to_lowercase().contains("cost 1")),
        "the standard cost-1 circle is the other option"
    );
    runner.execute_action(0, cost0).expect("pick cost 0");
    let _ = runner.auto_resolve();

    assert_eq!(runner.game.memory, mem_before, "cost-0 route pays nothing");
    let top = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding permanent remains")
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top, CARD_ID, "EX8-009 is stacked on the breeding [Gigimon]");
    // CR 4.3 3-4-7-4: no [When Digivolving] reveal from the breeding area, so the
    // Growlmon-named card and [X Antibody] stay in the deck (the hand holds only
    // the digivolution bonus draw).
    let hand = hand_ids(&runner);
    assert!(
        !hand.contains(&"GROWL".to_string()) && !hand.contains(&"XANTI".to_string()),
        "[When Digivolving] must not trigger in the breeding area: {hand:?}"
    );
}

/// Exact names: a battle-area [Guilmon] (Lv.3, so the Lv.2 circles do not
/// apply) is a legal base at cost 0 …
#[test]
fn ex8_009_battle_area_guilmon_is_a_cost_0_base() {
    let mut runner = builder()
        .add_card(lv3_named("GUIL", "Guilmon", vec![CardColor::Blue]))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD-1", "PAD-2", "PAD-3", "FILL"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    let base = runner.place_on_field(0, "GUIL", Some(0));
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "[Guilmon]: Cost 0 resolves with no memory"
    );
    assert_eq!(runner.game.memory, 0, "cost 0 route pays nothing");
}

/// … but a name that merely CONTAINS "Guilmon" does not satisfy the exact-name
/// gate (DCGO EqualsCardName), and an off-colour Lv.3 has no standard route.
#[test]
fn ex8_009_near_name_base_has_no_route() {
    let mut runner = builder()
        .add_card(lv3_named("NEAR", "Guilmon X", vec![CardColor::Blue]))
        .hand(0, &[CARD_ID])
        .deck(0, &["PAD-1", "PAD-2", "PAD-3", "FILL"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    let base = runner.place_on_field(0, "NEAR", Some(0));
    let mask = build_action_mask(&runner.game, 0);
    assert!(
        !(mask[encode_digivolve(0, base.index as u16) as usize] > 0.0),
        "\"Guilmon X\" is not [Guilmon]"
    );
}

// ─── Inherited: [Your Turn][OPT] opp Digimon deleted -> +1 memory ───────────

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Blue];
    c
}

fn memory_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .add_card(lv3_named("CARRIER", "Carrier", vec![CardColor::Red]))
        .add_card(opp_digimon("OPP-A"))
        .add_card(opp_digimon("OPP-B"))
        .deck(0, &["PAD-1", "PAD-2", "PAD-3"])
        .deck(1, &["PAD-1", "PAD-2", "PAD-3"])
        .memory(0)
        .start();
    let _carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let a = runner.place_on_field(1, "OPP-A", Some(0));
    let b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.game.set_memory(0);
    (runner, a, b)
}

#[test]
fn ex8_009_inherited_gains_1_memory_when_opponent_digimon_is_deleted_once_per_turn() {
    let (mut runner, a, b) = memory_runner();
    let before = runner.memory();
    runner
        .game
        .delete_permanent_with_cause(a, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before + 1, "first opp deletion gains 1");

    // B is now at a shifted index; look it up.
    let b_now = PermanentHandle {
        player: 1,
        index: runner
            .game
            .player(1)
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-B")
            .expect("OPP-B still present") as u8,
    };
    let _ = b;
    runner
        .game
        .delete_permanent_with_cause(b_now, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before + 1, "[Once Per Turn]: no second gain");
}

/// Deletion by an effect counts too (DCGO has no cause gate here).
#[test]
fn ex8_009_inherited_counts_effect_deletion() {
    let (mut runner, a, _b) = memory_runner();
    let before = runner.memory();
    runner
        .game
        .delete_permanent_with_cause(a, ReplacementCause::OwnEffect);
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before + 1);
}

/// Own Digimon deleted -> nothing.
#[test]
fn ex8_009_inherited_ignores_own_digimon_deletion() {
    let (mut runner, _a, _b) = memory_runner();
    let own = runner.place_on_field(0, "CARRIER", Some(0));
    let before = runner.memory();
    runner
        .game
        .delete_permanent_with_cause(own, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before, "only the opponent's deletions count");
}

/// [Your Turn]: on the opponent's turn, their Digimon being deleted gains nothing.
#[test]
fn ex8_009_inherited_is_your_turn_only() {
    let (mut runner, _a, _b) = memory_runner();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    runner.game.set_memory(0);
    let target = PermanentHandle {
        player: 1,
        index: 0,
    };
    let before = runner.memory();
    runner
        .game
        .delete_permanent_with_cause(target, ReplacementCause::Battle);
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before, "inherited effect is [Your Turn] only");
}
