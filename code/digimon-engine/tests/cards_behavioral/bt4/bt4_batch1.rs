#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
//! Behavioral tests for BT4 Batch 1: BT4-001 through BT4-030
//! (excluding BT4-010, BT4-014, BT4-018 which have dedicated modules).

use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, FIELD_EFFECT_END, FIELD_EFFECT_START,
    HATCH, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
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

fn set_evo_cost(r: &mut DebugRunner, card_id: &str, color: u8, level: u8, cost: u16) {
    if let Some(cd) = r.game.card_data.iter_mut().find(|c| c.card_id == card_id) {
        cd.evo_costs = vec![EvoCost {
            card_color: color,
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
fn bt4_001_sakuttomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-001")
        .expect("BT4-001 must load")
        .add_card(mk_card("LV7", "Lv7Mon", CardColor::Red, 7, 13000, 13))
        .add_card(mk_card("LV6", "Lv6Mon", CardColor::Red, 6, 11000, 11))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let lv7 = r.place_field_stack(0, &["BT4-001", "LV7"], false, 0);
    let before = r.memory();
    r.attack_player(lv7, 1, true);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), before + 1, "BT4-001 gains 1 memory when Lv7 attacks");

    // Non-Lv7 host does not gain memory
    let lv6 = r.place_field_stack(0, &["BT4-001", "LV6"], false, 0);
    let before2 = r.memory();
    r.attack_player(lv6, 1, true);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), before2, "BT4-001 does not trigger on Lv6");
}

#[test]
fn bt4_002_bukamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-002")
        .expect("BT4-002 must load")
        .add_card(mk_card("HOST", "BlueHost", CardColor::Blue, 4, 6000, 5))
        .add_card(mk_card("OPP_SRC", "OppSrc", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("OPP_LV4", "OppLv4", CardColor::Blue, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let attacker = r.place_field_stack(0, &["BT4-002", "HOST"], false, 0);
    let victim = r.place_field_stack(1, &["OPP_SRC", "OPP_LV4"], false, 0);
    assert_eq!(r.game.players[1].battle_area[victim.index as usize].card_sources.len(), 2);

    r.attack_player(attacker, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[victim.index as usize].card_sources.len(),
        1,
        "BT4-002 trashes bottom digivolution card of opponent's Lv4 or lower Digimon"
    );
}

#[test]
fn bt4_003_koromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-003")
        .expect("BT4-003 must load")
        .add_card(mk_card("HOST", "YellowHost", CardColor::Yellow, 4, 6000, 5))
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let attacker = r.place_field_stack(0, &["BT4-003", "HOST"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));
    assert_eq!(r.dp_of(opp), Some(5000));

    r.attack_player(attacker, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(r.dp_of(opp), Some(4000), "BT4-003 gives -1000 DP when security <= 3");
}

#[test]
fn bt4_004_budmon() {
    let mut burst_host = mk_card("BURST_HOST", "BurstMon", CardColor::Green, 5, 7000, 7);
    burst_host.keywords = vec![Keyword::DigiBurst(2)];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-004")
        .expect("BT4-004 must load")
        .add_card(burst_host)
        .add_card(mk_card("PLAIN_HOST", "PlainMon", CardColor::Green, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let with_burst = r.place_field_stack(0, &["BT4-004", "BURST_HOST"], false, 0);
    let without_burst = r.place_field_stack(0, &["BT4-004", "PLAIN_HOST"], false, 0);
    r.game.tick_declarative_effects();

    assert_eq!(r.effective_dp(with_burst), Some(8000), "BT4-004 grants +1000 DP to Digi-Burst host");
    assert_eq!(r.effective_dp(without_burst), Some(7000), "BT4-004 does not buff non-Digi-Burst host");
}

#[test]
fn bt4_005_missimon() {
    let mut d_brigade = mk_card("DBRIG", "Commandramon", CardColor::Black, 3, 3000, 3);
    d_brigade.traits = vec!["D-Brigade".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-005")
        .expect("BT4-005 must load")
        .add_card(d_brigade)
        .add_card(mk_card("OTHER", "OtherMon", CardColor::Black, 3, 3000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let db = r.place_field_stack(0, &["BT4-005", "DBRIG"], false, 0);
    let other = r.place_field_stack(0, &["BT4-005", "OTHER"], false, 0);
    r.game.tick_declarative_effects();

    assert_eq!(r.effective_dp(db), Some(4000), "BT4-005 grants +1000 DP to [D-Brigade] host");
    assert_eq!(r.effective_dp(other), Some(3000), "BT4-005 does not buff non-[D-Brigade] host");
}

#[test]
fn bt4_006_xiaomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-006")
        .expect("BT4-006 must load")
        .add_card(mk_card("HOST", "PurpleHost", CardColor::Purple, 4, 4000, 4))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let host = r.place_field_stack(0, &["BT4-006", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(host, Keyword::Retaliation), "no Retaliation with < 10 trash");

    for _ in 0..10 {
        add_to_trash(&mut r, 0, "FILL");
    }
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(host, Keyword::Retaliation), "BT4-006 grants Retaliation with >= 10 trash");
}

#[test]
fn bt4_007_otamamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-007")
        .expect("BT4-007 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-007").expect("compiled BT4-007");
    assert_eq!(compiled.name, "Otamamon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(2));
    assert_eq!(compiled.dp, Some(5000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-007");
    r.play(0, idx);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(5000));
}

#[test]
fn bt4_008_agumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-008")
        .expect("BT4-008 must load")
        .dsl_card("BT4-012")
        .expect("BT4-012 must load")
        .add_card(mk_card("SRC", "Src", CardColor::Red, 2, 1000, 0))
        .add_card(mk_card("OPP", "Opp", CardColor::Red, 3, 3000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["SRC", "BT4-008", "BT4-012"], false, 0);
    r.place_on_field(1, "OPP", Some(0));

    let act = first_field_effect_action(&r, 0).expect("Digi-Burst 2 action should be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    let in_hand = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT4-008");
    assert!(in_hand, "BT4-008 returns to hand when trashed for Digi-Burst");
}

#[test]
fn bt4_009_flamemon() {
    let mut hybrid = mk_card("HYB", "HybridMon", CardColor::Red, 4, 5000, 5);
    hybrid.traits = vec!["Hybrid".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-009")
        .expect("BT4-009 must load")
        .add_card(hybrid)
        .add_card(mk_tamer("RTAMER", "RedTamer", CardColor::Red, 3))
        .add_card(mk_card("MISS", "Miss", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.game.stage_inject_card(0, "MISS", "deck_top").unwrap();
    r.game.stage_inject_card(0, "RTAMER", "deck_top").unwrap();
    r.game.stage_inject_card(0, "HYB", "deck_top").unwrap();

    let idx = r.add_to_hand(0, "BT4-009");
    r.play(0, idx);
    let _ = r.auto_resolve();

    let hand_ids: Vec<_> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(hand_ids.contains(&"HYB".to_string()), "BT4-009 adds Hybrid Digimon");
    assert!(hand_ids.contains(&"RTAMER".to_string()), "BT4-009 adds Red Tamer");
}

#[test]
fn bt4_011_agunimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-011")
        .expect("BT4-011 must load")
        .add_card(mk_tamer("RTAMER", "RedTamer", CardColor::Red, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT4-011", 0, 3, 2);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "RTAMER", Some(0));
    let hand = r.add_to_hand(0, "BT4-011");
    let evo = encode_digivolve(hand as u16, 0);
    assert_eq!(build_action_mask(&r.game, 0)[evo as usize], 1.0, "BT4-011 can digivolve onto red Tamer");
    r.game.decode_action(evo, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), 3, "digivolve onto red Tamer costs 2");
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "BT4-011");
}

#[test]
fn bt4_012_geogreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-012")
        .expect("BT4-012 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Red, 2, 1000, 0))
        .add_card(mk_card("S2", "Src2", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("SMALL", "SmallOpp", CardColor::Red, 3, 4000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-012"], false, 0);
    r.place_on_field(1, "SMALL", Some(0));

    let act = first_field_effect_action(&r, 0).expect("BT4-012 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-012 deletes opponent Digimon with <= 4000 DP");
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}

#[test]
fn bt4_013_burninggreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-013")
        .expect("BT4-013 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let bg = r.place_on_field(0, "BT4-013", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bg), Some(9000), "BT4-013 gets +3000 DP on your turn (6000 -> 9000)");
}

#[test]
fn bt4_015_volcdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-015")
        .expect("BT4-015 must load")
        .add_card(mk_card("HOST", "RedLv6", CardColor::Red, 6, 12000, 12))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 4])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let host = r.place_field_stack(0, &["BT4-015", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert!(r.game.security_attack_keyword_bonus(host) >= 1, "BT4-015 grants inherited Security A. +1");

    r.attack_player(host, 1, true);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 2, "checks 2 security cards");
}

#[test]
fn bt4_016_aldamon() {
    let mut hybrid = mk_card("HYB", "HybridSrc", CardColor::Red, 4, 5000, 5);
    hybrid.traits = vec!["Hybrid".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-016")
        .expect("BT4-016 must load")
        .add_card(hybrid)
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let buffed = r.place_field_stack(0, &["HYB", "BT4-016"], false, 0);
    let plain = r.place_on_field(0, "BT4-016", Some(0));
    r.game.tick_declarative_effects();

    assert!(r.game.security_attack_keyword_bonus(plain) >= 1, "BT4-016 has Security A. +1");
    assert_eq!(r.effective_dp(buffed), Some(11000), "BT4-016 gets +4000 DP with Hybrid source");
    assert_eq!(r.effective_dp(plain), Some(7000), "BT4-016 without Hybrid/Tamer source stays 7000 DP");
}

#[test]
fn bt4_017_rizegreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-017")
        .expect("BT4-017 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("S2", "Src2", CardColor::Red, 4, 5000, 5))
        .add_card(mk_tamer("YTAMER", "YellowTamer", CardColor::Yellow, 4))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-017"], false, 0);
    r.add_to_hand(0, "YTAMER");

    let act = first_field_effect_action(&r, 0).expect("BT4-017 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 2, "BT4-017 plays red/yellow Tamer cost <= 4 for free");
    assert_eq!(r.memory(), 5, "Tamer was played without paying cost");
}

#[test]
fn bt4_019_victorygreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-019")
        .expect("BT4-019 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("S2", "Src2", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("LV5", "RedLv5", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("OPP", "Opp8k", CardColor::Red, 5, 8000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    set_evo_cost(&mut r, "BT4-019", 0, 5, 4);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "LV5"], false, 0);
    r.place_on_field(1, "OPP", Some(0));
    let hand = r.add_to_hand(0, "BT4-019");

    let evo = encode_digivolve(hand as u16, 0);
    r.game.decode_action(evo, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-019 When Digivolving Digi-Burst 2 deletes <= 8000 DP opp");
}

#[test]
fn bt4_020_shinegreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-020")
        .expect("BT4-020 must load")
        .add_card(mk_tamer("RTAMER", "RedTamer", CardColor::Red, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Red, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let sg = r.place_on_field(0, "BT4-020", Some(0));
    let tamer = r.place_on_field(0, "RTAMER", Some(0));
    assert_eq!(r.game.security_attack_keyword_bonus(sg), 0);

    r.game.suspend(tamer);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.security_attack_keyword_bonus(sg),
        1,
        "BT4-020 gains Security A. +1 when red/yellow Tamer suspends"
    );
}

#[test]
fn bt4_021_gaomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-021")
        .expect("BT4-021 must load")
        .dsl_card("BT4-026")
        .expect("BT4-026 must load")
        .add_card(mk_card("SRC", "Src", CardColor::Blue, 2, 1000, 0))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["SRC", "BT4-021", "BT4-026"], false, 0);
    let act = first_field_effect_action(&r, 0).expect("BT4-026 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    let in_hand = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT4-021");
    assert!(in_hand, "BT4-021 returns to hand when trashed for Digi-Burst");
}

#[test]
fn bt4_022_sangomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-022")
        .expect("BT4-022 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-022").expect("compiled BT4-022");
    assert_eq!(compiled.name, "Sangomon");
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(4000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-022");
    r.play(0, idx);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(4000));
}

#[test]
fn bt4_023_strabimon() {
    let mut hybrid = mk_card("HYB", "BlueHybrid", CardColor::Blue, 4, 5000, 5);
    hybrid.traits = vec!["Hybrid".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-023")
        .expect("BT4-023 must load")
        .add_card(hybrid)
        .add_card(mk_tamer("BTAMER", "BlueTamer", CardColor::Blue, 3))
        .add_card(mk_card("MISS", "Miss", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.game.stage_inject_card(0, "MISS", "deck_top").unwrap();
    r.game.stage_inject_card(0, "BTAMER", "deck_top").unwrap();
    r.game.stage_inject_card(0, "HYB", "deck_top").unwrap();

    let idx = r.add_to_hand(0, "BT4-023");
    r.play(0, idx);
    let _ = r.auto_resolve();

    let hand_ids: Vec<_> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(hand_ids.contains(&"HYB".to_string()), "BT4-023 adds Hybrid Digimon");
    assert!(hand_ids.contains(&"BTAMER".to_string()), "BT4-023 adds Blue Tamer");
}

#[test]
fn bt4_024_tobiumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-024")
        .expect("BT4-024 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-024").expect("compiled BT4-024");
    assert_eq!(compiled.name, "Tobiumon");
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(3000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-024");
    r.play(0, idx);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(3000));
}

#[test]
fn bt4_025_lobomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-025")
        .expect("BT4-025 must load")
        .add_card(mk_tamer("BTAMER", "BlueTamer", CardColor::Blue, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT4-025", 1, 3, 2);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "BTAMER", Some(0));
    let hand = r.add_to_hand(0, "BT4-025");
    let evo = encode_digivolve(hand as u16, 0);
    assert_eq!(build_action_mask(&r.game, 0)[evo as usize], 1.0, "BT4-025 digivolves onto blue Tamer");
    r.game.decode_action(evo, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), 3);
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "BT4-025");
}

#[test]
fn bt4_026_gaogamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-026")
        .expect("BT4-026 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Blue, 2, 1000, 0))
        .add_card(mk_card("S2", "Src2", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-026"], false, 0);
    let hand_before = r.hand_size(0);

    let act = first_field_effect_action(&r, 0).expect("BT4-026 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before + 1, "BT4-026 Digi-Burst 2 draws 1");
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}

#[test]
fn bt4_027_kendogarurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-027")
        .expect("BT4-027 must load")
        .add_card(mk_card("OPP_LV3", "OppLv3", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let kg = r.place_on_field(0, "BT4-027", Some(0));
    r.place_on_field(1, "OPP_LV3", Some(0));

    r.attack_player(kg, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-027 returns opponent's Lv3 Digimon to hand");
    assert_eq!(r.hand_size(1), 1);
}

#[test]
fn bt4_028_piranimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-028")
        .expect("BT4-028 must load")
        .add_card(mk_card("HOST", "BlueLv6", CardColor::Blue, 6, 11000, 11))
        .add_card(mk_card("OPP_SRC", "OppSrc", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("OPP_TOP", "OppTop", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let host = r.place_field_stack(0, &["BT4-028", "HOST"], false, 0);
    let opp = r.place_field_stack(1, &["OPP_SRC", "OPP_TOP"], false, 0);

    r.attack_player(host, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].card_sources.len(),
        1,
        "BT4-028 inherited trashes top digivolution card of opponent's Digimon"
    );
}

#[test]
fn bt4_029_gusokumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-029")
        .expect("BT4-029 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-029").expect("compiled BT4-029");
    assert_eq!(compiled.name, "Gusokumon");
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(10000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-029");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(10000));
}

#[test]
fn bt4_030_beowolfmon() {
    let mut hybrid = mk_card("HYB", "BlueHybrid", CardColor::Blue, 4, 5000, 5);
    hybrid.traits = vec!["Hybrid".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-030")
        .expect("BT4-030 must load")
        .add_card(hybrid)
        .add_card(mk_card("OPP", "OppAtk", CardColor::Red, 5, 9000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.set_first_player(1);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let beo = r.place_field_stack(0, &["HYB", "BT4-030"], true, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.tick_declarative_effects();

    assert!(r.game.has_keyword(beo, Keyword::Jamming), "BT4-030 has Jamming");
    let mask = build_action_mask(&r.game, 1);
    let atk = encode_attack(opp.index as u16, beo.index as u16);
    assert_eq!(
        mask[atk as usize], 0.0,
        "BT4-030 with Hybrid source cannot be attacked on opponent's turn"
    );
}
