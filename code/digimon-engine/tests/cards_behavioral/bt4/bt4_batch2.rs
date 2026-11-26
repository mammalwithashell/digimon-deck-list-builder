#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
//! Behavioral tests for BT4 Batch 2: BT4-031 through BT4-060.

use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, FIELD_EFFECT_END, FIELD_EFFECT_START,
    PASS, SECURITY_TARGET,
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

fn set_evo_cost(r: &mut DebugRunner, card_id: &str, color: CardColor, level: u8, cost: u16) {
    if let Some(cd) = r.game.card_data.iter_mut().find(|c| c.card_id == card_id) {
        cd.evo_costs = vec![EvoCost {
            card_color: color as u8,
            level,
            memory_cost: cost,
        }];
    }
}

fn first_field_effect_action(r: &DebugRunner, player: u8) -> Option<u16> {
    let mask = build_action_mask(&r.game, player);
    (FIELD_EFFECT_START..FIELD_EFFECT_END).find(|&a| mask[a as usize] > 0.5)
}

#[test]
fn bt4_031_marinechimairamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-031")
        .expect("BT4-031 must load")
        .add_card(mk_card("ALLY", "AllyMon", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("OPP", "OppNoSrc", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    let idx = r.add_to_hand(0, "BT4-031");
    r.play(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 1, "own other Digimon bounced");
    assert_eq!(r.battle_area_size(1), 0, "opponent Digimon with no sources bounced");
    assert_eq!(r.hand_size(1), 1);
}

#[test]
fn bt4_032_machgaogamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-032")
        .expect("BT4-032 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("S2", "Src2", CardColor::Blue, 4, 5000, 5))
        .add_card(mk_card("OPP4", "OppLv4", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("HOST", "BlueLv6", CardColor::Blue, 6, 11000, 11))
        .add_card(mk_tamer("TAMER", "BlueTamer", CardColor::Blue, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-032"], false, 0);
    r.place_on_field(1, "OPP4", Some(0));
    let act = first_field_effect_action(&r, 0).expect("BT4-032 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "BT4-032 bounces opponent Lv4 or lower");

    // Inherited +2000 DP with Tamer
    let host = r.place_field_stack(0, &["BT4-032", "HOST"], false, 0);
    r.place_on_field(0, "TAMER", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(13000), "BT4-032 inherited grants +2000 DP with Tamer");
}

#[test]
fn bt4_033_zeedgarurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-033")
        .expect("BT4-033 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Blue, 3, 2000, 3))
        .add_card(mk_card("S2", "Src2", CardColor::Blue, 4, 5000, 5))
        .add_card(mk_card("LV5", "BlueLv5", CardColor::Blue, 5, 7000, 7))
        .add_card(mk_card("OPP5", "OppLv5", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    set_evo_cost(&mut r, "BT4-033", CardColor::Blue, 5, 4);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "LV5"], false, 0);
    r.place_on_field(1, "OPP5", Some(0));
    let hand = r.add_to_hand(0, "BT4-033");
    let evo = encode_digivolve(hand as u16, 0);
    r.game.decode_action(evo, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-033 Digi-Burst 2 bounces opponent Lv5 or lower");
}

#[test]
fn bt4_034_regalecusmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-034")
        .expect("BT4-034 must load")
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

    let reg = r.place_on_field(0, "BT4-034", Some(0));
    let opp = r.place_field_stack(1, &["OPP_SRC", "OPP_TOP"], false, 0);
    let mem_before = r.memory();
    let hand_before = r.hand_size(0);

    r.attack_player(reg, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 1);
    assert_eq!(r.hand_size(0), hand_before + 1, "BT4-034 draws 1 when trashing source");
    assert_eq!(r.memory(), mem_before + 1, "BT4-034 gains 1 memory when trashing source");
}

#[test]
fn bt4_035_miragegaogamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-035")
        .expect("BT4-035 must load")
        .add_card(mk_card("LV5", "BlueLv5", CardColor::Blue, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Blue, 3, 2000, 3))
        .hand(1, &["FILL"; 8])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    set_evo_cost(&mut r, "BT4-035", CardColor::Blue, 5, 4);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "LV5", Some(0));
    let hand = r.add_to_hand(0, "BT4-035");
    let evo = encode_digivolve(hand as u16, 0);
    r.game.decode_action(evo, 0);
    let _ = r.auto_resolve();

    // Paid 4, gained 2 (8 cards / 4 = 2) -> 8 - 4 + 2 = 6
    assert_eq!(r.memory(), 6, "BT4-035 gains 1 memory per 4 cards in opponent's hand");
}

#[test]
fn bt4_036_falcomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-036")
        .expect("BT4-036 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-036").expect("compiled BT4-036");
    assert_eq!(compiled.name, "Falcomon");
    assert_eq!(compiled.cost, Some(2));
    assert_eq!(compiled.dp, Some(5000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-036");
    r.play(0, idx);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(5000));
}

#[test]
fn bt4_037_kudamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-037")
        .expect("BT4-037 must load")
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let opp = r.place_on_field(1, "OPP", Some(0));
    let idx = r.add_to_hand(0, "BT4-037");
    r.play(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(0), 2, "BT4-037 trashes top security");
    assert_eq!(r.dp_of(opp), Some(3000), "BT4-037 gives -2000 DP");
}

#[test]
fn bt4_038_bushiagumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-038")
        .expect("BT4-038 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let idx = r.add_to_hand(0, "BT4-038");
    r.play(0, idx);
    let h = r.perm_handle(0, 0);
    assert!(r.game.has_keyword(h, Keyword::Rush), "BT4-038 has Rush");
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[encode_attack(0, SECURITY_TARGET) as usize], 1.0, "BT4-038 can attack turn played");
}

#[test]
fn bt4_039_growlmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-039")
        .expect("BT4-039 must load")
        .add_card(mk_card("HOST", "YellowLv5", CardColor::Yellow, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let host = r.place_field_stack(0, &["BT4-039", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(8000), "BT4-039 inherited grants +1000 DP at <= 3 security");
}

#[test]
fn bt4_040_diatrymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-040")
        .expect("BT4-040 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-040").expect("compiled BT4-040");
    assert_eq!(compiled.name, "Diatrymon");
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(6000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-040");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(6000));
}

#[test]
fn bt4_041_meicoomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-041")
        .expect("BT4-041 must load")
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let opp = r.place_on_field(1, "OPP", Some(0));
    let idx = r.add_to_hand(0, "BT4-041");
    r.play(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.dp_of(opp), Some(3000), "BT4-041 gives -4000 DP at <= 3 security");
}

#[test]
fn bt4_042_piddomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-042")
        .expect("BT4-042 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let pid = r.place_on_field(0, "BT4-042", Some(0));
    assert!(r.game.has_keyword(pid, Keyword::Blocker), "BT4-042 has Blocker");

    let mem_before = r.memory();
    r.attack_player(pid, 1, true);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before - 2, "BT4-042 loses 2 memory when attacking");
}

#[test]
fn bt4_043_crowmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-043")
        .expect("BT4-043 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-043").expect("compiled BT4-043");
    assert_eq!(compiled.name, "Crowmon");
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(8000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-043");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(8000));
}

#[test]
fn bt4_044_hippogryphonmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-044")
        .expect("BT4-044 must load")
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let hippo = r.place_on_field(0, "BT4-044", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(hippo, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(r.dp_of(opp), Some(4000), "BT4-044 gives -3000 DP when attacking at <= 3 security");
}

#[test]
fn bt4_045_maycrackmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-045")
        .expect("BT4-045 must load")
        .add_card(mk_card("SEC_MON", "SecMon", CardColor::Yellow, 4, 5000, 5))
        .add_card(mk_card("OPP_ATK", "OppAtk", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["SEC_MON", "SEC_MON", "SEC_MON"])
        .memory(5)
        .start();
    r.set_first_player(1);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "BT4-045", Some(0));
    let opp = r.place_on_field(1, "OPP_ATK", Some(0));
    r.game.tick_declarative_effects();

    // Opp's 7000 DP attacker hits P0's 5000 + 4000 = 9000 DP security Digimon -> opp dies!
    r.attack_player(opp, 0, true);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "BT4-045 buffs own Security Digimon by +4000 DP on opponent's turn");
}

#[test]
fn bt4_046_wargrowlmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-046")
        .expect("BT4-046 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Yellow, 3, 2000, 3))
        .add_card(mk_card("S2", "Src2", CardColor::Yellow, 4, 5000, 5))
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-046"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));

    let act = first_field_effect_action(&r, 0).expect("BT4-046 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.effective_dp(opp), Some(3000), "BT4-046 Digi-Burst 2 gives -4000 DP");
}

#[test]
fn bt4_047_rasielmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-047")
        .expect("BT4-047 must load")
        .add_card(mk_card("LV5", "YellowLv5", CardColor::Yellow, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 2])
        .memory(8)
        .start();
    set_evo_cost(&mut r, "BT4-047", CardColor::Yellow, 5, 4);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "LV5", Some(0));
    let hand = r.add_to_hand(0, "BT4-047");
    let evo = encode_digivolve(hand as u16, 0);
    r.game.decode_action(evo, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(0), 4, "BT4-047 When Digivolving Recovery +2");
}

#[test]
fn bt4_048_wargreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-048")
        .expect("BT4-048 must load")
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 6, 10000, 10))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let wg = r.place_on_field(0, "BT4-048", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));

    r.attack_player(wg, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(0), 2, "BT4-048 adds top security to hand");
    assert_eq!(r.hand_size(0), 1);
    assert!(!r.game.players[0].battle_area[wg.index as usize].is_suspended, "BT4-048 unsuspends itself");
    assert_eq!(r.dp_of(opp), Some(4000), "BT4-048 gives -6000 DP");
}

#[test]
fn bt4_049_varodurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-049")
        .expect("BT4-049 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Yellow, 3, 2000, 3))
        .add_card(mk_card("S2", "Src2", CardColor::Yellow, 4, 5000, 5))
        .add_card(mk_card("S3", "Src3", CardColor::Yellow, 5, 7000, 7))
        .add_card(mk_card("OPP1", "Opp1", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("OPP2", "Opp2", CardColor::Red, 5, 8000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Yellow, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "S3", "BT4-049"], false, 0);
    let o1 = r.place_on_field(1, "OPP1", Some(0));
    let o2 = r.place_on_field(1, "OPP2", Some(0));

    let act = first_field_effect_action(&r, 0).expect("BT4-049 Digi-Burst 3 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.effective_dp(o1), Some(3000), "BT4-049 Digi-Burst 3 gives -4000 DP to opp 1");
    assert_eq!(r.effective_dp(o2), Some(4000), "BT4-049 Digi-Burst 3 gives -4000 DP to opp 2");
}

#[test]
fn bt4_050_liollmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-050")
        .expect("BT4-050 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-050").expect("compiled BT4-050");
    assert_eq!(compiled.name, "Liollmon");
    assert_eq!(compiled.cost, Some(2));
    assert_eq!(compiled.dp, Some(5000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-050");
    r.play(0, idx);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(5000));
}

#[test]
fn bt4_051_dokunemon() {
    let mut burst_mon = mk_card("BURST_MON", "BurstMon", CardColor::Green, 5, 7000, 7);
    burst_mon.keywords = vec![Keyword::DigiBurst(2)];
    burst_mon.effect_text = "[Main] ＜Digi-Burst 2＞".to_string();

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-051")
        .expect("BT4-051 must load")
        .add_card(burst_mon)
        .add_card(mk_card("MISS", "Miss", CardColor::Green, 3, 2000, 3))
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.game.stage_inject_card(0, "BURST_MON", "deck_top").unwrap();

    let idx = r.add_to_hand(0, "BT4-051");
    r.play(0, idx);
    let _ = r.auto_resolve();

    let found = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BURST_MON");
    assert!(found, "BT4-051 adds Digimon with <Digi-Burst> from top 3 to hand");
}

#[test]
fn bt4_052_lalamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-052")
        .expect("BT4-052 must load")
        .dsl_card("BT4-059")
        .expect("BT4-059 must load")
        .add_card(mk_card("SRC", "Src", CardColor::Green, 2, 1000, 0))
        .add_card(mk_card("OPP", "Opp", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["SRC", "BT4-052", "BT4-059"], false, 0);
    r.place_on_field(1, "OPP", Some(0));

    let act = first_field_effect_action(&r, 0).expect("BT4-059 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    let in_hand = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT4-052");
    assert!(in_hand, "BT4-052 returns to hand when trashed for Digi-Burst");
}

#[test]
fn bt4_053_roachmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-053")
        .expect("BT4-053 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-053").expect("compiled BT4-053");
    assert_eq!(compiled.name, "Roachmon");
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(3000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-053");
    r.play(0, idx);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(3000));
}

#[test]
fn bt4_054_sunflowmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-054")
        .expect("BT4-054 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Green, 2, 1000, 0))
        .add_card(mk_card("S2", "Src2", CardColor::Green, 3, 2000, 3))
        .add_card(mk_card("OPP", "OppSusp", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-054"], false, 0);
    let opp = r.place_field_stack(1, &["OPP"], true, 0);

    let act = first_field_effect_action(&r, 0).expect("BT4-054 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert!(
        r.modifiers()
            .has(opp, digimon_engine::enums::ModifierType::CannotUnsuspendInUnsuspendPhase),
        "BT4-054 locks suspended opponent Digimon from unsuspending"
    );
}

#[test]
fn bt4_055_leomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-055")
        .expect("BT4-055 must load")
        .add_card(mk_card("LV3", "GreenLv3", CardColor::Green, 3, 2000, 3))
        .add_card(mk_card("OPP", "Opp3k", CardColor::Red, 3, 3000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT4-055", CardColor::Green, 3, 2);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "LV3", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let hand = r.add_to_hand(0, "BT4-055");
    r.game.decode_action(encode_digivolve(hand as u16, 0), 0);
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "BT4-055 When Digivolving suspends opponent Digimon with <= 3000 DP"
    );
}

#[test]
fn bt4_056_skullscorpiomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-056")
        .expect("BT4-056 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-056").expect("compiled BT4-056");
    assert_eq!(compiled.name, "SkullScorpiomon");
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(6000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-056");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(6000));
}

#[test]
fn bt4_057_grapleomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-057")
        .expect("BT4-057 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let gl = r.place_on_field(0, "BT4-057", Some(0));
    let before = r.memory();
    r.attack_player(gl, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), before + 1, "BT4-057 gains 1 memory when attacking");
}

#[test]
fn bt4_058_orochimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-058")
        .expect("BT4-058 must load")
        .add_card(mk_card("LV4", "GreenLv4", CardColor::Green, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT4-058", CardColor::Green, 4, 3);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let host = r.place_on_field(0, "LV4", Some(0));
    let hand = r.add_to_hand(0, "BT4-058");
    r.game.decode_action(encode_digivolve(hand as u16, 0), 0);
    let _ = r.auto_resolve();

    assert!(r.game.has_keyword(host, Keyword::Piercing), "BT4-058 grants Piercing for the turn");
}

#[test]
fn bt4_059_lilamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-059")
        .expect("BT4-059 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Green, 3, 2000, 3))
        .add_card(mk_card("S2", "Src2", CardColor::Green, 4, 5000, 5))
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-059"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));

    let act = first_field_effect_action(&r, 0).expect("BT4-059 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "BT4-059 Digi-Burst 2 suspends 1 opponent Digimon"
    );
}

#[test]
fn bt4_060_lotosmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-060")
        .expect("BT4-060 must load")
        .add_card(mk_card("LV4", "OppLv4", CardColor::Red, 4, 5000, 4))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "BT4-060", Some(0));
    let idx = r.add_to_hand(0, "LV4");
    r.play(0, idx);
    let _ = r.auto_resolve();

    assert!(
        r.game.players[0].battle_area[1].is_suspended,
        "BT4-060 suspends a Lv4 or lower Digimon when it is played"
    );
}
