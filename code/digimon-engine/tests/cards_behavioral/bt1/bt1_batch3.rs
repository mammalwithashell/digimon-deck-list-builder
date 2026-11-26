//! Behavioral tests for BT1-061 through BT1-090 (excluding BT1-085, BT1-087,
//! and BT1-090, which have dedicated per-card modules — 27 cards total).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::CompiledColor;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, PlayerId};
use digimon_engine::selection::TriggerSource;

fn make_digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn set_evo_cost(r: &mut DebugRunner, card_id: &str, card_color: u8, level: u8, memory_cost: u16) {
    let cd = r
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == card_id)
        .expect("card_data must contain card_id");
    cd.evo_costs = vec![EvoCost {
        card_color,
        level,
        memory_cost,
    }];
}

fn act(r: &mut DebugRunner, player: PlayerId, action: u16) {
    assert_eq!(
        build_action_mask(&r.game, player)[action as usize],
        1.0,
        "action {} must be legal for player {} in phase {:?}",
        action,
        player,
        r.current_phase()
    );
    r.game.decode_action(action, player);
}

fn resolve_first_pending(r: &mut DebugRunner) {
    let mut guard = 0;
    while let Some(sel) = r.pending_selection() {
        let player = sel.selecting_player;
        let action = sel.valid_action_ids.first().copied().unwrap_or(PASS);
        r.execute_action(player, action)
            .expect("pending selection action must succeed");
        guard += 1;
        assert!(guard < 20, "pending selection loop did not terminate");
    }
}

// ─── BT1-061 Mistymon ────────────────────────────────────────────────────────

#[test]
fn bt1_061_mistymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-061")
        .expect("BT1-061 loads")
        .add_card(make_digimon("OPP-A", "OppA", CardColor::Red, 4, 5000))
        .add_card(make_digimon("OPP-B", "OppB", CardColor::Red, 4, 6000))
        .memory(8)
        .start();

    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    let h = r.add_to_hand(0, "BT1-061");
    r.play(0, h).expect("Mistymon plays");
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(a), Some(2000), "first opp Digimon gets -3000 DP");
    assert_eq!(r.effective_dp(b), Some(3000), "second opp Digimon gets -3000 DP");
}

// ─── BT1-062 SlashAngemon ────────────────────────────────────────────────────

#[test]
fn bt1_062_slashangemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-062")
        .expect("BT1-062 loads")
        .add_card(make_digimon("YEL-LV5", "YelLv5", CardColor::Yellow, 5, 7000))
        .add_card(make_digimon("OPP-BIG", "OppBig", CardColor::Red, 6, 11000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT1-062", 2, 5, 3); // Yellow = 2, Lv5 -> cost 3
    r.game.turn_count = 2;

    let _host = r.place_on_field(0, "YEL-LV5", Some(0));
    let opp = r.place_on_field(1, "OPP-BIG", Some(0));
    let h = r.add_to_hand(0, "BT1-062");
    act(&mut r, 0, encode_digivolve(h as u16, 0));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(opp), Some(3000), "SlashAngemon gives -8000 DP");
}

// ─── BT1-063 Seraphimon ──────────────────────────────────────────────────────

#[test]
fn bt1_063_seraphimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-063")
        .expect("BT1-063 loads")
        .add_card(make_digimon("YEL-LV5", "YelLv5", CardColor::Yellow, 5, 7000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .security(0, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT1-063", 2, 5, 3);
    r.game.turn_count = 2;

    let host = r.place_on_field(0, "YEL-LV5", Some(0));
    let h = r.add_to_hand(0, "BT1-063");
    act(&mut r, 0, encode_digivolve(h as u16, 0));
    resolve_first_pending(&mut r);
    r.game.tick_declarative_effects();

    assert_eq!(r.security_count(0), 3, "When Digivolving Recovery +1 raises security 2 -> 3");
    assert!(
        r.game.has_keyword(host, Keyword::SecurityAttackPlus(1)),
        "at 3+ security Seraphimon gains <Security A. +1>"
    );
}

// ─── BT1-064 Goblimon (Vanilla) ──────────────────────────────────────────────

#[test]
fn bt1_064_goblimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-064")
        .expect("BT1-064 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-064").unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.color, vec![CompiledColor::Green]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-064");
    r.play(0, h).expect("Goblimon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(3000));
}

// ─── BT1-065 Mushroomon (Vanilla) ────────────────────────────────────────────

#[test]
fn bt1_065_mushroomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-065")
        .expect("BT1-065 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-065").unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::Green]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-065");
    r.play(0, h).expect("Mushroomon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(4000));
}

// ─── BT1-066 Tentomon ────────────────────────────────────────────────────────

#[test]
fn bt1_066_tentomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-066")
        .expect("BT1-066 loads")
        .add_card(make_digimon("GRN-HOST", "GrnHost", CardColor::Green, 4, 5000))
        .add_card(make_digimon("OPP-SMALL", "OppSmall", CardColor::Red, 3, 3000))
        .add_card(make_digimon("OPP-BIG", "OppBig", CardColor::Red, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _host = r.place_field_stack(0, &["BT1-066", "GRN-HOST"], false, 0);
    let _small = r.place_on_field(1, "OPP-SMALL", Some(0));
    let _big = r.place_on_field(1, "OPP-BIG", Some(0));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert!(r.game.players[1].battle_area[0].is_suspended, "3000 DP opp suspended");
    assert!(!r.game.players[1].battle_area[1].is_suspended, "5000 DP opp untouched");
}

// ─── BT1-067 Palmon ──────────────────────────────────────────────────────────

#[test]
fn bt1_067_palmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-067")
        .expect("BT1-067 loads")
        .add_card(make_digimon("LV4-MON", "Lv4Mon", CardColor::Green, 4, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 2000))
        // Last element is top of deck.
        .deck(0, &["FILLER", "FILLER", "FILLER", "LV4-MON"])
        .memory(5)
        .start();

    let h = r.add_to_hand(0, "BT1-067");
    r.play(0, h).expect("Palmon plays");
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "LV4-MON");
    assert_eq!(r.deck_size(0), 3);
}

// ─── BT1-068 Kokuwamon ───────────────────────────────────────────────────────

#[test]
fn bt1_068_kokuwamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-068")
        .expect("BT1-068 loads")
        .add_card(make_digimon("GRN-LV5", "GrnLv5", CardColor::Green, 5, 7000))
        .add_card(make_digimon("GRN-LV6", "GrnLv6", CardColor::Green, 6, 11000))
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let lv5 = r.place_stack(0, &["BT1-068", "GRN-LV5"]);
    let lv6 = r.place_stack(0, &["BT1-068", "GRN-LV6"]);
    r.game.tick_declarative_effects();

    assert!(!r.game.has_keyword(lv5, Keyword::SecurityAttackPlus(1)), "Lv5 does not qualify");
    assert!(r.game.has_keyword(lv6, Keyword::SecurityAttackPlus(1)), "Lv6 gains <Security A. +1>");
}

// ─── BT1-069 Ogremon ─────────────────────────────────────────────────────────

#[test]
fn bt1_069_ogremon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-069")
        .expect("BT1-069 loads")
        .add_card(make_digimon("BIG-SEC", "BigSec", CardColor::Red, 6, 12000))
        .security(1, &["BIG-SEC", "BIG-SEC"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let ogre = r.place_on_field(0, "BT1-069", Some(0));
    assert!(r.game.has_keyword(ogre, Keyword::Jamming));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert_eq!(r.battle_area_size(0), 1, "Ogremon Jamming survives 12000 DP security");
}

// ─── BT1-070 Kuwagamon ───────────────────────────────────────────────────────

#[test]
fn bt1_070_kuwagamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-070")
        .expect("BT1-070 loads")
        .add_card(make_digimon("OPP-DIGI", "OppDigi", CardColor::Red, 4, 5000))
        .memory(6)
        .start();

    let _opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    let h = r.add_to_hand(0, "BT1-070");
    r.play(0, h).expect("Kuwagamon plays");
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert!(r.game.players[1].battle_area[0].is_suspended, "Kuwagamon On Play suspends opp Digimon");
}

// ─── BT1-071 Vegiemon (Vanilla) ──────────────────────────────────────────────

#[test]
fn bt1_071_vegiemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-071")
        .expect("BT1-071 loads")
        .memory(6)
        .start();

    let card = r.compiled_card("BT1-071").unwrap();
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.color, vec![CompiledColor::Green]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-071");
    r.play(0, h).expect("Vegiemon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT1-072 Woodmon ─────────────────────────────────────────────────────────

#[test]
fn bt1_072_woodmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-072")
        .expect("BT1-072 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(4)
        .start();
    r.game.turn_count = 2;

    let wood = r.place_on_field(0, "BT1-072", Some(0));
    assert!(r.game.has_keyword(wood, Keyword::Blocker));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert_eq!(r.memory(), 2, "Woodmon When Attacking loses 2 memory");
}

// ─── BT1-073 Kabuterimon ─────────────────────────────────────────────────────

#[test]
fn bt1_073_kabuterimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-073")
        .expect("BT1-073 loads")
        .add_card(make_digimon("GRN-HOST", "GrnHost", CardColor::Green, 5, 7000))
        .add_card(make_digimon("OPP-DIGI", "OppDigi", CardColor::Red, 3, 2000))
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let host = r.place_stack(0, &["BT1-073", "GRN-HOST"]);
    r.place_field_stack(1, &["OPP-DIGI"], true, 0);
    r.place_field_stack(1, &["OPP-DIGI"], true, 0);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(host),
        Some(9000),
        "Kabuterimon inherited grants +1000 DP per suspended opp Digimon (+2000)"
    );
}

// ─── BT1-074 Togemon ─────────────────────────────────────────────────────────

#[test]
fn bt1_074_togemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-074")
        .expect("BT1-074 loads")
        .add_card(make_digimon("GRN-LV3", "GrnLv3", CardColor::Green, 3, 3000))
        .add_card(make_digimon("LV5-MON", "Lv5Mon", CardColor::Green, 5, 7000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        // Last element is drawn by digivolve draw; next 3 from end are revealed (includes LV5-MON).
        .deck(0, &["FILLER", "FILLER", "LV5-MON", "FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT1-074", 3, 3, 2); // Green = 3, Lv3 -> cost 2
    r.game.turn_count = 2;

    let _base = r.place_on_field(0, "GRN-LV3", Some(0));
    let h = r.add_to_hand(0, "BT1-074");
    act(&mut r, 0, encode_digivolve(h as u16, 0));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 2, "1 evo draw + 1 Lv5+ Digimon added from reveal");
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LV5-MON"));
}

// ─── BT1-075 Digitamamon ─────────────────────────────────────────────────────

#[test]
fn bt1_075_digitamamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-075")
        .expect("BT1-075 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(2)
        .start();
    r.game.turn_count = 2;

    let _digi = r.place_on_field(0, "BT1-075", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 5, "When Attacking gains 3 memory");
    r.game.fire_end_of_your_turn(0);
    assert_eq!(r.memory(), 2, "End of turn loses 3 memory");
}

// ─── BT1-076 MegaKabuterimon ─────────────────────────────────────────────────

#[test]
fn bt1_076_megakabuterimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-076")
        .expect("BT1-076 loads")
        .add_card(make_digimon("GRN-LV6", "GrnLv6", CardColor::Green, 6, 11000))
        .add_card(make_digimon("OPP-DIGI", "OppDigi", CardColor::Red, 3, 2000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(2)
        .start();
    r.game.turn_count = 2;

    let _host = r.place_field_stack(0, &["BT1-076", "GRN-LV6"], false, 0);
    r.place_field_stack(1, &["OPP-DIGI"], true, 0);
    r.place_field_stack(1, &["OPP-DIGI"], true, 0);

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 3, "MegaKabuterimon inherited gains 1 memory with 2+ suspended opps");
}

// ─── BT1-077 Okuwamon ────────────────────────────────────────────────────────

#[test]
fn bt1_077_okuwamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-077")
        .expect("BT1-077 loads")
        .add_card(make_digimon("GRN-LV6", "GrnLv6", CardColor::Green, 6, 11000))
        .add_card(make_digimon("OPP-WEAK", "OppWeak", CardColor::Red, 3, 3000))
        .memory(2)
        .start();
    r.game.turn_count = 2;

    let _host = r.place_field_stack(0, &["BT1-077", "GRN-LV6"], false, 0);
    let _opp = r.place_field_stack(1, &["OPP-WEAK"], true, 0);

    act(&mut r, 0, encode_attack(0, 0));
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.memory(), 3, "Okuwamon inherited gains 1 memory on battle deletion + survival");
}

// ─── BT1-078 Jagamon ─────────────────────────────────────────────────────────

#[test]
fn bt1_078_jagamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-078")
        .expect("BT1-078 loads")
        .add_card(make_digimon("GRN-LV6", "GrnLv6", CardColor::Green, 6, 11000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        // Last element is top of deck (in top 3 revealed).
        .deck(0, &["FILLER", "FILLER", "FILLER", "GRN-LV6"])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "GRN-LV6", 3, 5, 3);
    r.game.turn_count = 2;

    let _jag = r.place_on_field(0, "BT1-078", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some(), "Jagamon must prompt to digivolve from top 3");
    resolve_first_pending(&mut r);

    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "GRN-LV6",
        "Jagamon digivolved into revealed Lv6 green Digimon for free"
    );
    assert_eq!(r.memory(), 5);
}

// ─── BT1-079 Lillymon ────────────────────────────────────────────────────────

#[test]
fn bt1_079_lillymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-079")
        .expect("BT1-079 loads")
        .dsl_card("BT1-072")
        .expect("BT1-072 (Woodmon, <Blocker>) loads")
        .add_card(make_digimon("GRN-LV6", "GrnLv6", CardColor::Green, 6, 11000))
        .add_card(make_digimon("OPP-PLAIN", "OppPlain", CardColor::Red, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _host = r.place_field_stack(0, &["BT1-079", "GRN-LV6"], false, 0);
    let _plain = r.place_on_field(1, "OPP-PLAIN", Some(0));
    let _blk = r.place_on_field(1, "BT1-072", Some(0));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    // First pending is Lillymon's When Attacking suspend (filters out Blocker)
    let sel = r.pending_selection().expect("Lillymon selection must prompt");
    let action = sel.valid_action_ids[0];
    r.execute_action(sel.selecting_player, action).unwrap();

    assert!(r.game.players[1].battle_area[0].is_suspended, "non-Blocker suspended");
    assert!(!r.game.players[1].battle_area[1].is_suspended, "Blocker excluded from target");
}

// ─── BT1-080 Titamon (Vanilla) ───────────────────────────────────────────────

#[test]
fn bt1_080_titamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-080")
        .expect("BT1-080 loads")
        .memory(10)
        .start();

    let card = r.compiled_card("BT1-080").unwrap();
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(10));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.color, vec![CompiledColor::Green]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-080");
    r.play(0, h).expect("Titamon plays");
    assert_eq!(r.memory(), 0);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(12000));
}

// ─── BT1-081 HerculesKabuterimon ─────────────────────────────────────────────

#[test]
fn bt1_081_herculeskabuterimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-081")
        .expect("BT1-081 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let herc = r.place_on_field(0, "BT1-081", Some(0));
    assert!(r.game.has_keyword(herc, Keyword::Piercing));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some(), "End of Attack optional unsuspend must prompt");
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 2, "End of Attack paid 3 memory (5 -> 2)");
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "HerculesKabuterimon unsuspended"
    );
}

// ─── BT1-082 Rosemon ─────────────────────────────────────────────────────────

#[test]
fn bt1_082_rosemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-082")
        .expect("BT1-082 loads")
        .add_card(make_digimon("OPP-ATK", "OppAtk", CardColor::Red, 4, 5000))
        .add_card(make_digimon("OPP-OTHER", "OppOther", CardColor::Red, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .security(0, &["FILLER", "FILLER"])
        .build();
    r.set_first_player(1);
    r.set_turn(2);
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);

    // Rosemon belongs to player 0 and is suspended during player 1's turn.
    let _rose = r.place_field_stack(0, &["BT1-082"], true, 0);
    let _atk = r.place_on_field(1, "OPP-ATK", Some(0));
    let _other = r.place_on_field(1, "OPP-OTHER", Some(0));

    act(&mut r, 1, encode_attack(0, SECURITY_TARGET));
    assert!(
        r.pending_selection().is_some(),
        "Rosemon Opponent's Turn trigger must prompt to suspend an opponent's Digimon"
    );
    // Pick the unsuspended second opponent Digimon (index 1) if available, or first valid.
    let sel = r.pending_selection().unwrap();
    let action = sel.valid_action_ids.last().copied().unwrap();
    r.execute_action(sel.selecting_player, action).unwrap();
    resolve_first_pending(&mut r);

    assert!(
        r.game.players[1].battle_area[1].is_suspended,
        "Rosemon suspended the chosen opponent Digimon"
    );
}

// ─── BT1-083 GranKuwagamon ───────────────────────────────────────────────────

#[test]
fn bt1_083_grankuwagamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-083")
        .expect("BT1-083 loads")
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let gk = r.place_on_field(0, "BT1-083", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(gk, Keyword::Piercing));
    assert_eq!(r.effective_dp(gk), Some(15000), "Your Turn aura grants +4000 DP (11000 -> 15000)");

    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(gk), Some(11000), "aura inactive on opponent's turn");
}

// ─── BT1-084 Omnimon ─────────────────────────────────────────────────────────

#[test]
fn bt1_084_omnimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-084")
        .expect("BT1-084 loads")
        .add_card(make_digimon("BLUE-LV6", "BlueLv6", CardColor::Blue, 6, 11000))
        .add_card(make_digimon("OPP-A1", "Agumon", CardColor::Red, 3, 2000))
        .add_card(make_digimon("OPP-A2", "Agumon", CardColor::Red, 3, 2000))
        .add_card(make_digimon("OPP-B", "Gabumon", CardColor::Blue, 3, 2000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(8)
        .start();
    r.game.turn_count = 2;

    let _base = r.place_on_field(0, "BLUE-LV6", Some(0));
    let _a1 = r.place_on_field(1, "OPP-A1", Some(0));
    let _a2 = r.place_on_field(1, "OPP-A2", Some(0));
    let _b = r.place_on_field(1, "OPP-B", Some(0));

    // Digivolve onto Blue Lv6 via alt_paths (cost 6) -> When Digivolving deletes all "Agumon"
    let h = r.add_to_hand(0, "BT1-084");
    act(&mut r, 0, encode_digivolve(h as u16, 0));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(1), 1, "both Agumon deleted; Gabumon survives");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "OPP-B"
    );

    // When Attacking: return Lv6 source (BLUE-LV6) to hand to unsuspend
    let hand_before = r.hand_size(0);
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), hand_before + 1, "Lv6 source returned to hand");
    assert!(!r.game.players[0].battle_area[0].is_suspended, "Omnimon unsuspended");
}

// ─── BT1-086 Matt Ishida ─────────────────────────────────────────────────────

#[test]
fn bt1_086_matt_ishida() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-086")
        .expect("BT1-086 loads")
        .add_card(make_digimon("BLUE-LV3", "BlueLv3", CardColor::Blue, 3, 2000))
        .add_card(make_digimon("OPP-SRC", "OppSrc", CardColor::Red, 3, 2000))
        .add_card(make_digimon("OPP-TOP", "OppTop", CardColor::Red, 4, 5000))
        .memory(1)
        .start();
    r.game.turn_count = 1;

    let matt = r.place_on_field(0, "BT1-086", Some(0));
    // Start of Your Turn: memory <= 2 -> set to 3
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(matt));
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 3, "Matt sets memory to 3");

    // Your Turn: play blue Digimon -> suspend Matt -> trash bottom source of opp Digimon
    r.game.set_memory(6);
    let _opp = r.place_stack(1, &["OPP-SRC", "OPP-TOP"]);
    let h = r.add_to_hand(0, "BLUE-LV3");
    r.play(0, h).expect("BlueLv3 plays");
    assert!(r.pending_selection().is_some(), "Matt optional trigger must prompt");
    resolve_first_pending(&mut r);

    assert!(r.game.players[0].battle_area[0].is_suspended, "Matt suspended as cost");
    assert_eq!(
        r.game.players[1].battle_area[0].card_sources.len(),
        1,
        "bottom digivolution card trashed"
    );
}

// ─── BT1-088 Izzy Izumi ──────────────────────────────────────────────────────

#[test]
fn bt1_088_izzy_izumi() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-088")
        .expect("BT1-088 loads")
        .add_card(make_digimon("GRN-LV5", "GrnLv5", CardColor::Green, 5, 7000))
        .add_card(make_digimon("TOP-DIGI", "TopDigi", CardColor::Green, 4, 4000))
        // Last element is top of deck.
        .deck(0, &["GRN-LV5", "TOP-DIGI"])
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let _izzy = r.place_on_field(0, "BT1-088", Some(0));
    let _lv5 = r.place_on_field(0, "GRN-LV5", Some(0));

    let main_act = FIELD_EFFECT_START + 0 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    act(&mut r, 0, main_act);
    resolve_first_pending(&mut r);

    assert!(r.game.players[0].battle_area[0].is_suspended, "Izzy suspended");
    assert_eq!(r.hand_size(0), 1, "revealed Digimon added to hand");
    assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "TOP-DIGI");
}

// ─── BT1-089 Mimi Tachikawa ──────────────────────────────────────────────────

#[test]
fn bt1_089_mimi_tachikawa() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-089")
        .expect("BT1-089 loads")
        .add_card(make_digimon("GRN-LV5", "GrnLv5", CardColor::Green, 5, 7000))
        .add_card(make_test_egg("GRN-EGG", "GrnEgg"))
        .digitama(0, &["GRN-EGG"])
        .memory(1)
        .start();
    r.game.turn_count = 1;
    // With a digitama egg present, start() stops in Breeding phase; advance to Main.
    r.set_phase(GamePhase::Main);

    let mimi = r.place_on_field(0, "BT1-089", Some(0));
    let _lv5 = r.place_on_field(0, "GRN-LV5", Some(0));

    // Start of Your Turn: memory 1 -> 3
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(mimi));
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 3, "Mimi sets memory to 3");

    // [Main]: suspend Mimi to hatch into empty breeding area
    let main_act = FIELD_EFFECT_START + 0 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN;
    act(&mut r, 0, main_act);
    resolve_first_pending(&mut r);

    assert!(r.game.players[0].battle_area[0].is_suspended, "Mimi suspended");
    assert!(
        r.game.players[0].breeding_area.is_some(),
        "Mimi hatched Digi-Egg into breeding area"
    );
}
