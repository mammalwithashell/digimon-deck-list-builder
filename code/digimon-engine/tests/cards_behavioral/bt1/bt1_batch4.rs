//! Behavioral tests for BT1-091 through BT1-115 (25 cards: Options + Secret Rares).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType, PlayerId};

fn make_digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn make_tamer(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
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

// ─── BT1-091 Scrap Claw ──────────────────────────────────────────────────────

#[test]
fn bt1_091_scrap_claw() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-091")
        .expect("BT1-091 loads")
        .add_card(make_digimon("RED-D", "RedD", CardColor::Red, 4, 5000))
        .memory(5)
        .start();

    let ally = r.place_on_field(0, "RED-D", Some(0));
    let h = r.add_to_hand(0, "BT1-091");
    act(&mut r, 0, h as u16);
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert!(r.game.has_keyword(ally, Keyword::Piercing));
    assert_eq!(r.memory(), 2);
}

// ─── BT1-092 Nuclear Laser ───────────────────────────────────────────────────

#[test]
fn bt1_092_nuclear_laser() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-092")
        .expect("BT1-092 loads")
        .add_card(make_digimon("RED-D", "RedD", CardColor::Red, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    let ally = r.place_on_field(0, "RED-D", Some(0));
    let h = r.add_to_hand(0, "BT1-092");
    act(&mut r, 0, h as u16);
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 2, "Nuclear Laser draws 2");
    assert_eq!(r.effective_dp(ally), Some(7000), "chosen Digimon gets +2000 DP");
}

// ─── BT1-093 Great Tornado ───────────────────────────────────────────────────

#[test]
fn bt1_093_great_tornado() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-093")
        .expect("BT1-093 loads")
        .add_card(make_digimon("RED-D", "RedD", CardColor::Red, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .security(1, &["FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let ally = r.place_on_field(0, "RED-D", Some(0));
    let h = r.add_to_hand(0, "BT1-093");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(ally), Some(7000), "Great Tornado gives +2000 DP");
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert_eq!(r.security_count(1), 1, "Great Tornado <Security A. +1> checks 2 security");
}

// ─── BT1-094 Oblivion Bird ───────────────────────────────────────────────────

#[test]
fn bt1_094_oblivion_bird() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-094")
        .expect("BT1-094 loads")
        .dsl_card("BT1-072")
        .expect("BT1-072 (Woodmon, <Blocker>) loads")
        .add_card(make_digimon("RED-D", "RedD", CardColor::Red, 4, 5000))
        .add_card(make_digimon("OPP-PLAIN", "OppPlain", CardColor::Red, 4, 5000))
        .memory(6)
        .start();

    let _red = r.place_on_field(0, "RED-D", Some(0));
    let _blk = r.place_on_field(1, "BT1-072", Some(0));
    let _plain = r.place_on_field(1, "OPP-PLAIN", Some(0));

    let h = r.add_to_hand(0, "BT1-094");
    act(&mut r, 0, h as u16);
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(1), 1, "Oblivion Bird deletes Blocker only");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "OPP-PLAIN"
    );
}

// ─── BT1-095 Brave Shield ────────────────────────────────────────────────────

#[test]
fn bt1_095_brave_shield() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-095")
        .expect("BT1-095 loads")
        .add_card(make_digimon("RED-D", "RedD", CardColor::Red, 4, 5000))
        .memory(6)
        .start();

    let ally = r.place_field_stack(0, &["RED-D"], true, 0);
    let h = r.add_to_hand(0, "BT1-095");
    act(&mut r, 0, h as u16);
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert!(!r.game.players[0].battle_area[0].is_suspended, "Brave Shield unsuspends target");
    assert!(r.game.has_keyword(ally, Keyword::Blocker), "Brave Shield grants <Blocker>");
}

// ─── BT1-096 Mad Dog Fire ────────────────────────────────────────────────────

#[test]
fn bt1_096_mad_dog_fire() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-096")
        .expect("BT1-096 loads")
        .add_card(make_digimon("BLUE-D", "BlueD", CardColor::Blue, 4, 4000))
        .memory(5)
        .start();

    let ally = r.place_on_field(0, "BLUE-D", Some(0));
    let h = r.add_to_hand(0, "BT1-096");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(ally), Some(7000), "Mad Dog Fire gives +3000 DP");
    assert_eq!(r.memory(), 4);
}

// ─── BT1-097 Boring Storm ────────────────────────────────────────────────────

#[test]
fn bt1_097_boring_storm() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-097")
        .expect("BT1-097 loads")
        .add_card(make_digimon("BLUE-D", "BlueD", CardColor::Blue, 4, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        // Last element is top of security stack (checked first).
        .security(1, &["FILLER", "BT1-097"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    // Main: Draw 1
    let _ally = r.place_on_field(0, "BLUE-D", Some(0));
    let h = r.add_to_hand(0, "BT1-097");
    act(&mut r, 0, h as u16);
    assert_eq!(r.hand_size(0), 1, "Boring Storm [Main] draws 1");

    // Security: Draw 2
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert_eq!(r.hand_size(1), 2, "Boring Storm [Security] draws 2 for defender");
}

// ─── BT1-098 V-Nova Blast ────────────────────────────────────────────────────

#[test]
fn bt1_098_v_nova_blast() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-098")
        .expect("BT1-098 loads")
        .add_card(make_digimon("BLUE-D", "BlueD", CardColor::Blue, 4, 4000))
        .memory(5)
        .start();

    let ally = r.place_on_field(0, "BLUE-D", Some(0));
    let h = r.add_to_hand(0, "BT1-098");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert!(r.game.has_keyword(ally, Keyword::Jamming), "V-Nova Blast grants <Jamming>");
}

// ─── BT1-099 Hearts Attack ───────────────────────────────────────────────────

#[test]
fn bt1_099_hearts_attack() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-099")
        .expect("BT1-099 loads")
        .add_card(make_digimon("BLUE-D", "BlueD", CardColor::Blue, 4, 4000))
        .add_card(make_digimon("SRC", "Src", CardColor::Red, 3, 2000))
        .add_card(make_digimon("OPP-TOP", "OppTop", CardColor::Red, 5, 7000))
        .memory(5)
        .start();

    let _ally = r.place_on_field(0, "BLUE-D", Some(0));
    let _opp = r.place_stack(1, &["SRC", "SRC", "OPP-TOP"]);
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 3);

    let h = r.add_to_hand(0, "BT1-099");
    act(&mut r, 0, h as u16);
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(
        r.game.players[1].battle_area[0].card_sources.len(),
        1,
        "Hearts Attack trashes all digivolution cards under target"
    );
    assert_eq!(r.trash_size(1), 2);
}

// ─── BT1-100 Grace Cross Freezer ─────────────────────────────────────────────

#[test]
fn bt1_100_grace_cross_freezer() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-100")
        .expect("BT1-100 loads")
        .add_card(make_digimon("BLUE-D", "BlueD", CardColor::Blue, 4, 4000))
        .add_card(make_digimon("OPP-BARE", "OppBare", CardColor::Red, 4, 5000))
        .add_card(make_digimon("OPP-SRC", "OppSrc", CardColor::Red, 3, 2000))
        .memory(6)
        .start();
    r.game.turn_count = 2;

    let _ally = r.place_on_field(0, "BLUE-D", Some(0));
    let bare = r.place_on_field(1, "OPP-BARE", Some(0));
    let stacked = r.place_stack(1, &["OPP-SRC", "OPP-BARE"]);

    let h = r.add_to_hand(0, "BT1-100");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);
    r.game.tick_declarative_effects();

    assert!(
        r.game.modifiers.has(bare, ModifierType::CannotAttack),
        "sourceless opponent Digimon receives CannotAttack"
    );
    assert!(
        !r.game.modifiers.has(stacked, ModifierType::CannotAttack),
        "stacked opponent Digimon is unaffected"
    );
}

// ─── BT1-101 Howling Crusher ─────────────────────────────────────────────────

#[test]
fn bt1_101_howling_crusher() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-101")
        .expect("BT1-101 loads")
        .add_card(make_digimon("BLUE-D", "BlueD", CardColor::Blue, 4, 4000))
        .add_card(make_digimon("SRC", "Src", CardColor::Red, 3, 2000))
        .add_card(make_digimon("OPP-TOP", "OppTop", CardColor::Red, 5, 7000))
        .memory(8)
        .start();

    let _ally = r.place_on_field(0, "BLUE-D", Some(0));
    let _opp1 = r.place_stack(1, &["SRC", "OPP-TOP"]);
    let _opp2 = r.place_stack(1, &["SRC", "SRC", "OPP-TOP"]);

    let h = r.add_to_hand(0, "BT1-101");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 1);
    assert_eq!(r.game.players[1].battle_area[1].card_sources.len(), 1);
    assert_eq!(r.trash_size(1), 3, "Howling Crusher trashes all sources from all opp Digimon");
}

// ─── BT1-102 Blade of the True ───────────────────────────────────────────────

#[test]
fn bt1_102_blade_of_the_true() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-102")
        .expect("BT1-102 loads")
        .add_card(make_digimon("YEL-D", "YelD", CardColor::Yellow, 4, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER"; 5])
        .security(0, &["FILLER"; 5])
        .memory(5)
        .start();

    let _ally = r.place_on_field(0, "YEL-D", Some(0));
    let h = r.add_to_hand(0, "BT1-102");
    act(&mut r, 0, h as u16);

    // floor(5 / 2) = 2 cards drawn
    assert_eq!(r.hand_size(0), 2, "5 security => Draw 2");
}

// ─── BT1-103 Testament ───────────────────────────────────────────────────────

#[test]
fn bt1_103_testament() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-103")
        .expect("BT1-103 loads")
        .add_card(make_digimon("YEL-D", "YelD", CardColor::Yellow, 4, 4000))
        .memory(5)
        .start();

    let ally = r.place_on_field(0, "YEL-D", Some(0));
    let h = r.add_to_hand(0, "BT1-103");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert!(r.game.has_keyword(ally, Keyword::Blocker), "Testament grants <Blocker>");
}

// ─── BT1-104 Golden Ripper ───────────────────────────────────────────────────

#[test]
fn bt1_104_golden_ripper() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-104")
        .expect("BT1-104 loads")
        .add_card(make_digimon("YEL-D", "YelD", CardColor::Yellow, 4, 4000))
        .add_card(make_digimon("OPP-D", "OppD", CardColor::Red, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _ally = r.place_on_field(0, "YEL-D", Some(0));
    let opp = r.place_on_field(1, "OPP-D", Some(0));

    let h = r.add_to_hand(0, "BT1-104");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    // Attacking with own Digimon triggers granted "[When Attacking] -2000 DP"
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(opp), Some(3000), "Golden Ripper granted When Attacking -2000 DP");
}

// ─── BT1-105 Blast Fire ──────────────────────────────────────────────────────

#[test]
fn bt1_105_blast_fire() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-105")
        .expect("BT1-105 loads")
        .add_card(make_digimon("YEL-D", "YelD", CardColor::Yellow, 4, 4000))
        .add_card(make_digimon("OPP-BIG", "OppBig", CardColor::Red, 6, 12000))
        .memory(6)
        .start();

    let _ally = r.place_on_field(0, "YEL-D", Some(0));
    let opp = r.place_on_field(1, "OPP-BIG", Some(0));

    let h = r.add_to_hand(0, "BT1-105");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(opp), Some(3000), "Blast Fire changes original DP to 3000");
}

// ─── BT1-106 Symphony No.1 <Polyphony> ───────────────────────────────────────

#[test]
fn bt1_106_symphony_no1_polyphony() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-106")
        .expect("BT1-106 loads")
        .add_card(make_digimon("YEL-D", "YelD", CardColor::Yellow, 4, 4000))
        .add_card(make_digimon("OPP-BIG", "OppBig", CardColor::Red, 6, 10000))
        .memory(6)
        .start();

    let _ally = r.place_on_field(0, "YEL-D", Some(0));
    let opp = r.place_on_field(1, "OPP-BIG", Some(0));

    let h = r.add_to_hand(0, "BT1-106");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(opp), Some(3000), "Symphony No.1 gives -7000 DP");
}

// ─── BT1-107 Holy Wave ───────────────────────────────────────────────────────

#[test]
fn bt1_107_holy_wave() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-107")
        .expect("BT1-107 loads")
        .add_card(make_digimon("YEL-D", "YelD", CardColor::Yellow, 4, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .security(0, &["FILLER", "FILLER"])
        .memory(8)
        .start();

    let _ally = r.place_on_field(0, "YEL-D", Some(0));
    let h = r.add_to_hand(0, "BT1-107");
    act(&mut r, 0, h as u16);

    assert_eq!(r.security_count(0), 3, "Holy Wave Recovery +1 raises security 2 -> 3");
    assert_eq!(r.memory(), 2);
}

// ─── BT1-108 Horn Buster ─────────────────────────────────────────────────────

#[test]
fn bt1_108_horn_buster() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-108")
        .expect("BT1-108 loads")
        .add_card(make_digimon("GRN-D", "GrnD", CardColor::Green, 4, 4000))
        .memory(5)
        .start();

    let ally = r.place_on_field(0, "GRN-D", Some(0));
    let h = r.add_to_hand(0, "BT1-108");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(ally), Some(7000), "Horn Buster gives +3000 DP");
}

// ─── BT1-109 Smashed Potatoes ────────────────────────────────────────────────

#[test]
fn bt1_109_smashed_potatoes() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-109")
        .expect("BT1-109 loads")
        .add_card(make_digimon("GRN-LV5", "GrnLv5", CardColor::Green, 5, 7000))
        .add_card(make_digimon("GRN-LV6", "GrnLv6", CardColor::Green, 6, 11000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "GRN-LV6", 3, 5, 4); // Green = 3, Lv5 -> cost 4
    r.game.turn_count = 2;

    let _lv5 = r.place_on_field(0, "GRN-LV5", Some(0));
    let opt = r.add_to_hand(0, "BT1-109");
    act(&mut r, 0, opt as u16);
    assert_eq!(r.memory(), 5, "Smashed Potatoes costs 0");

    let evo = r.add_to_hand(0, "GRN-LV6");
    act(&mut r, 0, encode_digivolve(evo as u16, 0));
    resolve_first_pending(&mut r);

    assert_eq!(
        r.memory(),
        5,
        "Smashed Potatoes reduced Green Lv5->Lv6 digivolution cost by 4 (4 -> 0)"
    );
}

// ─── BT1-110 Flower Cannon ───────────────────────────────────────────────────

#[test]
fn bt1_110_flower_cannon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-110")
        .expect("BT1-110 loads")
        .add_card(make_digimon("GRN-D", "GrnD", CardColor::Green, 4, 4000))
        .add_card(make_digimon("OPP-D", "OppD", CardColor::Red, 4, 5000))
        .memory(5)
        .start();

    let _ally = r.place_on_field(0, "GRN-D", Some(0));
    let _opp = r.place_on_field(1, "OPP-D", Some(0));

    let h = r.add_to_hand(0, "BT1-110");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert!(r.game.players[1].battle_area[0].is_suspended, "Flower Cannon suspends target");
}

// ─── BT1-111 Giga Blaster ────────────────────────────────────────────────────

#[test]
fn bt1_111_giga_blaster() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-111")
        .expect("BT1-111 loads")
        .add_card(make_digimon("GRN-D", "GrnD", CardColor::Green, 4, 4000))
        .add_card(make_digimon("OPP-S1", "OppS1", CardColor::Red, 3, 3000))
        .add_card(make_digimon("OPP-S2", "OppS2", CardColor::Red, 4, 5000))
        .memory(5)
        .start();

    let _ally = r.place_on_field(0, "GRN-D", Some(0));
    let _s1 = r.place_on_field(1, "OPP-S1", Some(0));
    let _s2 = r.place_on_field(1, "OPP-S2", Some(0));

    let h = r.add_to_hand(0, "BT1-111");
    act(&mut r, 0, h as u16);
    // Choose branch 1: "Suspend 2 of your opponent's Digimon with 5000 DP or less"
    r.execute_branch(1).unwrap();
    resolve_first_pending(&mut r);

    assert!(r.game.players[1].battle_area[0].is_suspended);
    assert!(r.game.players[1].battle_area[1].is_suspended);
}

// ─── BT1-112 Dimension Scissor ───────────────────────────────────────────────

#[test]
fn bt1_112_dimension_scissor() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-112")
        .expect("BT1-112 loads")
        .add_card(make_digimon("GRN-BIG", "GrnBig", CardColor::Green, 6, 11000))
        .add_card(make_digimon("OPP-WEAK", "OppWeak", CardColor::Red, 3, 3000))
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _ally = r.place_on_field(0, "GRN-BIG", Some(0));
    let _opp = r.place_field_stack(1, &["OPP-WEAK"], true, 0);

    let h = r.add_to_hand(0, "BT1-112");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    act(&mut r, 0, encode_attack(0, 0));
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(1), 0, "opponent Digimon deleted in battle");
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "Dimension Scissor unsuspends attacker after winning battle"
    );
}

// ─── BT1-113 Forbidden Temptation ────────────────────────────────────────────

#[test]
fn bt1_113_forbidden_temptation() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-113")
        .expect("BT1-113 loads")
        .dsl_card("BT1-072")
        .expect("BT1-072 (Woodmon, <Blocker>) loads")
        .add_card(make_digimon("GRN-D", "GrnD", CardColor::Green, 4, 4000))
        .memory(6)
        .start();

    let _ally = r.place_on_field(0, "GRN-D", Some(0));
    let opp = r.place_on_field(1, "BT1-072", Some(0));

    let h = r.add_to_hand(0, "BT1-113");
    act(&mut r, 0, h as u16);
    resolve_first_pending(&mut r);

    assert!(r.game.modifiers.has(opp, ModifierType::CannotAttack));
    assert!(r.game.modifiers.has(opp, ModifierType::CannotBlock));
}

// ─── BT1-114 MetalGreymon (SEC) ──────────────────────────────────────────────

#[test]
fn bt1_114_metalgreymon_sec() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-114")
        .expect("BT1-114 loads")
        .add_card(make_digimon("RED-LV6", "RedLv6", CardColor::Red, 6, 11000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .security(1, &["FILLER"; 4])
        .memory(6)
        .start();
    r.game.turn_count = 2;

    let mg = r.place_on_field(0, "BT1-114", Some(0));
    assert!(r.game.has_keyword(mg, Keyword::SecurityAttackPlus(2)));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 1, "When Attacking loses 5 memory (6 -> 1)");
    assert_eq!(r.security_count(1), 1, "<Security A. +2> checks 3 security cards");

    // Inherited Your Turn +3000 DP
    let host = r.place_stack(0, &["BT1-114", "RED-LV6"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(14000), "inherited +3000 DP on Your Turn");
}

// ─── BT1-115 Veedramon (SEC) ─────────────────────────────────────────────────

#[test]
fn bt1_115_veedramon_sec() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-115")
        .expect("BT1-115 loads")
        .add_card(make_tamer("BLUE-TAMER", "BlueTamer", CardColor::Blue))
        .add_card(make_digimon("BLUE-LV5", "BlueLv5", CardColor::Blue, 5, 7000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _tamer = r.place_on_field(0, "BLUE-TAMER", Some(0));
    let _vee = r.place_on_field(0, "BT1-115", Some(0));

    // When Attacking OPT: with a Tamer in play, unsuspends self
    act(&mut r, 0, encode_attack(1, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert!(
        !r.game.players[0].battle_area[1].is_suspended,
        "Veedramon unsuspends on attack while you have a Tamer in play"
    );

    // Inherited All Turns: +1000 DP while you have a blue Tamer in play
    let host = r.place_stack(0, &["BT1-115", "BLUE-LV5"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(8000), "Veedramon inherited grants +1000 DP with blue Tamer");
}
