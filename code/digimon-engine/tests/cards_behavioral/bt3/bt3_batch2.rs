#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;

fn make_digimon(id: &str, name: &str, color: CardColor, level: u8, dp: i32, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost;
    c
}

fn set_evo_cost(r: &mut DebugRunner, cid: &str, color: CardColor, level: u8, cost: u8) {
    let cd = r
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == cid)
        .unwrap_or_else(|| panic!("card_data missing {cid}"));
    cd.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level,
        memory_cost: cost as u16,
    }];
}

fn hand_index_of(r: &DebugRunner, player: u8, cid: &str) -> usize {
    r.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == cid)
        .unwrap_or_else(|| panic!("{cid} not in P{player} hand"))
}

fn digivolve_from_hand(r: &mut DebugRunner, player: u8, cid: &str, target: PermanentHandle) {
    let h = hand_index_of(r, player, cid) as u16;
    r.game
        .decode_action(encode_digivolve(h, target.index as u16), player);
}

fn put_in_trash(r: &mut DebugRunner, player: u8, cid: &str) {
    let idx = r.add_to_hand(player, cid);
    let c = r.game.players[player as usize].hand.remove(idx);
    r.game.players[player as usize].put_in_trash(c);
}

// ─── BT3-031 Imperialdramon Dragon Mode ──────────────────────────────────────
#[test]
fn bt3_031_imperialdramon_dragon_mode() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-031")
        .expect("BT3-031")
        .dsl_card("BT3-021")
        .expect("BT3-021 Veemon with Jamming")
        .add_card(make_digimon("PAIL", "Paildramon", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-031"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT3-031", CardColor::Blue, 5, 5);

    let pail = r.place_on_field(0, "PAIL", Some(0));
    let veemon = r.place_on_field(0, "BT3-021", Some(0));
    r.game.suspend(pail);
    r.game.suspend(veemon);

    digivolve_from_hand(&mut r, 0, "BT3-031", pail);
    let _ = r.auto_resolve();

    // Cost 5 reduced by 2 -> 3, memory 5 - 3 = 2
    assert_eq!(r.memory(), 2, "Digivolving over Paildramon reduced cost by 2");
    assert!(r.game.has_keyword(pail, Keyword::Jamming));
    assert!(
        !r.game.players[0].battle_area[pail.index as usize].is_suspended,
        "Imperialdramon unsuspended itself (has Jamming)"
    );
    assert!(
        !r.game.players[0].battle_area[veemon.index as usize].is_suspended,
        "Imperialdramon unsuspended ally Veemon (has Jamming)"
    );
}

// ─── BT3-032 Armadillomon ────────────────────────────────────────────────────
#[test]
fn bt3_032_armadillomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-032")
        .expect("BT3-032")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-032"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-032").expect("BT3-032");
    assert_eq!(card.name, "Armadillomon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(4000));
}

// ─── BT3-033 Salamon ─────────────────────────────────────────────────────────
#[test]
fn bt3_033_salamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-033")
        .expect("BT3-033")
        .add_card(make_digimon("CARRIER", "Gatomon", CardColor::Yellow, 4, 4000, 4))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-033", "CARRIER"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.dp_of(opp), Some(4000), "Salamon inherited gave -1000 DP");
}

// ─── BT3-034 Lopmon ──────────────────────────────────────────────────────────
#[test]
fn bt3_034_lopmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-034")
        .expect("BT3-034")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 3])
        .hand(0, &["BT3-034"])
        .memory(5)
        .start();

    let sec_before = r.security_count(0);
    let hand_before = r.hand_size(0);

    r.play(0, 0);
    let _ = r.auto_resolve();

    // Playing Lopmon (-1 hand), adding top security to hand (+1 hand, -1 sec), drawing 1 (+1 hand)
    assert_eq!(r.security_count(0), sec_before - 1, "Top security added to hand");
    assert_eq!(r.hand_size(0), hand_before + 1, "Added security + drew 1 card");
}

// ─── BT3-035 Gatomon ─────────────────────────────────────────────────────────
#[test]
fn bt3_035_gatomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-035")
        .expect("BT3-035")
        .add_card(make_digimon("CARRIER", "Angewomon", CardColor::Yellow, 5, 6000, 7))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-035", "CARRIER"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.dp_of(opp), Some(4000), "Gatomon inherited gave -1000 DP");
}

// ─── BT3-036 Ankylomon ───────────────────────────────────────────────────────
#[test]
fn bt3_036_ankylomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-036")
        .expect("BT3-036")
        .add_card(make_digimon("ATK", "Attacker", CardColor::Red, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD", "BT3-036"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "Ankylomon played from security");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-036"
    );
}

// ─── BT3-037 Turuiemon ───────────────────────────────────────────────────────
#[test]
fn bt3_037_turuiemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-037")
        .expect("BT3-037")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-037"])
        .memory(8)
        .start();

    let card = r.compiled_card("BT3-037").expect("BT3-037");
    assert_eq!(card.name, "Turuiemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(6000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT3-038 Antylamon ───────────────────────────────────────────────────────
#[test]
fn bt3_038_antylamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-038")
        .expect("BT3-038")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-038"])
        .memory(8)
        .start();

    let card = r.compiled_card("BT3-038").expect("BT3-038");
    assert_eq!(card.name, "Antylamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(8000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(8000));
}

// ─── BT3-039 Angewomon ───────────────────────────────────────────────────────
#[test]
fn bt3_039_angewomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-039")
        .expect("BT3-039")
        .add_card(make_digimon("BASE", "Gatomon", CardColor::Yellow, 4, 4000, 4))
        .add_card(make_digimon("LV6", "Ophanimon", CardColor::Yellow, 6, 11000, 11))
        .add_card(make_digimon("Y-LV3", "Patamon", CardColor::Yellow, 3, 2000, 3))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-039", "Y-LV3"])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    set_evo_cost(&mut r, "BT3-039", CardColor::Yellow, 4, 3);

    // 1. When Digivolving: opponent gets Security A. -2
    let base = r.place_on_field(0, "BASE", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    digivolve_from_hand(&mut r, 0, "BT3-039", base);
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.security_attack_keyword_bonus(opp),
        -2,
        "Angewomon When Digivolving gives opponent Security A. -2"
    );

    // 2. Inherited When Attacking (security <= 3): play yellow Lv.3 from hand free
    let carrier = r.place_field_stack(0, &["BT3-039", "LV6"], false, 0);
    let ba_before = r.battle_area_size(0);
    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(
        r.battle_area_size(0),
        ba_before + 1,
        "Angewomon inherited played yellow Lv.3 Digimon from hand free"
    );
}

// ─── BT3-040 Shakkoumon ──────────────────────────────────────────────────────
#[test]
fn bt3_040_shakkoumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-040")
        .expect("BT3-040")
        .add_card(make_digimon("OPP", "Sourceless", CardColor::Red, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let shak = r.place_on_field(0, "BT3-040", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));

    // Your turn: also treated as Blue
    r.game.enter_main_phase();
    let colors = r.game.players[0].battle_area[shak.index as usize].colors_for_rules(
        &r.game.card_data,
        &r.game.modifiers,
        shak,
    );
    assert!(
        colors.contains(&CardColor::Yellow) && colors.contains(&CardColor::Blue),
        "Shakkoumon is also treated as blue on your turn: {colors:?}"
    );

    // Opponent's turn: opponent's sourceless Digimon get Security A. -1
    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "Shakkoumon gives opponent's sourceless Digimon Security A. -1 on opponent's turn"
    );
}

// ─── BT3-041 Cherubimon ──────────────────────────────────────────────────────
#[test]
fn bt3_041_cherubimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-041")
        .expect("BT3-041")
        .add_card(make_digimon("Y-MON", "Patamon", CardColor::Yellow, 3, 2000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    put_in_trash(&mut r, 0, "Y-MON");

    let cherub = r.place_on_field(0, "BT3-041", Some(0));
    let sec_before = r.security_count(0);

    r.attack_player(cherub, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(0),
        sec_before + 1,
        "Cherubimon placed yellow Digimon from trash on top of security"
    );
}

// ─── BT3-042 ClavisAngemon ───────────────────────────────────────────────────
#[test]
fn bt3_042_clavisangemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-042")
        .expect("BT3-042")
        .add_card(make_digimon("OPP", "BigOpp", CardColor::Blue, 6, 10000, 11))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let clavis = r.place_on_field(0, "BT3-042", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));

    r.attack_player(clavis, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.dp_of(opp), Some(4000), "ClavisAngemon gave -6000 DP when security <= 3");
}

// ─── BT3-043 Kentaurosmon ────────────────────────────────────────────────────
#[test]
fn bt3_043_kentaurosmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-043")
        .expect("BT3-043")
        .add_card(make_digimon("BASE", "Lv5", CardColor::Yellow, 5, 7000, 7))
        .add_card(make_digimon("OPP1", "Opp1", CardColor::Blue, 6, 12000, 12))
        .add_card(make_digimon("OPP2", "Opp2", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-043"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-043", CardColor::Yellow, 5, 4);

    let base = r.place_on_field(0, "BASE", Some(0));
    let opp1 = r.place_on_field(1, "OPP1", Some(0));
    let opp2 = r.place_on_field(1, "OPP2", Some(0));

    digivolve_from_hand(&mut r, 0, "BT3-043", base);
    let _ = r.auto_resolve_declaring_max();

    assert_eq!(
        r.game.security_attack_keyword_bonus(opp1),
        -2,
        "Kentaurosmon When Digivolving gave Opp1 Security A. -2"
    );
    assert_eq!(
        r.game.security_attack_keyword_bonus(opp2),
        -2,
        "Kentaurosmon When Digivolving gave Opp2 Security A. -2"
    );

    // On Deletion: 1 opponent Digimon gets -11000 DP
    r.game.delete_permanent_with_effects(base);
    let _ = r.auto_resolve();
    // Either opp1 dropped to 1000 DP or opp2 was deleted by 0 DP rule
    let opp1_reduced = r.battle_area_size(1) == 2 && r.dp_of(opp1) == Some(1000);
    let opp2_deleted = r.battle_area_size(1) == 1;
    assert!(
        opp1_reduced || opp2_deleted,
        "Kentaurosmon On Deletion gave -11000 DP to an opponent Digimon"
    );
}

// ─── BT3-044 Aruraumon ───────────────────────────────────────────────────────
#[test]
fn bt3_044_aruraumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-044")
        .expect("BT3-044")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-044"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-044").expect("BT3-044");
    assert_eq!(card.name, "Aruraumon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(5000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(5000));
}

// ─── BT3-045 Kunemon ─────────────────────────────────────────────────────────
#[test]
fn bt3_045_kunemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-045")
        .expect("BT3-045")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-045"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-045").expect("BT3-045");
    assert_eq!(card.name, "Kunemon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(4000));
}

// ─── BT3-046 Terriermon ──────────────────────────────────────────────────────
#[test]
fn bt3_046_terriermon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-046")
        .expect("BT3-046")
        .dsl_card("BT3-005")
        .expect("BT3-005 Kakkinmon gains 1 memory")
        .add_card(make_digimon("LV7", "Lv7", CardColor::Black, 7, 14000, 14))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 3])
        .memory(5)
        .start();

    // P0 has Terriermon on field -> P1 cannot gain memory except by Tamer effects
    let _terrier = r.place_on_field(0, "BT3-046", Some(0));
    let opp_lv7 = r.place_field_stack(1, &["BT3-005", "LV7"], false, 0);

    r.end_turn(); // Now P1's turn
    r.game.set_memory(3);
    r.game.turn_count = 3;
    let mem_before = r.memory();
    r.attack_player(opp_lv7, 0, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        mem_before,
        "Terriermon blocked opponent's non-Tamer memory gain"
    );
}

// ─── BT3-047 Wormmon ─────────────────────────────────────────────────────────
#[test]
fn bt3_047_wormmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-047")
        .expect("BT3-047")
        .add_card(make_digimon("LV4", "Stingmon", CardColor::Green, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD", "PAD", "PAD", "LV4"])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let worm = r.place_on_field(0, "BT3-047", Some(0));
    let hand_before = r.hand_size(0);

    r.game.delete_permanent_with_effects(worm);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before + 1, "Wormmon On Deletion added Lv.4 to hand");
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "LV4"));
}

// ─── BT3-048 Gargomon ────────────────────────────────────────────────────────
#[test]
fn bt3_048_gargomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-048")
        .expect("BT3-048")
        .add_card(make_digimon("CARRIER", "Rapidmon", CardColor::Green, 5, 6000, 6))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-048", "CARRIER"], false, 0);
    let o1 = r.place_on_field(1, "OPP", Some(0));
    let o2 = r.place_on_field(1, "OPP", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(6000));

    r.game.suspend(o1);
    r.game.suspend(o2);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.effective_dp(carrier),
        Some(8000),
        "Gargomon inherited grants +1000 DP per opponent suspended Digimon"
    );
}

// ─── BT3-049 Flymon ──────────────────────────────────────────────────────────
#[test]
fn bt3_049_flymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-049")
        .expect("BT3-049")
        .add_card(make_digimon("ATK", "Attacker", CardColor::Red, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD", "BT3-049"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "Flymon played from security");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-049"
    );
}

// ─── BT3-050 Stingmon ────────────────────────────────────────────────────────
#[test]
fn bt3_050_stingmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-050")
        .expect("BT3-050")
        .add_card(make_digimon("CARRIER", "Dinobeemon", CardColor::Green, 5, 7000, 8))
        .add_card(make_digimon("OPP", "SmallOpp", CardColor::Blue, 3, 3000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(3)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-050", "CARRIER"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.suspend(opp);

    let mem_before = r.memory();
    r.attack_digimon(carrier, opp, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "opponent deleted in battle");
    assert_eq!(
        r.memory(),
        mem_before + 1,
        "Stingmon inherited gained 1 memory on battle deletion"
    );
}

// ─── BT3-051 Dokugumon ───────────────────────────────────────────────────────
#[test]
fn bt3_051_dokugumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-051")
        .expect("BT3-051")
        .add_card(make_digimon("LV5", "Lv5", CardColor::Green, 5, 7000, 7))
        .add_card(make_digimon("LV6", "Lv6", CardColor::Green, 6, 11000, 11))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD", "PAD", "LV5", "LV6"])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-051"])
        .memory(8)
        .start();

    let hand_before = r.hand_size(0);
    r.play(0, 0);
    let _ = r.auto_resolve();

    // Played BT3-051 (-1), added LV5 (+1) and LV6 (+1), trashed PAD (1 in trash)
    assert_eq!(r.hand_size(0), hand_before + 1);
    assert_eq!(r.trash_size(0), 1, "Remaining revealed card was trashed");
}

// ─── BT3-052 Rapidmon ────────────────────────────────────────────────────────
#[test]
fn bt3_052_rapidmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-052")
        .expect("BT3-052")
        .add_card(make_digimon("CARRIER", "MegaGargomon", CardColor::Green, 6, 11000, 12))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-052", "CARRIER"], false, 0);
    let o1 = r.place_on_field(1, "OPP", Some(0));
    r.game.suspend(o1);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(carrier),
        Some(12000),
        "Rapidmon inherited grants +1000 DP per opponent suspended Digimon"
    );
}

// ─── BT3-053 JewelBeemon ─────────────────────────────────────────────────────
#[test]
fn bt3_053_jewelbeemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-053")
        .expect("BT3-053")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-053"])
        .memory(8)
        .start();

    let card = r.compiled_card("BT3-053").expect("BT3-053");
    assert_eq!(card.name, "JewelBeemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(10000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(10000));
}

// ─── BT3-055 Dinobeemon ──────────────────────────────────────────────────────
#[test]
fn bt3_055_dinobeemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-055")
        .expect("BT3-055")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT3-055", Some(0));
    assert!(r.game.has_keyword(h, Keyword::Piercing));
    assert!(r.game.has_keyword(h, Keyword::Jamming));
    assert_eq!(r.dp_of(h), Some(7000));
}

// ─── BT3-056 Ceresmon ────────────────────────────────────────────────────────
#[test]
fn bt3_056_ceresmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-056")
        .expect("BT3-056")
        .add_card(make_digimon("G-LV5", "Blossomon", CardColor::Green, 5, 7000, 7))
        .add_card(make_digimon("ALLY", "Terriermon", CardColor::Green, 3, 2000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-056"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT3-056", CardColor::Green, 5, 4);

    let base = r.place_on_field(0, "G-LV5", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    digivolve_from_hand(&mut r, 0, "BT3-056", base);
    r.accept_optional_trigger().expect("accept Digisorption -3");
    r.execute_action(0, ATTACK_START + ally.index as u16)
        .expect("suspend ally for Digisorption");
    let _ = r.auto_resolve();

    // Cost 4 - 3 = 1 -> memory 5 - 1 = 4
    assert_eq!(r.memory(), 4, "Digisorption -3 reduced Ceresmon digivolve cost from 4 to 1");
    assert!(r.game.players[0].battle_area[ally.index as usize].is_suspended);
}

// ─── BT3-057 MegaGargomon ────────────────────────────────────────────────────
#[test]
fn bt3_057_megagargomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-057")
        .expect("BT3-057")
        .add_card(make_digimon("BASE", "Rapidmon", CardColor::Green, 5, 6000, 6))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-057"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-057", CardColor::Green, 5, 4);

    let base = r.place_on_field(0, "BASE", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));

    digivolve_from_hand(&mut r, 0, "BT3-057", base);
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "MegaGargomon suspended opponent Digimon"
    );
    r.game.tick_declarative_effects();
    assert_eq!(
        r.modifiers().sum(base, ModifierType::SecurityAttackChange),
        1,
        "MegaGargomon gains Security A. +1 while opponent has a suspended Digimon"
    );

    // Opponent's next unsuspend phase: opp stays suspended
    r.end_turn();
    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "Opponent Digimon did not unsuspend during opponent's unsuspend phase"
    );
}

// ─── BT3-058 BanchoStingmon ──────────────────────────────────────────────────
#[test]
fn bt3_058_banchostingmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-058")
        .expect("BT3-058")
        .add_card(make_digimon("BIG-OPP", "BigOpp", CardColor::Blue, 6, 12000, 12))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 5])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let bancho = r.place_on_field(0, "BT3-058", Some(0));
    let opp = r.place_on_field(1, "BIG-OPP", Some(0));
    r.game.suspend(opp);

    assert!(r.game.has_keyword(bancho, Keyword::Piercing));
    let sec_before = r.security_count(1);

    // 9000 + 7000 = 16000 > 12000 -> deletes opp, Piercing checks 1 + 2 = 3 security!
    r.attack_digimon(bancho, opp, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "12000 DP opponent deleted by 16000 DP BanchoStingmon");
    assert_eq!(r.dp_of(bancho), Some(16000), "+7000 DP lasts for the turn");
    assert_eq!(
        r.security_count(1),
        sec_before - 3,
        "Piercing + Security A. +2 checked 3 security cards"
    );
}

// ─── BT3-059 Commandramon ────────────────────────────────────────────────────
#[test]
fn bt3_059_commandramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-059")
        .expect("BT3-059")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-059"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-059").expect("BT3-059");
    assert_eq!(card.name, "Commandramon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(3000));
}

// ─── BT3-060 Psychemon ───────────────────────────────────────────────────────
#[test]
fn bt3_060_psychemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-060")
        .expect("BT3-060")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-060"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-060").expect("BT3-060");
    assert_eq!(card.name, "Psychemon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(5000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(5000));
}
