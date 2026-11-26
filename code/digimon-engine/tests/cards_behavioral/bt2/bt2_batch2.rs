#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

//! Behavioral tests for BT2-031 through BT2-060 (excluding BT2-041 and BT2-047).

use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS};
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

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
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

// ── BT2-031 Vikemon ──────────────────────────────────────────────────────────

#[test]
fn bt2_031_vikemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-031")
        .expect("BT2-031")
        .add_card(digimon("OPP-NOSRC", CardColor::Red, 3, 2000, 3))
        .add_card(digimon("SEC-WEAK", CardColor::Red, 3, 1000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC-WEAK"; 4])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_on_field(0, "BT2-031", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(12000), "no opp Digimon -> base 12000 DP");

    r.place_on_field(1, "OPP-NOSRC", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.dp_of(h), Some(13000), "opp Digimon with 0 sources -> +1000 DP");

    let sec_before = r.security_count(1);
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), sec_before - 2, "<Security A. +1> checks 2 security");
}

// ── BT2-032 UlforceVeedramon ─────────────────────────────────────────────────

#[test]
fn bt2_032_ulforceveedramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-032")
        .expect("BT2-032")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();

    r.set_phase(GamePhase::Main);
    let ulforce = r.place_field_stack(0, &["BT2-032"], true, 0);
    let bt = r.place_on_field(0, "BLUE-TAMER", Some(0));
    assert!(r.game.players[0].battle_area[ulforce.index as usize].is_suspended);

    let mem_before = r.memory();
    r.game.suspend(bt);
    let _ = r.auto_resolve();
    assert!(
        !r.game.players[0].battle_area[ulforce.index as usize].is_suspended,
        "suspending blue Tamer unsuspended UlforceVeedramon"
    );
    assert_eq!(r.memory(), mem_before + 1, "unsuspending in Main gained 1 memory");
}

// ── BT2-033 Agumon (Yellow) ──────────────────────────────────────────────────

#[test]
fn bt2_033_agumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-033")
        .expect("BT2-033")
        .add_card(digimon("HOST", CardColor::Yellow, 4, 5000, 4))
        .add_card(tamer("YT", CardColor::Yellow))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    for _ in 0..3 {
        r.place_on_field(0, "YT", Some(0));
    }
    let h = r.place_field_stack(0, &["BT2-033", "HOST"], false, 0);
    let hand_before = r.hand_size(0);
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "3+ yellow Tamers -> Draw 1");
}

// ── BT2-034 Salamon ──────────────────────────────────────────────────────────

#[test]
fn bt2_034_salamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-034")
        .expect("BT2-034")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 2])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-034", Some(0));
    assert_eq!(r.security_count(0), 2);
    r.game.delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3, "<= 3 security on deletion -> Recovery +1");
}

// ── BT2-035 GeoGreymon ───────────────────────────────────────────────────────

#[test]
fn bt2_035_geogreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-035")
        .expect("BT2-035")
        .add_card(digimon("HOST", CardColor::Yellow, 5, 8000, 7))
        .add_card(tamer("YT", CardColor::Yellow))
        .add_card(digimon("OPP", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    for _ in 0..3 {
        r.place_on_field(0, "YT", Some(0));
    }
    let h = r.place_field_stack(0, &["BT2-035", "HOST"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.dp_of(opp), Some(5000), "3+ yellow Tamers -> opp Digimon -2000 DP");
}

// ── BT2-036 Gatomon ──────────────────────────────────────────────────────────

#[test]
fn bt2_036_gatomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-036")
        .expect("BT2-036")
        .add_card(digimon("PURP", CardColor::Purple, 3, 2000, 3))
        .add_card(digimon("OPP", CardColor::Blue, 5, 8000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-036"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    let purp = r.place_on_field(0, "PURP", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    let idx = r.play(0, 0).expect("play BT2-036");
    let _ = r.auto_resolve();
    let gato = r.perm_handle(0, idx);
    assert_eq!(r.dp_of(opp), Some(4000), "[On Play] with purple Digimon -> opp -4000 DP");

    // [Your Turn] when another own Digimon is deleted -> +3000 DP for the turn.
    r.game.delete_permanent_with_cause(purp, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    // Re-find Gatomon since deleting index 0 shifted battle_area indices.
    let gato_idx = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "BT2-036")
        .expect("Gatomon still on field");
    assert_eq!(
        r.dp_of(r.perm_handle(0, gato_idx)),
        Some(6000),
        "other own Digimon deleted -> +3000 DP"
    );
}

// ── BT2-037 Angewomon ────────────────────────────────────────────────────────

#[test]
fn bt2_037_angewomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-037")
        .expect("BT2-037")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-037"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    let card = r.compiled_card("BT2-037").expect("compiled");
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(10000));
    assert_eq!(card.level, Some(5));

    let idx = r.play(0, 0).expect("play BT2-037");
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(10000));
}

// ── BT2-038 RizeGreymon ──────────────────────────────────────────────────────

#[test]
fn bt2_038_rizegreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-038")
        .expect("BT2-038")
        .add_card(digimon("Y-LV4", CardColor::Yellow, 4, 5000, 4))
        .add_card(digimon("Y-LV6", CardColor::Yellow, 6, 11000, 11))
        .add_card(tamer("YT", CardColor::Yellow))
        .add_card(digimon("SEC-WEAK", CardColor::Blue, 3, 1000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-038", "YT"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC-WEAK"; 4])
        .memory(6)
        .start();

    set_evo_cost(&mut r, "BT2-038", CardColor::Yellow, 4, 3);
    let base = r.place_on_field(0, "Y-LV4", Some(0));
    let hi = hand_idx(&r, 0, "BT2-038") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    // Accept optional trigger and pick YT from hand.
    if r.pending_selection().is_some() {
        let _ = r.auto_resolve();
    }
    let tamer_count = r.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&r.game.card_data) == "YT")
        .count();
    assert_eq!(tamer_count, 1, "[When Digivolving] played yellow Tamer for free");

    // Inherited: 3+ yellow Tamers -> <Security A. +1>.
    r.place_on_field(0, "YT", Some(0));
    r.place_on_field(0, "YT", Some(0));
    r.game.turn_count = 2;
    let host = r.place_field_stack(0, &["BT2-038", "Y-LV6"], false, 0);
    let sec_before = r.security_count(1);
    let _ = r.attack_player(host, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), sec_before - 2, "inherited <Security A. +1> checked 2");
}

// ── BT2-039 Magnadramon ──────────────────────────────────────────────────────

#[test]
fn bt2_039_magnadramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-039")
        .expect("BT2-039")
        .add_card(digimon("Y-LV3", CardColor::Yellow, 3, 2000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-039", "Y-LV3"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 2])
        .security(1, &["FILL"; 3])
        .memory(12)
        .start();

    let idx = r.play(0, 0).expect("play BT2-039");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 4, "[On Play] <= 3 security -> Recovery +2");

    r.game.turn_count = 2;
    let h = r.perm_handle(0, idx);
    r.game.players[0].battle_area[idx].turn_played = 0;
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "Y-LV3"),
        "[When Attacking] played yellow Lv.3 Digimon from hand for free"
    );
}

// ── BT2-040 Ophanimon ────────────────────────────────────────────────────────

#[test]
fn bt2_040_ophanimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-040")
        .expect("BT2-040")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["FILL"; 2])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-040", Some(0));
    r.game.delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3, "[On Deletion] placed BT2-040 on top of security");
    assert_eq!(
        r.game.players[0].security.last().unwrap().card_id(&r.game.card_data),
        "BT2-040"
    );
}

// ── BT2-042 Argomon (Lv.3) ───────────────────────────────────────────────────

#[test]
fn bt2_042_argomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-042")
        .expect("BT2-042")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-042"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let card = r.compiled_card("BT2-042").expect("compiled");
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.level, Some(3));

    let idx = r.play(0, 0).expect("play BT2-042");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(3000));
}

// ── BT2-043 Agumon (Green) ───────────────────────────────────────────────────

#[test]
fn bt2_043_agumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-043")
        .expect("BT2-043")
        .add_card(digimon("HOST", CardColor::Green, 4, 5000, 4))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_field_stack(0, &["BT2-043", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(6000), "inherited [Your Turn] +1000 DP");

    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(5000), "inactive on opponent's turn");
}

// ── BT2-044 Tyrannomon ───────────────────────────────────────────────────────

#[test]
fn bt2_044_tyrannomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-044")
        .expect("BT2-044")
        .add_card(digimon("G-LV3", CardColor::Green, 3, 2000, 3))
        .add_card(digimon("LV5", CardColor::Green, 5, 7000, 7))
        .add_card(tamer("G-TAMER", CardColor::Green))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-044"])
        .deck(0, &["LV5", "G-TAMER", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    set_evo_cost(&mut r, "BT2-044", CardColor::Green, 3, 2);
    let base = r.place_on_field(0, "G-LV3", Some(0));
    let hi = hand_idx(&r, 0, "BT2-044") as u16;
    // Note: digivolve draws 1 first (top card), so seed 4 cards where index 0 is draw, 1..4 are reveal.
    r.game.players[0].deck.clear();
    for id in &["FILL", "LV5", "G-TAMER", "FILL", "FILL"] {
        let idx = r.add_to_hand(0, id);
        let c = r.game.players[0].hand.remove(idx);
        r.game.players[0].deck.push(c);
    }
    // deck[0] is top of deck in DigimonEngine? Let's put LV5 and G-TAMER at both ends or check:
    // Actually `deck` is popped from index 0 (top = front) in this engine, so index 0 = draw, 1 = LV5, 2 = G-TAMER, 3 = FILL.
    let hi = hand_idx(&r, 0, "BT2-044") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    let _ = r.auto_resolve();

    let in_hand: Vec<String> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(in_hand.contains(&"LV5".to_string()), "added Lv.5 Digimon: {in_hand:?}");
    assert!(in_hand.contains(&"G-TAMER".to_string()), "added green Tamer: {in_hand:?}");
}

// ── BT2-045 Argomon (Lv.4) ───────────────────────────────────────────────────

#[test]
fn bt2_045_argomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-045")
        .expect("BT2-045")
        .add_card(digimon("G-LV3", CardColor::Green, 3, 2000, 3))
        .add_card(digimon("ALLY", CardColor::Green, 3, 2000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-045"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    set_evo_cost(&mut r, "BT2-045", CardColor::Green, 3, 2);
    let base = r.place_on_field(0, "G-LV3", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let hi = hand_idx(&r, 0, "BT2-045") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    r.accept_optional_trigger().expect("accept Digisorption -2");
    r.execute_action(0, ATTACK_START + ally.index as u16).expect("suspend ally");
    let _ = r.auto_resolve();
    assert!(r.game.players[0].battle_area[ally.index as usize].is_suspended);
    assert_eq!(r.memory(), 5, "cost 2 - Digisorption 2 = 0");
}

// ── BT2-046 MetalTyrannomon ──────────────────────────────────────────────────

#[test]
fn bt2_046_metaltyrannomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-046")
        .expect("BT2-046")
        .add_card(digimon("HOST", CardColor::Green, 6, 12000, 12))
        .add_card(digimon("OPP-LV6", CardColor::Blue, 6, 10000, 11))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let attacker = r.place_field_stack(0, &["BT2-046", "HOST"], false, 0);
    let defender = r.place_field_stack(1, &["OPP-LV6"], true, 0);
    let _ = r.attack_digimon(attacker, defender, false);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "opponent Lv.6 deleted in battle");
    assert!(
        !r.game.players[0].battle_area[attacker.index as usize].is_suspended,
        "inherited trigger unsuspended attacker after deleting Lv.6+"
    );
}

// ── BT2-048 Cherrymon ────────────────────────────────────────────────────────

#[test]
fn bt2_048_cherrymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-048")
        .expect("BT2-048")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-048"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let idx = r.play(0, 0).expect("play BT2-048");
    let h = r.perm_handle(0, idx);
    assert!(r.game.has_keyword(h, Keyword::Blocker), "has <Blocker>");
    assert_eq!(r.dp_of(h), Some(7000));
}

// ── BT2-049 Puppetmon ────────────────────────────────────────────────────────

#[test]
fn bt2_049_puppetmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-049")
        .expect("BT2-049")
        .add_card(digimon("OPP", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-049"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(12)
        .start();

    let opp = r.place_on_field(1, "OPP", Some(0));
    let idx = r.play(0, 0).expect("play BT2-049");
    let _ = r.auto_resolve();
    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "[On Play] suspended opponent Digimon"
    );

    // [When Attacking] Gain 1 memory.
    r.game.turn_player_idx = 0;
    r.game.set_memory(3);
    r.game.turn_count = 2;
    let h = r.perm_handle(0, idx);
    r.game.players[0].battle_area[idx].turn_played = 0;
    let mem_before = r.memory();
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before + 1, "[When Attacking] gained 1 memory");
}

// ── BT2-050 Argomon (Lv.6) ───────────────────────────────────────────────────

#[test]
fn bt2_050_argomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-050")
        .expect("BT2-050")
        .add_card(digimon("G-LV5", CardColor::Green, 5, 7000, 7))
        .add_card(digimon("ALLY-A", CardColor::Green, 3, 2000, 3))
        .add_card(digimon("ALLY-B", CardColor::Green, 3, 2000, 3))
        .add_card(digimon("SEC-WEAK", CardColor::Blue, 3, 1000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-050"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["SEC-WEAK"; 5])
        .memory(5)
        .start();

    set_evo_cost(&mut r, "BT2-050", CardColor::Green, 5, 4);
    r.game.turn_count = 2;
    let base = r.place_on_field(0, "G-LV5", Some(0));
    let a1 = r.place_on_field(0, "ALLY-A", Some(0));
    let _a2 = r.place_field_stack(0, &["ALLY-B"], true, 0);

    let hi = hand_idx(&r, 0, "BT2-050") as u16;
    r.game.decode_action(encode_digivolve(hi, base.index as u16), 0);
    r.accept_optional_trigger().expect("accept Digisorption -3");
    r.execute_action(0, ATTACK_START + a1.index as u16).expect("suspend ALLY-A");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 4, "cost 4 - Digisorption 3 = 1 memory spent");

    // 2 other suspended Digimon -> Security A. +2 -> checks 3 security cards.
    let sec_before = r.security_count(1);
    let _ = r.attack_player(base, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(
        r.security_count(1),
        sec_before - 3,
        "2 other suspended Digimon -> Security A. +2 (3 checks)"
    );
}

// ── BT2-051 RustTyrannomon ───────────────────────────────────────────────────

#[test]
fn bt2_051_rusttyrannomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-051")
        .expect("BT2-051")
        .add_card(tamer("G-TAMER", CardColor::Green))
        .add_card(digimon("OPP-A", CardColor::Blue, 4, 5000, 5))
        .add_card(digimon("OPP-B", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_on_field(0, "BT2-051", Some(0));
    r.place_on_field(0, "G-TAMER", Some(0));
    r.game.tick_declarative_effects();
    assert!(
        r.game.modifiers.has(h, ModifierType::CanAttackUnsuspended),
        "green Tamer grants CanAttackUnsuspended"
    );

    let oa = r.place_on_field(1, "OPP-A", Some(0));
    let ob = r.place_on_field(1, "OPP-B", Some(0));
    let _ = r.attack_digimon(h, oa, false);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1, "OPP-A deleted in battle");
    assert!(
        r.game.players[1].battle_area[0].is_suspended,
        "surviving battle win suspended OPP-B"
    );
}

// ── BT2-052 Hagurumon ────────────────────────────────────────────────────────

#[test]
fn bt2_052_hagurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-052")
        .expect("BT2-052")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-052"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let card = r.compiled_card("BT2-052").expect("compiled");
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.level, Some(3));

    let idx = r.play(0, 0).expect("play BT2-052");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(3000));
}

// ── BT2-053 Keramon ──────────────────────────────────────────────────────────

#[test]
fn bt2_053_keramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-053")
        .expect("BT2-053")
        .add_card(named_digimon("DIAB-A", "Diaboromon", CardColor::Black, 6, 11000, 11))
        .add_card(named_digimon("DIAB-B", "Diaboromon", CardColor::Black, 6, 11000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["DIAB-B"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    r.place_field_stack(0, &["BT2-053", "DIAB-A"], false, 0);
    let hand_before = r.hand_size(0);
    r.play(0, 0).expect("play DIAB-B");
    let _ = r.auto_resolve();
    // Spent 1 card from hand, drew 1 card -> net 0 change (equals hand_before).
    assert_eq!(r.hand_size(0), hand_before, "playing same-name Digimon drew 1");
}

// ── BT2-054 Gotsumon ─────────────────────────────────────────────────────────

#[test]
fn bt2_054_gotsumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-054")
        .expect("BT2-054")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(1, &["FILL"; 3])
        .memory(5)
        .start();

    r.game.turn_count = 2;
    let h = r.place_on_field(0, "BT2-054", Some(0));
    assert!(r.game.has_keyword(h, Keyword::Blocker), "has <Blocker>");

    let mem_before = r.memory();
    let _ = r.attack_player(h, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before - 2, "[When Attacking] lost 2 memory");
}

// ── BT2-055 ToyAgumon ────────────────────────────────────────────────────────

#[test]
fn bt2_055_toyagumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-055")
        .expect("BT2-055")
        .add_card(digimon("HOST", CardColor::Black, 4, 5000, 5))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_field_stack(0, &["BT2-055", "HOST"], false, 0);
    assert!(r.game.has_keyword(h, Keyword::Reboot), "inherited <Reboot>");
}

// ── BT2-056 Numemon ──────────────────────────────────────────────────────────

#[test]
fn bt2_056_numemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-056")
        .expect("BT2-056")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-056"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let card = r.compiled_card("BT2-056").expect("compiled");
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.level, Some(4));

    let idx = r.play(0, 0).expect("play BT2-056");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(3000));
}

// ── BT2-057 Greymon (Black) ──────────────────────────────────────────────────

#[test]
fn bt2_057_greymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-057")
        .expect("BT2-057")
        .dsl_card("BT2-055")
        .expect("BT2-055 (inherited Reboot)")
        .add_card(digimon("HOST", CardColor::Black, 5, 7000, 7))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let no_reboot = r.place_field_stack(0, &["BT2-057", "HOST"], false, 0);
    let with_reboot = r.place_field_stack(0, &["BT2-055", "BT2-057", "HOST"], false, 0);
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(no_reboot, Keyword::Jamming));
    assert!(
        r.game.has_keyword(with_reboot, Keyword::Jamming),
        "inherited aura grants <Jamming> while host has <Reboot>"
    );
}

// ── BT2-058 Guardromon ───────────────────────────────────────────────────────

#[test]
fn bt2_058_guardromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-058")
        .expect("BT2-058")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT2-058", Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Blocker), "has <Blocker>");
    assert!(
        r.game.modifiers.has(h, ModifierType::CannotAttack),
        "[Your Turn] CannotAttack aura active"
    );
}

// ── BT2-059 Kurisarimon ──────────────────────────────────────────────────────

#[test]
fn bt2_059_kurisarimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-059")
        .expect("BT2-059")
        .add_card(named_digimon("DIAB-A", "Diaboromon", CardColor::Black, 6, 11000, 11))
        .add_card(named_digimon("DIAB-B", "Diaboromon", CardColor::Black, 6, 11000, 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["DIAB-B"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(6)
        .start();

    r.place_field_stack(0, &["BT2-059", "DIAB-A"], false, 0);
    r.play(0, 0).expect("play DIAB-B");
    let _ = r.auto_resolve();
    // 6 - 3 (play cost) + 1 (Kurisarimon) = 4.
    assert_eq!(r.memory(), 4, "playing same-name Digimon gained 1 memory");
}

// ── BT2-060 Megadramon ───────────────────────────────────────────────────────

#[test]
fn bt2_060_megadramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT2-060")
        .expect("BT2-060")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT2-060"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(8)
        .start();

    let card = r.compiled_card("BT2-060").expect("compiled");
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(9000));
    assert_eq!(card.level, Some(5));

    let idx = r.play(0, 0).expect("play BT2-060");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, idx)), Some(9000));
}
