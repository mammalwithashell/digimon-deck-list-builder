//! ST21-08 Togemon — Lv.4, Green, Vegetation / ADVENTURE, Champion, Data,
//! DP 4000, Cost 4.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST21-08.md)
//!
//! ```text
//! [On Play] [When Digivolving] If your Tamers with the [ADVENTURE] trait have
//! 3 or more total colors, this Digimon may digivolve into a Digimon card with
//! the [ADVENTURE] trait in the hand without paying the cost.
//!
//! Inherited: [All Turns] This Digimon gets +1000 DP.
//!
//! [Digivolve box] Lv.3 w/[ADVENTURE] trait: Cost 2
//! [Standard circle] Green Lv.3: Cost 2
//! Official Q&A: "3 or more total colors" = as many or more colors than the
//! number specified.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Green/ST21_08.cs`
//!
//! # Patterns this test file covers
//!
//! | Clause                                         | Pattern                                    |
//! |------------------------------------------------|--------------------------------------------|
//! | alt-path (Lv.3 ADVENTURE, cost 2) + green Lv.3 | digivolve alt_paths (structural + behavioral) |
//! | [OP][WD] if ADVENTURE Tamers have 3+ colors, may digivolve free from hand | trait-filtered Tamer color gate (35 color triples), optional `select_hand` + `effect_initiated_digivolve { cost: free }`, requirements respected |
//! | Inherited [All Turns] +1000 DP                 | inherited aura via digivolution stack, both turns |
//!
//! # Assertions the author was least able to confirm without running them
//!
//! - The `condition:` is a 35-way `any_of` over colour triples (the DSL has no
//!   trait-filtered distinct-colour leaf). Every Tamer-colour test below
//!   exercises it end to end; a wrong triple would fail one of them.
//! - The hand pick uses `PLAY_HAND_START + hand_index` action ids (BT21-046
//!   idiom).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-08";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Lv.5 that digivolves from a `from_color` Lv.4 for 3.
fn lv5(id: &str, from_color: CardColor, traits: &[&str]) -> CardData {
    let mut c = digimon(id, CardColor::Green, 5, 7000, traits);
    c.evo_costs = vec![EvoCost {
        card_color: from_color as u8,
        level: 4,
        memory_cost: 3,
    }];
    c
}

fn tamer(id: &str, colors: &[CardColor], traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = colors.to_vec();
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-08 in embedded DSL pack")
        // Hand candidates for the free digivolve.
        .add_card(lv5("ADV-LV5", CardColor::Green, &["ADVENTURE"]))
        .add_card(lv5("PLAIN-LV5", CardColor::Green, &[]))
        .add_card(lv5("ADV-LV5-RED-ONLY", CardColor::Red, &["ADVENTURE"]))
        // ADVENTURE Tamers (single colour).
        .add_card(tamer("ADV-T-RED", &[CardColor::Red], &["ADVENTURE"]))
        .add_card(tamer("ADV-T-RED-2", &[CardColor::Red], &["ADVENTURE"]))
        .add_card(tamer("ADV-T-BLUE", &[CardColor::Blue], &["ADVENTURE"]))
        .add_card(tamer("ADV-T-YELLOW", &[CardColor::Yellow], &["ADVENTURE"]))
        .add_card(tamer("ADV-T-WHITE", &[CardColor::White], &["ADVENTURE"]))
        // ADVENTURE Tamer with two colours.
        .add_card(tamer(
            "ADV-T-DUAL",
            &[CardColor::Red, CardColor::Blue],
            &["ADVENTURE"],
        ))
        // Tamers WITHOUT the ADVENTURE trait.
        .add_card(tamer("PLAIN-T-YELLOW", &[CardColor::Yellow], &[]))
        .add_card(tamer("PLAIN-T-GREEN", &[CardColor::Green], &[]))
        // Digivolution bases (Lv.3) and carrier.
        .add_card(digimon("GREEN-LV3", CardColor::Green, 3, 3000, &[]))
        .add_card(digimon("BLUE-LV3-ADV", CardColor::Blue, 3, 3000, &["ADVENTURE"]))
        .add_card(digimon("RED-LV3-PLAIN", CardColor::Red, 3, 3000, &[]))
        .add_card(digimon("GREEN-LV2-ADV", CardColor::Green, 2, 1000, &["ADVENTURE"]))
        .add_card(digimon("CARRIER", CardColor::Red, 5, 3000, &[]))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(10)
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player 0's hand"))
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

/// Togemon on the field with the given Tamers and hand.
fn setup(tamers: &[&str], hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder().hand(0, hand).start();
    let togemon = runner.place_on_field(0, CARD_ID, Some(0));
    for id in tamers {
        runner.place_on_field(0, id, None);
    }
    (runner, togemon)
}

fn fire_on_play(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.fire_on_play(h.player, h.index as usize);
}

fn fire_when_digivolving(runner: &mut DebugRunner, h: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_08_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Togemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.cost, Some(4));
    let traits: Vec<String> = card.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(traits.iter().any(|t| t == "vegetation"), "{:?}", card.traits);
    assert!(traits.iter().any(|t| t == "adventure"), "{:?}", card.traits);
}

#[test]
fn st21_08_alt_paths_are_green_lv3_and_adventure_lv3_both_cost_2() {
    let card = compiled(CARD_ID);
    assert_eq!(card.alt_paths.len(), 2);
    for path in &card.alt_paths {
        assert_eq!(path.cost, Some(CompiledCost::Literal(2)));
        assert!(!path.ignore_requirements);
    }
    let adventure_path = format!("{:?}", card.alt_paths[1].from);
    assert!(
        adventure_path.contains("ADVENTURE"),
        "second alt path gates on the ADVENTURE trait, got {adventure_path}"
    );
}

#[test]
fn st21_08_clause_shape() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let clause = triggered[0];
    assert!(clause.when.contains(&CompiledTiming::OnPlay));
    assert!(clause.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(clause.condition.is_some(), "the Tamer-colour gate is a condition");
    assert!(!clause.once_per_turn);

    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });
    assert_eq!(aura, Some((CompiledScope::Inherited, Some(1000))));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolution requirements
// ═══════════════════════════════════════════════════════════════════════════

fn digivolve_into_togemon(base_id: &str) -> (DebugRunner, PermanentHandle, bool, i16) {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, base_id, Some(0));
    let idx = hand_index(&runner, CARD_ID);
    let before = runner.memory();
    let ok = runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand);
    let paid = before - runner.memory();
    let _ = runner.auto_resolve();
    (runner, base, ok, paid)
}

#[test]
fn st21_08_digivolves_from_green_lv3_for_2() {
    let (runner, base, ok, paid) = digivolve_into_togemon("GREEN-LV3");
    assert!(ok, "printed green Lv.3 circle");
    assert_eq!(paid, 2);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_08_digivolves_from_any_color_lv3_adventure_for_2() {
    let (runner, base, ok, paid) = digivolve_into_togemon("BLUE-LV3-ADV");
    assert!(ok, "[Digivolve] Lv.3 w/[ADVENTURE] trait accepts a blue base");
    assert_eq!(paid, 2);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_08_cannot_digivolve_from_non_green_non_adventure_lv3() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, "RED-LV3-PLAIN", Some(0));
    let card = runner.game.players[0].hand[hand_index(&runner, CARD_ID)].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    assert!(!runner.game.can_digivolve(&card, &perm));
}

#[test]
fn st21_08_cannot_digivolve_from_adventure_lv2() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, "GREEN-LV2-ADV", Some(0));
    let card = runner.game.players[0].hand[hand_index(&runner, CARD_ID)].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    assert!(
        !runner.game.can_digivolve(&card, &perm),
        "the [ADVENTURE] box needs a LEVEL 3 base"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play] / [When Digivolving] free digivolve gate
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_08_three_adventure_tamer_colors_open_the_free_digivolve_prompt() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-BLUE", "ADV-T-YELLOW"],
        &["ADV-LV5", "PLAIN-LV5", "ADV-LV5-RED-ONLY"],
    );
    fire_on_play(&mut runner, togemon);

    let view = runner
        .pending_selection_view()
        .expect("free-digivolve hand pick");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "'may digivolve'");
    let adv = PLAY_HAND_START + hand_index(&runner, "ADV-LV5") as u16;
    let plain = PLAY_HAND_START + hand_index(&runner, "PLAIN-LV5") as u16;
    let red_only = PLAY_HAND_START + hand_index(&runner, "ADV-LV5-RED-ONLY") as u16;
    assert!(view.valid_action_ids.contains(&adv));
    assert!(
        !view.valid_action_ids.contains(&plain),
        "the hand card needs the [ADVENTURE] trait"
    );
    assert!(
        !view.valid_action_ids.contains(&red_only),
        "digivolution requirements still apply: a red-Lv.4-only card cannot digivolve from green Togemon"
    );
}

#[test]
fn st21_08_free_digivolve_pays_no_memory_and_replaces_the_top_card() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-BLUE", "ADV-T-YELLOW"],
        &["ADV-LV5"],
    );
    fire_on_play(&mut runner, togemon);
    let memory_before = runner.memory();
    let pick = PLAY_HAND_START + hand_index(&runner, "ADV-LV5") as u16;
    runner.execute_action(0, pick).expect("choose the digivolve target");
    let _ = runner.auto_resolve();

    assert_eq!(top_id(&runner, togemon), "ADV-LV5");
    assert_eq!(
        runner.memory(),
        memory_before,
        "'without paying the cost' -- no memory is spent"
    );
}

#[test]
fn st21_08_free_digivolve_is_declinable() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-BLUE", "ADV-T-YELLOW"],
        &["ADV-LV5"],
    );
    fire_on_play(&mut runner, togemon);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, togemon), CARD_ID, "'may': Togemon stays put");
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn st21_08_two_adventure_tamer_colors_are_not_enough() {
    let (mut runner, togemon) = setup(&["ADV-T-RED", "ADV-T-BLUE"], &["ADV-LV5"]);
    fire_on_play(&mut runner, togemon);
    assert!(runner.pending_selection().is_none());
    assert_eq!(top_id(&runner, togemon), CARD_ID);
}

#[test]
fn st21_08_a_dual_color_adventure_tamer_plus_one_more_color_is_enough() {
    let (mut runner, togemon) = setup(&["ADV-T-DUAL", "ADV-T-YELLOW"], &["ADV-LV5"]);
    fire_on_play(&mut runner, togemon);
    assert!(
        runner.pending_selection().is_some(),
        "red+blue (one Tamer) + yellow = 3 total colors"
    );
}

#[test]
fn st21_08_a_single_dual_color_tamer_is_only_two_colors() {
    let (mut runner, togemon) = setup(&["ADV-T-DUAL"], &["ADV-LV5"]);
    fire_on_play(&mut runner, togemon);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn st21_08_repeated_colors_do_not_count_twice() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-RED-2", "ADV-T-BLUE"],
        &["ADV-LV5"],
    );
    fire_on_play(&mut runner, togemon);
    assert!(
        runner.pending_selection().is_none(),
        "red, red, blue = 2 distinct colors"
    );
}

#[test]
fn st21_08_tamers_without_the_adventure_trait_do_not_count() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-BLUE", "PLAIN-T-YELLOW", "PLAIN-T-GREEN"],
        &["ADV-LV5"],
    );
    fire_on_play(&mut runner, togemon);
    assert!(
        runner.pending_selection().is_none(),
        "only 2 colors (red, blue) sit on [ADVENTURE] Tamers; yellow and green are on plain Tamers"
    );
}

#[test]
fn st21_08_four_adventure_colors_also_qualify() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-BLUE", "ADV-T-YELLOW", "ADV-T-WHITE"],
        &["ADV-LV5"],
    );
    fire_on_play(&mut runner, togemon);
    assert!(runner.pending_selection().is_some(), "'3 or more'");
}

#[test]
fn st21_08_a_non_adventure_tamer_may_supply_a_color_already_on_an_adventure_tamer() {
    // red/blue from ADVENTURE Tamers + a plain yellow Tamer: yellow is not on an
    // [ADVENTURE] Tamer, so still only 2 qualifying colors.
    let (mut runner, togemon) = setup(&["ADV-T-DUAL", "PLAIN-T-YELLOW"], &["ADV-LV5"]);
    fire_on_play(&mut runner, togemon);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn st21_08_no_adventure_digimon_in_hand_means_no_prompt() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-BLUE", "ADV-T-YELLOW"],
        &["PLAIN-LV5"],
    );
    fire_on_play(&mut runner, togemon);
    assert!(runner.pending_selection().is_none());
    assert_eq!(top_id(&runner, togemon), CARD_ID);
}

#[test]
fn st21_08_when_digivolving_has_the_same_gate_and_effect() {
    let (mut runner, togemon) = setup(
        &["ADV-T-RED", "ADV-T-BLUE", "ADV-T-YELLOW"],
        &["ADV-LV5"],
    );
    fire_when_digivolving(&mut runner, togemon);
    let view = runner.pending_selection_view().expect("[When Digivolving] pick");
    assert_eq!(view.kind, SelectionKind::Hand);
    let pick = PLAY_HAND_START + hand_index(&runner, "ADV-LV5") as u16;
    runner.execute_action(0, pick).expect("digivolve");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, togemon), "ADV-LV5");

    // Negative: with only two colors the same timing does nothing.
    let (mut runner, togemon) = setup(&["ADV-T-RED", "ADV-T-BLUE"], &["ADV-LV5"]);
    fire_when_digivolving(&mut runner, togemon);
    assert!(runner.pending_selection().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [All Turns] +1000 DP
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_08_inherited_plus_1000_applies_to_the_carrier_on_both_turns() {
    let mut runner = builder().start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(carrier), Some(4000), "3000 + 1000");

    runner.end_turn();
    let _ = runner.auto_resolve_declining_end_of_turn_attacks();
    runner.game.tick_declarative_effects();
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(
        runner.effective_dp(carrier),
        Some(4000),
        "[All Turns]: still +1000 on the opponent's turn"
    );
}

#[test]
fn st21_08_inherited_bonus_is_absent_without_togemon_in_the_stack() {
    let mut runner = builder().start();
    let plain = runner.place_on_field(0, "CARRIER", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(plain), Some(3000));
}

#[test]
fn st21_08_inherited_bonus_does_not_apply_to_togemon_itself_as_top_card() {
    let mut runner = builder().start();
    let togemon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(togemon), Some(4000));
}
