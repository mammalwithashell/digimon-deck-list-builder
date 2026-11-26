#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
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

fn make_option(id: &str, name: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Option;
    c.colors = vec![color];
    c.level = None;
    c.dp = None;
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

fn play_option(r: &mut DebugRunner, player: u8, hand_index: usize) {
    r.game.enter_main_phase();
    r.game.play_option_from_hand(player, hand_index);
}

// ─── BT3-061 Chuumon ─────────────────────────────────────────────────────────
#[test]
fn bt3_061_chuumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-061")
        .expect("BT3-061")
        .dsl_card("BT3-005")
        .expect("BT3-005 Kakkinmon gains 1 memory")
        .add_card(make_digimon("LV7", "Lv7", CardColor::Black, 7, 14000, 14))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 3])
        .memory(5)
        .start();

    let _chuu = r.place_on_field(0, "BT3-061", Some(0));
    let opp_lv7 = r.place_field_stack(1, &["BT3-005", "LV7"], false, 0);

    r.end_turn(); // P1's turn
    r.game.set_memory(3);
    r.game.turn_count = 3;
    let mem_before = r.memory();
    r.attack_player(opp_lv7, 0, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        mem_before,
        "Chuumon blocked opponent's non-Tamer memory gain"
    );
}

// ─── BT3-062 Ludomon ─────────────────────────────────────────────────────────
#[test]
fn bt3_062_ludomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-062")
        .expect("BT3-062")
        .dsl_card("BT3-019")
        .expect("BT3-019 RagnaLoardmon")
        .dsl_card("BT3-064")
        .expect("BT3-064 TiaLudomon")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "BT3-064", "BT3-019"])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-062"])
        .memory(5)
        .start();

    let hand_before = r.hand_size(0);
    r.play(0, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before + 1);
    let hand_ids: Vec<&str> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert!(hand_ids.contains(&"BT3-019"));
    assert!(hand_ids.contains(&"BT3-064"));
}

// ─── BT3-063 Sukamon ─────────────────────────────────────────────────────────
#[test]
fn bt3_063_sukamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-063")
        .expect("BT3-063")
        .dsl_card("BT3-061")
        .expect("BT3-061 Chuumon")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD", "PAD", "PAD", "BT3-061"])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let suka = r.place_on_field(0, "BT3-063", Some(0));
    r.game.delete_permanent_with_effects(suka);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 1, "Sukamon On Deletion played Chuumon free");
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-061"
    );
}

// ─── BT3-064 TiaLudomon ──────────────────────────────────────────────────────
#[test]
fn bt3_064_tialudomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-064")
        .expect("BT3-064")
        .add_card(make_digimon("LV7", "RagnaLoardmon", CardColor::Black, 7, 14000, 14))
        .add_card(make_digimon("O3", "Lv3", CardColor::Blue, 3, 3000, 3))
        .add_card(make_digimon("O4", "Lv4", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-064", "LV7"], false, 0);
    let opp = r.place_field_stack(1, &["O3", "O4"], false, 0);

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "TiaLudomon inherited De-Digivolve 1 trashed top card of opponent Digimon"
    );
}

// ─── BT3-065 Gururumon ───────────────────────────────────────────────────────
#[test]
fn bt3_065_gururumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-065")
        .expect("BT3-065")
        .add_card(make_digimon("ATK", "Attacker", CardColor::Red, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD", "BT3-065"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "Gururumon played from security");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-065"
    );
}

// ─── BT3-066 Clockmon ────────────────────────────────────────────────────────
#[test]
fn bt3_066_clockmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-066")
        .expect("BT3-066")
        .add_card(make_digimon("CARRIER", "Giromon", CardColor::Black, 5, 7000, 6))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-066", "CARRIER"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(7000), "No boost on your turn");

    r.end_turn(); // Opponent's turn
    r.game.tick_declarative_effects();
    assert_eq!(
        r.effective_dp(carrier),
        Some(8000),
        "Clockmon inherited grants +1000 DP on opponent's turn"
    );
}

// ─── BT3-067 Tankmon ─────────────────────────────────────────────────────────
#[test]
fn bt3_067_tankmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-067")
        .expect("BT3-067")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-067"])
        .memory(8)
        .start();

    let card = r.compiled_card("BT3-067").expect("BT3-067");
    assert_eq!(card.name, "Tankmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(6000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT3-068 Giromon ─────────────────────────────────────────────────────────
#[test]
fn bt3_068_giromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-068")
        .expect("BT3-068")
        .add_card(make_digimon("CARRIER", "Craniamon", CardColor::Black, 6, 12000, 13))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-068", "CARRIER"], false, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(carrier), Some(12000));

    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(
        r.effective_dp(carrier),
        Some(13000),
        "Giromon inherited grants +1000 DP on opponent's turn"
    );
}

// ─── BT3-069 RaijiLudomon ────────────────────────────────────────────────────
#[test]
fn bt3_069_raijiludomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-069")
        .expect("BT3-069")
        .add_card(make_digimon("LV7", "RagnaLoardmon", CardColor::Black, 7, 14000, 14))
        .add_card(make_digimon("O3", "Lv3", CardColor::Blue, 3, 3000, 3))
        .add_card(make_digimon("O4", "Lv4", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let carrier = r.place_field_stack(0, &["BT3-069", "LV7"], false, 0);
    let opp = r.place_field_stack(1, &["O3", "O4"], false, 0);

    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "RaijiLudomon inherited De-Digivolve 1 trashed top card of opponent Digimon"
    );
}

// ─── BT3-070 Etemon ──────────────────────────────────────────────────────────
#[test]
fn bt3_070_etemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-070")
        .expect("BT3-070")
        .dsl_card("BT3-074")
        .expect("BT3-074 MetalEtemon")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "PAD", "BT3-074"])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let ete = r.place_on_field(0, "BT3-070", Some(0));
    assert!(r.game.has_keyword(ete, Keyword::Blocker), "Etemon has Blocker");

    r.game.delete_permanent_with_effects(ete);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 1, "Etemon On Deletion played MetalEtemon free");
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-074"
    );
}

// ─── BT3-071 MetalMamemon ────────────────────────────────────────────────────
#[test]
fn bt3_071_metalmamemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-071")
        .expect("BT3-071")
        .dsl_card("BT3-112")
        .expect("BT3-112 Omnimon Alter-S (Lv.7 Virus)")
        .add_card(make_digimon("BASE", "Tankmon", CardColor::Black, 4, 6000, 6))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-071"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT3-071", CardColor::Black, 4, 3);
    put_in_trash(&mut r, 0, "BT3-112");

    let base = r.place_on_field(0, "BASE", Some(0));
    digivolve_from_hand(&mut r, 0, "BT3-071", base);
    let _ = r.auto_resolve();

    assert!(r.game.has_keyword(base, Keyword::Reboot), "MetalMamemon has Reboot");
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "BT3-112"),
        "Returned Lv.7 Virus Digimon from trash to hand"
    );
}

// ─── BT3-072 BryweLudramon ───────────────────────────────────────────────────
#[test]
fn bt3_072_bryweludramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-072")
        .expect("BT3-072")
        .add_card(make_digimon("LV7", "RagnaLoardmon", CardColor::Black, 7, 14000, 14))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-072", "LV7"], false, 0);
    assert!(
        r.game.has_keyword(carrier, Keyword::Blocker),
        "BryweLudramon grants inherited Blocker"
    );
}

// ─── BT3-073 CresGarurumon ───────────────────────────────────────────────────
#[test]
fn bt3_073_cresgarurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-073")
        .expect("BT3-073")
        .add_card(make_digimon("BASE", "MetalMamemon", CardColor::Black, 5, 7000, 8))
        .add_card(make_digimon("BLK5", "Etemon", CardColor::Black, 5, 6000, 7))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["BLK5", "PAD", "PAD", "PAD"])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-073"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-073", CardColor::Black, 5, 4);

    let base = r.place_on_field(0, "BASE", Some(0));
    let _opp = r.place_on_field(1, "OPP", Some(0));

    digivolve_from_hand(&mut r, 0, "BT3-073", base);
    let _ = r.auto_resolve();

    assert!(r.game.has_keyword(base, Keyword::Reboot), "CresGarurumon has Reboot");
    assert_eq!(
        r.battle_area_size(0),
        2,
        "CresGarurumon revealed 1 card (1 opponent Digimon) and played black Lv.5 free"
    );
}

// ─── BT3-074 MetalEtemon ─────────────────────────────────────────────────────
#[test]
fn bt3_074_metaletemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-074")
        .expect("BT3-074")
        .dsl_card("BT3-070")
        .expect("BT3-070 Etemon Blocker")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let metal = r.place_on_field(0, "BT3-074", Some(0));
    let _opp_blocker = r.place_on_field(1, "BT3-070", Some(0));
    let sec_before = r.security_count(1);

    // Your turn: cannot be blocked -> goes straight to security without blocker prompt
    r.attack_player(metal, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "MetalEtemon cannot be blocked on your turn"
    );
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), sec_before - 1);
    assert_eq!(r.dp_of(metal), Some(10000));

    // Opponent's turn: +2000 DP
    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(
        r.effective_dp(metal),
        Some(12000),
        "MetalEtemon gets +2000 DP on opponent's turn"
    );
}

// ─── BT3-075 Craniamon ───────────────────────────────────────────────────────
#[test]
fn bt3_075_craniamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-075")
        .expect("BT3-075")
        .dsl_card("BT3-017")
        .expect("BT3-017 Valkyrimon deletes 4000 DP- on attack")
        .add_card(make_digimon("SMALL-BLK", "SmallBlocker", CardColor::Black, 4, 4000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 3])
        .memory(5)
        .start();

    let crania = r.place_on_field(0, "BT3-075", Some(0));
    assert!(r.game.has_keyword(crania, Keyword::Blocker), "Craniamon has Blocker");

    let small_blk = r.place_on_field(0, "SMALL-BLK", Some(0));
    r.game.modifiers.grant_keyword(
        small_blk,
        Keyword::Blocker,
        digimon_engine::enums::Expiry::Permanent,
        0,
    );
    let opp_valk = r.place_on_field(1, "BT3-017", Some(0));

    // Switch to P1's turn and attack P0 with Valkyrimon (triggers opponent delete <= 4000 DP)
    r.end_turn();
    r.game.turn_count = 3;
    r.attack_player(opp_valk, 0, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(0),
        2,
        "Craniamon prevented opponent's effect from deleting P0's Blocker"
    );
}

// ─── BT3-076 Candlemon ───────────────────────────────────────────────────────
#[test]
fn bt3_076_candlemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-076")
        .expect("BT3-076")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-076"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-076").expect("BT3-076");
    assert_eq!(card.name, "Candlemon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(3000));
}

// ─── BT3-078 Shamanmon ───────────────────────────────────────────────────────
#[test]
fn bt3_078_shamanmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-078")
        .expect("BT3-078")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-078"])
        .memory(5)
        .start();

    let card = r.compiled_card("BT3-078").expect("BT3-078");
    assert_eq!(card.name, "Shamanmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(4000));
}

// ─── BT3-079 Tsukaimon ───────────────────────────────────────────────────────
#[test]
fn bt3_079_tsukaimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-079")
        .expect("BT3-079")
        .add_card(make_digimon("CARRIER", "Devidramon", CardColor::Purple, 4, 4000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(3)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-079", "CARRIER"], false, 0);
    let mem_before = r.memory();

    r.game.delete_permanent_with_effects(carrier);
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), mem_before + 1, "Tsukaimon inherited gained 1 memory on deletion");
}

// ─── BT3-080 Saberdramon ─────────────────────────────────────────────────────
#[test]
fn bt3_080_saberdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-080")
        .expect("BT3-080")
        .add_card(make_digimon("CARRIER", "LadyDevimon", CardColor::Purple, 5, 8000, 8))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(5)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-080", "CARRIER"], false, 0);
    assert!(
        r.game.has_keyword(carrier, Keyword::Retaliation),
        "Saberdramon grants inherited Retaliation"
    );
}

// ─── BT3-081 Devidramon ──────────────────────────────────────────────────────
#[test]
fn bt3_081_devidramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-081")
        .expect("BT3-081")
        .add_card(make_digimon("CARRIER", "LadyDevimon", CardColor::Purple, 5, 8000, 8))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(3)
        .start();

    let carrier = r.place_field_stack(0, &["BT3-081", "CARRIER"], false, 0);
    let mem_before = r.memory();

    r.game.delete_permanent_with_effects(carrier);
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), mem_before + 1, "Devidramon inherited gained 1 memory on deletion");
}

// ─── BT3-082 BlackGatomon ────────────────────────────────────────────────────
#[test]
fn bt3_082_blackgatomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-082")
        .expect("BT3-082")
        .add_card(make_digimon("ATK", "Attacker", CardColor::Red, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD", "BT3-082"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "BlackGatomon played from security");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT3-082"
    );
}

// ─── BT3-083 Meramon ─────────────────────────────────────────────────────────
#[test]
fn bt3_083_meramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-083")
        .expect("BT3-083")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-083"])
        .memory(6)
        .start();

    let card = r.compiled_card("BT3-083").expect("BT3-083");
    assert_eq!(card.name, "Meramon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(5000));
}

// ─── BT3-084 Raremon ─────────────────────────────────────────────────────────
#[test]
fn bt3_084_raremon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-084")
        .expect("BT3-084")
        .add_card(make_option("OPT", "TestOption", CardColor::Purple, 2))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD", "PAD", "PAD", "OPT"])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-084"])
        .memory(6)
        .start();

    r.play(0, 0);
    let _ = r.auto_resolve();

    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "OPT"),
        "Raremon added Option card to hand"
    );
    assert_eq!(r.trash_size(0), 2, "Remaining 2 revealed cards were trashed");
}

// ─── BT3-085 SkullMeramon ────────────────────────────────────────────────────
#[test]
fn bt3_085_skullmeramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-085")
        .expect("BT3-085")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-085"])
        .memory(6)
        .start();

    let card = r.compiled_card("BT3-085").expect("BT3-085");
    assert_eq!(card.name, "SkullMeramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT3-086 Arukenimon ──────────────────────────────────────────────────────
#[test]
fn bt3_086_arukenimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-086")
        .expect("BT3-086")
        .add_card(make_digimon("MALO", "MaloMyotismon", CardColor::Purple, 6, 12000, 12))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .hand(0, &["MALO"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let aru = r.place_on_field(0, "BT3-086", Some(0));
    r.attack_player(aru, 1, false);
    if r.pending_is_optional() {
        r.accept_optional_trigger().expect("accept Arukenimon trigger");
    }
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), 2, "Paid 3 memory");
    assert_eq!(r.battle_area_size(0), 1, "MaloMyotismon played and Arukenimon deleted");
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "MALO"
    );
}

// ─── BT3-087 Mummymon ────────────────────────────────────────────────────────
#[test]
fn bt3_087_mummymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-087")
        .expect("BT3-087")
        .add_card(make_digimon("MALO", "MaloMyotismon", CardColor::Purple, 6, 12000, 12))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    put_in_trash(&mut r, 0, "MALO");

    let mummy = r.place_on_field(0, "BT3-087", Some(0));
    r.attack_player(mummy, 1, false);
    if r.pending_is_optional() {
        r.accept_optional_trigger().expect("accept Mummymon trigger");
    }
    let _ = r.auto_resolve();

    assert_eq!(r.memory(), 2, "Paid 3 memory");
    assert_eq!(r.battle_area_size(0), 1, "MaloMyotismon played from trash and Mummymon deleted");
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "MALO"
    );
}

// ─── BT3-088 LadyDevimon ─────────────────────────────────────────────────────
#[test]
fn bt3_088_ladydevimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-088")
        .expect("BT3-088")
        .add_card(make_digimon("BASE", "Devidramon", CardColor::Purple, 4, 4000, 5))
        .add_card(make_digimon("LV6", "Mastemon", CardColor::Purple, 6, 12000, 12))
        .add_card(make_digimon("OPP3", "OppLv3", CardColor::Blue, 3, 2000, 3))
        .add_card(make_option("P-OPT", "PurpleOpt", CardColor::Purple, 0))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-088", "P-OPT"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-088", CardColor::Purple, 4, 3);

    // 1. When Digivolving: Draw 2, then trash 2
    let base = r.place_on_field(0, "BASE", Some(0));
    let trash_before = r.trash_size(0);
    digivolve_from_hand(&mut r, 0, "BT3-088", base);
    let _ = r.auto_resolve();
    assert_eq!(r.trash_size(0), trash_before + 2, "LadyDevimon trashed 2 cards from hand");

    // 2. Inherited: When you use an Option card, delete 1 opponent Lv.3 Digimon
    let _carrier = r.place_field_stack(0, &["BT3-088", "LV6"], false, 0);
    let _opp3 = r.place_on_field(1, "OPP3", Some(0));
    let opt_idx = r.add_to_hand(0, "P-OPT");
    play_option(&mut r, 0, opt_idx);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "LadyDevimon inherited deleted opponent Lv.3");
}

// ─── BT3-089 Boltmon ─────────────────────────────────────────────────────────
#[test]
fn bt3_089_boltmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-089")
        .expect("BT3-089")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-089"])
        .memory(10)
        .start();

    let card = r.compiled_card("BT3-089").expect("BT3-089");
    assert_eq!(card.name, "Boltmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(10));
    assert_eq!(card.dp, Some(12000));
    assert!(card.effects.is_empty());

    r.play(0, 0);
    assert_eq!(r.memory(), 0);
    assert_eq!(r.dp_of(r.perm_handle(0, 0)), Some(12000));
}

// ─── BT3-090 Mastemon ────────────────────────────────────────────────────────
#[test]
fn bt3_090_mastemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-090")
        .expect("BT3-090")
        .add_card(make_digimon("BASE", "LadyDevimon", CardColor::Purple, 5, 8000, 8))
        .add_card(make_digimon("Y-LV4", "Gatomon", CardColor::Yellow, 4, 3000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-090"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-090", CardColor::Purple, 5, 4);
    put_in_trash(&mut r, 0, "Y-LV4");

    let base = r.place_on_field(0, "BASE", Some(0));
    digivolve_from_hand(&mut r, 0, "BT3-090", base);
    let _ = r.auto_resolve();

    assert_eq!(r.security_count(0), 2, "P0 trashed 1 top security");
    assert_eq!(r.security_count(1), 2, "P1 trashed 1 top security");
    assert_eq!(
        r.battle_area_size(0),
        2,
        "Mastemon played yellow Lv.4 Digimon from trash free"
    );
}
