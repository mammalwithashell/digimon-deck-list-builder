#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

//! Behavioral tests for BT2-061 through BT2-090 (excluding BT2-069 and BT2-076).

use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

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

// ── BT2-061 Andromon ─────────────────────────────────────────────────────────

#[test]
fn bt2_061_andromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-061")
        .expect("BT2-061")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-061"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    let idx = r.play(0, 0).expect("play BT2-061");
    let h = r.perm_handle(0, idx);
    assert!(r.game.has_keyword(h, Keyword::Blocker), "has <Blocker>");
    assert_eq!(r.dp_of(h), Some(7000));
}

// ── BT2-062 Infermon ─────────────────────────────────────────────────────────

#[test]
fn bt2_062_infermon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-062")
        .expect("BT2-062")
        .add_card(named_digimon("DIAB", "Diaboromon", CardColor::White, 6, 10000, 14))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["DIAB"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    set_evo_cost(&mut r, "DIAB", CardColor::Black, 5, 4);
    let base = r.place_on_field(0, "BT2-062", Some(0));
    let hi = hand_idx(&r, 0, "DIAB") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3, "cost 4 - Infermon 1 = 3 memory spent");
}

// ── BT2-063 MetalGreymon ─────────────────────────────────────────────────────

#[test]
fn bt2_063_metalgreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-063")
        .expect("BT2-063")
        .dsl_card("BT2-055")
        .expect("BT2-055 (inherited Reboot)")
        .add_card(digimon("HOST", CardColor::Black, 6, 11000, 11))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let top = r.place_on_field(0, "BT2-063", Some(0));
    assert!(r.game.has_keyword(top, Keyword::Reboot), "top has <Reboot>");

    let host = r.place_field_stack(0, &["BT2-055", "BT2-063", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.security_attack_keyword_bonus(host),
        1,
        "inherited aura grants <Security A. +1> while host has <Reboot>"
    );
}

// ── BT2-064 HiAndromon ───────────────────────────────────────────────────────

#[test]
fn bt2_064_hiandromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-064")
        .expect("BT2-064")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-064"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let card = r.compiled_card("BT2-064").expect("compiled");
    assert_eq!(card.cost, Some(10));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.level, Some(6));

    let idx = r.play(0, 0).expect("play BT2-064");
    assert_eq!(r.memory(), 0);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(12000));
}

// ── BT2-065 WarGreymon (Black) ───────────────────────────────────────────────

#[test]
fn bt2_065_wargreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-065")
        .expect("BT2-065")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-065", Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker), "has <Blocker>");
    assert!(r.game.has_keyword(h, Keyword::Reboot), "has <Reboot>");
    assert_eq!(r.dp_of(h), Some(11000));
}

// ── BT2-066 Machinedramon ────────────────────────────────────────────────────

#[test]
fn bt2_066_machinedramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-066")
        .expect("BT2-066")
        .add_card(digimon("LV3", CardColor::Blue, 3, 2000, 3))
        .add_card(digimon("LV4", CardColor::Blue, 4, 4000, 4))
        .add_card(digimon("LV5", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-066"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(14)
        .start();

    let o1 = r.place_field_stack(1, &["LV3", "LV4", "LV5"], false, 0);
    let o2 = r.place_field_stack(1, &["LV3", "LV4", "LV5"], false, 0);
    let idx = r.play(0, 0).expect("play BT2-066");
    let _ = r.auto_resolve_declaring_max();
    let h = r.perm_handle(0, idx);
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert_eq!(
        r.game.players[1].battle_area[o1.index as usize].card_sources.len(),
        1,
        "o1 De-Digivolved 2 to Lv.3"
    );
    assert_eq!(
        r.game.players[1].battle_area[o2.index as usize].card_sources.len(),
        1,
        "o2 De-Digivolved 2 to Lv.3"
    );
}

// ── BT2-067 DemiDevimon ──────────────────────────────────────────────────────

#[test]
fn bt2_067_demidevimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-067")
        .expect("BT2-067")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-067"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let card = r.compiled_card("BT2-067").expect("compiled");
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.level, Some(3));

    let idx = r.play(0, 0).expect("play BT2-067");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(3000));
}

// ── BT2-068 Impmon ───────────────────────────────────────────────────────────

#[test]
fn bt2_068_impmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-068")
        .expect("BT2-068")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-068", Some(0));
    let deck_before = r.deck_size(0);
    r.game.delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 3, "[On Deletion] trashed top 3 of deck");
}

// ── BT2-070 Tapirmon ─────────────────────────────────────────────────────────

#[test]
fn bt2_070_tapirmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-070")
        .expect("BT2-070")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-070", Some(0));
    let hand_before = r.hand_size(0);
    r.game.delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "[On Deletion] Draw 1");
}

// ── BT2-071 Wizardmon ────────────────────────────────────────────────────────

#[test]
fn bt2_071_wizardmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-071")
        .expect("BT2-071")
        .add_card(digimon("Y-MON", CardColor::Yellow, 3, 2000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let wiz = r.place_on_field(0, "BT2-071", Some(0));
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(wiz, Keyword::Retaliation));

    r.place_on_field(0, "Y-MON", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(wiz, Keyword::Retaliation), "yellow Digimon grants <Retaliation>");

    let hand_before = r.hand_size(0);
    r.game.delete_permanent_with_cause(wiz, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "[On Deletion] Draw 1");
}

// ── BT2-072 Vilemon ──────────────────────────────────────────────────────────

#[test]
fn bt2_072_vilemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-072")
        .expect("BT2-072")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_on_field(0, "BT2-072", Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker));

    let mem_before = r.memory();
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before - 2, "[When Attacking] lost 2 memory");
}

// ── BT2-073 Garurumon (Purple) ───────────────────────────────────────────────

#[test]
fn bt2_073_garurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-073")
        .expect("BT2-073")
        .add_card(digimon("HOST", CardColor::Purple, 5, 7000, 7))
        .add_card(digimon("ALLY", CardColor::Purple, 3, 2000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();

    r.place_field_stack(0, &["BT2-073", "HOST"], false, 0);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let mem_before = r.memory();
    r.game.delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before + 1, "other own Digimon deleted -> +1 memory");
}

// ── BT2-074 Devimon ──────────────────────────────────────────────────────────

#[test]
fn bt2_074_devimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-074")
        .expect("BT2-074")
        .add_card(digimon("HOST", CardColor::Purple, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let top = r.place_on_field(0, "BT2-074", Some(0));
    assert!(r.game.has_keyword(top, Keyword::Retaliation), "top <Retaliation>");

    let host = r.place_field_stack(0, &["BT2-074", "HOST"], false, 0);
    assert!(r.game.has_keyword(host, Keyword::Retaliation), "inherited <Retaliation>");
}

// ── BT2-075 Myotismon ────────────────────────────────────────────────────────

#[test]
fn bt2_075_myotismon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-075")
        .expect("BT2-075")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-075"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    let card = r.compiled_card("BT2-075").expect("compiled");
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.level, Some(5));

    let idx = r.play(0, 0).expect("play BT2-075");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(7000));
}

// ── BT2-077 Kimeramon ────────────────────────────────────────────────────────

#[test]
fn bt2_077_kimeramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-077")
        .expect("BT2-077")
        .add_card(digimon("SAC", CardColor::Purple, 3, 2000, 3))
        .add_card(digimon("OPP-LV5", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-077"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    r.place_on_field(0, "SAC", Some(0));
    r.place_on_field(1, "OPP-LV5", Some(0));
    r.play(0, 0).expect("play BT2-077");
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 1, "SAC deleted, Kimeramon remains");
    assert_eq!(r.battle_area_size(1), 0, "opp Lv.5 Digimon deleted");
}

// ── BT2-078 WereGarurumon ────────────────────────────────────────────────────

#[test]
fn bt2_078_weregarurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-078")
        .expect("BT2-078")
        .add_card(digimon("HOST", CardColor::Purple, 6, 11000, 11))
        .add_card(digimon("SAC", CardColor::Purple, 3, 2000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let host = r.place_field_stack(0, &["BT2-078", "HOST"], false, 0);
    r.place_on_field(0, "SAC", Some(0));
    let _ = r.attack_player(host, 1, false);
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 1, "SAC deleted");
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "inherited trigger unsuspended host"
    );
}

// ── BT2-079 VenomMyotismon ───────────────────────────────────────────────────

#[test]
fn bt2_079_venommyotismon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-079")
        .expect("BT2-079")
        .add_card(digimon("OPP", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(2)
        .start();

    let h = r.place_on_field(0, "BT2-079", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    assert_eq!(r.game.security_attack_keyword_bonus(h), 1, "has <Security A. +1>");

    // On opponent's turn, suspending opponent's Digimon gives P0 +1 memory (decreases P1's memory by 1).
    r.game.turn_player_idx = 1;
    r.game.set_memory(3);
    r.game.suspend(opp);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 2, "P0 gained 1 memory on opponent's turn");
}

// ── BT2-080 Piedmon ──────────────────────────────────────────────────────────

#[test]
fn bt2_080_piedmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-080")
        .expect("BT2-080")
        .add_card(digimon("P-LV3", CardColor::Purple, 3, 2000, 3))
        .add_card(digimon("P-LV4", CardColor::Purple, 4, 4000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-080"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(14)
        .start();

    add_to_trash(&mut r, 0, "P-LV3", 1);
    add_to_trash(&mut r, 0, "P-LV4", 1);
    let idx = r.play(0, 0).expect("play BT2-080");
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve_declaring_max();
    let h = r.perm_handle(0, idx);
    assert!(r.game.has_keyword(h, Keyword::Retaliation));
    assert_eq!(r.battle_area_size(0), 3, "Piedmon + 2 revived Lv.4- purple Digimon");
}

// ── BT2-081 MetalGarurumon (Purple) ──────────────────────────────────────────

#[test]
fn bt2_081_metalgarurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-081")
        .expect("BT2-081")
        .add_card(digimon("P-LV3", CardColor::Purple, 3, 2000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    add_to_trash(&mut r, 0, "P-LV3", 1);
    let h = r.place_on_field(0, "BT2-081", Some(0));
    let _ = r.attack_player(h, 1, false);
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 2, "[When Attacking] revived purple Lv.3 from trash");
}

// ── BT2-082 Diaboromon ───────────────────────────────────────────────────────

#[test]
fn bt2_082_diaboromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-082")
        .expect("BT2-082")
        .add_card(digimon("OPP-BIG", CardColor::Red, 6, 13000, 12))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let diab = r.place_on_field(0, "BT2-082", Some(0));
    let _ = r.attack_player(diab, 1, false);
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 2, "[When Attacking] played Diaboromon Token");

    // Replacement: when BT2-082 would be deleted in battle, delete Token to survive.
    r.game.unsuspend(diab);
    let opp = r.place_field_stack(1, &["OPP-BIG"], true, 0);
    let _ = r.attack_digimon(diab, opp, false);
    // Accept optional replacement and pick Token.
    while r.pending_selection().is_some() {
        if r.accept_optional_trigger().is_err() {
            let _ = r.auto_resolve();
            break;
        }
    }
    let _ = r.auto_resolve();
    assert_eq!(
        r.battle_area_size(0),
        2,
        "2nd attack spawned 2nd Token, 1 Token sacrificed, BT2-082 + 1 Token remain"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "BT2-082"),
        "BT2-082 survived battle via replacement"
    );
}

// ── BT2-083 Millenniummon ────────────────────────────────────────────────────

#[test]
fn bt2_083_millenniummon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-083")
        .expect("BT2-083")
        .add_card(digimon("LV6", CardColor::Black, 6, 11000, 11))
        .add_card(digimon("OPP", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-083"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    set_evo_cost(&mut r, "BT2-083", CardColor::Black, 6, 6);
    let base = r.place_on_field(0, "LV6", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    let opp_deck_before = r.deck_size(1);

    let hi = hand_idx(&r, 0, "BT2-083") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "[When Digivolving] bottom-decked opp Digimon");
    assert_eq!(r.deck_size(1), opp_deck_before + 1);

    // [On Deletion] had digivolution cards -> replay self from trash free.
    r.game.delete_permanent_with_cause(base, ReplacementCause::OpponentEffect);
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 1, "[On Deletion] replayed BT2-083 from trash");
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),
        "BT2-083"
    );
}

// ── BT2-084 Sora Takenouchi ──────────────────────────────────────────────────

#[test]
fn bt2_084_sora_takenouchi() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-084")
        .expect("BT2-084")
        .add_card(digimon("RED-ATK", CardColor::Red, 4, 5000, 5))
        .add_card(digimon("SEC-WEAK", CardColor::Blue, 3, 1000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC-WEAK"; 2])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let sora = r.place_on_field(0, "BT2-084", Some(0));
    let atk = r.place_on_field(0, "RED-ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[sora.index as usize].is_suspended);
    assert_eq!(r.dp_of(atk), Some(7000), "Sora suspended -> red attacker +2000 DP");
}

// ── BT2-085 Joe Kido ─────────────────────────────────────────────────────────

#[test]
fn bt2_085_joe_kido() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-085")
        .expect("BT2-085")
        .dsl_card("BT2-025")
        .expect("BT2-025 (Ikkakumon source trash)")
        .add_card(digimon("HOST", CardColor::Blue, 5, 7000, 7))
        .add_card(digimon("OPP-BOT", CardColor::Red, 3, 2000, 3))
        .add_card(digimon("OPP-TOP", CardColor::Red, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();

    r.game.turn_count = 2;
    let joe = r.place_on_field(0, "BT2-085", Some(0));
    let atk = r.place_field_stack(0, &["BT2-025", "HOST"], false, 0);
    r.place_field_stack(1, &["OPP-BOT", "OPP-TOP"], false, 0);

    let mem_before = r.memory();
    let _ = r.attack_player(atk, 1, false);
    while r.pending_selection().is_some() {
        if r.accept_optional_trigger().is_err() {
            let _ = r.auto_resolve();
            break;
        }
    }
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[joe.index as usize].is_suspended);
    assert_eq!(r.memory(), mem_before + 1, "Joe suspended when opp source trashed -> +1 memory");
}

// ── BT2-086 Rina Shinomiya ───────────────────────────────────────────────────

#[test]
fn bt2_086_rina_shinomiya() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-086")
        .expect("BT2-086")
        .add_card(named_digimon("VEEMON", "Veemon", CardColor::Blue, 3, 2000, 3))
        .add_card(digimon("BLUE-ATK", CardColor::Blue, 4, 5000, 5))
        .add_card(digimon("SEC-WEAK", CardColor::Red, 3, 1000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-086"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "VEEMON"])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC-WEAK"; 2])
        .memory(5)
        .start();

    let idx = r.play(0, 0).expect("play BT2-086");
    let _ = r.auto_resolve();
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "VEEMON"),
        "[On Play] added [Vee] Digimon to hand"
    );

    // [Your Turn] When blue Digimon attacks, suspend Rina -> +1000 DP.
    r.game.turn_count = 2;
    let atk = r.place_on_field(0, "BLUE-ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[idx].is_suspended);
    assert_eq!(r.dp_of(atk), Some(6000), "Rina suspended -> blue attacker +1000 DP");
}

// ── BT2-087 Kari Kamiya ──────────────────────────────────────────────────────

#[test]
fn bt2_087_kari_kamiya() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-087")
        .expect("BT2-087")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 3])
        .memory(1)
        .start();

    r.place_on_field(0, "BT2-087", Some(0));
    // End P1's turn with -1 memory so P0 starts turn with +1, then Kari adds +1 -> 2.
    r.game.turn_player_idx = 1;
    r.game.set_memory(-1);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player_idx, 0);
    assert_eq!(r.memory(), 2, "[Start of Your Turn] <= 3 security -> +1 memory");
}

// ── BT2-088 Taiga ────────────────────────────────────────────────────────────

#[test]
fn bt2_088_taiga() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-088")
        .expect("BT2-088")
        .add_card(digimon("G-LV3", CardColor::Green, 3, 2000, 3))
        .add_card(named_digimon("TYRAN", "Tyrannomon", CardColor::Green, 4, 5000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["TYRAN"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    set_evo_cost(&mut r, "TYRAN", CardColor::Green, 3, 2);
    let taiga = r.place_on_field(0, "BT2-088", Some(0));
    let base = r.place_on_field(0, "G-LV3", Some(0));

    let hi = hand_idx(&r, 0, "TYRAN") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    if r.pending_selection().is_some() {
        let _ = r.accept_optional_trigger();
    }
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[taiga.index as usize].is_suspended);
    assert_eq!(r.memory(), 4, "cost 2 - Taiga 1 = 1 memory spent");

    r.game.tick_declarative_effects();
    assert!(
        r.game.has_keyword(base, Keyword::Piercing),
        "[Your Turn] Tyrannomon gains <Piercing>"
    );
}

// ── BT2-089 Tai Kamiya (Black) ───────────────────────────────────────────────

#[test]
fn bt2_089_tai_kamiya() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-089")
        .expect("BT2-089")
        .add_card(digimon("BLK", CardColor::Black, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(1)
        .start();

    r.place_on_field(0, "BT2-089", Some(0));
    let blk = r.place_on_field(0, "BLK", Some(0));

    // [Opponent's Turn] Black Digimon +1000 DP.
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(blk), Some(6000), "[Opponent's Turn] black Digimon +1000 DP");

    // [Start of Your Turn] memory <= 2 -> set to 3.
    r.game.set_memory(-1);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player_idx, 0);
    assert_eq!(r.memory(), 3, "[Start of Your Turn] set memory to 3");
}

// ── BT2-090 Matt Ishida (Purple) ─────────────────────────────────────────────

#[test]
fn bt2_090_matt_ishida() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-090")
        .expect("BT2-090")
        .add_card(digimon("P-MON", CardColor::Purple, 3, 2000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-090"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    add_to_trash(&mut r, 0, "P-MON", 1);
    r.play(0, 0).expect("play BT2-090");
    let _ = r.auto_resolve();
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "P-MON"),
        "[On Play] returned purple Digimon from trash to hand"
    );

    // [Start of Your Turn] memory <= 2 -> set to 3.
    r.game.turn_player_idx = 1;
    r.game.set_memory(-1);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player_idx, 0);
    assert_eq!(r.memory(), 3, "[Start of Your Turn] set memory to 3");
}
