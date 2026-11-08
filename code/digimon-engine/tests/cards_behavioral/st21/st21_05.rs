//! ST21-05 Angemon — Digimon, Lv.4, Yellow, DP 4000, Cost 4.
//! Traits: Angel / ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (official bundle `data/card_bundles/ST21-05.md`)
//!
//! ```text
//! Digivolve: Yellow Lv.3 / cost 2.
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2
//!
//! [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer card
//! with the [ADVENTURE] trait from your hand without paying the cost.
//!
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon
//! gets -2000 DP for the turn.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Yellow/ST21_05.cs` — WD optional,
//! CanActivate: ADVENTURE Tamer in hand && own battle-area Tamer count <= 1;
//! `SelectHandEffect(canNoSelect: true, maxCount 1)` → `PlayPermanentCards(
//! payCost: false)`. Inherited OnAllyAttack maxCount 1: mandatory pick of an
//! opponent Digimon → `ChangeDigimonDP(-2000, UntilEachTurnEnd)`.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST21-05";

fn tamer(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut adv_lv3 = make_test_card("ADV-LV3", "Adv Rookie");
    adv_lv3.colors = vec![CardColor::Purple];
    adv_lv3.traits = vec!["ADVENTURE".to_string()];
    let mut adv_digi = make_test_card("ADV-DIGI", "Adv Digimon");
    adv_digi.traits = vec!["ADVENTURE".to_string()];
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(5);
    carrier.dp = Some(6000);
    let mut opp = make_test_card("OPP", "Opp");
    opp.dp = Some(5000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-05 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("ADV-TAMER", true))
        .add_card(tamer("ADV-TAMER-2", true))
        .add_card(tamer("PLAIN-TAMER", false))
        .add_card(adv_lv3)
        .add_card(adv_digi)
        .add_card(carrier)
        .add_card(opp)
        .add_card(make_test_card("FILL", "Fill"))
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn hand_actions(runner: &DebugRunner, id: &str) -> [u16; 2] {
    let slot = hand_index(runner, id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
}

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

fn on_field(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

/// Digivolve Angemon onto an ADVENTURE Lv.3 with `tamers` already in play.
fn digivolve_with(tamers: &[&str], hand: &[&str]) -> DebugRunner {
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
    let base = runner.place_on_field(0, "ADV-LV3", Some(0));
    let slot = hand_index(&runner, CARD_ID) as u16;
    runner.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
    assert_eq!(runner.game.players[base.player as usize].battle_area[base.index as usize].top_card().card_id(&runner.game.card_data), CARD_ID);
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st21_05_identity_paths_and_clauses() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((card.level, card.dp, card.cost), (Some(4), Some(4000), Some(4)));
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 2);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let wd = triggered.iter().find(|t| t.scope == CompiledScope::FaceUp).unwrap();
    assert_eq!(wd.when, vec![CompiledTiming::WhenDigivolving], "[When Digivolving] only");
    let inh = triggered.iter().find(|t| t.scope == CompiledScope::Inherited).unwrap();
    assert_eq!(inh.when, vec![CompiledTiming::WhenAttacking]);
    assert!(inh.once_per_turn);
}

// ─── Section 2/3 — [When Digivolving] free-play an ADVENTURE Tamer ───────────

#[test]
fn st21_05_with_no_tamers_plays_an_adventure_tamer_for_free() {
    let mut runner = digivolve_with(&[], &["ADV-TAMER"]);
    let before = runner.memory();
    let view = walk_to_hand_prompt(&mut runner).expect("play prompt");
    assert!(view.is_optional, "'you may'");
    let action = hand_actions(&runner, "ADV-TAMER")
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .expect("ADV-TAMER is offered");
    runner.execute_action(view.selecting_player, action).expect("play tamer");
    let _ = runner.auto_resolve();
    assert!(on_field(&runner, "ADV-TAMER"));
    assert_eq!(before, runner.memory(), "without paying the cost");
}

#[test]
fn st21_05_with_one_tamer_still_fires() {
    let mut runner = digivolve_with(&["PLAIN-TAMER"], &["ADV-TAMER"]);
    let view = walk_to_hand_prompt(&mut runner).expect("play prompt");
    assert!(hand_actions(&runner, "ADV-TAMER").iter().any(|a| view.valid_action_ids.contains(a)));
}

#[test]
fn st21_05_with_two_tamers_does_not_fire() {
    let mut runner = digivolve_with(&["PLAIN-TAMER", "ADV-TAMER-2"], &["ADV-TAMER"]);
    assert!(walk_to_hand_prompt(&mut runner).is_none(), "2 Tamers: condition fails");
    let _ = runner.auto_resolve();
    assert!(!on_field(&runner, "ADV-TAMER"));
}

#[test]
fn st21_05_only_adventure_tamers_are_offered() {
    let mut runner = digivolve_with(&[], &["ADV-TAMER", "PLAIN-TAMER", "ADV-DIGI"]);
    let view = walk_to_hand_prompt(&mut runner).expect("play prompt");
    assert!(!hand_actions(&runner, "PLAIN-TAMER").iter().any(|a| view.valid_action_ids.contains(a)));
    assert!(!hand_actions(&runner, "ADV-DIGI").iter().any(|a| view.valid_action_ids.contains(a)));
}

#[test]
fn st21_05_play_is_optional() {
    let mut runner = digivolve_with(&[], &["ADV-TAMER"]);
    for _ in 0..4 {
        let Some(view) = runner.pending_selection_view() else { break };
        assert!(view.is_optional);
        runner.execute_action(view.selecting_player, PASS).expect("decline");
    }
    assert!(!on_field(&runner, "ADV-TAMER"));
}

#[test]
fn st21_05_on_play_does_not_fire() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "ADV-TAMER"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.play(0, 0).expect("Angemon plays");
    assert!(runner.pending_selection().is_none(), "no [On Play] effect");
}

// ─── Section 4 — inherited [When Attacking][OPT] -2000 DP ────────────────────

#[test]
fn st21_05_inherited_when_attacking_gives_minus_2000_for_the_turn() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp: PermanentHandle = runner.place_on_field(1, "OPP", Some(0));
    runner.attack_player(carrier, 1, false);
    let view = runner.pending_selection_view().expect("target prompt");
    assert!(!view.is_optional, "mandatory");
    let action = encode_attack(0, opp.index as u16);
    assert!(view.valid_action_ids.contains(&action), "view={view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick");
    assert_eq!(runner.effective_dp(opp), Some(3000));
    let _ = runner.auto_resolve();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(5000), "for the turn");
}

#[test]
fn st21_05_inherited_is_once_per_turn() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(3000));
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(opp), Some(3000), "second attack: no further -2000");
}
