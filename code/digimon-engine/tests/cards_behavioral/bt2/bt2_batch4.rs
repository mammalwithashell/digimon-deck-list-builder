#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

//! Behavioral tests for BT2-091 through BT2-112.

use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c
}

fn named_digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn add_to_trash(r: &mut DebugRunner, player: u8, card_id: &str, count: usize) {
    for _ in 0..count {
        let idx = r.add_to_hand(player, card_id);
        let c = r.game.players[player as usize].hand.remove(idx);
        r.game.players[player as usize].put_in_trash(c);
    }
}

fn set_evo_cost(r: &mut DebugRunner, card_id: &str, color: CardColor, level: u8, memory_cost: u16) {
    if let Some(cd) = r.game.card_data.iter_mut().find(|c| c.card_id == card_id) {
        cd.evo_costs = vec![EvoCost {
            card_color: color as u8,
            level,
            memory_cost,
        }];
    }
}

fn hand_idx(r: &DebugRunner, player: u8, id: &str) -> usize {
    r.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in P{player} hand"))
}

// ── BT2-091 Volcanic Flare ───────────────────────────────────────────────────

#[test]
fn bt2_091_volcanic_flare() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-091")
        .expect("BT2-091")
        .add_card(digimon("RED-MON", CardColor::Red, 3, 2000, 3))
        .add_card(digimon("OPP-4K", CardColor::Blue, 4, 4000, 4))
        .add_card(digimon("OPP-7K", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-091"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.place_on_field(0, "RED-MON", Some(0));
    r.place_on_field(1, "OPP-4K", Some(0));
    r.place_on_field(1, "OPP-7K", Some(0));

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1, "4000 DP deleted, 7000 DP survives");
    assert_eq!(
        r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),
        "OPP-7K"
    );
}

// ── BT2-092 Radiation Blade ──────────────────────────────────────────────────

#[test]
fn bt2_092_radiation_blade() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-092")
        .expect("BT2-092")
        .add_card(digimon("RED-A", CardColor::Red, 4, 5000, 5))
        .add_card(digimon("RED-B", CardColor::Red, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-092"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let a = r.place_on_field(0, "RED-A", Some(0));
    let b = r.place_on_field(0, "RED-B", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve_declaring_max();
    assert_eq!(r.game.security_attack_keyword_bonus(a), 1, "RED-A gained <Security A. +1>");
    assert_eq!(r.game.security_attack_keyword_bonus(b), 1, "RED-B gained <Security A. +1>");
}

// ── BT2-093 Shield of the Just ───────────────────────────────────────────────

#[test]
fn bt2_093_shield_of_the_just() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-093")
        .expect("BT2-093")
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("OPP-8K", CardColor::Blue, 5, 8000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-093"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    r.place_on_field(0, "RED-TAMER", Some(0));
    r.place_on_field(1, "OPP-8K", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "with red Tamer, deleted 8000 DP opp Digimon");
}

// ── BT2-094 Arctic Blizzard ──────────────────────────────────────────────────

#[test]
fn bt2_094_arctic_blizzard() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-094")
        .expect("BT2-094")
        .add_card(digimon("BLUE-MON", CardColor::Blue, 4, 5000, 5))
        .add_card(digimon("OPP-BOT", CardColor::Red, 3, 2000, 3))
        .add_card(digimon("OPP-TOP", CardColor::Red, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-094"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let own = r.place_on_field(0, "BLUE-MON", Some(0));
    let opp = r.place_field_stack(1, &["OPP-BOT", "OPP-TOP"], false, 0);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].card_sources.len(),
        1,
        "trashed 1 digivolution card from opponent Digimon"
    );
    assert_eq!(r.dp_of(own), Some(7000), "own Digimon got +2000 DP");
}

// ── BT2-095 River of Power ───────────────────────────────────────────────────

#[test]
fn bt2_095_river_of_power() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-095")
        .expect("BT2-095")
        .add_card(digimon("BLUE-MON", CardColor::Blue, 3, 2000, 3))
        .add_card(digimon("OPP-LV3", CardColor::Red, 3, 2000, 3))
        .add_card(digimon("OPP-LV4", CardColor::Red, 4, 4000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-095"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.place_on_field(0, "BLUE-MON", Some(0));
    r.place_on_field(1, "OPP-LV3", Some(0));
    r.place_on_field(1, "OPP-LV3", Some(0));
    r.place_on_field(1, "OPP-LV4", Some(0));

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve_declaring_max();
    assert_eq!(r.battle_area_size(1), 1, "both Lv.3 bounced, Lv.4 remains");
    assert_eq!(r.hand_size(1), 2);
}

// ── BT2-096 The Ray of Victory ───────────────────────────────────────────────

#[test]
fn bt2_096_the_ray_of_victory() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-096")
        .expect("BT2-096")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(digimon("OWN-MON", CardColor::Blue, 4, 5000, 5))
        .add_card(digimon("OPP-LV5", CardColor::Red, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-096"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    r.place_on_field(0, "BLUE-TAMER", Some(0));
    let own = r.place_field_stack(0, &["OWN-MON"], true, 0);
    r.place_on_field(1, "OPP-LV5", Some(0));

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "bounced opp Lv.5 Digimon");
    assert!(
        !r.game.players[0].battle_area[own.index as usize].is_suspended,
        "with blue Tamer, unsuspended own Digimon"
    );
}

// ── BT2-097 Lightning Paw ────────────────────────────────────────────────────

#[test]
fn bt2_097_lightning_paw() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-097")
        .expect("BT2-097")
        .add_card(digimon("Y-MON", CardColor::Yellow, 3, 2000, 3))
        .add_card(digimon("OPP-LV3", CardColor::Red, 3, 5000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-097"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.place_on_field(0, "Y-MON", Some(0));
    let o1 = r.place_on_field(1, "OPP-LV3", Some(0));
    let o2 = r.place_on_field(1, "OPP-LV3", Some(0));
    let o3 = r.place_on_field(1, "OPP-LV3", Some(0));

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve_declaring_max();
    assert_eq!(r.dp_of(o1), Some(1000), "5000 - 4000 = 1000");
    assert_eq!(r.dp_of(o2), Some(1000), "5000 - 4000 = 1000");
    assert_eq!(r.dp_of(o3), Some(1000), "5000 - 4000 = 1000");
}

// ── BT2-098 Eden's Javelin ───────────────────────────────────────────────────

#[test]
fn bt2_098_edens_javelin() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-098")
        .expect("BT2-098")
        .add_card(digimon("Y-MON", CardColor::Yellow, 3, 2000, 3))
        .add_card(digimon("OPP", CardColor::Blue, 5, 9000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-098", "FILL", "FILL"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    r.place_on_field(0, "Y-MON", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    // Playing BT2-098 leaves 2 cards in hand, then Draw 1 -> 3 cards in hand -> -3000 DP.
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 3);
    assert_eq!(r.dp_of(opp), Some(6000), "9000 - 3*1000 = 6000 DP");
}

// ── BT2-099 Glorious Burst ───────────────────────────────────────────────────

#[test]
fn bt2_099_glorious_burst() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-099")
        .expect("BT2-099")
        .add_card(tamer("YT", CardColor::Yellow))
        .add_card(digimon("OPP", CardColor::Blue, 6, 13000, 12))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-099"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    // 3 yellow Tamers -> cost 9 - 3 = 6.
    for _ in 0..3 {
        r.place_on_field(0, "YT", Some(0));
    }
    let opp = r.place_on_field(1, "OPP", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 4, "cost 9 reduced by 3 = 6 memory spent");
    assert_eq!(r.dp_of(opp), Some(1000), "13000 - 12000 = 1000 DP");
}

// ── BT2-100 Puppet Pummel ────────────────────────────────────────────────────

#[test]
fn bt2_100_puppet_pummel() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-100")
        .expect("BT2-100")
        .add_card(digimon("G-MON", CardColor::Green, 4, 5000, 5))
        .add_card(digimon("OPP", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-100"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let own = r.place_on_field(0, "G-MON", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
    assert_eq!(r.dp_of(own), Some(7000), "own Digimon got +2000 DP");
}

// ── BT2-101 Cherry Blast ─────────────────────────────────────────────────────

#[test]
fn bt2_101_cherry_blast() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-101")
        .expect("BT2-101")
        .add_card(digimon("G-MON", CardColor::Green, 3, 2000, 3))
        .add_card(digimon("OPP-6K", CardColor::Blue, 4, 6000, 5))
        .add_card(digimon("OPP-8K", CardColor::Blue, 5, 8000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-101"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    r.place_on_field(0, "G-MON", Some(0));
    let o6 = r.place_on_field(1, "OPP-6K", Some(0));
    let o8 = r.place_on_field(1, "OPP-8K", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(r.game.players[1].battle_area[o6.index as usize].is_suspended, "<= 6000 DP suspended");
    assert!(!r.game.players[1].battle_area[o8.index as usize].is_suspended, "8000 DP not suspended");
}

// ── BT2-102 Terrors Cluster ──────────────────────────────────────────────────

#[test]
fn bt2_102_terrors_cluster() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-102")
        .expect("BT2-102")
        .add_card(digimon("G-MON", CardColor::Green, 3, 2000, 3))
        .add_card(digimon("OPP", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-102"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    r.place_on_field(0, "G-MON", Some(0));
    r.place_field_stack(1, &["OPP"], true, 0);
    let opp_deck_before = r.deck_size(1);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "suspended opp Digimon bottom-decked");
    assert_eq!(r.deck_size(1), opp_deck_before + 1);
}

// ── BT2-103 Spiral Sword ─────────────────────────────────────────────────────

#[test]
fn bt2_103_spiral_sword() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-103")
        .expect("BT2-103")
        .add_card(digimon("BLK", CardColor::Black, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-103"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let own = r.place_on_field(0, "BLK", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(own), Some(8000), "+3000 DP for the turn");
}

// ── BT2-104 Atomic Ray ───────────────────────────────────────────────────────

#[test]
fn bt2_104_atomic_ray() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-104")
        .expect("BT2-104")
        .dsl_card("BT2-061")
        .expect("BT2-061 (Andromon Blocker)")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-104"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let blk = r.place_field_stack(0, &["BT2-061"], true, 0);
    assert!(r.game.players[0].battle_area[blk.index as usize].is_suspended);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert!(
        !r.game.players[0].battle_area[blk.index as usize].is_suspended,
        "unsuspended own <Blocker> Digimon"
    );
}

// ── BT2-105 Spider Shooter ───────────────────────────────────────────────────

#[test]
fn bt2_105_spider_shooter() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-105")
        .expect("BT2-105")
        .add_card(digimon("BLK", CardColor::Black, 3, 2000, 3))
        .add_card(digimon("LV3", CardColor::Blue, 3, 2000, 3))
        .add_card(digimon("LV4", CardColor::Blue, 4, 4000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-105"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.place_on_field(0, "BLK", Some(0));
    let opp = r.place_field_stack(1, &["LV3", "LV4"], false, 0);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].card_sources.len(),
        1,
        "<De-Digivolve 1> trashed top card"
    );
}

// ── BT2-106 Infinity Cannon ──────────────────────────────────────────────────

#[test]
fn bt2_106_infinity_cannon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-106")
        .expect("BT2-106")
        .add_card(digimon("BLK", CardColor::Black, 3, 2000, 3))
        .add_card(digimon("LV3", CardColor::Blue, 3, 2000, 3))
        .add_card(digimon("LV4", CardColor::Blue, 4, 4000, 4))
        .add_card(digimon("LV5", CardColor::Blue, 5, 7000, 7))
        .add_card(digimon("LV6", CardColor::Blue, 6, 11000, 11))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-106"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    r.place_on_field(0, "BLK", Some(0));
    let opp = r.place_field_stack(1, &["LV3", "LV4", "LV5", "LV6"], false, 0);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve_declaring_max();
    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].card_sources.len(),
        1,
        "<De-Digivolve 4> stopped at Lv.3"
    );
}

// ── BT2-107 Darkness Claw ────────────────────────────────────────────────────

#[test]
fn bt2_107_darkness_claw() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-107")
        .expect("BT2-107")
        .add_card(digimon("P-MON", CardColor::Purple, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-107"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let own = r.place_on_field(0, "P-MON", Some(0));
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(own), Some(8000), "+3000 DP for the turn");
}

// ── BT2-108 Night Raid ───────────────────────────────────────────────────────

#[test]
fn bt2_108_night_raid() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-108")
        .expect("BT2-108")
        .add_card(digimon("P-LV3", CardColor::Purple, 3, 2000, 3))
        .add_card(digimon("P-FIELD", CardColor::Purple, 4, 4000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-108"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.place_on_field(0, "P-FIELD", Some(0));
    add_to_trash(&mut r, 0, "P-LV3", 1);
    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 2, "played purple Lv.3 Digimon from trash free");
}

// ── BT2-109 Heat Viper ───────────────────────────────────────────────────────

#[test]
fn bt2_109_heat_viper() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-109")
        .expect("BT2-109")
        .add_card(digimon("P-SAC", CardColor::Purple, 3, 2000, 3))
        .add_card(digimon("OPP-LV4", CardColor::Blue, 4, 4000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-109"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();

    r.place_on_field(0, "P-SAC", Some(0));
    r.place_on_field(1, "OPP-LV4", Some(0));
    r.place_on_field(1, "OPP-LV4", Some(0));

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve_declaring_max();
    assert_eq!(r.battle_area_size(0), 0, "sacrificed own Digimon");
    assert_eq!(r.battle_area_size(1), 0, "deleted both opponent Lv.4 Digimon");
}

// ── BT2-110 Trump Sword ──────────────────────────────────────────────────────

#[test]
fn bt2_110_trump_sword() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-110")
        .expect("BT2-110")
        .add_card(digimon("P-MON", CardColor::Purple, 3, 2000, 3))
        .add_card(digimon("OPP-UNSUSP", CardColor::Blue, 6, 12000, 12))
        .add_card(digimon("OPP-SUSP", CardColor::Blue, 6, 12000, 12))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-110"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    r.place_on_field(0, "P-MON", Some(0));
    r.place_on_field(1, "OPP-UNSUSP", Some(0));
    r.place_field_stack(1, &["OPP-SUSP"], true, 0);

    let _ = r.game.play_option_from_hand(0, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1, "unsuspended opp Digimon deleted, suspended survives");
    assert_eq!(
        r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),
        "OPP-SUSP"
    );
}

// ── BT2-111 Beelzemon ────────────────────────────────────────────────────────

#[test]
fn bt2_111_beelzemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-111")
        .expect("BT2-111")
        .add_card(named_digimon("IMPMON", "Impmon", CardColor::Purple, 3, 1000, 3))
        .add_card(digimon("OPP-LV4", CardColor::Blue, 4, 4000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-111"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    // Alt-path: digivolve onto [Impmon] for 4 when 10+ cards in trash.
    add_to_trash(&mut r, 0, "FILL", 10);
    let imp = r.place_on_field(0, "IMPMON", Some(0));
    r.place_on_field(1, "OPP-LV4", Some(0));

    let hi = hand_idx(&r, 0, "BT2-111") as u16;
    r.game.decode_action(encode_digivolve(hi, imp.index as u16), 0);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 2, "alt-digivolved onto Impmon for 4");
    assert_eq!(r.battle_area_size(1), 0, "[When Digivolving] deleted opp Lv.4 Digimon");
}

// ── BT2-112 BlackWarGreymon ──────────────────────────────────────────────────

#[test]
fn bt2_112_blackwargreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-112")
        .expect("BT2-112")
        .add_card(digimon("OPP-10K", CardColor::Blue, 6, 10000, 11))
        .add_card(digimon("OPP-5K", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-112"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    // Play cost reduction: opponent has 10000+ DP Digimon -> cost 13 - 6 = 7.
    let opp_high = r.place_field_stack(1, &["OPP-10K"], true, 0);
    r.place_field_stack(1, &["OPP-5K"], true, 0);
    let idx = r.play(0, 0).expect("play BT2-112");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3, "cost 13 - 6 = 7 memory spent");

    // [When Attacking] attacking opponent's highest-DP Digimon -> unsuspend self.
    r.game.turn_count = 2;
    let bwg = r.perm_handle(0, idx);
    r.game.players[0].battle_area[idx].turn_played = 0;
    let _ = r.attack_digimon(bwg, opp_high, false);
    let _ = r.auto_resolve();
    assert!(
        !r.game.players[0].battle_area[idx].is_suspended,
        "[When Attacking] highest-DP opp Digimon unsuspended BT2-112"
    );
}
