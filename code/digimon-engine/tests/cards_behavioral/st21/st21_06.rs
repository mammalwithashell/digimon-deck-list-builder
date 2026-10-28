//! ST21-06 MagnaAngemon — Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Archangel, ADVENTURE. Form: Ultimate. Attribute: Vaccine.
//!
//! Printed (official Bandai DB, data/card_bundles/ST21-06.md):
//!   Digivolve: Yellow Lv.4 / Cost 3; [Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3
//!   [On Play] [When Digivolving] Place 1 of your opponent's 6000 DP or lower
//!     Digimon as the top security card. For every 2 colors your Tamers have,
//!     add 2000 to this effect's DP maximum.
//!   [Your Turn] [Once Per Turn] shared ADVENTURE observer (see st20_04.rs).
//!   Inherited: <Alliance>
//! DCGO: ST21/Yellow/ST21_06.cs — OP / WD: mandatory pick (canNoSelect false)
//! of an opponent battle-area Digimon with DP <= 6000 + 2000 *
//! (distinct own-Tamer colours / 2) → IPutSecurityPermanent(toTop: true) into
//! its owner's (the opponent's) security.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "ST21-06";

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

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("card YAML in embedded pack")
        .add_card(digimon("ADV", CardColor::Red, 4, 4000, &["ADVENTURE"]))
        .add_card(digimon("ALLY", CardColor::Red, 4, 5000, &["Beast"]))
        .add_card(digimon("BLUE-ADV4", CardColor::Blue, 4, 4000, &["ADVENTURE"]))
        .add_card(digimon("BLUE-PLAIN4", CardColor::Blue, 4, 4000, &["Beast"]))
        .add_card(digimon("OPP6K", CardColor::Blue, 4, 6000, &[]))
        .add_card(digimon("OPP7K", CardColor::Blue, 5, 7000, &[]))
        .add_card(digimon("OPP8K", CardColor::Blue, 5, 8000, &[]))
        .add_card(digimon("OPP9K", CardColor::Blue, 5, 9000, &[]))
        .add_card(digimon("OPP10K", CardColor::Blue, 6, 10000, &[]))
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

fn opp_on_field(runner: &DebugRunner, id: &str) -> bool {
    runner
        .game
        .player(1)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

fn opp_index(runner: &DebugRunner, id: &str) -> u8 {
    runner
        .game
        .player(1)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not on opp field")) as u8
}

fn top_security_id(runner: &DebugRunner, player: u8) -> String {
    runner
        .game
        .player(player)
        .security
        .last()
        .expect("security not empty")
        .card_id(&runner.game.card_data)
        .to_string()
}

/// Candidate opponent ids offered by the current pending pick.
fn offered(runner: &DebugRunner, ids: &[&str]) -> Vec<String> {
    let v = runner.pending_selection_view().expect("place pick pending");
    ids.iter()
        .filter(|id| opp_on_field(runner, id))
        .filter(|id| v.valid_action_ids.contains(&field_action(opp_index(runner, id))))
        .map(|id| id.to_string())
        .collect()
}

const OPPS: [&str; 5] = ["OPP6K", "OPP7K", "OPP8K", "OPP9K", "OPP10K"];

fn setup(tamers: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    for t in tamers {
        runner.place_on_field(0, t, Some(0));
    }
    for id in OPPS {
        runner.place_on_field(1, id, Some(0));
    }
    let magna = runner.place_on_field(0, CARD_ID, None);
    (runner, magna)
}

// ─── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn st21_06_printed_stats_and_digivolve_paths() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "MagnaAngemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    for t in ["Archangel", "ADVENTURE"] {
        assert!(card.traits.iter().any(|x| x == t), "missing trait {t}");
    }
    let digi: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 2, "yellow Lv.4 circle + Lv.4 [ADVENTURE] box");
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(3))));
}

#[test]
fn st21_06_digivolve_box_lv4_adventure_for_cost_3() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "BLUE-ADV4", Some(0));
    let idx = put_in_hand(&mut runner, 0, CARD_ID);
    let mem = runner.game.memory;
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    assert_eq!(runner.game.memory, mem - 3);

    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "BLUE-PLAIN4", Some(0));
    let idx = put_in_hand(&mut runner, 0, CARD_ID);
    assert!(!runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
}

#[test]
fn st21_06_inherited_alliance() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let host = runner.place_stack(0, &[CARD_ID, "ALLY"]);
    assert!(runner.game.has_keyword(host, Keyword::Alliance));
}

// ─── Section 2: [On Play] / [When Digivolving] ───────────────────────────────

/// No Tamers: cap 6000. Mandatory pick, only the 6000 DP Digimon is offered;
/// it becomes the opponent's TOP security card (face down).
#[test]
fn st21_06_on_play_places_6000_dp_opponent_digimon_as_top_security() {
    let (mut runner, magna) = setup(&[]);
    let sec_before = runner.security_count(1);
    runner.fire_on_play(0, magna.index as usize);

    let v = runner.pending_selection_view().expect("mandatory pick installs");
    assert!(!v.is_optional && !v.valid_action_ids.contains(&PASS), "mandatory pick");
    assert_eq!(offered(&runner, &OPPS), vec!["OPP6K"], "cap 6000 with no Tamers");
    let a = field_action(opp_index(&runner, "OPP6K"));
    runner.execute_action(0, a).unwrap();
    runner.auto_resolve().unwrap();

    assert!(!opp_on_field(&runner, "OPP6K"));
    assert_eq!(runner.security_count(1), sec_before + 1);
    assert_eq!(top_security_id(&runner, 1), "OPP6K", "placed as the TOP security card");
    let top_idx = runner.game.player(1).security.last().unwrap().card_index;
    assert!(
        !runner.game.player(1).face_up_security.contains(&top_idx),
        "placed face down"
    );
    assert_eq!(runner.security_count(0), 0, "never into your own security");
}

/// One 1-colour Tamer: floor(1/2) = 0 → cap stays 6000.
#[test]
fn st21_06_one_tamer_color_keeps_cap_at_6000() {
    let (mut runner, magna) = setup(&["T-R"]);
    runner.fire_on_play(0, magna.index as usize);
    assert_eq!(offered(&runner, &OPPS), vec!["OPP6K"]);
}

/// 2 Tamer colours → cap 8000.
#[test]
fn st21_06_two_tamer_colors_raise_cap_to_8000() {
    let (mut runner, magna) = setup(&["T-RB"]);
    runner.fire_on_play(0, magna.index as usize);
    assert_eq!(offered(&runner, &OPPS), vec!["OPP6K", "OPP7K", "OPP8K"]);
    let a = field_action(opp_index(&runner, "OPP8K"));
    runner.execute_action(0, a).unwrap();
    runner.auto_resolve().unwrap();
    assert_eq!(top_security_id(&runner, 1), "OPP8K");
}

/// 4 Tamer colours → cap 10000 (WD fires the same body).
#[test]
fn st21_06_when_digivolving_four_colors_raise_cap_to_10000() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "T-RB", Some(0));
    runner.place_on_field(0, "T-YG", Some(0));
    for id in OPPS {
        runner.place_on_field(1, id, Some(0));
    }
    let magna = runner.place_stack(0, &["ALLY", CARD_ID]);
    fire_when_digivolving(&mut runner, magna);
    assert_eq!(offered(&runner, &OPPS), OPPS.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let a = field_action(opp_index(&runner, "OPP10K"));
    runner.execute_action(0, a).unwrap();
    runner.auto_resolve().unwrap();
    assert_eq!(top_security_id(&runner, 1), "OPP10K");
}

/// No legal target → nothing pending, nothing moves.
#[test]
fn st21_06_no_target_within_cap_does_nothing() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(1, "OPP7K", Some(0));
    let magna = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, magna.index as usize);
    assert!(runner.pending_selection_view().is_none());
    assert!(opp_on_field(&runner, "OPP7K"));
    assert_eq!(runner.security_count(1), 3);
}

// ─── Section 3: [Your Turn][OPT] shared ADVENTURE observer ───────────────────

#[test]
fn st21_06_shared_observer_grants_alliance_then_optional_attack_once_per_turn() {
    let mut runner = builder().start();
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
    play_other(&mut runner, 0, "ADV");
    assert!(runner.pending_selection_view().is_none(), "[Once Per Turn]");
    runner.end_turn();
    assert!(!runner.game.has_keyword(ally, Keyword::Alliance), "for the turn");
}

#[test]
fn st21_06_shared_observer_ignores_non_adventure_and_opponents_turn() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, CARD_ID, Some(0));
    play_other(&mut runner, 0, "ALLY");
    assert!(runner.pending_selection_view().is_none(), "non-[ADVENTURE] play");
    runner.end_turn();
    runner.game.memory = 5;
    play_other(&mut runner, 0, "ADV");
    assert!(runner.pending_selection_view().is_none(), "[Your Turn] gate");
}
