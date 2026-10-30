//! ST21-09 Lillymon — Lv.5, Green, DP 7000, Cost 7. Traits: Fairy, ADVENTURE.
//!
//! Printed (official Bandai DB, data/card_bundles/ST21-09.md):
//!   Digivolve: Green Lv.4 / Cost 3; [Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3
//!   [On Play] [When Digivolving] Suspend all of your opponent's 5000 DP or
//!     lower Digimon. Then, for every 2 colors your Tamers have, return 1 of
//!     their suspended Digimon to the bottom of the deck.
//!   [Your Turn] [Once Per Turn] shared ADVENTURE observer (see st20_04.rs).
//!   Inherited: <Alliance>
//! DCGO: ST21/Green/ST21_09.cs — SuspendPermanentsClass over opp Digimon with
//! DP <= 5000; then mandatory pick of exactly min(suspended opp Digimon,
//! floor(colours/2)) → PutLibraryBottom.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures (shared ADVENTURE Lv.5 test idiom — see st20/st20_04.rs) ───────

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, colors: &[CardColor], traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = colors.to_vec();
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("card YAML in embedded pack")
        .add_card(digimon("ADV", CardColor::Red, 4, 4000, &["ADVENTURE"]))
        .add_card(digimon("ALLY", CardColor::Red, 4, 5000, &["Beast"]))
        .add_card(tamer("T-RB", &[CardColor::Red, CardColor::Blue], &["ADVENTURE"]))
        .add_card(tamer("T-YG", &[CardColor::Yellow, CardColor::Green], &["ADVENTURE"]))
        .add_card(tamer("T-R", &[CardColor::Red], &["ADVENTURE"]))
        .add_card(digimon("SEC", CardColor::Blue, 3, 1000, &[]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
}

fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

fn field_action(index: u8) -> u16 {
    encode_attack(0, index as u16)
}

fn play_other(runner: &mut DebugRunner, player: u8, id: &str) -> PermanentHandle {
    let h = runner.place_on_field(player, id, None);
    runner.fire_play_event_triggers(player, h.index as usize, false, false);
    h
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

fn is_alliance_prompt(runner: &DebugRunner) -> bool {
    runner
        .pending_selection_view()
        .map(|v| v.kind == SelectionKind::OwnField && v.prompt.contains("Alliance"))
        .unwrap_or(false)
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.player(h.player).battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn stack_len(runner: &DebugRunner, h: PermanentHandle) -> usize {
    runner.game.player(h.player).battle_area[h.index as usize]
        .card_sources
        .len()
}

/// Drive a mandatory multi-pick to completion, asserting PASS is never legal
/// before `n` picks are made; returns the number of picks made.
fn pick_first_n(runner: &mut DebugRunner, n: usize) {
    for i in 0..n {
        let v = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("pick {}/{} must be pending", i + 1, n));
        assert!(
            !v.valid_action_ids.contains(&PASS) && !v.is_optional,
            "mandatory pick {}/{} must not allow PASS",
            i + 1,
            n
        );
        let a = v.valid_action_ids[0];
        runner.execute_action(0, a).expect("pick");
    }
}

/// Printed structure shared by every ADVENTURE Lv.5 in this family.
fn assert_adventure_lv5_structure(name: &str, traits: &[&str]) {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, name);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for t in traits {
        assert!(card.traits.iter().any(|x| x == t), "missing trait {t}");
    }
    let digi: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2, "colour Lv.4 circle + printed Lv.4 alt box");
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(3))));
}

/// Inherited <Alliance> on the carrier.
fn assert_inherited_alliance() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;
    let host = runner.place_stack(0, &[CARD_ID, "ALLY"]);
    assert!(runner.game.has_keyword(host, Keyword::Alliance));
}

/// Shared [Your Turn][OPT] observer smoke: an own other [ADVENTURE] play →
/// mandatory Alliance pick → optional attack (declined); OPT holds.
fn assert_shared_observer() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    play_other(&mut runner, 0, "ADV");
    assert!(is_alliance_prompt(&runner), "Alliance pick must install");
    let v = runner.pending_selection_view().unwrap();
    assert!(!v.is_optional, "Alliance grant is mandatory");
    runner.execute_action(0, field_action(ally.index)).unwrap();
    assert!(runner.game.has_keyword(ally, Keyword::Alliance));
    let v = runner.pending_selection_view().expect("may-attack pick");
    assert!(v.is_optional, "'may attack' is optional");
    assert!(v.valid_action_ids.contains(&field_action(me.index)));
    runner.execute_action(0, PASS).unwrap();
    assert!(runner.pending_selection_view().is_none());
    assert_eq!(runner.security_count(1), 3);
    // Once per turn.
    play_other(&mut runner, 0, "ADV");
    assert!(runner.pending_selection_view().is_none(), "[Once Per Turn]");
    runner.end_turn();
    assert!(!runner.game.has_keyword(ally, Keyword::Alliance), "for the turn");
}

/// The observer ignores Digimon without [ADVENTURE].
fn assert_shared_observer_ignores_non_adventure() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    play_other(&mut runner, 0, "ALLY");
    assert!(runner.pending_selection_view().is_none());
}

const CARD_ID: &str = "ST21-09";

fn builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(digimon("OPP3K", CardColor::Blue, 3, 3000, &[]))
        .add_card(digimon("OPP5K", CardColor::Blue, 4, 5000, &[]))
        .add_card(digimon("OPP6K", CardColor::Blue, 4, 6000, &[]))
        .add_card(digimon("OPP7K", CardColor::Blue, 5, 7000, &[]))
}

fn suspended(runner: &DebugRunner, id: &str) -> bool {
    runner
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .map(|p| p.is_suspended)
        .unwrap_or_else(|| panic!("{id} not on opp field"))
}

fn on_field(runner: &DebugRunner, id: &str) -> bool {
    runner
        .game
        .player(1)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

fn deck_bottom_ids(runner: &DebugRunner, n: usize) -> Vec<String> {
    let deck = &runner.game.player(1).deck;
    deck.iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect::<Vec<_>>()
}

#[test]
fn st21_09_structure() {
    assert_adventure_lv5_structure("Lillymon", &["Fairy", "ADVENTURE"]);
    assert_inherited_alliance();
}

/// Suspends exactly the opponent's ≤5000 DP Digimon; with 4 Tamer colours
/// 2 suspended Digimon (incl. a previously-suspended high-DP one) must be
/// returned to the deck — a mandatory exactly-2 pick over SUSPENDED Digimon.
#[test]
fn st21_09_on_play_suspends_low_dp_then_bottom_decks_per_two_colors() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    runner.place_on_field(0, "T-YG", Some(0));
    for id in ["OPP3K", "OPP5K", "OPP6K", "OPP7K"] {
        runner.place_on_field(1, id, Some(0));
    }
    let i7 = runner.game.player(1).battle_area.len() - 1;
    runner.game.players[1].battle_area[i7].is_suspended = true;
    let deck_before = runner.game.player(1).deck.len();
    let lilly = runner.place_on_field(0, CARD_ID, None);
    fire_on_play(&mut runner, lilly);

    assert!(suspended(&runner, "OPP3K") && suspended(&runner, "OPP5K"));
    assert!(!suspended(&runner, "OPP6K"), "6000 DP is not suspended");

    let v = runner.pending_selection_view().expect("bottom-deck pick");
    assert_eq!(
        v.valid_action_ids.iter().filter(|a| **a != PASS).count(),
        3,
        "3 suspended opp Digimon are candidates (6000 DP unsuspended is not)"
    );
    pick_first_n(&mut runner, 2);
    runner.auto_resolve().unwrap();
    assert_eq!(runner.game.player(1).battle_area.len(), 2);
    assert_eq!(runner.game.player(1).deck.len(), deck_before + 2);
    assert!(on_field(&runner, "OPP6K"));
}

/// 1 Tamer colour → suspend only, no return.
#[test]
fn st21_09_one_color_suspends_but_returns_nothing() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-R", Some(0));
    runner.place_on_field(1, "OPP3K", Some(0));
    let lilly = runner.place_stack(0, &["ALLY", CARD_ID]);
    fire_when_digivolving(&mut runner, lilly);
    assert!(runner.pending_selection_view().is_none());
    assert!(suspended(&runner, "OPP3K"));
    assert!(on_field(&runner, "OPP3K"));
}

#[test]
fn st21_09_shared_observer() {
    assert_shared_observer();
    assert_shared_observer_ignores_non_adventure();
}
