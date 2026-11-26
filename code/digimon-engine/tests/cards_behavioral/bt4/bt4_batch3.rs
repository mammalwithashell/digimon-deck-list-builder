#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
//! Behavioral tests for BT4 Batch 3: BT4-061 through BT4-090
//! (excluding BT4-072, BT4-086 which have dedicated modules).

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

fn mk_option(id: &str, name: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Option;
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
fn bt4_061_bancholeomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-061")
        .expect("BT4-061 must load")
        .add_card(mk_card("OPP1", "Opp1", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("OPP2", "Opp2", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let bl = r.place_on_field(0, "BT4-061", Some(0));
    let o1 = r.place_on_field(1, "OPP1", Some(0));
    let o2 = r.place_on_field(1, "OPP2", Some(0));

    r.game.delete_permanent_with_effects(bl);
    let _ = r.auto_resolve_declaring_max();

    assert!(r.game.players[1].battle_area[o1.index as usize].is_suspended);
    assert!(r.game.players[1].battle_area[o2.index as usize].is_suspended);
}

#[test]
fn bt4_062_nidhoggmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-062")
        .expect("BT4-062 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Green, 2, 1000, 0))
        .add_card(mk_card("S2", "Src2", CardColor::Green, 3, 2000, 3))
        .add_card(mk_card("S3", "Src3", CardColor::Green, 4, 5000, 5))
        .add_card(mk_card("LV5", "GreenLv5", CardColor::Green, 5, 7000, 7))
        .add_card(mk_card("OPP", "Opp5k", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Green, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    set_evo_cost(&mut r, "BT4-062", CardColor::Green, 5, 5);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "S3", "LV5"], false, 0);
    r.place_on_field(1, "OPP", Some(0));
    let hand = r.add_to_hand(0, "BT4-062");

    r.game.decode_action(encode_digivolve(hand as u16, 0), 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-062 Digi-Burst 4 suspends <= 5000 DP and bottoms suspended Digimon");
}

#[test]
fn bt4_063_commandramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-063")
        .expect("BT4-063 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.game.stage_inject_card(0, "BT4-063", "deck_top").unwrap();
    let cmd = r.place_on_field(0, "BT4-063", Some(0));
    r.game.delete_permanent_with_effects(cmd);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 1, "BT4-063 On Deletion plays revealed Commandramon for free");
    assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data), "BT4-063");
}

#[test]
fn bt4_064_sunarizamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-064")
        .expect("BT4-064 must load")
        .dsl_card("BT4-068")
        .expect("BT4-068 must load")
        .add_card(mk_card("SRC", "Src", CardColor::Black, 2, 1000, 0))
        .add_card(mk_card("OPP_SRC", "OppSrc", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("OPP_TOP", "OppTop", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["SRC", "BT4-064", "BT4-068"], false, 0);
    r.place_field_stack(1, &["OPP_SRC", "OPP_TOP"], false, 0);

    let act = first_field_effect_action(&r, 0).expect("BT4-068 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    let in_hand = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT4-064");
    assert!(in_hand, "BT4-064 returns to hand when trashed for Digi-Burst");
}

#[test]
fn bt4_065_gotsumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-065")
        .expect("BT4-065 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-065").expect("compiled BT4-065");
    assert_eq!(compiled.name, "Gotsumon");
    assert_eq!(compiled.cost, Some(4));
    assert_eq!(compiled.dp, Some(6000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-065");
    r.play(0, idx);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(6000));
}

#[test]
fn bt4_066_golemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-066")
        .expect("BT4-066 must load")
        .add_card(mk_card("BLK", "BlackAlly", CardColor::Black, 4, 5000, 5))
        .add_card(mk_card("RED", "RedAlly", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let gol = r.place_on_field(0, "BT4-066", Some(0));
    let blk = r.place_on_field(0, "BLK", Some(0));
    let red = r.place_on_field(0, "RED", Some(0));
    r.game.tick_declarative_effects();

    assert_eq!(r.dp_of(gol), Some(4000), "BT4-066 buffs itself +1000 DP");
    assert_eq!(r.dp_of(blk), Some(6000), "BT4-066 buffs own black Digimon +1000 DP");
    assert_eq!(r.dp_of(red), Some(5000), "BT4-066 does not buff non-black Digimon");
}

#[test]
fn bt4_067_sealsdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-067")
        .expect("BT4-067 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let seals = r.place_on_field(0, "BT4-067", Some(0));
    assert!(r.game.has_keyword(seals, Keyword::Blocker), "BT4-067 has Blocker");

    let before = r.memory();
    r.attack_player(seals, 1, true);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), before - 2, "BT4-067 loses 2 memory when attacking");
}

#[test]
fn bt4_068_baboongamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-068")
        .expect("BT4-068 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Black, 2, 1000, 0))
        .add_card(mk_card("S2", "Src2", CardColor::Black, 3, 2000, 3))
        .add_card(mk_card("OPP3", "OppLv3", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("OPP4", "OppLv4", CardColor::Red, 4, 5000, 5))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-068"], false, 0);
    let opp = r.place_field_stack(1, &["OPP3", "OPP4"], false, 0);

    let act = first_field_effect_action(&r, 0).expect("BT4-068 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].top_card().card_id(&r.game.card_data),
        "OPP3",
        "BT4-068 De-Digivolves 1 on opponent Digimon with play cost <= 7"
    );
}

#[test]
fn bt4_069_blimpmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-069")
        .expect("BT4-069 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-069").expect("compiled BT4-069");
    assert_eq!(compiled.name, "Blimpmon");
    assert_eq!(compiled.cost, Some(6));
    assert_eq!(compiled.dp, Some(7000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-069");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(7000));
}

#[test]
fn bt4_070_meteormon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-070")
        .expect("BT4-070 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let m = r.place_on_field(0, "BT4-070", Some(0));
    assert!(r.game.has_keyword(m, Keyword::Reboot), "BT4-070 has Reboot");
}

#[test]
fn bt4_071_tankdramon() {
    let mut dbrig = mk_card("DB_ALLY", "DBAlly", CardColor::Black, 3, 2000, 3);
    dbrig.traits = vec!["D-Brigade".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-071")
        .expect("BT4-071 must load")
        .dsl_card("BT4-063")
        .expect("BT4-063 must load")
        .add_card(dbrig)
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.game.stage_inject_card(0, "BT4-063", "deck_top").unwrap();
    r.place_on_field(0, "BT4-071", Some(0));
    let ally = r.place_on_field(0, "DB_ALLY", Some(0));

    r.game.delete_permanent_with_effects(ally);
    let _ = r.auto_resolve();

    let has_cmd = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BT4-063");
    assert!(has_cmd, "BT4-071 reveals top 2 and plays Commandramon free when other D-Brigade is deleted");
}

#[test]
fn bt4_073_banchogolemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-073")
        .expect("BT4-073 must load")
        .add_card(mk_card("OPP", "OppMon", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.set_first_player(1);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let bg = r.place_on_field(0, "BT4-073", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(bg, Keyword::Blocker), "BT4-073 has Blocker");
    assert_eq!(r.effective_dp(bg), Some(11000), "only 2 opponent Digimon -> base 11000 DP");

    r.place_on_field(1, "OPP", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(bg), Some(14000), "3+ opponent Digimon on opp turn -> +3000 DP");
}

#[test]
fn bt4_074_darkdramon() {
    let mut dbrig = mk_card("DB", "Commandramon", CardColor::Black, 3, 2000, 3);
    dbrig.traits = vec!["D-Brigade".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT4-074")
        .expect("BT4-074 must load")
        .add_card(dbrig)
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    for _ in 0..3 {
        add_to_trash(&mut r, 0, "DB");
    }
    let idx = r.add_to_hand(0, "BT4-074");
    r.play(0, idx);
    let _ = r.auto_resolve_declaring_max();

    let h = r.perm_handle(0, 0);
    assert!(r.game.has_keyword(h, Keyword::Rush), "BT4-074 has Rush");
    // Played for 13 (10 - 13 = -3), returned 3 D-Brigade (+6) -> +3
    assert_eq!(r.memory(), 3, "BT4-074 gains 2 memory per returned D-Brigade");
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn bt4_075_blastmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-075")
        .expect("BT4-075 must load")
        .add_card(mk_card("OPP_CHUMP", "OppChump", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Black, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let blast = r.place_on_field(0, "BT4-075", Some(0));
    r.place_on_field(1, "OPP_CHUMP", Some(0));
    assert!(r.game.security_attack_keyword_bonus(blast) >= 1, "BT4-075 has Security A. +1");

    r.attack_player(blast, 1, true);
    let _ = r.auto_resolve();

    // Opponent redirected attack to their unsuspended chump -> chump deleted, security untouched
    assert_eq!(r.battle_area_size(1), 0, "redirected target deleted in battle");
    assert_eq!(r.security_count(1), 3, "security untouched after opponent redirected attack");
}

#[test]
fn bt4_076_gabumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-076")
        .expect("BT4-076 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-076").expect("compiled BT4-076");
    assert_eq!(compiled.name, "Gabumon");
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(4000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-076");
    r.play(0, idx);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(4000));
}

#[test]
fn bt4_077_ghostmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-077")
        .expect("BT4-077 must load")
        .dsl_card("BT4-081")
        .expect("BT4-081 must load")
        .add_card(mk_card("SRC", "Src", CardColor::Purple, 2, 1000, 0))
        .add_card(mk_card("OPP3", "OppLv3", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["SRC", "BT4-077", "BT4-081"], false, 0);
    r.place_on_field(1, "OPP3", Some(0));

    let act = first_field_effect_action(&r, 0).expect("BT4-081 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    let in_hand = r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "BT4-077");
    assert!(in_hand, "BT4-077 returns to hand when trashed for Digi-Burst");
}

#[test]
fn bt4_078_soundbirdmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-078")
        .expect("BT4-078 must load")
        .add_card(mk_option("OPT", "TestOpt", CardColor::Purple, 2))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let sb = r.place_on_field(0, "BT4-078", Some(0));
    r.add_to_hand(0, "OPT");
    let before = r.memory();

    r.attack_player(sb, 1, true);
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), before + 1, "BT4-078 trashes Option from hand to gain 1 memory");
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn bt4_079_labramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-079")
        .expect("BT4-079 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let deck_before = r.deck_size(0);
    let idx = r.add_to_hand(0, "BT4-079");
    r.play(0, idx);
    let _ = r.auto_resolve();

    assert_eq!(r.deck_size(0), deck_before - 1, "BT4-079 draws 1");
    assert_eq!(r.trash_size(0), 1, "BT4-079 trashes 1 from hand");
}

#[test]
fn bt4_080_bakemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-080")
        .expect("BT4-080 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-080").expect("compiled BT4-080");
    assert_eq!(compiled.name, "Bakemon");
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(6000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-080");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(6000));
}

#[test]
fn bt4_081_devimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-081")
        .expect("BT4-081 must load")
        .add_card(mk_card("S1", "Src1", CardColor::Purple, 2, 1000, 0))
        .add_card(mk_card("S2", "Src2", CardColor::Purple, 3, 2000, 3))
        .add_card(mk_card("OPP3", "OppLv3", CardColor::Red, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["S1", "S2", "BT4-081"], false, 0);
    r.place_on_field(1, "OPP3", Some(0));

    let act = first_field_effect_action(&r, 0).expect("BT4-081 Digi-Burst 2 must be legal");
    r.game.decode_action(act, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "BT4-081 Digi-Burst 2 deletes opponent Lv3 Digimon");
}

#[test]
fn bt4_082_dobermon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-082")
        .expect("BT4-082 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(7)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-082").expect("compiled BT4-082");
    assert_eq!(compiled.name, "Dobermon");
    assert_eq!(compiled.cost, Some(6));
    assert_eq!(compiled.dp, Some(7000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-082");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(7000));
}

#[test]
fn bt4_083_cerberusmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-083")
        .expect("BT4-083 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let cerb = r.place_on_field(0, "BT4-083", Some(0));
    let deck_before = r.deck_size(0);
    r.game.delete_permanent_with_effects(cerb);
    let _ = r.auto_resolve();

    assert_eq!(r.deck_size(0), deck_before - 2, "BT4-083 On Deletion draws 2");
    assert_eq!(r.hand_size(0), 1, "BT4-083 trashes 1 of the 2 drawn cards");
}

#[test]
fn bt4_084_neodevimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-084")
        .expect("BT4-084 must load")
        .add_card(mk_tamer("OPP_TAMER", "OppTamer", CardColor::Red, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();
    r.set_first_player(1);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "BT4-084", Some(0));
    let before = r.memory(); // from P1's perspective: +6
    let idx = r.add_to_hand(1, "OPP_TAMER");
    r.play(1, idx);
    let _ = r.auto_resolve();

    // P1 paid 3 (6 -> 3), P0 gained 3 (3 -> 0 from P1's perspective)
    assert_eq!(r.memory(), before - 6, "BT4-084 gains 3 memory when opponent plays a Tamer");
}

#[test]
fn bt4_085_phantomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-085")
        .expect("BT4-085 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let compiled = r.compiled_card("BT4-085").expect("compiled BT4-085");
    assert_eq!(compiled.name, "Phantomon");
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(10000));
    assert!(compiled.effects.is_empty());

    let idx = r.add_to_hand(0, "BT4-085");
    r.play(0, idx);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(10000));
}

#[test]
fn bt4_087_anubismon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-087")
        .expect("BT4-087 must load")
        .add_card(mk_card("LV5", "PurpleLv5", CardColor::Purple, 5, 7000, 7))
        .add_card(mk_card("LV3", "PurpleLv3", CardColor::Purple, 3, 2000, 3))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();
    set_evo_cost(&mut r, "BT4-087", CardColor::Purple, 5, 3);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "LV5", Some(0));
    add_to_trash(&mut r, 0, "LV3");
    let hand = r.add_to_hand(0, "BT4-087");

    r.game.decode_action(encode_digivolve(hand as u16, 0), 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 2, "BT4-087 plays Lv3 Digimon from trash free");
    let revived = r.perm_handle(0, 1);
    assert!(r.game.has_keyword(revived, Keyword::Rush), "BT4-087 grants Rush to Digimon played from trash");
}

#[test]
fn bt4_088_dandevimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-088")
        .expect("BT4-088 must load")
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .hand(1, &["FILL"; 3])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    let dd = r.place_on_field(0, "BT4-088", Some(0));
    r.game.delete_permanent_with_effects(dd);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(1), 1, "BT4-088 On Deletion forces opponent to trash 2 cards from hand");
}

#[test]
fn bt4_089_plutomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-089")
        .expect("BT4-089 must load")
        .dsl_card("BT4-112")
        .expect("BT4-112 must load")
        .add_card(mk_card("LV5", "PurpleLv5", CardColor::Purple, 5, 7000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::Purple, 3, 2000, 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(8)
        .start();
    set_evo_cost(&mut r, "BT4-089", CardColor::Purple, 5, 4);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_on_field(0, "LV5", Some(0));
    let deck_before = r.deck_size(0);
    let hand = r.add_to_hand(0, "BT4-089");

    r.game.decode_action(encode_digivolve(hand as u16, 0), 0);
    let _ = r.auto_resolve();

    // 1 evo draw + 2 Plutomon draw = 3 cards drawn
    assert_eq!(r.deck_size(0), deck_before - 3, "BT4-089 When Digivolving draws 2");
}

#[test]
fn bt4_090_chaosmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT4-090")
        .expect("BT4-090 must load")
        .add_card(mk_card("LV6", "GreenLv6", CardColor::Green, 6, 11000, 11))
        .add_card(mk_card("OPP", "OppUnsusp", CardColor::Red, 5, 8000, 7))
        .add_card(mk_card("FILL", "Filler", CardColor::White, 3, 2000, 3))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(10)
        .start();
    set_evo_cost(&mut r, "BT4-090", CardColor::Green, 6, 6);
    r.game.turn_count = 2;
    r.set_phase(GamePhase::Main);

    r.place_field_stack(0, &["LV6"], true, 0);
    r.place_on_field(1, "OPP", Some(0));
    let hand = r.add_to_hand(0, "BT4-090");

    r.game.decode_action(encode_digivolve(hand as u16, 0), 0);
    let _ = r.auto_resolve();

    let h = r.perm_handle(0, 0);
    assert!(r.game.has_keyword(h, Keyword::Piercing), "BT4-090 has Piercing");
    assert_eq!(r.battle_area_size(1), 0, "BT4-090 unsuspends and attacks opponent's unsuspended Digimon");
}
