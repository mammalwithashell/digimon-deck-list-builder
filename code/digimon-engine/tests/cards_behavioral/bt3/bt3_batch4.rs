#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{encode_digivolve, PASS};
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

// ─── BT3-091 Lilithmon ───────────────────────────────────────────────────────
#[test]
fn bt3_091_lilithmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-091")
        .expect("BT3-091")
        .add_card(make_digimon("BASE", "LadyDevimon", CardColor::Purple, 5, 8000, 8))
        .add_card(make_option("P-OPT", "PurpleOpt", CardColor::Purple, 0))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-091"])
        .memory(6)
        .start();
    set_evo_cost(&mut r, "BT3-091", CardColor::Purple, 5, 4);

    // Put 8 PAD + 2 P-OPT = 10 cards in trash
    for _ in 0..8 {
        put_in_trash(&mut r, 0, "PAD");
    }
    put_in_trash(&mut r, 0, "P-OPT");
    put_in_trash(&mut r, 0, "P-OPT");

    let base = r.place_on_field(0, "BASE", Some(0));
    digivolve_from_hand(&mut r, 0, "BT3-091", base);
    let _ = r.auto_resolve_declaring_max();

    let recovered_opts = r.game.players[0]
        .hand
        .iter()
        .filter(|c| c.card_id(&r.game.card_data) == "P-OPT")
        .count();
    assert_eq!(recovered_opts, 2, "Lilithmon returned 2 purple Options from trash");

    // Your Turn OPT: when you use an Option card, gain 2 memory
    let mem_before = r.memory();
    let opt_idx = hand_index_of(&r, 0, "P-OPT");
    play_option(&mut r, 0, opt_idx);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before + 2, "Lilithmon gained 2 memory when using an Option");
}

// ─── BT3-092 MaloMyotismon ───────────────────────────────────────────────────
#[test]
fn bt3_092_malomyotismon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-092")
        .expect("BT3-092")
        .add_card(make_digimon("ALLY", "Rookie", CardColor::Purple, 3, 2000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(3)
        .start();

    let malo = r.place_on_field(0, "BT3-092", Some(0));
    assert!(r.game.has_keyword(malo, Keyword::Piercing), "MaloMyotismon has Piercing");

    let ally = r.place_on_field(0, "ALLY", Some(0));
    let mem_before = r.memory();
    r.game.delete_permanent_with_effects(ally);
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        mem_before + 1,
        "MaloMyotismon gained 1 memory when another Digimon was deleted"
    );
}

// ─── BT3-094 Ken Ichijoji ────────────────────────────────────────────────────
#[test]
fn bt3_094_ken_ichijoji() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-094")
        .expect("BT3-094")
        .add_card(make_digimon("GRN", "Stingmon", CardColor::Green, 4, 6000, 5))
        .add_card(make_digimon("OPP", "SmallOpp", CardColor::Red, 3, 2000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(1)
        .start();

    let ken = r.place_on_field(0, "BT3-094", Some(0));
    let grn = r.place_on_field(0, "GRN", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.suspend(opp);

    // Start of Your Turn: if memory <= 2, set to 3
    r.end_turn(); // P1's turn
    r.game.set_memory(-1); // P0 will start with 1 memory
    r.end_turn(); // Back to P0's turn -> Start of Your Turn sets memory to 3
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3, "Ken Ichijoji set memory to 3 at Start of Your Turn");

    // When green Digimon deletes opponent Digimon in battle and survives, suspend Ken for +1 memory
    r.game.turn_count = 4;
    r.attack_digimon(grn, opp, false);
    if r.pending_is_optional() {
        r.accept_optional_trigger().expect("accept Ken trigger");
    }
    let _ = r.auto_resolve();

    assert!(r.game.players[0].battle_area[ken.index as usize].is_suspended);
    assert_eq!(r.memory(), 4, "Ken Ichijoji suspended to gain +1 memory");
}

// ─── BT3-095 Joe Kido ────────────────────────────────────────────────────────
#[test]
fn bt3_095_joe_kido() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-095")
        .expect("BT3-095")
        .dsl_card("BT3-070")
        .expect("BT3-070 Etemon Blocker")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .memory(1)
        .start();

    let _joe = r.place_on_field(0, "BT3-095", Some(0));
    let _blocker = r.place_on_field(0, "BT3-070", Some(0));

    r.end_turn(); // P1's turn
    r.game.set_memory(-1); // P0 starts next turn with 1 memory
    r.end_turn(); // Back to P0's turn -> Joe Kido gains +1 memory (1 -> 2)
    let _ = r.auto_resolve();

    assert_eq!(
        r.memory(),
        2,
        "Joe Kido gained 1 memory at Start of Your Turn with a Blocker in play"
    );
}

// ─── BT3-097 A Delicate Plan ─────────────────────────────────────────────────
#[test]
fn bt3_097_a_delicate_plan() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-097")
        .expect("BT3-097")
        .dsl_card("BT3-098")
        .expect("BT3-098 Plasma Stake deletes 13000+ DP in security")
        .add_card(make_digimon("RED-BIG", "BigRed", CardColor::Red, 7, 14000, 14))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD", "BT3-098"])
        .hand(0, &["BT3-097"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_on_field(0, "RED-BIG", Some(0));
    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    // Attack into Plasma Stake security -> Option [Security] effect must NOT activate!
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(0),
        1,
        "A Delicate Plan suppressed Plasma Stake's [Security] deletion"
    );
}

// ─── BT3-098 Plasma Stake ────────────────────────────────────────────────────
#[test]
fn bt3_098_plasma_stake() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-098")
        .expect("BT3-098")
        .add_card(make_digimon("RED", "Agumon", CardColor::Red, 3, 2000, 3))
        .add_card(make_digimon("OPP-13K", "BigOpp", CardColor::Blue, 7, 13000, 14))
        .add_card(make_digimon("OPP-12K", "SmallerOpp", CardColor::Blue, 6, 12000, 12))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-098"])
        .memory(6)
        .start();

    let _red = r.place_on_field(0, "RED", Some(0));
    let _big = r.place_on_field(1, "OPP-13K", Some(0));
    let _smaller = r.place_on_field(1, "OPP-12K", Some(0));

    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 1, "Plasma Stake deleted 13000+ DP opponent only");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "OPP-12K"
    );
}

// ─── BT3-099 We Have to Stop Fighting! ───────────────────────────────────────
#[test]
fn bt3_099_we_have_to_stop_fighting() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-099")
        .expect("BT3-099")
        .add_card(make_digimon("BLUE", "Veemon", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("OPP-BIG", "BigOpp", CardColor::Red, 6, 12000, 12))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-099"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let blue = r.place_on_field(0, "BLUE", Some(0));
    let opp = r.place_on_field(1, "OPP-BIG", Some(0));
    r.game.suspend(opp);

    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    // 2000 DP Veemon attacks 12000 DP opponent -> neither deleted in battle!
    r.attack_digimon(blue, opp, false);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 1, "Veemon survived losing battle");
    assert_eq!(r.battle_area_size(1), 1, "Opponent survived battle");
}

// ─── BT3-100 Death Parade Blaster ────────────────────────────────────────────
#[test]
fn bt3_100_death_parade_blaster() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-100")
        .expect("BT3-100")
        .add_card(make_digimon("BLUE", "Veemon", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("GRN", "Wormmon", CardColor::Green, 3, 2000, 3))
        .add_card(make_digimon("O3", "Lv3", CardColor::Red, 3, 2000, 3))
        .add_card(make_digimon("O4", "Lv4", CardColor::Red, 4, 4000, 4))
        .add_card(make_digimon("O5", "Lv5", CardColor::Red, 5, 7000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-100"])
        .memory(5)
        .start();

    let _blue = r.place_on_field(0, "BLUE", Some(0));
    let _grn = r.place_on_field(0, "GRN", Some(0));
    let opp = r.place_field_stack(1, &["O3", "O4", "O5"], false, 0);

    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize].stack_size(),
        1,
        "Death Parade Blaster trashed 2 bottom digivolution cards"
    );
    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "With a green Digimon in play, suspended opponent Digimon with no digivolution cards"
    );
}

// ─── BT3-101 Bifrost ─────────────────────────────────────────────────────────
#[test]
fn bt3_101_bifrost() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-101")
        .expect("BT3-101")
        .add_card(make_digimon("YEL", "Patamon", CardColor::Yellow, 3, 2000, 3))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-101"])
        .memory(5)
        .start();

    let _yel = r.place_on_field(0, "YEL", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));

    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.effective_dp(opp), Some(5000), "Bifrost gave -3000 DP");
    assert_eq!(
        r.modifiers().sum(opp, ModifierType::SecurityAttackChange),
        -1,
        "Bifrost gave Security A. -1"
    );
}

// ─── BT3-102 Code Cracking ───────────────────────────────────────────────────
#[test]
fn bt3_102_code_cracking() {
    // Branch 0: opponent chooses to trash their top security card
    let mut r0 = DebugRunner::builder()
        .dsl_card("BT3-102")
        .expect("BT3-102")
        .add_card(make_digimon("YEL", "Patamon", CardColor::Yellow, 3, 2000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-102"])
        .memory(5)
        .start();
    let _y0 = r0.place_on_field(0, "YEL", Some(0));
    play_option(&mut r0, 0, 0);
    r0.execute_branch(0).expect("opponent chooses to trash top security");
    let _ = r0.auto_resolve();
    assert_eq!(r0.security_count(1), 2, "Opponent trashed 1 security");
    assert_eq!(r0.security_count(0), 2, "P0 did not recover");

    // Branch 1: opponent declines -> P0 performs Recovery +1 (Deck)
    let mut r1 = DebugRunner::builder()
        .dsl_card("BT3-102")
        .expect("BT3-102")
        .add_card(make_digimon("YEL", "Patamon", CardColor::Yellow, 3, 2000, 3))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-102"])
        .memory(5)
        .start();
    let _y1 = r1.place_on_field(0, "YEL", Some(0));
    play_option(&mut r1, 0, 0);
    r1.execute_branch(1).expect("opponent declines to trash");
    let _ = r1.auto_resolve();
    assert_eq!(r1.security_count(1), 3, "Opponent kept security");
    assert_eq!(r1.security_count(0), 3, "P0 recovered +1 security");
}

// ─── BT3-104 Positron Laser ──────────────────────────────────────────────────
#[test]
fn bt3_104_positron_laser() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-104")
        .expect("BT3-104")
        .add_card(make_digimon("GRN", "Wormmon", CardColor::Green, 3, 2000, 3))
        .add_card(make_digimon("BLUE", "Veemon", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("OPP1", "Opp1", CardColor::Red, 4, 5000, 4))
        .add_card(make_digimon("OPP2", "Opp2", CardColor::Red, 4, 5000, 4))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-104"])
        .memory(7)
        .start();

    let _grn = r.place_on_field(0, "GRN", Some(0));
    let _blue = r.place_on_field(0, "BLUE", Some(0));
    let opp1 = r.place_on_field(1, "OPP1", Some(0));
    let opp2 = r.place_on_field(1, "OPP2", Some(0));
    r.game.suspend(opp2);

    let opp_hand_before = r.hand_size(1);
    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve_declaring_max();

    // opp2 was suspended and bounced to P1's hand because P0 has a Blue Digimon
    assert_eq!(r.battle_area_size(1), 1, "Suspended opponent bounced to hand");
    assert_eq!(r.hand_size(1), opp_hand_before + 1);
    // opp1 cannot attack or block
    assert!(
        r.modifiers().has(opp1, ModifierType::CannotAttack),
        "Opp1 has CannotAttack"
    );
    assert!(
        r.modifiers().has(opp1, ModifierType::CannotBlock),
        "Opp1 has CannotBlock"
    );
}

// ─── BT3-106 Beast Cyclone ───────────────────────────────────────────────────
#[test]
fn bt3_106_beast_cyclone() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-106")
        .expect("BT3-106")
        .dsl_card("BT3-070")
        .expect("BT3-070 Etemon Blocker")
        .dsl_card("BT3-071")
        .expect("BT3-071 MetalMamemon Reboot")
        .add_card(make_digimon("PLAIN", "Tankmon", CardColor::Black, 4, 6000, 6))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-106"])
        .memory(5)
        .start();

    let blk = r.place_on_field(0, "BT3-070", Some(0));
    let rbt = r.place_on_field(0, "BT3-071", Some(0));
    let plain = r.place_on_field(0, "PLAIN", Some(0));

    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();
    r.game.tick_declarative_effects();

    assert_eq!(r.modifiers().sum(blk, ModifierType::SecurityAttackChange), 1, "Blocker gets SA +1");
    assert_eq!(r.modifiers().sum(rbt, ModifierType::SecurityAttackChange), 1, "Reboot gets SA +1");
    assert_eq!(r.modifiers().sum(plain, ModifierType::SecurityAttackChange), 0, "Plain Digimon gets 0");
}

// ─── BT3-107 Looking Back on the Good Times ──────────────────────────────────
#[test]
fn bt3_107_looking_back_on_the_good_times() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-107")
        .expect("BT3-107")
        .add_card(make_digimon("BLK", "Tankmon", CardColor::Black, 4, 6000, 6))
        .add_card(make_digimon("O3", "SmallLv3", CardColor::Blue, 3, 2000, 3))
        .add_card(make_digimon("O4", "BigLv4", CardColor::Blue, 4, 6000, 6))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-107"])
        .memory(5)
        .start();

    let _blk = r.place_on_field(0, "BLK", Some(0));
    let _opp = r.place_field_stack(1, &["O3", "O4"], false, 0);

    // De-Digivolve 1 trashes O4 -> reveals O3 (play cost 3 <= 4) -> deletes it!
    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        0,
        "De-Digivolve 1 exposed play cost 3 Digimon and deleted it"
    );
}

// ─── BT3-108 Dark Despair ────────────────────────────────────────────────────
#[test]
fn bt3_108_dark_despair() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-108")
        .expect("BT3-108")
        .add_card(make_digimon("PUR", "Candlemon", CardColor::Purple, 3, 3000, 2))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-108"])
        .memory(5)
        .start();

    let pur = r.place_on_field(0, "PUR", Some(0));
    assert!(!r.game.has_keyword(pur, Keyword::Retaliation));

    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    assert!(
        r.game.has_keyword(pur, Keyword::Retaliation),
        "Dark Despair granted Retaliation"
    );
}

// ─── BT3-110 Necrophobia ─────────────────────────────────────────────────────
#[test]
fn bt3_110_necrophobia() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-110")
        .expect("BT3-110")
        .add_card(make_digimon("PUR", "Candlemon", CardColor::Purple, 3, 3000, 2))
        .add_card(make_digimon("P-LV5", "SkullMeramon", CardColor::Purple, 5, 6000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .hand(0, &["BT3-110"])
        .memory(6)
        .start();
    put_in_trash(&mut r, 0, "P-LV5");

    let _pur = r.place_on_field(0, "PUR", Some(0));
    play_option(&mut r, 0, 0);
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(0), 2, "Necrophobia played Lv.5 purple Digimon from trash");
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "P-LV5"
    );
}

// ─── BT3-111 Imperialdramon Dragon Mode (Green) ──────────────────────────────
#[test]
fn bt3_111_imperialdramon_dragon_mode_green() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-111")
        .expect("BT3-111")
        .add_card(make_digimon("DINO", "Dinobeemon", CardColor::Green, 5, 7000, 8))
        .add_card(make_digimon("OPP", "Opp", CardColor::Blue, 4, 5000, 5))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-111"])
        .memory(5)
        .start();
    r.game.turn_count = 2;
    set_evo_cost(&mut r, "BT3-111", CardColor::Green, 5, 5);

    let dino = r.place_on_field(0, "DINO", Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.game.suspend(opp);

    // Digivolve over Dinobeemon: cost 5 - 2 = 3 -> memory 5 - 3 = 2
    digivolve_from_hand(&mut r, 0, "BT3-111", dino);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 2, "Digivolving over Dinobeemon reduced cost by 2");
    assert!(r.game.has_keyword(dino, Keyword::Piercing));

    // Attack and delete opp in battle -> survives -> unsuspends itself
    r.attack_digimon(dino, opp, false);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "Opponent deleted in battle");
    assert!(
        !r.game.players[0].battle_area[dino.index as usize].is_suspended,
        "Imperialdramon unsuspended after deleting opponent Digimon in battle"
    );
}

// ─── BT3-112 Omnimon Alter-S ─────────────────────────────────────────────────
#[test]
fn bt3_112_omnimon_alter_s() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT3-112")
        .expect("BT3-112")
        .dsl_card("BT3-070")
        .expect("BT3-070 Etemon Blocker")
        .add_card(make_digimon("LV6", "BlitzGreymon", CardColor::Red, 6, 12000, 12))
        .add_card(make_digimon("O3", "SmallLv3", CardColor::Blue, 3, 3000, 3))
        .add_card(make_digimon("O5", "MidLv5", CardColor::Blue, 5, 8000, 7))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 6])
        .deck(1, &["PAD"; 6])
        .security(1, &["PAD"; 3])
        .hand(0, &["BT3-112"])
        .memory(8)
        .start();
    r.game.turn_count = 2;
    set_evo_cost(&mut r, "BT3-112", CardColor::Red, 6, 6);

    let base = r.place_on_field(0, "LV6", Some(0));
    let _opp_stack = r.place_field_stack(1, &["O3", "O5"], false, 0);

    // When Digivolving: De-Digivolve 1 exposes O3 (3000 DP <= 5000) -> deleted!
    digivolve_from_hand(&mut r, 0, "BT3-112", base);
    let _ = r.auto_resolve();
    assert_eq!(
        r.battle_area_size(1),
        0,
        "Omnimon Alter-S De-Digivolved O5->O3 and deleted <=5000 DP"
    );

    // Place an opponent Blocker, then attack: return Lv.6 source to hand -> CannotBeBlocked!
    let _opp_blocker = r.place_on_field(1, "BT3-070", Some(0));
    let hand_before = r.hand_size(0);
    r.attack_player(base, 1, false);
    let _ = r.auto_resolve();

    assert_eq!(r.hand_size(0), hand_before + 1, "Returned Lv.6 source to hand");
    assert!(
        r.modifiers().has(base, ModifierType::CannotBeBlocked),
        "Omnimon Alter-S gained CannotBeBlocked for the turn"
    );
}
