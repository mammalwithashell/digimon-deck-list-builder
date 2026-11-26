#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

//! Behavioral tests for BT2-001 through BT2-030.

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

fn named_digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 4;
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

// ── BT2-001 Gigimon ──────────────────────────────────────────────────────────

#[test]
fn bt2_001_gigimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-001")
        .expect("BT2-001")
        .add_card(digimon("HOST", CardColor::Red, 3, 4000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_field_stack(0, &["BT2-001", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(4000), "below 5 opp trash -> base DP");

    add_to_trash(&mut r, 1, "FILL", 5);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(5000), "5+ opp trash on your turn -> +1000 DP");

    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(4000), "inactive on opponent's turn");
}

// ── BT2-002 DemiVeemon ───────────────────────────────────────────────────────

#[test]
fn bt2_002_demiveemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-002")
        .expect("BT2-002")
        .add_card(digimon("HOST", CardColor::Blue, 3, 3000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.set_phase(GamePhase::Main);
    let h = r.place_field_stack(0, &["BT2-002", "HOST"], true, 0);
    assert_eq!(r.dp_of(h), Some(3000));

    r.game.unsuspend(h);
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(h), Some(4000), "unsuspend in Main -> +1000 DP");

    // OPT: second unsuspend in same turn does not stack.
    r.game.suspend(h);
    r.game.unsuspend(h);
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(h), Some(4000), "OPT prevents second +1000 DP");
}

// ── BT2-003 Nyaromon ─────────────────────────────────────────────────────────

#[test]
fn bt2_003_nyaromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-003")
        .expect("BT2-003")
        .add_card(digimon("HOST", CardColor::Yellow, 3, 3000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_field_stack(0, &["BT2-003", "HOST"], true, 0);
    r.game.turn_player_idx = 0;
    assert_eq!(r.game.defender_security_dp_adjustment(0), 0, "not active on your turn");

    r.game.turn_player_idx = 1;
    assert_eq!(
        r.game.defender_security_dp_adjustment(0),
        1000,
        "suspended on opponent's turn -> Security Digimon +1000 DP"
    );

    r.game.unsuspend(h);
    assert_eq!(
        r.game.defender_security_dp_adjustment(0),
        0,
        "unsuspended -> no security DP boost"
    );
}

// ── BT2-004 Argomon (Digi-Egg) ───────────────────────────────────────────────

#[test]
fn bt2_004_argomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-004")
        .expect("BT2-004")
        .add_card(digimon("HOST", CardColor::Green, 3, 3000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(2)
        .start();

    let h = r.place_field_stack(0, &["BT2-004", "HOST"], true, 0);
    r.set_phase(GamePhase::Unsuspend);
    let mem_before = r.memory();
    r.game.unsuspend(h);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before + 1, "unsuspend during Unsuspend phase -> +1 memory");
}

// ── BT2-005 Kapurimon ────────────────────────────────────────────────────────

#[test]
fn bt2_005_kapurimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-005")
        .expect("BT2-005")
        .dsl_card("BT2-055")
        .expect("BT2-055 (inherited Reboot)")
        .add_card(digimon("HOST", CardColor::Black, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let no_reboot = r.place_field_stack(0, &["BT2-005", "HOST"], false, 0);
    let with_reboot = r.place_field_stack(0, &["BT2-005", "BT2-055", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(no_reboot), Some(5000), "without Reboot -> base DP");
    assert!(r.game.has_keyword(with_reboot, Keyword::Reboot));
    assert_eq!(r.dp_of(with_reboot), Some(6000), "with Reboot on your turn -> +1000 DP");
}

// ── BT2-006 Tsumemon ─────────────────────────────────────────────────────────

#[test]
fn bt2_006_tsumemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-006")
        .expect("BT2-006")
        .add_card(named_digimon("KURISA-A", "Kurisarimon", CardColor::Black, 4, 5000))
        .add_card(named_digimon("KURISA-B", "Kurisarimon", CardColor::Black, 4, 5000))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_field_stack(0, &["BT2-006", "KURISA-A"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(5000), "alone -> base DP");

    r.place_on_field(0, "KURISA-B", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(7000), "another same-name Digimon -> +2000 DP");
}

// ── BT2-007 Pagumon ──────────────────────────────────────────────────────────

#[test]
fn bt2_007_pagumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-007")
        .expect("BT2-007")
        .add_card(digimon("HOST", CardColor::Purple, 3, 4000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_field_stack(0, &["BT2-007", "HOST"], false, 0);
    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before - 1, "trashed 1 from top of deck");
    assert_eq!(r.trash_size(0), trash_before + 1, "trash grew by 1");
}

// ── BT2-008 Yaamon ───────────────────────────────────────────────────────────

#[test]
fn bt2_008_yaamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-008")
        .expect("BT2-008")
        .add_card(digimon("HOST", CardColor::Purple, 3, 3000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_field_stack(0, &["BT2-008", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(3000));

    add_to_trash(&mut r, 0, "FILL", 5);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(4000), "5+ cards in own trash -> +1000 DP");
}

// ── BT2-009 Guilmon ──────────────────────────────────────────────────────────

#[test]
fn bt2_009_guilmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-009")
        .expect("BT2-009")
        .add_card(digimon("HOST", CardColor::Red, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_field_stack(0, &["BT2-009", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(5000));

    add_to_trash(&mut r, 1, "FILL", 5);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(6000), "5+ opp trash -> +1000 DP");
}

// ── BT2-010 Biyomon ──────────────────────────────────────────────────────────

#[test]
fn bt2_010_biyomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-010")
        .expect("BT2-010")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();

    let h = r.place_on_field(0, "BT2-010", Some(0));
    r.game.turn_player_idx = 0;
    let mem_before = r.memory();
    r.game.delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before + 1, "[On Deletion] on your turn -> +1 memory");
}

// ── BT2-011 Vorvomon ─────────────────────────────────────────────────────────

#[test]
fn bt2_011_vorvomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-011")
        .expect("BT2-011")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-011"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    let card = r.compiled_card("BT2-011").expect("compiled");
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.level, Some(3));

    let idx = r.play(0, 0).expect("play BT2-011");
    let h = r.perm_handle(0, idx);
    assert_eq!(r.memory(), 2, "cost 4 paid");
    assert_eq!(r.dp_of(h), Some(5000));
}

// ── BT2-012 Birdramon ────────────────────────────────────────────────────────

#[test]
fn bt2_012_birdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-012")
        .expect("BT2-012")
        .add_card(digimon("SEC-WEAK", CardColor::Blue, 3, 1000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC-WEAK"; 2])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_on_field(0, "BT2-012", Some(0));
    assert_eq!(r.dp_of(h), Some(3000));
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(h), Some(7000), "attacking player -> +4000 DP for the turn");
}

// ── BT2-013 Growlmon ─────────────────────────────────────────────────────────

#[test]
fn bt2_013_growlmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-013")
        .expect("BT2-013")
        .add_card(digimon("HOST", CardColor::Red, 5, 8000, 7))
        .add_card(digimon("WEAK", CardColor::Blue, 3, 2000, 3))
        .add_card(digimon("STRONG", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_field_stack(0, &["BT2-013", "HOST"], false, 0);
    r.place_on_field(1, "WEAK", Some(0));
    r.place_on_field(1, "STRONG", Some(0));
    assert_eq!(r.battle_area_size(1), 2);

    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1, "2000 DP Digimon deleted, 5000 DP survives");
    assert_eq!(
        r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),
        "STRONG"
    );
}

// ── BT2-014 Lavorvomon ───────────────────────────────────────────────────────

#[test]
fn bt2_014_lavorvomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-014")
        .expect("BT2-014")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-014"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    let card = r.compiled_card("BT2-014").expect("compiled");
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.level, Some(4));

    let idx = r.play(0, 0).expect("play BT2-014");
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(6000));
}

// ── BT2-015 Garudamon ────────────────────────────────────────────────────────

#[test]
fn bt2_015_garudamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-015")
        .expect("BT2-015")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_on_field(0, "BT2-015", Some(0));
    let hand_before = r.hand_size(0);
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "attacking player -> Draw 1");
}

// ── BT2-016 Lavogaritamon ────────────────────────────────────────────────────

#[test]
fn bt2_016_lavogaritamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-016")
        .expect("BT2-016")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-016"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    let card = r.compiled_card("BT2-016").expect("compiled");
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(8000));
    assert_eq!(card.level, Some(5));

    let idx = r.play(0, 0).expect("play BT2-016");
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(8000));
}

// ── BT2-017 WarGrowlmon ──────────────────────────────────────────────────────

#[test]
fn bt2_017_wargrowlmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-017")
        .expect("BT2-017")
        .add_card(digimon("RED-LV4", CardColor::Red, 4, 5000, 4))
        .add_card(digimon("RED-LV6", CardColor::Red, 6, 11000, 11))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("OPP-3K", CardColor::Blue, 3, 3000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-017"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    set_evo_cost(&mut r, "BT2-017", CardColor::Red, 4, 3);
    r.place_on_field(0, "RED-TAMER", Some(0));
    let base = r.place_on_field(0, "RED-LV4", Some(0));
    r.place_on_field(1, "OPP-3K", Some(0));

    let hi = hand_idx(&r, 0, "BT2-017") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "[When Digivolving] deleted 3000 DP opp Digimon");

    // Inherited: +1000 DP when opp has 5+ trash.
    let host = r.place_field_stack(0, &["BT2-017", "RED-LV6"], false, 0);
    add_to_trash(&mut r, 1, "FILL", 5);
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(host), Some(12000), "inherited +1000 DP with 5+ opp trash");
}

// ── BT2-018 Volcanicdramon ───────────────────────────────────────────────────

#[test]
fn bt2_018_volcanicdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-018")
        .expect("BT2-018")
        .add_card(digimon("OPP-3K", CardColor::Blue, 3, 3000, 3))
        .add_card(digimon("OPP-4K", CardColor::Blue, 4, 4000, 4))
        .add_card(digimon("OPP-6K", CardColor::Blue, 5, 6000, 6))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-018"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(12)
        .start();

    r.place_on_field(1, "OPP-3K", Some(0));
    r.place_on_field(1, "OPP-4K", Some(0));
    r.place_on_field(1, "OPP-6K", Some(0));

    let idx = r.play(0, 0).expect("play BT2-018");
    let _ = r.auto_resolve();
    let h = r.perm_handle(0, idx);
    assert_eq!(r.game.security_attack_keyword_bonus(h), 1, "has <Security A. +1>");
    assert_eq!(r.battle_area_size(1), 1, "both <= 4000 DP deleted, 6000 DP survives");
    assert_eq!(
        r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),
        "OPP-6K"
    );
}

// ── BT2-019 Phoenixmon ───────────────────────────────────────────────────────

#[test]
fn bt2_019_phoenixmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-019")
        .expect("BT2-019")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();

    r.game.turn_count = 2;
    let h = r.place_on_field(0, "BT2-019", Some(0));
    let mem_before = r.memory();
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before + 1, "attacking player -> +1 memory");
}

// ── BT2-020 Gallantmon ───────────────────────────────────────────────────────

#[test]
fn bt2_020_gallantmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-020")
        .expect("BT2-020")
        .add_card(digimon("RED-LV5", CardColor::Red, 5, 7000, 7))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("OPP-6K", CardColor::Blue, 5, 6000, 6))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-020"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 5])
        .memory(8)
        .start();

    set_evo_cost(&mut r, "BT2-020", CardColor::Red, 5, 4);
    r.game.turn_count = 2;
    r.place_on_field(0, "RED-TAMER", Some(0));
    let base = r.place_on_field(0, "RED-LV5", Some(0));
    r.place_on_field(1, "OPP-6K", Some(0));

    let hi = hand_idx(&r, 0, "BT2-020") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "[When Digivolving] deleted 6000 DP opp Digimon");

    // [When Attacking]: 20 cards in opp trash -> trashes 2 top security + 1 security check.
    add_to_trash(&mut r, 1, "FILL", 20);
    let sec_before = r.security_count(1);
    let _ = r.attack_player(base, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(
        r.security_count(1),
        sec_before - 3,
        "2 trashed by [When Attacking] + 1 checked by attack"
    );
}

// ── BT2-021 Veemon ───────────────────────────────────────────────────────────

#[test]
fn bt2_021_veemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-021")
        .expect("BT2-021")
        .add_card(digimon("HOST", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.set_phase(GamePhase::Main);
    let h = r.place_field_stack(0, &["BT2-021", "HOST"], true, 0);
    let hand_before = r.hand_size(0);
    r.game.unsuspend(h);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "unsuspend in Main -> Draw 1");

    // OPT: second unsuspend does not draw again.
    r.game.suspend(h);
    r.game.unsuspend(h);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "OPT prevents second draw");
}

// ── BT2-022 Betamon ──────────────────────────────────────────────────────────

#[test]
fn bt2_022_betamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-022")
        .expect("BT2-022")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-022"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let card = r.compiled_card("BT2-022").expect("compiled");
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.level, Some(3));

    let idx = r.play(0, 0).expect("play BT2-022");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(5000));
}

// ── BT2-023 Gomamon ──────────────────────────────────────────────────────────

#[test]
fn bt2_023_gomamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-023")
        .expect("BT2-023")
        .add_card(digimon("OPP-A", CardColor::Red, 3, 3000, 3))
        .add_card(digimon("OPP-B", CardColor::Red, 4, 4000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-023"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    // 2 opponent Digimon with no digivolution cards -> cost 4 - 2 = 2.
    r.place_on_field(1, "OPP-A", Some(0));
    r.place_on_field(1, "OPP-B", Some(0));
    r.play(0, 0).expect("play BT2-023");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3, "cost 4 reduced by 2 = 2 memory spent");
}

// ── BT2-024 Seadramon ────────────────────────────────────────────────────────

#[test]
fn bt2_024_seadramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-024")
        .expect("BT2-024")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-024"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let card = r.compiled_card("BT2-024").expect("compiled");
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.level, Some(4));

    let idx = r.play(0, 0).expect("play BT2-024");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(4000));
}

// ── BT2-025 Ikkakumon ────────────────────────────────────────────────────────

#[test]
fn bt2_025_ikkakumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-025")
        .expect("BT2-025")
        .add_card(digimon("HOST", CardColor::Blue, 5, 7000, 7))
        .add_card(digimon("OPP-BOT", CardColor::Red, 3, 2000, 3))
        .add_card(digimon("OPP-TOP", CardColor::Red, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let attacker = r.place_field_stack(0, &["BT2-025", "HOST"], false, 0);
    let opp = r.place_field_stack(1, &["OPP-BOT", "OPP-TOP"], false, 0);
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].card_sources.len(), 2);

    let _ = r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].card_sources.len(),
        1,
        "top digivolution card trashed"
    );
    assert_eq!(r.trash_size(1), 2, "1 trashed digivolution card + 1 checked security");
}

// ── BT2-026 Veedramon ────────────────────────────────────────────────────────

#[test]
fn bt2_026_veedramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-026")
        .expect("BT2-026")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-026", Some(0));
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(h, Keyword::Jamming), "no blue Tamer -> no Jamming");

    r.place_on_field(0, "BLUE-TAMER", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Jamming), "blue Tamer on your turn -> Jamming");
}

// ── BT2-027 Zudomon ──────────────────────────────────────────────────────────

#[test]
fn bt2_027_zudomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-027")
        .expect("BT2-027")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-027"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    let card = r.compiled_card("BT2-027").expect("compiled");
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(9000));
    assert_eq!(card.level, Some(5));

    let idx = r.play(0, 0).expect("play BT2-027");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(9000));
}

// ── BT2-028 AeroVeedramon ────────────────────────────────────────────────────

#[test]
fn bt2_028_aeroveedramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-028")
        .expect("BT2-028")
        .add_card(digimon("BLUE-LV4", CardColor::Blue, 4, 5000, 4))
        .add_card(digimon("BLUE-LV6", CardColor::Blue, 6, 11000, 11))
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-028"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    set_evo_cost(&mut r, "BT2-028", CardColor::Blue, 4, 3);
    r.place_on_field(0, "BLUE-TAMER", Some(0));
    let base = r.place_field_stack(0, &["BLUE-LV4"], true, 0);
    assert!(r.game.players[0].battle_area[base.index as usize].is_suspended);

    let hi = hand_idx(&r, 0, "BT2-028") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    let _ = r.auto_resolve();
    assert!(
        !r.game.players[0].battle_area[base.index as usize].is_suspended,
        "[When Digivolving] unsuspended blue Digimon"
    );

    // Inherited: when unsuspended during Main Phase, gains Jamming for the turn.
    r.set_phase(GamePhase::Main);
    let host = r.place_field_stack(0, &["BT2-028", "BLUE-LV6"], true, 0);
    assert!(!r.game.has_keyword(host, Keyword::Jamming));
    r.game.unsuspend(host);
    let _ = r.auto_resolve();
    assert!(
        r.game.has_keyword(host, Keyword::Jamming),
        "inherited on_unsuspend in Main -> Jamming"
    );
}

// ── BT2-029 MegaSeadramon ────────────────────────────────────────────────────

#[test]
fn bt2_029_megaseadramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-029")
        .expect("BT2-029")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-029", Some(0));
    r.game.tick_declarative_effects();
    assert!(
        r.game.modifiers.has(h, ModifierType::CannotBeBlockedByNoSources),
        "aura grants CannotBeBlockedByNoSources on your turn"
    );

    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert!(
        !r.game.modifiers.has(h, ModifierType::CannotBeBlockedByNoSources),
        "inactive on opponent's turn"
    );
}

// ── BT2-030 MetalSeadramon ───────────────────────────────────────────────────

#[test]
fn bt2_030_metalseadramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-030")
        .expect("BT2-030")
        .add_card(digimon("OPP-LV3", CardColor::Red, 3, 2000, 3))
        .add_card(digimon("OPP-LV4", CardColor::Red, 4, 4000, 4))
        .add_card(digimon("OPP-LV5", CardColor::Red, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-030"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(12)
        .start();

    r.place_on_field(1, "OPP-LV3", Some(0));
    r.place_on_field(1, "OPP-LV4", Some(0));
    r.place_on_field(1, "OPP-LV5", Some(0));
    let opp_hand_before = r.hand_size(1);

    let idx = r.play(0, 0).expect("play BT2-030");
    let _ = r.auto_resolve_declaring_max();
    let h = r.perm_handle(0, idx);
    r.game.turn_player_idx = 0;
    r.game.tick_declarative_effects();
    assert!(r.game.modifiers.has(h, ModifierType::CannotBeBlockedByNoSources));
    assert_eq!(r.battle_area_size(1), 1, "both Lv.4- bounced, Lv.5 remains");
    assert!(r.game.players[1].hand.iter().any(|c| c.card_id(&r.game.card_data) == "OPP-LV3"));
    assert!(r.game.players[1].hand.iter().any(|c| c.card_id(&r.game.card_data) == "OPP-LV4"));
}
