#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
//! Behavioral tests for BT4 Batch 4: BT4-091 through BT4-115
//! (excluding BT4-097, BT4-098, BT4-104, BT4-105, BT4-111 which have dedicated modules).

use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, FIELD_EFFECT_END, FIELD_EFFECT_START,
    PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};

fn mk_card(id: &str, name: &str, color: CardColor, level: u8, dp: i32, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c
}

fn mk_tamer(id: &str, name: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c
}

fn set_evo_cost(r: &mut DebugRunner, card_id: &str, color: CardColor, level: u8, cost: u16) {
    if let Some(cd) = r.game.card_data.iter_mut().find(|c| c.card_id == card_id) {
        cd.evo_costs = vec![EvoCost {
            card_color: color as u8,
            level,
            memory_cost: cost,
        }];
    }
}

fn add_to_trash(r: &mut DebugRunner, player: u8, card_id: &str) {
    let idx = r.add_to_hand(player, card_id);
    let c = r.game.players[player as usize].hand.remove(idx);
    r.game.players[player as usize].put_in_trash(c);
}

fn first_field_effect_action(r: &DebugRunner, player: u8) -> Option<u16> {
    let mask = build_action_mask(&r.game, player);
    (FIELD_EFFECT_START..FIELD_EFFECT_END).find(|&a| mask[a as usize] > 0.5)
}

#[test]
fn bt4_091_chaosmon_valdur_arm() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-091")
        .expect("BT4-091 must load")
        .add_card(mk_card("LV6", "GreenLv6", CardColor::Green, 6, 11000, 11))
        .add_card(mk_card("OPP", "OppBig", CardColor::Red, 6, 15000, 12))
        .add_card(mk_card("FILL", "Filler", CardColor::White, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    set_evo_cost(&mut r, "BT4-091", CardColor::Green, 6, 6);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "LV6", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let hand = r.add_to_hand(0, "BT4-091");

    r.game.decode_action(encode_digivolve(hand as u16, 0), 0);
    let _ = r.auto_resolve();

    // Two -7000 DP activations on the same 15000 DP target -> 1000 DP
    assert_eq!(r.effective_dp(opp), Some(1000), "BT4-091 applies -7000 DP twice");

    // On Deletion gains 3 memory
    let before = r.memory();
    let va = r.perm_handle(0, 0);
    r.game.delete_permanent_with_effects(va);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), before + 3, "BT4-091 On Deletion gains 3 memory");
}

#[test]
fn bt4_092_marcus_damon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-092")
        .expect("BT4-092 must load")
        .add_card(mk_card("GREY", "AgumonGreymon", CardColor::Red, 4, 6000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let marcus = r.place_on_field(0, "BT4-092", Some(0));
    let grey = r.place_on_field(0, "GREY", Some(0));
    let before = r.memory();

    r.attack_player(grey, 1, true);
    let _ = r.auto_resolve();

    assert!(r.game.players[0].battle_area[marcus.index as usize].is_suspended);
    assert_eq!(r.memory(), before + 1, "BT4-092 suspends to gain 1 memory when Greymon attacks");
}

#[test]
fn bt4_093_thomas_h_norstein() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-093")
        .expect("BT4-093 must load")
        .add_card(mk_card("GAO", "Gaogamon", CardColor::Blue, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .hand(1, &["FILL"; 8])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let gao = r.place_field_stack(0, &["GAO"], true, 0);
    let deck_before = r.deck_size(0);
    let idx = r.add_to_hand(0, "BT4-093");
    r.play(0, idx);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 1, "BT4-093 On Play draws 1");

    let act = first_field_effect_action(&r, 0).expect("BT4-093 Main effect must be legal with opp hand >= 8");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert!(!r.game.players[0].battle_area[gao.index as usize].is_suspended, "BT4-093 unsuspends [Gao] Digimon");
}

#[test]
fn bt4_094_tai_kamiya() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-094")
        .expect("BT4-094 must load")
        .add_card(mk_card("ALLY", "YellowAlly", CardColor::Yellow, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "BT4-094", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.tick_declarative_effects();

    assert_eq!(r.dp_of(ally), Some(6000), "BT4-094 grants +1000 DP to all your Digimon at <= 3 security");
}

#[test]
fn bt4_095_yoshino_fujieda() {
    let mut egg = make_test_egg("EGG", "TestEgg");
    egg.colors = vec![CardColor::Green];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-095")
        .expect("BT4-095 must load")
        .add_card(egg)
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    add_to_trash(&mut r, 0, "EGG");
    assert_eq!(r.trash_size(0), 1);

    let idx = r.add_to_hand(0, "BT4-095");
    r.play(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.trash_size(0), 0, "BT4-095 On Play returns Digi-Egg from trash");
}

#[test]
fn bt4_096_izzy_izumi() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-096")
        .expect("BT4-096 must load")
        .add_card(mk_card("BLK", "BlackCard", CardColor::Black, 3, 2000, 3))
        .deck(0, &["BLK", "BLK", "BLK", "BLK"])
        .deck(1, &["BLK"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let idx = r.add_to_hand(0, "BT4-096");
    r.play(0, idx);
    let _ = r.auto_resolve();

    // Paid 4 (6 -> 2), all 3 revealed are black -> gain 1 memory (2 -> 3)
    assert_eq!(r.memory(), 3, "BT4-096 gains 1 memory when all 3 revealed cards are black");
}

#[test]
fn bt4_099_heir_of_dragons() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-099")
        .expect("BT4-099 must load")
        .add_card(mk_card("GREY", "Greymon", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("OPP", "Opp4k", CardColor::Blue, 3, 4000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "GREY", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    let deck_before = r.deck_size(0);

    let idx = r.add_to_hand(0, "BT4-099");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.deck_size(0), deck_before - 2, "BT4-099 draws 2");
    assert_eq!(r.battle_area_size(1), 0, "BT4-099 deletes <= 4000 DP opp when Greymon/Dramon on field");
}

#[test]
fn bt4_100_trident_revolver() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-100")
        .expect("BT4-100 must load")
        .add_card(mk_card("RED", "RedMon", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("OPP", "Opp6k", CardColor::Blue, 4, 6000, 5))
        .add_card(mk_tamer("TAMER", "RedTamer", CardColor::Red, 4))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "RED", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.add_to_hand(0, "TAMER");

    let idx = r.add_to_hand(0, "BT4-100");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-100 deletes <= 6000 DP opp");
    assert_eq!(r.battle_area_size(0), 2, "BT4-100 plays Tamer cost <= 4 from hand free");
}

#[test]
fn bt4_101_ill_drag_you_into_the_depths() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-101")
        .expect("BT4-101 must load")
        .add_card(mk_card("BLUE", "BlueSmall", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("OPP_BIG", "OppNoSrc", CardColor::Red, 6, 12000, 12))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let blue = r.place_on_field(0, "BLUE", Some(0));
    let opp = r.place_field_stack(1, &["OPP_BIG"], true, 0);

    let idx = r.add_to_hand(0, "BT4-101");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    // Small 2000 DP Blue attacks 12000 DP opp with no sources -> opp deleted by granted effect!
    r.attack_digimon(blue, opp, true);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-101 deletes attacked opponent Digimon with no sources");
}

#[test]
fn bt4_102_aqua_viper() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-102")
        .expect("BT4-102 must load")
        .add_card(mk_card("BLUE", "BlueAlly", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("OPP1", "OppLv4A", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("OPP2", "OppLv3B", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "BLUE", Some(0));
    r.place_on_field(1, "OPP1", Some(0));
    r.place_on_field(1, "OPP2", Some(0));

    let idx = r.add_to_hand(0, "BT4-102");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve_declaring_max();

    assert_eq!(r.battle_area_size(0), 0, "own Digimon returned to hand");
    assert_eq!(r.battle_area_size(1), 0, "BT4-102 bounces up to 2 opponent Lv4 or lower Digimon");
}

#[test]
fn bt4_103_full_moon_blaster() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-103")
        .expect("BT4-103 must load")
        .add_card(mk_card("BLUE", "BlueAlly", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("OPP5", "OppLv5", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .hand(1, &["FILL"; 8])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "BLUE", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    let opp_deck_before = r.deck_size(1);

    let idx = r.add_to_hand(0, "BT4-103");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-103 removes opponent Lv5 or lower");
    assert_eq!(r.deck_size(1), opp_deck_before + 1, "with opp hand >= 8, returns to deck bottom");
}

#[test]
fn bt4_106_purge_shine() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-106")
        .expect("BT4-106 must load")
        .add_card(mk_card("YEL", "YellowAlly", CardColor::Yellow, 3, 2000, 3))
        .add_card(mk_card("OPP1", "Opp1", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("OPP2", "Opp2", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "YEL", Some(0));
    let o1 = r.place_on_field(1, "OPP1", Some(0));
    let o2 = r.place_on_field(1, "OPP2", Some(0));

    let idx = r.add_to_hand(0, "BT4-106");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.effective_dp(o1), Some(2000), "BT4-106 gives -3000 DP to opp 1");
    assert_eq!(r.effective_dp(o2), Some(4000), "BT4-106 gives -3000 DP to opp 2");
}

#[test]
fn bt4_107_pollen_spray() {
    let mut burst_mon = mk_card("BURST", "BurstMon", CardColor::Green, 5, 7000, 7);
    burst_mon.keywords = vec![Keyword::DigiBurst(2)];
    burst_mon.effect_text = "[Main] ＜Digi-Burst 2＞".to_string();

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-107")
        .expect("BT4-107 must load")
        .add_card(burst_mon)
        .add_card(mk_card("GRN", "GreenAlly", CardColor::Green, 3, 2000, 3))
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.game.stage_inject_card(0, "BURST", "deck_top").unwrap();
    r.place_on_field(0, "GRN", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));

    let idx = r.add_to_hand(0, "BT4-107");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    let in_hand = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BURST");
    assert!(in_hand, "BT4-107 adds Digi-Burst Digimon to hand");
    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "BT4-107 suspends 1 opponent Digimon per added card"
    );
}

#[test]
fn bt4_108_cyclonic_kick() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-108")
        .expect("BT4-108 must load")
        .add_card(mk_card("GRN", "GreenAlly", CardColor::Green, 4, 5000, 5))
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let grn = r.place_field_stack(0, &["GRN"], true, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));

    let idx = r.add_to_hand(0, "BT4-108");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert!(!r.game.players[0].battle_area[grn.index as usize].is_suspended, "BT4-108 unsuspends own Digimon");
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended, "BT4-108 suspends opp Digimon");
}

#[test]
fn bt4_109_final_zubagon_punch() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-109")
        .expect("BT4-109 must load")
        .add_card(mk_card("BIG_BLK", "BigBlack", CardColor::Black, 6, 13000, 12))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let blk = r.place_on_field(0, "BIG_BLK", Some(0));
    let idx = r.add_to_hand(0, "BT4-109");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.effective_dp(blk), Some(16000), "BT4-109 grants +3000 DP (13000 -> 16000)");
    assert!(r.game.has_keyword(blk, Keyword::Blocker), "BT4-109 grants Blocker at 16000+ DP");
    assert!(r.game.has_keyword(blk, Keyword::Reboot), "BT4-109 grants Reboot at 16000+ DP");
}

#[test]
fn bt4_110_dark_roar() {
    let mut dbrig = mk_card("DB", "Commandramon", CardColor::Black, 3, 2000, 3);
    dbrig.traits = vec!["D-Brigade".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-110")
        .expect("BT4-110 must load")
        .add_card(dbrig)
        .add_card(mk_card("OPP5", "OppCost5", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    // 2 D-Brigade on field -> max play cost = 3 + 2 = 5
    r.place_on_field(0, "DB", Some(0));
    r.place_on_field(0, "DB", Some(0));
    r.place_on_field(1, "OPP5", Some(0));

    let idx = r.add_to_hand(0, "BT4-110");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-110 deletes opponent Digimon with cost <= 3 + D-Brigade count");
}

#[test]
fn bt4_112_hells_gate() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-112")
        .expect("BT4-112 must load")
        .add_card(mk_card("PURP", "PurpleAlly", CardColor::Purple, 3, 2000, 3))
        .add_card(mk_card("OPP6", "OppLv6", CardColor::Red, 6, 12000, 12))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "PURP", Some(0));
    r.place_on_field(1, "OPP6", Some(0));

    let idx = r.add_to_hand(0, "BT4-112");
    let _ = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-112 deletes opponent Lv6 or higher Digimon");
}

#[test]
fn bt4_113_ancientgreymon() {
    let mut hybrid = mk_card("HYB", "RedHybrid", CardColor::Red, 4, 5000, 5);
    hybrid.traits = vec!["Hybrid".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-113")
        .expect("BT4-113 must load")
        .add_card(hybrid)
        .add_card(mk_card("GREY", "Greymon", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let ag = r.place_field_stack(0, &["GREY", "HYB", "BT4-113"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.dynamic_security_attack_aura_bonus(ag).unwrap_or(0),
        2,
        "BT4-113 gains Security A. +1 per Greymon/Hybrid source"
    );

    r.add_to_hand(0, "HYB");
    r.game.delete_permanent_with_effects(ag);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 1, "BT4-113 On Deletion plays red Lv4 or lower Hybrid free");
}

#[test]
fn bt4_114_ancientgarurumon() {
    let mut hybrid = mk_card("HYB", "BlueHybrid", CardColor::Blue, 4, 5000, 5);
    hybrid.traits = vec!["Hybrid".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-114")
        .expect("BT4-114 must load")
        .add_card(hybrid)
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let ag = r.place_on_field(0, "BT4-114", Some(0));
    let hyb = r.place_field_stack(0, &["HYB"], true, 0);

    // Attack suspends AncientGarurumon itself; its [When Attacking] unsuspends up to 2 Garurumon/Hybrid Digimon (itself + hyb)
    r.attack_player(ag, 1, true);
    let _ = r.auto_resolve_declaring_max();

    assert!(!r.game.players[0].battle_area[ag.index as usize].is_suspended, "BT4-114 unsuspends itself (Garurumon)");
    assert!(!r.game.players[0].battle_area[hyb.index as usize].is_suspended, "BT4-114 unsuspends Hybrid");
}

#[test]
fn bt4_115_lucemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-115")
        .expect("BT4-115 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 2])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    for _ in 0..10 {
        add_to_trash(&mut r, 0, "FILL");
    }
    let idx = r.add_to_hand(0, "BT4-115");
    r.play(0, idx);
    let _ = r.auto_resolve();

    // Base cost 13 - 8 = 5 -> 6 - 5 = 1 memory
    assert_eq!(r.memory(), 1, "BT4-115 reduces play cost by 8 with 10+ cards in trash");
    assert_eq!(r.security_count(0), 3, "BT4-115 On Play Recovery +1");
}
