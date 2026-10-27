//! ST20-03 Birdramon — Digimon, Lv.4, Red, DP 4000, Cost 4.
//! Traits: Giant Bird / ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (official bundle `data/card_bundles/ST20-03.md`)
//!
//! ```text
//! Digivolve: Red Lv.3 / cost 2.
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2
//!
//! [On Play] [When Digivolving] If your Tamers with the [ADVENTURE] trait have
//! 3 or more total colors, this Digimon may digivolve into a Digimon card with
//! the [ADVENTURE] trait in the hand without paying the cost.
//!
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_03.cs` — the colour count only
//! includes Tamers whose top card HasAdventureTraits
//! (`GetDifferenetColorCardCount(adventureTamers) >= 3`); body
//! `DigivolveIntoHandOrTrashCard(this, IsDigimon && HasAdventureTraits,
//! payCost: false, ignoreDigivolutionRequirementFixedCost: -1)` (requirements
//! still apply). Optional (`SetUpActivateClass(..., true, ...)`).
//!
//! The shared body is also authored on ST21-08 Togemon (see `st21_08.rs`).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST20-03";
const CARD_COLOR: u8 = 0; // Red (evo-cost colour index)

fn tamer(id: &str, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// Lv.5 that digivolves from a `CARD_COLOR` Lv.4 for 3.
fn lv5(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.evo_costs = vec![EvoCost { card_color: CARD_COLOR, level: 4, memory_cost: 3 }];
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut adv_lv3 = make_test_card("ADV-LV3", "Adv Rookie");
    adv_lv3.colors = vec![CardColor::Purple];
    adv_lv3.traits = vec!["ADVENTURE".to_string()];
    let mut plain_lv3 = make_test_card("PLAIN-LV3", "Plain Rookie");
    plain_lv3.colors = vec![CardColor::Purple];
    let mut wrong_route = lv5("ADV-NO-ROUTE", true);
    wrong_route.evo_costs = vec![EvoCost { card_color: 6, level: 4, memory_cost: 3 }];
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(5);
    carrier.dp = Some(6000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-03 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("T-RED", CardColor::Red, true))
        .add_card(tamer("T-BLUE", CardColor::Blue, true))
        .add_card(tamer("T-GREEN", CardColor::Green, true))
        .add_card(tamer("T-GREEN-PLAIN", CardColor::Green, false))
        .add_card(lv5("ADV-LV5", true))
        .add_card(lv5("PLAIN-LV5", false))
        .add_card(wrong_route)
        .add_card(adv_lv3)
        .add_card(plain_lv3)
        .add_card(carrier)
        .add_card(make_test_card("FILL", "Fill"))
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize].top_card().card_id(&runner.game.card_data).to_string()
}

fn hand_actions(runner: &DebugRunner, id: &str) -> [u16; 2] {
    let slot = hand_index(runner, id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
}

/// Accept any leading yes/no gate and return the hand prompt's view.
fn walk_to_hand_prompt(runner: &mut DebugRunner) -> Option<digimon_engine::selection::PendingSelectionView> {
    for _ in 0..4 {
        let view = runner.pending_selection_view()?;
        if view.kind == SelectionKind::Hand {
            return Some(view);
        }
        let action = view.valid_action_ids.iter().copied().find(|&a| a != PASS)?;
        runner.execute_action(view.selecting_player, action).expect("accept gate");
    }
    None
}

fn offered(runner: &mut DebugRunner, id: &str) -> bool {
    let Some(view) = walk_to_hand_prompt(runner) else { return false };
    hand_actions(runner, id).iter().any(|a| view.valid_action_ids.contains(a))
}

fn pick(runner: &mut DebugRunner, id: &str) {
    let view = walk_to_hand_prompt(runner).expect("hand prompt");
    let action = hand_actions(runner, id)
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable; view={view:?}"));
    runner.execute_action(view.selecting_player, action).expect("pick");
}

/// Play the card from hand with the given Tamers on the field.
fn play_with_tamers(tamers: &[&str], hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut all = vec![CARD_ID];
    all.extend_from_slice(hand);
    let mut runner = builder()
        .hand(0, &all)
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    for t in tamers {
        runner.place_on_field(0, t, None);
    }
    let slot = hand_index(&runner, CARD_ID);
    let field = runner.play(0, slot).expect("card plays");
    (runner, PermanentHandle { player: 0, index: field as u8 })
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st20_03_identity_paths_and_clauses() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((card.level, card.dp, card.cost), (Some(4), Some(4000), Some(4)));
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 2, "Red Lv.3/2 circle + Lv.3 w/[ADVENTURE]/2 box");
    let face: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(face.len(), 1);
    assert!(face[0].when.contains(&CompiledTiming::OnPlay));
    assert!(face[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!face[0].once_per_turn);
    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope, dp_modifier, active_when, ..
            }) => Some((*scope, *dp_modifier, active_when.is_some())),
            _ => None,
        })
        .collect();
    assert_eq!(auras, vec![(CompiledScope::Inherited, Some(2000), true)], "[Your Turn] +2000");
}

// ─── Section 2/3 — [On Play] conditional free digivolve ──────────────────────

#[test]
fn st20_03_on_play_with_three_adventure_tamer_colors_digivolves_for_free() {
    let (mut runner, me) = play_with_tamers(&["T-RED", "T-BLUE", "T-GREEN"], &["ADV-LV5"]);
    let before = runner.memory();
    pick(&mut runner, "ADV-LV5");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, me), "ADV-LV5", "digivolved into the [ADVENTURE] card");
    assert_eq!(before, runner.memory(), "without paying the cost");
}

#[test]
fn st20_03_digivolve_is_optional() {
    let (mut runner, me) = play_with_tamers(&["T-RED", "T-BLUE", "T-GREEN"], &["ADV-LV5"]);
    let view = runner.pending_selection_view().expect("may-digivolve prompt");
    assert!(view.is_optional, "'may' must expose PASS");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, me), CARD_ID);
    assert!(hand_ids(&runner).contains(&"ADV-LV5".to_string()));
}

#[test]
fn st20_03_non_adventure_tamer_colors_do_not_count() {
    // Red + Blue ADVENTURE Tamers plus a GREEN non-ADVENTURE Tamer: 3 Tamer
    // colours in total but only 2 among [ADVENTURE] Tamers.
    let (runner, me) = play_with_tamers(&["T-RED", "T-BLUE", "T-GREEN-PLAIN"], &["ADV-LV5"]);
    assert!(runner.pending_selection().is_none(), "condition fails: no prompt");
    assert_eq!(top_id(&runner, me), CARD_ID);
}

#[test]
fn st20_03_two_adventure_tamer_colors_is_not_enough() {
    let (runner, me) = play_with_tamers(&["T-RED", "T-BLUE"], &["ADV-LV5"]);
    assert!(runner.pending_selection().is_none());
    assert_eq!(top_id(&runner, me), CARD_ID);
}

#[test]
fn st20_03_only_adventure_digimon_with_a_legal_route_are_offered() {
    let (mut runner, _me) =
        play_with_tamers(&["T-RED", "T-BLUE", "T-GREEN"], &["ADV-LV5", "PLAIN-LV5", "ADV-NO-ROUTE"]);
    assert!(!offered(&mut runner, "PLAIN-LV5"), "no [ADVENTURE] trait");
    assert!(!offered(&mut runner, "ADV-NO-ROUTE"), "requirements are not ignored");
    assert!(offered(&mut runner, "ADV-LV5"));
}

#[test]
fn st20_03_when_digivolving_also_fires() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "ADV-LV5"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    for t in ["T-RED", "T-BLUE", "T-GREEN"] {
        runner.place_on_field(0, t, None);
    }
    let base = runner.place_on_field(0, "ADV-LV3", Some(0));
    let before = runner.memory();
    let slot = hand_index(&runner, CARD_ID) as u16;
    runner.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
    assert_eq!(before - runner.memory(), 2, "Lv.3 w/[ADVENTURE] box: cost 2");
    pick(&mut runner, "ADV-LV5");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, base), "ADV-LV5");
    assert_eq!(before - runner.memory(), 2, "second digivolve is free");
}

#[test]
fn st20_03_cannot_digivolve_from_a_plain_off_color_lv3() {
    let mut runner = builder().hand(0, &[CARD_ID]).deck(0, &["FILL", "FILL"]).memory(10).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "PLAIN-LV3", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(top_id(&runner, base), "PLAIN-LV3");
}

// ─── Section 4 — inherited ───────────────────────────────────────────────────

#[test]
fn st20_03_inherited_plus_2000_on_your_turn_only() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(runner.effective_dp(carrier), Some(6000 + 2000));
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(carrier), Some(6000));
}
