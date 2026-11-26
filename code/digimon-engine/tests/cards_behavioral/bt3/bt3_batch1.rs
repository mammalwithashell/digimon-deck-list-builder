#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::mask::build_action_mask;
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

// ─── BT3-001 Poromon ─────────────────────────────────────────────────────────
#[test]
fn bt3_001_poromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-001")
        .expect("BT3-001")
        .add_card(make_digimon("CARRIER", "Agumon", CardColor::Red, 3, 3000, 3))
        .add_card(make_digimon("OPP-SMALL", "Small", CardColor::Blue, 3, 1000, 3))
        .add_card(make_digimon("OPP-BIG", "Big", CardColor::Blue, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-001", "CARRIER"], false, 0);
    let _small = r.place_on_field(1, "OPP-SMALL", Some(0));
    let _big = r.place_on_field(1, "OPP-BIG", Some(0));

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "1000 DP opponent deleted, 5000 DP remains");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "OPP-BIG"
    );
}

// ─── BT3-003 Upamon ──────────────────────────────────────────────────────────
#[test]
fn bt3_003_upamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-003")
        .expect("BT3-003")
        .add_card(make_digimon("CARRIER", "Patamon", CardColor::Yellow, 3, 3000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 5])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-003", "CARRIER"], false, 0);
    let hand_before = r.hand_size(0);

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "Upamon draws 1 when security <= 3"
    );

    // Second attack same turn blocked by OPT
    r.game.unsuspend(carrier);
    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), hand_before + 1, "OPT prevents second draw");
}

// ─── BT3-004 Minomon ─────────────────────────────────────────────────────────
#[test]
fn bt3_004_minomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-004")
        .expect("BT3-004")
        .add_card(make_digimon("CARRIER", "Wormmon", CardColor::Green, 3, 4000, 3))
        .add_card(make_digimon("OPP", "Target", CardColor::Blue, 4, 4000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-004", "CARRIER"], false, 0);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.suspend(opp);

    // 4000 + 1000 = 5000 > 4000 so carrier survives and deletes opponent
    r.attack_digimon(carrier, opp, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 1, "carrier survives with +1000 DP");
    assert_eq!(r.battle_area_size(1), 0, "opponent deleted in battle");
    assert_eq!(r.dp_of(carrier), Some(5000), "DP boost lasts for the turn");
}

// ─── BT3-005 Kakkinmon ───────────────────────────────────────────────────────
#[test]
fn bt3_005_kakkinmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-005")
        .expect("BT3-005")
        .add_card(make_digimon("LV7", "RagnaLoardmon", CardColor::Black, 7, 14000, 14))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 4])
        .memory(3)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-005", "LV7"], false, 0);
    let mem_before = r.memory();

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), mem_before + 1, "Kakkinmon grants +1 memory when carrier is Lv.7");
}

// ─── BT3-008 Zubamon ─────────────────────────────────────────────────────────
#[test]
fn bt3_008_zubamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-008")
        .expect("BT3-008")
        .dsl_card("BT3-019")
        .expect("BT3-019 RagnaLoardmon")
        .dsl_card("BT3-010")
        .expect("BT3-010 ZubaEagermon")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "BT3-010", "BT3-019"])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-008"])
        .memory(5)
        .start();

    let hand_before = r.hand_size(0);
    r.play(0, 0);
    let _ = r.auto_resolve();

    // Played BT3-008 (-1), added BT3-019 (+1) and BT3-010 (+1) -> hand_before + 1
    assert_eq!(r.hand_size(0), hand_before + 1, "Zubamon adds RagnaLoardmon and Legend-Arms");
    let hand_ids: Vec<&str> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert!(hand_ids.contains(&"BT3-019"), "RagnaLoardmon added to hand");
    assert!(hand_ids.contains(&"BT3-010"), "Legend-Arms Digimon added to hand");
}

// ─── BT3-009 Hawkmon ─────────────────────────────────────────────────────────
#[test]
fn bt3_009_hawkmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-009")
        .expect("BT3-009")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-009"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-009").expect("compiled BT3-009");
    assert_eq!(card.name, "Hawkmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.battle_area_size(0), 1);
    let h = r.perm_handle(0, 0);
    assert_eq!(r.dp_of(h), Some(4000));
}

// ─── BT3-010 ZubaEagermon ────────────────────────────────────────────────────
#[test]
fn bt3_010_zubaeagermon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-010")
        .expect("BT3-010")
        .add_card(make_digimon("LV7", "RagnaLoardmon", CardColor::Red, 7, 14000, 14))
        .add_card(make_digimon("LV5", "Duramon", CardColor::Red, 5, 7000, 6))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let lv7 = r.place_field_stack(0, &["BT3-010", "LV7"], false, 0);
    let lv5 = r.place_field_stack(0, &["BT3-010", "LV5"], false, 0);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.modifiers().sum(lv7, ModifierType::SecurityAttackChange),
        1,
        "Lv.7 carrier gets Security A. +1 on your turn"
    );
    assert_eq!(
        r.modifiers().sum(lv5, ModifierType::SecurityAttackChange),
        0,
        "Lv.5 carrier does not get Security A. +1"
    );
}

// ─── BT3-011 Greymon ─────────────────────────────────────────────────────────
#[test]
fn bt3_011_greymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-011")
        .expect("BT3-011")
        .add_card(make_digimon("ATK", "Attacker", CardColor::Blue, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD", "BT3-011"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        1,
        "BT3-011 Greymon plays itself from security without paying cost"
    );
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-011"
    );
}

// ─── BT3-012 Aquilamon ───────────────────────────────────────────────────────
#[test]
fn bt3_012_aquilamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-012")
        .expect("BT3-012")
        .add_card(make_digimon("CARRIER", "Silphymon", CardColor::Red, 5, 7000, 7))
        .add_card(make_digimon("OPP-2K", "Small", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("OPP-3K", "Bigger", CardColor::Blue, 3, 3000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-012", "CARRIER"], false, 0);
    let _small = r.place_on_field(1, "OPP-2K", Some(0));
    let _big = r.place_on_field(1, "OPP-3K", Some(0));

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "2000 DP opponent deleted, 3000 DP survives");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "OPP-3K"
    );
}

// ─── BT3-013 Duramon ─────────────────────────────────────────────────────────
#[test]
fn bt3_013_duramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-013")
        .expect("BT3-013")
        .add_card(make_digimon("LV7", "RagnaLoardmon", CardColor::Red, 7, 14000, 14))
        .add_card(make_digimon("LV6", "Durandamon", CardColor::Red, 6, 12000, 10))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let lv7 = r.place_field_stack(0, &["BT3-013", "LV7"], false, 0);
    let lv6 = r.place_field_stack(0, &["BT3-013", "LV6"], false, 0);
    r.game.tick_declarative_effects();

    assert_eq!(r.modifiers().sum(lv7, ModifierType::SecurityAttackChange), 1);
    assert_eq!(r.modifiers().sum(lv6, ModifierType::SecurityAttackChange), 0);
}

// ─── BT3-014 Silphymon ───────────────────────────────────────────────────────
#[test]
fn bt3_014_silphymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-014")
        .expect("BT3-014")
        .add_card(make_digimon("BASE", "Aquilamon", CardColor::Red, 4, 5000, 5))
        .add_card(make_digimon("OPP-LV4", "Target", CardColor::Blue, 4, 6000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-014"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT3-014", CardColor::Red, 4, 3);

    let base = r.place_on_field(0, "BASE", Some(0));
    let opp = r.place_on_field(1, "OPP-LV4", Some(0));

    digivolve_from_hand(&mut r, 0, "BT3-014", base);
    let _ = r.auto_resolve();

    assert_eq!(
        r.effective_dp(opp),
        Some(1000),
        "When Digivolving changes opponent Lv.4- original DP to 1000"
    );
    let colors = r.game.players[0].battle_area[base.index as usize].colors_for_rules(
        &r.game.card_data,
        &r.game.modifiers,
        base,
    );
    assert!(
        colors.contains(&CardColor::Red) && colors.contains(&CardColor::Yellow),
        "Silphymon is also treated as yellow on your turn: {colors:?}"
    );
}

// ─── BT3-015 MetalGreymon ────────────────────────────────────────────────────
#[test]
fn bt3_015_metalgreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-015")
        .expect("BT3-015")
        .dsl_card("BT3-112")
        .expect("BT3-112 Omnimon Alter-S (Lv.7 Virus)")
        .add_card(make_digimon("BASE", "Greymon", CardColor::Red, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-015"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT3-015", CardColor::Red, 4, 3);
    put_in_trash(&mut r, 0, "BT3-112");

    let base = r.place_on_field(0, "BASE", Some(0));
    digivolve_from_hand(&mut r, 0, "BT3-015", base);
    let _ = r.auto_resolve();

    assert!(r.game.has_keyword(base, Keyword::Piercing), "MetalGreymon has Piercing");
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "BT3-112"),
        "Returned Lv.7 Virus Digimon from trash to hand"
    );
}

// ─── BT3-016 Durandamon ──────────────────────────────────────────────────────
#[test]
fn bt3_016_durandamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-016")
        .expect("BT3-016")
        .add_card(make_digimon("LV7", "RagnaLoardmon", CardColor::Red, 7, 14000, 14))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-016", "LV7"], false, 0);
    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "Durandamon grants inherited Piercing"
    );
}

// ─── BT3-017 Valkyrimon ──────────────────────────────────────────────────────
#[test]
fn bt3_017_valkyrimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-017")
        .expect("BT3-017")
        .add_card(make_digimon("BASE", "Silphymon", CardColor::Red, 5, 7000, 7))
        .add_card(make_digimon("OPP1", "Target1", CardColor::Blue, 4, 4000, 4))
        .add_card(make_digimon("OPP2", "Target2", CardColor::Blue, 3, 3000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-017"])
        .memory(6)
        .start();
    r.game.turn_count = 2;
    set_evo_cost(&mut r, "BT3-017", CardColor::Red, 5, 3);

    let base = r.place_on_field(0, "BASE", Some(0));
    let _o1 = r.place_on_field(1, "OPP1", Some(0));
    let _o2 = r.place_on_field(1, "OPP2", Some(0));

    // When Digivolving deletes first 4000 DP- opponent
    digivolve_from_hand(&mut r, 0, "BT3-017", base);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1, "When Digivolving deleted 1 opponent");

    // When Attacking deletes second 4000 DP- opponent
    r.attack_player(base, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "When Attacking deleted second opponent");
}

// ─── BT3-018 BlitzGreymon ────────────────────────────────────────────────────
#[test]
fn bt3_018_blitzgreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-018")
        .expect("BT3-018")
        .add_card(make_digimon("BASE", "MetalGreymon", CardColor::Red, 5, 7000, 7))
        .add_card(make_digimon("O3", "Lv3", CardColor::Blue, 3, 3000, 3))
        .add_card(make_digimon("O4", "Lv4", CardColor::Blue, 4, 5000, 5))
        .add_card(make_digimon("O5", "Lv5", CardColor::Blue, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-018"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-018", CardColor::Red, 5, 4);

    let base = r.place_on_field(0, "BASE", Some(0));
    let opp = r.place_field_stack(1, &["O3", "O4", "O5"], false, 0);

    digivolve_from_hand(&mut r, 0, "BT3-018", base);
    let _ = r.auto_resolve_declaring_max();

    assert!(r.game.has_keyword(base, Keyword::Piercing));
    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "De-Digivolve 2 trashed O5 and O4 down to Lv.3 O3"
    );
    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "O3"
    );
}

// ─── BT3-019 RagnaLoardmon ───────────────────────────────────────────────────
#[test]
fn bt3_019_ragnaloardmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-019")
        .expect("BT3-019")
        .dsl_card("BT3-016")
        .expect("BT3-016 Durandamon")
        .add_card(make_digimon("BASE", "Lv6", CardColor::Red, 6, 11000, 11))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-019", "BT3-016"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT3-019", CardColor::Red, 6, 3);

    let base = r.place_on_field(0, "BASE", Some(0));
    digivolve_from_hand(&mut r, 0, "BT3-019", base);
    let _ = r.auto_resolve();

    assert!(r.game.has_keyword(base, Keyword::Reboot));
    assert_eq!(r.game.security_attack_keyword_bonus(base), 1);
    // Paid 3 memory to digivolve (5 -> 2), placed Durandamon from hand to gain 3 memory (2 -> 5)
    assert_eq!(r.memory(), 5, "Placing Durandamon gained +3 memory");
    assert_eq!(
        r.game.players[0].battle_area[base.index as usize].stack_size(),
        3,
        "Durandamon placed as top digivolution source under RagnaLoardmon"
    );
}

// ─── BT3-020 Patamon ─────────────────────────────────────────────────────────
#[test]
fn bt3_020_patamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-020")
        .expect("BT3-020")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-020"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-020").expect("compiled BT3-020");
    assert_eq!(card.name, "Patamon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.battle_area_size(0), 1);
}

// ─── BT3-021 Veemon ──────────────────────────────────────────────────────────
#[test]
fn bt3_021_veemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-021")
        .expect("BT3-021")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let h = r.place_on_field(0, "BT3-021", Some(0));
    assert!(r.game.has_keyword(h, Keyword::Jamming), "Veemon has Jamming");
    assert_eq!(r.dp_of(h), Some(2000));
}

// ─── BT3-022 Penguinmon ──────────────────────────────────────────────────────
#[test]
fn bt3_022_penguinmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-022")
        .expect("BT3-022")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-022"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-022").expect("compiled BT3-022");
    assert_eq!(card.name, "Penguinmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(5000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(5000));
}

// ─── BT3-023 Angemon ─────────────────────────────────────────────────────────
#[test]
fn bt3_023_angemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-023")
        .expect("BT3-023")
        .add_card(make_digimon("CARRIER", "MagnaAngemon", CardColor::Blue, 5, 7000, 6))
        .add_card(make_digimon("SRC", "Src", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-023", "CARRIER"], false, 0);
    let opp = r.place_field_stack(1, &["SRC", "OPP"], false, 0);
    assert_eq!(r.game.players[1].battle_area[opp.index as usize].stack_size(), 2);

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "Angemon inherited trashed bottom digivolution card of opponent Digimon"
    );
    assert!(r.trash_size(1) >= 1);
}

// ─── BT3-024 Airdramon ───────────────────────────────────────────────────────
#[test]
fn bt3_024_airdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-024")
        .expect("BT3-024")
        .add_card(make_digimon("ATK", "Attacker", CardColor::Red, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD", "BT3-024"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "Airdramon played from security");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-024"
    );
}

// ─── BT3-025 ExVeemon ────────────────────────────────────────────────────────
#[test]
fn bt3_025_exveemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-025")
        .expect("BT3-025")
        .add_card(make_digimon("BASE", "Veemon", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("ALLY", "Rookie", CardColor::Blue, 3, 3000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-025"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT3-025", CardColor::Blue, 3, 2);

    let base = r.place_on_field(0, "BASE", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.suspend(ally);
    assert!(r.game.players[0].battle_area[ally.index as usize].is_suspended);

    digivolve_from_hand(&mut r, 0, "BT3-025", base);
    r.execute_action(0, ATTACK_START + ally.index as u16)
        .expect("select suspended ally to unsuspend");
    let _ = r.auto_resolve();

    assert!(
        !r.game.players[0].battle_area[ally.index as usize].is_suspended,
        "ExVeemon unsuspended Lv.4- ally"
    );
}

// ─── BT3-026 MagnaAngemon ────────────────────────────────────────────────────
#[test]
fn bt3_026_magnaangemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-026")
        .expect("BT3-026")
        .add_card(make_digimon("CARRIER", "Seraphimon", CardColor::Blue, 6, 11000, 11))
        .add_card(make_digimon("SRC", "Src", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-026", "CARRIER"], false, 0);
    let opp = r.place_field_stack(1, &["SRC", "OPP"], false, 0);

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "MagnaAngemon inherited trashed bottom digivolution card"
    );
}

// ─── BT3-027 Paildramon ──────────────────────────────────────────────────────
#[test]
fn bt3_027_paildramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-027")
        .expect("BT3-027")
        .add_card(make_digimon(
            "IMP",
            "Imperialdramon Dragon Mode",
            CardColor::Blue,
            6,
            11000,
            12,
        ))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let solo = r.place_on_field(0, "BT3-027", Some(0));
    assert!(r.game.has_keyword(solo, Keyword::Jamming), "Paildramon has Jamming");

    let imp = r.place_field_stack(0, &["BT3-027", "IMP"], false, 0);
    r.attack_player(imp, 1, false);
    let _ = r.auto_resolve();

    assert!(
        !r.game.players[0].battle_area[imp.index as usize].is_suspended,
        "Paildramon inherited unsuspends Imperialdramon when attacking"
    );
}

// ─── BT3-028 Bastemon ────────────────────────────────────────────────────────
#[test]
fn bt3_028_bastemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-028")
        .expect("BT3-028")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-028"])
        .memory(8)
        .start();

    let card = r.compiled_card("BT3-028").expect("compiled BT3-028");
    assert_eq!(card.name, "Bastemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(8000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(8000));
}

// ─── BT3-029 Goldramon ───────────────────────────────────────────────────────
#[test]
fn bt3_029_goldramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-029")
        .expect("BT3-029")
        .add_card(make_digimon("ALLY", "Patamon", CardColor::Blue, 3, 3000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["ALLY"])
        .memory(5)
        .start();

    let gold = r.place_on_field(0, "BT3-029", Some(0));
    r.game.suspend(gold);
    assert!(r.game.players[0].battle_area[gold.index as usize].is_suspended);

    r.play(0, 0);
    let _ = r.auto_resolve();

    assert!(
        !r.game.players[0].battle_area[gold.index as usize].is_suspended,
        "Goldramon unsuspends when you play another Digimon on your turn"
    );
}

// ─── BT3-030 Leopardmon ──────────────────────────────────────────────────────
#[test]
fn bt3_030_leopardmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-030")
        .expect("BT3-030")
        .add_card(make_digimon("LV4", "ExVeemon", CardColor::Blue, 4, 4000, 5))
        .add_card(make_digimon("LV5", "Paildramon", CardColor::Blue, 5, 7000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-030"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-030", CardColor::Blue, 5, 4);

    let stack = r.place_field_stack(0, &["LV4", "LV5"], false, 0);
    assert_eq!(r.battle_area_size(0), 1);

    digivolve_from_hand(&mut r, 0, "BT3-030", stack);
    let _ = r.auto_resolve();

    // LV4 played out of stack as a separate Digimon without paying cost
    assert_eq!(
        r.battle_area_size(0),
        2,
        "Leopardmon played Lv.4 source as another Digimon"
    );
    let played = r.perm_handle(0, 1);
    assert!(
        r.game.has_keyword(played, Keyword::Jamming),
        "Leopardmon aura grants Jamming to Lv.4 or lower Digimon on your turn"
    );
}
