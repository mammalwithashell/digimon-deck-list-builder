//! BT21-078 WereGarurumon — Lv.5, Purple, DP 7000, Cost 7. Traits: Beastkin, ADVENTURE.
//!
//! Printed (official Bandai DB, data/card_bundles/BT21-078.md):
//!   Digivolve: Purple Lv.4 / Cost 3;
//!   [Digivolve] Lv.4 w/[Garurumon] in name or w/[ADVENTURE] trait: Cost 3
//!   [On Play] [When Digivolving] Delete 1 of your opponent's level 4 or lower
//!     Digimon. If your Tamers have 2 or more total colors, add 1 to this
//!     effect's level maximum.
//!   [Your Turn] [Once Per Turn] shared ADVENTURE observer (see st20_04.rs).
//!   Inherited: <Alliance>
//! DCGO: BT21/Purple/BT21_078.cs — mandatory delete pick over opp Digimon
//! with Level <= (colours >= 2 ? 5 : 4).

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

const CARD_ID: &str = "BT21-078";

fn builder() -> DebugRunnerBuilder {
    base_builder()
        .add_card(digimon("L4", CardColor::Blue, 4, 4000, &[]))
        .add_card(digimon("L5", CardColor::Blue, 5, 6000, &[]))
        .add_card(digimon("L6", CardColor::Blue, 6, 9000, &[]))
}

fn opp_ids(runner: &DebugRunner) -> Vec<String> {
    runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn bt21_078_structure() {
    assert_adventure_lv5_structure("WereGarurumon", &["Beastkin", "ADVENTURE"]);
    assert_inherited_alliance();
}

/// Alt box accepts a non-purple Lv.4 with [Garurumon] in its name.
#[test]
fn bt21_078_digivolves_from_lv4_garurumon_name_for_cost_3() {
    let mut garu = digimon("GARU", CardColor::Blue, 4, 4000, &[]);
    garu.card_name = "Garurumon".to_string();
    let mut runner = builder().add_card(garu).start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "GARU", Some(0));
    let idx = put_in_hand(&mut runner, 0, CARD_ID);
    let mem = runner.game.memory;
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    assert_eq!(runner.game.memory, mem - 3);
}

/// 1 Tamer colour → level max 4: only the Lv.4 is a (mandatory) candidate.
#[test]
fn bt21_078_one_color_deletes_level_4_or_lower_only() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-R", Some(0));
    for id in ["L4", "L5", "L6"] {
        runner.place_on_field(1, id, Some(0));
    }
    let wg = runner.place_on_field(0, CARD_ID, None);
    fire_on_play(&mut runner, wg);
    let v = runner.pending_selection_view().expect("delete pick");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(!v.is_optional, "the deletion is mandatory");
    assert_eq!(v.valid_action_ids, vec![encode_attack(0, 0)], "only L4");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    runner.auto_resolve().unwrap();
    assert_eq!(opp_ids(&runner), vec!["L5", "L6"]);
}

/// 2+ Tamer colours → level max 5: L4 and L5 are candidates, L6 is not.
#[test]
fn bt21_078_two_colors_raise_level_max_to_5() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    for id in ["L4", "L5", "L6"] {
        runner.place_on_field(1, id, Some(0));
    }
    let wg = runner.place_stack(0, &["ALLY", CARD_ID]);
    fire_when_digivolving(&mut runner, wg);
    let v = runner.pending_selection_view().expect("delete pick");
    assert_eq!(v.valid_action_ids.len(), 2, "L4 + L5");
    assert!(!v.valid_action_ids.contains(&encode_attack(0, 2)), "L6 excluded");
    runner.execute_action(0, encode_attack(0, 1)).unwrap();
    runner.auto_resolve().unwrap();
    assert_eq!(opp_ids(&runner), vec!["L4", "L6"]);
}

#[test]
fn bt21_078_shared_observer() {
    assert_shared_observer();
    assert_shared_observer_ignores_non_adventure();
}
