//! Shared fixtures for the Keramon / Kurisarimon / Infermon / Diaboromon
//! [Unidentified] line (BT5-063, BT5-067, BT17-053, BT17-055, BT22-053,
//! BT22-057, BT22-059, BT22-064, EX6-036, EX6-043 and the Diaboromon Token).
//! Every board is hand-built; no recording data.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

pub(crate) fn base() -> DebugRunnerBuilder {
    let mut inert = make_test_card("INERT", "Inert Fixture");
    inert.level = Some(3);
    inert.dp = Some(1000);
    inert.play_cost = 3;
    inert.colors = vec![CardColor::Red];
    // A Lv.3 black Digimon that is NOT a Keramon (wrong name for BT5-067's
    // special route, right level/colour for nothing but the standard route).
    let mut other_black3 = make_test_card("BLACK3", "Other Black Rookie");
    other_black3.level = Some(3);
    other_black3.dp = Some(3000);
    other_black3.play_cost = 3;
    other_black3.colors = vec![CardColor::Black];
    DebugRunner::builder()
        .add_card(inert)
        .add_card(other_black3)
        .deck(0, &["INERT"; 10])
        .deck(1, &["INERT"; 10])
        .security(0, &["INERT"; 3])
        .security(1, &["INERT"; 3])
}

pub(crate) fn into_main(mut r: DebugRunner, seat: u8, memory: i16) -> DebugRunner {
    r.game.turn_player_idx = seat as usize;
    r.game.memory_pair = (seat, 1 - seat);
    r.game.memory = memory;
    r.game.turn_count = 3;
    r.game.current_phase = GamePhase::Main;
    r
}

pub(crate) fn act(r: &mut DebugRunner, actor: u8, id: u16) {
    assert!(r.pending_selection().is_none(), "no prompt may be open");
    assert_eq!(
        build_action_mask(&r.game, actor)[id as usize],
        1.0,
        "action {id} must be legal"
    );
    r.game.decode_action(id, actor);
}

pub(crate) fn legal(r: &DebugRunner, actor: u8, id: u16) -> bool {
    build_action_mask(&r.game, actor)[id as usize] == 1.0
}

pub(crate) fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

// ---------------------------------------------------------------- BT5-067
pub(crate) fn infermon_runner(seat: u8, memory: i16) -> DebugRunner {
    let r = base()
        .dsl_card("BT5-067")
        .unwrap()
        .dsl_card("BT17-053")
        .unwrap()
        .dsl_card("BT22-053")
        .unwrap()
        .build();
    into_main(r, seat, memory)
}

// ------------------------------------------------------- Diaboromon token
pub(crate) fn token_on_field(r: &DebugRunner, seat: u8) -> Vec<PermanentHandle> {
    r.game.players[seat as usize]
        .battle_area
        .iter()
        .enumerate()
        .filter(|(_, p)| p.top_card().card_id(&r.game.card_data) == "TOKEN_DIABOROMON")
        .map(|(i, _)| PermanentHandle { player: seat, index: i as u8 })
        .collect()
}

// ---------------------------------------------------------------- BT17-053
pub(crate) fn lv(card_id: &str, name: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(card_id, name);
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16;
    c.colors = vec![CardColor::Red];
    c
}

pub(crate) fn keramon_runner(seat: u8) -> DebugRunner {
    let r = base()
        .add_card(lv("OPP-LV5", "Opp Lv5", 5))
        .add_card(lv("OPP-LV4", "Opp Lv4", 4))
        .dsl_card("BT5-067")
        .unwrap()
        .dsl_card("BT17-053")
        .unwrap()
        .dsl_card("BT22-053")
        .unwrap()
        .build();
    // seat `seat` owns the Keramon; the OTHER seat has the turn.
    into_main(r, 1 - seat, 6)
}

// ------------------------------------------------- reveal-3 search Keramons
pub(crate) fn fixtures(b: DebugRunnerBuilder) -> DebugRunnerBuilder {
    let mut tamer_dia = make_test_card("TAMER-DIA", "Dia Tamer");
    tamer_dia.card_kind = CardKind::Tamer;
    tamer_dia.level = None;
    tamer_dia.dp = None;
    tamer_dia.effect_text = "[Main] Search for [Diaboromon].".to_string();
    let mut opt_dia = make_test_card("OPT-DIA", "Dia Option");
    opt_dia.card_kind = CardKind::Option;
    opt_dia.level = None;
    opt_dia.dp = None;
    opt_dia.effect_text = "[Main] Return [Diaboromon] to hand.".to_string();
    let mut tamer_plain = make_test_card("TAMER-PLAIN", "Plain Tamer");
    tamer_plain.card_kind = CardKind::Tamer;
    tamer_plain.level = None;
    tamer_plain.dp = None;
    tamer_plain.effect_text = "[Main] Draw 1.".to_string();
    let mut unid = make_test_card("UNID-1", "Unid One");
    unid.traits = vec!["Unidentified".to_string()];
    let mut cs = make_test_card("CS-1", "Cs One");
    cs.traits = vec!["CS".to_string()];
    let mut arata = make_test_card("ARATA", "Arata Sanada");
    arata.card_kind = CardKind::Tamer;
    arata.level = None;
    arata.dp = None;
    b.add_card(tamer_dia)
        .add_card(opt_dia)
        .add_card(tamer_plain)
        .add_card(unid)
        .add_card(cs)
        .add_card(arata)
}

pub(crate) fn hand_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    let mut v: Vec<String> = r.game.players[seat as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}

pub(crate) fn trash_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    let mut v: Vec<String> = r.game.players[seat as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}

pub(crate) fn search_runner(deck: &[&str]) -> DebugRunner {
    let r = fixtures(base())
        .dsl_card("EX6-036")
        .unwrap()
        .dsl_card("BT22-053")
        .unwrap()
        .deck(0, deck)
        .build();
    into_main(r, 0, 6)
}

// ------------------------------------------------ the CS / Diaboromon cards
pub(crate) fn full_runner() -> DebugRunner {
    let mut cs2 = lv("CS-LV2", "Cs Two", 2);
    cs2.traits = vec!["CS".to_string()];
    let mut cs3 = lv("CS-LV3", "Cs Three", 3);
    cs3.traits = vec!["CS".to_string()];
    let mut cs4 = lv("CS-LV4", "Cs Four", 4);
    cs4.traits = vec!["CS".to_string()];
    let mut cs5 = lv("CS-LV5", "Cs Five", 5);
    cs5.traits = vec!["CS".to_string()];
    let plain2 = lv("PLAIN-LV2", "Plain Two", 2);
    let plain3 = lv("PLAIN-LV3", "Plain Three", 3);
    let plain5 = lv("PLAIN-LV5", "Plain Five", 5);
    let mut infermon_named = lv("INF-NAMED", "Infermon", 5);
    infermon_named.colors = vec![CardColor::Red];
    let mut dia_x = lv("DIA-X", "Diaboromon X", 6);
    dia_x.colors = vec![CardColor::Red];
    let mut unid_wd = lv("UNID-LV3", "Unid Three", 3);
    unid_wd.traits = vec!["Unidentified".to_string()];
    let mut opp4 = lv("OPP-C4", "Opp Cost4", 4);
    opp4.play_cost = 4;
    let mut opp6 = lv("OPP-C6", "Opp Cost6", 6);
    opp6.play_cost = 6;
    let mut opp9 = lv("OPP-C9", "Opp Cost9", 7);
    opp9.play_cost = 9;
    let mut adam = make_test_card("ADAM", "Eater Adam");
    adam.card_kind = CardKind::Tamer;
    adam.level = None;
    adam.dp = None;
    let mut dia_text = lv("DIA-TEXT", "Dia Texty", 4);
    dia_text.effect_text = "[On Play] Play 1 [Diaboromon] Token.".to_string();
    let r = fixtures(base())
        .add_card(cs2)
        .add_card(cs3)
        .add_card(cs4)
        .add_card(cs5)
        .add_card(plain2)
        .add_card(plain3)
        .add_card(plain5)
        .add_card(infermon_named)
        .add_card(dia_x)
        .add_card(unid_wd)
        .add_card(opp4)
        .add_card(opp6)
        .add_card(opp9)
        .add_card(adam)
        .add_card(dia_text)
        .dsl_card("BT5-067")
        .unwrap()
        .dsl_card("BT17-053")
        .unwrap()
        .dsl_card("BT22-053")
        .unwrap()
        .dsl_card("BT22-057")
        .unwrap()
        .dsl_card("BT22-059")
        .unwrap()
        .dsl_card("BT22-064")
        .unwrap()
        .dsl_card("EX6-043")
        .unwrap()
        .dsl_card("BT17-055")
        .unwrap()
        .dsl_card("BT5-063")
        .unwrap()
        .dsl_card("EX6-036")
        .unwrap()
        .build();
    into_main(r, 0, 8)
}

pub(crate) fn digivolve_via_action(r: &mut DebugRunner, seat: u8, hand_card: &str, host: PermanentHandle) {
    let hand = r.add_to_hand(seat, hand_card);
    let id = encode_digivolve(hand as u16, host.index as u16);
    act(r, seat, id);
}

pub(crate) fn can_digivolve(r: &mut DebugRunner, seat: u8, hand_card: &str, host: PermanentHandle) -> bool {
    let hand = r.add_to_hand(seat, hand_card);
    let id = encode_digivolve(hand as u16, host.index as u16);
    legal(r, seat, id)
}

pub(crate) fn choose_cost_route(r: &mut DebugRunner, cost: i32) {
    let s = r.pending_selection().expect("two priced routes are offered");
    let entries = s.effect_choices.as_ref().expect("route choice");
    let action = entries
        .iter()
        .find(|e| e.label == format!("Digivolve for cost {cost}"))
        .unwrap_or_else(|| panic!("no 'Digivolve for cost {cost}' route"))
        .action_id;
    let player = s.selecting_player;
    r.execute_action(player, action).unwrap();
}

// ---------------------------------------------- BT22-053 / BT22-057 substitute
pub(crate) fn substitute_board() -> (DebugRunner, PermanentHandle) {
    let mut r = full_runner();
    // BT22-053 (Keramon) as the digivolution SOURCE of a host whose top card
    // has [Diaboromon] in its text/name (BT22-064).
    let host = r.place_stack(0, &["BT22-053", "BT22-064"]);
    (r, host)
}

pub(crate) fn field_top_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

// -------------------------------------------------------------- BT22-057
pub(crate) fn tamers_on_field(r: &DebugRunner, seat: u8) -> usize {
    r.game.players[seat as usize]
        .battle_area
        .iter()
        .filter(|p| r.game.card_data[p.top_card().data_index].card_kind == CardKind::Tamer)
        .count()
}
