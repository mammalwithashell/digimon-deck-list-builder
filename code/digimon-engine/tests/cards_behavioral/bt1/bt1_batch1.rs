//! Behavioral tests for BT1-001 through BT1-030 (excluding BT1-009 and BT1-014,
//! which have dedicated per-card modules).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
use digimon_engine::action::{
    build_action_mask, encode_attack, encode_digivolve, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId,
};

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

fn add_to_trash(r: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let idx = r.add_to_hand(player, card_id);
    let c = r.game.players[player as usize].hand.remove(idx);
    r.game.players[player as usize].put_in_trash(c);
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

// ─── BT1-001 Yokomon ─────────────────────────────────────────────────────────

#[test]
fn bt1_001_yokomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-001")
        .expect("BT1-001 loads")
        .add_card(make_digimon("RED-ATK", "RedAtk", CardColor::Red, 3, 4000))
        .add_card(make_digimon("OPP-DIGI", "OppDigi", CardColor::Blue, 3, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    // Attacking an opponent's Digimon triggers +1000 DP (4000 -> 5000 vs 4000 => attacker survives).
    let attacker = r.place_field_stack(0, &["BT1-001", "RED-ATK"], false, 0);
    let _opp = r.place_field_stack(1, &["OPP-DIGI"], true, 0);
    act(&mut r, 0, encode_attack(0, 0));
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(0), 1, "attacker boosted to 5000 DP must survive");
    assert_eq!(r.battle_area_size(1), 0, "4000 DP opponent must be deleted");
    assert_eq!(r.effective_dp(attacker), Some(5000), "+1000 DP lasts for the turn");

    // Attacking player security must NOT trigger the Digimon-only DP boost.
    let mut r2 = DebugRunner::builder()
        .dsl_card("BT1-001")
        .expect("BT1-001 loads")
        .add_card(make_digimon("RED-ATK", "RedAtk", CardColor::Red, 3, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r2.game.turn_count = 2;
    let atk2 = r2.place_field_stack(0, &["BT1-001", "RED-ATK"], false, 0);
    act(&mut r2, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r2);
    assert_eq!(r2.effective_dp(atk2), Some(4000), "attacking security must not grant +1000 DP");
}

// ─── BT1-002 Bebydomon ───────────────────────────────────────────────────────

#[test]
fn bt1_002_bebydomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-002")
        .expect("BT1-002 loads")
        .dsl_card("BT1-026")
        .expect("BT1-026 (Breakdramon with Piercing) loads")
        .add_card(make_digimon("PLAIN-RED", "PlainRed", CardColor::Red, 4, 4000))
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let piercer = r.place_stack(0, &["BT1-002", "BT1-026"]);
    let plain = r.place_stack(0, &["BT1-002", "PLAIN-RED"]);
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(piercer),
        Some(13000),
        "Bebydomon must grant +2000 DP while host has <Piercing>"
    );
    assert_eq!(
        r.effective_dp(plain),
        Some(4000),
        "Bebydomon must not boost a host without <Piercing>"
    );
}

// ─── BT1-003 Upamon ──────────────────────────────────────────────────────────

#[test]
fn bt1_003_upamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-003")
        .expect("BT1-003 loads")
        .add_card(make_digimon("BLUE-ATK", "BlueAtk", CardColor::Blue, 3, 4000))
        .add_card(make_digimon("OPP-BARE", "OppBare", CardColor::Red, 3, 2000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _atk = r.place_field_stack(0, &["BT1-003", "BLUE-ATK"], false, 0);
    let _opp = r.place_on_field(1, "OPP-BARE", Some(0));
    let hand_before = r.hand_size(0);

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "Upamon must draw 1 when opponent has a Digimon with no digivolution cards"
    );
}

// ─── BT1-004 Wanyamon ────────────────────────────────────────────────────────

#[test]
fn bt1_004_wanyamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-004")
        .expect("BT1-004 loads")
        .add_card(make_digimon("BLUE-HOST", "BlueHost", CardColor::Blue, 3, 4000))
        .add_card(make_digimon("OPP-BARE", "OppBare", CardColor::Red, 3, 2000))
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let host = r.place_stack(0, &["BT1-004", "BLUE-HOST"]);
    r.place_on_field(1, "OPP-BARE", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(4000), "1 sourceless opp Digimon is below threshold 2");

    r.place_on_field(1, "OPP-BARE", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(6000), "2 sourceless opp Digimon grants +2000 DP");
}

// ─── BT1-005 Kyaromon ────────────────────────────────────────────────────────

#[test]
fn bt1_005_kyaromon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-005")
        .expect("BT1-005 loads")
        .add_card(make_digimon("YEL-HOST", "YelHost", CardColor::Yellow, 3, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .security(0, &["FILLER"; 6])
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let host = r.place_stack(0, &["BT1-005", "YEL-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(6000), "6 security grants +2000 DP");

    r.game.players[0].security.pop();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(4000), "5 security removes +2000 DP");
}

// ─── BT1-006 Cupimon ─────────────────────────────────────────────────────────

#[test]
fn bt1_006_cupimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-006")
        .expect("BT1-006 loads")
        .add_card(make_digimon("YEL-ATK", "YelAtk", CardColor::Yellow, 3, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _atk = r.place_field_stack(0, &["BT1-006", "YEL-ATK"], false, 0);
    let hand_before = r.hand_size(0);
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "Cupimon must draw 1 when attacking with 5+ security"
    );
}

// ─── BT1-007 Tanemon ─────────────────────────────────────────────────────────

#[test]
fn bt1_007_tanemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-007")
        .expect("BT1-007 loads")
        .add_card(make_digimon("GRN-LV3", "GrnLv3", CardColor::Green, 3, 3000))
        .add_card(make_digimon("GRN-LV4", "GrnLv4", CardColor::Green, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Green, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "GRN-LV4", 3, 3, 1); // Green = 3, Lv3 -> cost 1
    r.game.turn_count = 2;

    let stack = r.place_field_stack(0, &["BT1-007", "GRN-LV3"], false, 0);
    let hand_idx = r.add_to_hand(0, "GRN-LV4");
    act(&mut r, 0, encode_digivolve(hand_idx as u16, 0));
    resolve_first_pending(&mut r);

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(
        r.effective_dp(stack),
        Some(6000),
        "Tanemon must grant +1000 DP when attacking after digivolving this turn"
    );
}

// ─── BT1-008 Frimon ──────────────────────────────────────────────────────────

#[test]
fn bt1_008_frimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-008")
        .expect("BT1-008 loads")
        .add_card(make_digimon("GRN-HOST", "GrnHost", CardColor::Green, 3, 4000))
        .add_card(make_digimon("OPP-DIGI", "OppDigi", CardColor::Red, 3, 2000))
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let host = r.place_stack(0, &["BT1-008", "GRN-HOST"]);
    r.place_field_stack(1, &["OPP-DIGI"], true, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(4000), "only 1 suspended opp Digimon");

    r.place_field_stack(1, &["OPP-DIGI"], true, 0);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(6000), "2 suspended opp Digimon grants +2000 DP");
}

// ─── BT1-010 Agumon ──────────────────────────────────────────────────────────

#[test]
fn bt1_010_agumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-010")
        .expect("BT1-010 loads")
        .add_card(make_tamer("TEST-TAMER", "TestTamer", CardColor::Red))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 2000))
        // Last element is top of deck.
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "TEST-TAMER"])
        .memory(5)
        .start();

    let h = r.add_to_hand(0, "BT1-010");
    r.play(0, h).expect("BT1-010 plays");
    assert!(r.pending_selection().is_some(), "reveal bucket must prompt for Tamer pick");
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 1, "1 Tamer card added to hand from top 5 reveal");
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "TEST-TAMER"
    );
    assert_eq!(r.deck_size(0), 5, "remaining 4 revealed + 1 unrevealed stay in deck");
}

// ─── BT1-011 Agumon Expert ───────────────────────────────────────────────────

#[test]
fn bt1_011_agumon_expert() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-011")
        .expect("BT1-011 loads")
        .dsl_card("BT1-010")
        .expect("BT1-010 (Agumon) loads")
        .add_card(make_digimon("OTHER", "OtherMon", CardColor::Red, 3, 2000))
        .memory(5)
        .start();

    add_to_trash(&mut r, 0, "BT1-010");
    add_to_trash(&mut r, 0, "OTHER");

    let h = r.add_to_hand(0, "BT1-011");
    r.play(0, h).expect("BT1-011 plays");
    assert!(r.pending_selection().is_some(), "must prompt to recover [Agumon] from trash");
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 1);
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "BT1-010",
        "recovered Agumon from trash"
    );
    assert_eq!(r.trash_size(0), 1, "non-Agumon remains in trash");
}

// ─── BT1-012 Biyomon ─────────────────────────────────────────────────────────

#[test]
fn bt1_012_biyomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-012")
        .expect("BT1-012 loads")
        .dsl_card("BT1-072")
        .expect("BT1-072 (Woodmon, 6000 DP <Blocker>) loads")
        .add_card(make_digimon("RED-ATK", "RedAtk", CardColor::Red, 4, 5000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let atk = r.place_field_stack(0, &["BT1-012", "RED-ATK"], false, 0);
    let _blk = r.place_on_field(1, "BT1-072", Some(0));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert_eq!(r.current_phase(), GamePhase::BlockTiming, "blocker window must open");
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(0), 1, "boosted 7000 DP attacker beats 6000 DP blocker");
    assert_eq!(r.battle_area_size(1), 0, "6000 DP blocker is deleted");
    assert_eq!(r.effective_dp(atk), Some(7000), "+2000 DP lasts for the turn");
}

// ─── BT1-013 Muchomon (Vanilla) ──────────────────────────────────────────────

#[test]
fn bt1_013_muchomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-013")
        .expect("BT1-013 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-013").expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::Red]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-013");
    r.play(0, h).expect("Muchomon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(5000));
}

// ─── BT1-015 Greymon ─────────────────────────────────────────────────────────

#[test]
fn bt1_015_greymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-015")
        .expect("BT1-015 loads")
        .add_card(make_digimon("RED-HOST", "RedHost", CardColor::Red, 5, 7000))
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let host = r.place_stack(0, &["BT1-015", "RED-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(9000), "Greymon inherited gives +2000 DP on your turn");

    r.end_turn();
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(7000), "Greymon inherited is inactive on opponent's turn");
}

// ─── BT1-016 Tyrannomon ──────────────────────────────────────────────────────

#[test]
fn bt1_016_tyrannomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-016")
        .expect("BT1-016 loads")
        .add_card(make_digimon("BIG-SEC", "BigSec", CardColor::Red, 6, 12000))
        .security(1, &["BIG-SEC", "BIG-SEC"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let tyr = r.place_on_field(0, "BT1-016", Some(0));
    assert!(r.game.has_keyword(tyr, Keyword::Jamming));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(0), 1, "Jamming prevents deletion in security battle");
    assert_eq!(r.security_count(1), 1);
}

// ─── BT1-017 Birdramon ───────────────────────────────────────────────────────

#[test]
fn bt1_017_birdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-017")
        .expect("BT1-017 loads")
        .add_card(make_digimon("ALLY", "Ally", CardColor::Red, 4, 5000))
        .memory(6)
        .start();

    let ally = r.place_on_field(0, "ALLY", Some(0));
    let h = r.add_to_hand(0, "BT1-017");
    r.play(0, h).expect("BT1-017 plays");
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert!(
        r.game.has_keyword(ally, Keyword::SecurityAttackPlus(1))
            || r.game
                .has_keyword(r.perm_handle(0, 1), Keyword::SecurityAttackPlus(1)),
        "selected own Digimon must gain <Security A. +1>"
    );
}

// ─── BT1-018 Flarerizamon ────────────────────────────────────────────────────

#[test]
fn bt1_018_flarerizamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-018")
        .expect("BT1-018 loads")
        .memory(3)
        .start();
    r.game.turn_count = 1;

    let flare = r.place_on_field(0, "BT1-018", Some(0));
    r.game.tick_declarative_effects();
    assert!(
        r.game.has_keyword(flare, Keyword::SecurityAttackPlus(1)),
        "at 3 memory Flarerizamon gains <Security A. +1>"
    );

    r.game.set_memory(2);
    r.game.tick_declarative_effects();
    assert!(
        !r.game.has_keyword(flare, Keyword::SecurityAttackPlus(1)),
        "at 2 memory Flarerizamon loses <Security A. +1>"
    );
}

// ─── BT1-019 DarkTyrannomon (Vanilla) ────────────────────────────────────────

#[test]
fn bt1_019_darktyrannomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-019")
        .expect("BT1-019 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-019").unwrap();
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.color, vec![CompiledColor::Red]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-019");
    r.play(0, h).expect("DarkTyrannomon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT1-020 Groundramon (Vanilla) ───────────────────────────────────────────

#[test]
fn bt1_020_groundramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-020")
        .expect("BT1-020 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-020").unwrap();
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.color, vec![CompiledColor::Red]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-020");
    r.play(0, h).expect("Groundramon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT1-021 MetalGreymon ────────────────────────────────────────────────────

#[test]
fn bt1_021_metalgreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-021")
        .expect("BT1-021 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(2)
        .start();
    r.game.turn_count = 2;

    let _mg = r.place_on_field(0, "BT1-021", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 5, "When Attacking must gain 3 memory (2 -> 5)");
    r.game.fire_end_of_your_turn(0);
    assert_eq!(r.memory(), 2, "End of turn delayed trigger must lose 3 memory (5 -> 2)");
}

// ─── BT1-022 Garudamon ───────────────────────────────────────────────────────

#[test]
fn bt1_022_garudamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-022")
        .expect("BT1-022 loads")
        .dsl_card("BT1-072")
        .expect("BT1-072 (Woodmon, <Blocker>) loads")
        .add_card(make_digimon("RED-LV6", "RedLv6", CardColor::Red, 6, 11000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let garuda = r.place_on_field(0, "BT1-022", Some(0));
    assert!(r.game.has_keyword(garuda, Keyword::Piercing));

    let _host = r.place_field_stack(0, &["BT1-022", "RED-LV6"], false, 0);
    let _blk = r.place_on_field(1, "BT1-072", Some(0));
    let hand_before = r.hand_size(0);

    act(&mut r, 0, encode_attack(1, SECURITY_TARGET));
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    resolve_first_pending(&mut r);

    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "Garudamon inherited must draw 1 when blocked"
    );
}

// ─── BT1-023 SkullGreymon ────────────────────────────────────────────────────

#[test]
fn bt1_023_skullgreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-023")
        .expect("BT1-023 loads")
        .dsl_card("BT1-072")
        .expect("BT1-072 (Woodmon, <Blocker>) loads")
        .add_card(make_digimon("OPP-PLAIN", "OppPlain", CardColor::Red, 4, 5000))
        .memory(8)
        .start();

    let _blk = r.place_on_field(1, "BT1-072", Some(0));
    let _plain = r.place_on_field(1, "OPP-PLAIN", Some(0));

    let h = r.add_to_hand(0, "BT1-023");
    r.play(0, h).expect("BT1-023 plays");
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(1), 1, "only Blocker deleted, plain Digimon survives");
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "OPP-PLAIN"
    );
}

// ─── BT1-024 MetalTyrannomon (Vanilla) ───────────────────────────────────────

#[test]
fn bt1_024_metaltyrannomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-024")
        .expect("BT1-024 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-024").unwrap();
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(10000));
    assert_eq!(card.color, vec![CompiledColor::Red]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-024");
    r.play(0, h).expect("MetalTyrannomon plays");
    assert_eq!(r.memory(), 1);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(10000));
}

// ─── BT1-025 WarGreymon ──────────────────────────────────────────────────────

#[test]
fn bt1_025_wargreymon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-025")
        .expect("BT1-025 loads")
        .add_card(make_digimon("RED-LV5", "RedLv5", CardColor::Red, 5, 7000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT1-025", 0, 5, 3); // Red = 0, Lv5 -> cost 3
    r.game.turn_count = 2;

    let perm = r.place_on_field(0, "RED-LV5", Some(0));
    let h = r.add_to_hand(0, "BT1-025");
    act(&mut r, 0, encode_digivolve(h as u16, 0));
    resolve_first_pending(&mut r);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(perm, Keyword::SecurityAttackPlus(1)),
        "When Digivolving must grant <Security A. +1> for the turn"
    );
    assert!(
        r.game
            .modifiers
            .has(perm, ModifierType::DoesNotActivateOptionSecurityEffects),
        "Your Turn aura must suppress Option [Security] effects"
    );
}

// ─── BT1-026 Breakdramon ─────────────────────────────────────────────────────

#[test]
fn bt1_026_breakdramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-026")
        .expect("BT1-026 loads")
        .add_card(make_digimon("OPP-WEAK", "OppWeak", CardColor::Blue, 3, 3000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Red, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let brk = r.place_on_field(0, "BT1-026", Some(0));
    assert!(r.game.has_keyword(brk, Keyword::Piercing));
    let _opp = r.place_field_stack(1, &["OPP-WEAK"], true, 0);

    act(&mut r, 0, encode_attack(0, 0));
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(1), 0, "opponent Digimon deleted");
    assert_eq!(r.security_count(1), 1, "Piercing checks 1 security after battle win");
}

// ─── BT1-027 Armadillomon (Vanilla) ──────────────────────────────────────────

#[test]
fn bt1_027_armadillomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-027")
        .expect("BT1-027 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-027").unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-027");
    r.play(0, h).expect("Armadillomon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(4000));
}

// ─── BT1-028 Elecmon (Vanilla) ───────────────────────────────────────────────

#[test]
fn bt1_028_elecmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-028")
        .expect("BT1-028 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-028").unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-028");
    r.play(0, h).expect("Elecmon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(3000));
}

// ─── BT1-029 Gabumon ─────────────────────────────────────────────────────────

#[test]
fn bt1_029_gabumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-029")
        .expect("BT1-029 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .memory(5)
        .start();

    let h = r.add_to_hand(0, "BT1-029");
    let deck_before = r.deck_size(0);
    r.play(0, h).expect("BT1-029 plays");
    assert_eq!(r.hand_size(0), 1, "On Play draws 1");
    assert_eq!(r.deck_size(0), deck_before - 1);
}

// ─── BT1-030 Gomamon ─────────────────────────────────────────────────────────

#[test]
fn bt1_030_gomamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-030")
        .expect("BT1-030 loads")
        .add_card(make_digimon("BLUE-HOST", "BlueHost", CardColor::Blue, 4, 5000))
        .memory(2)
        .start();

    let host = r.place_stack(0, &["BT1-030", "BLUE-HOST"]);
    r.game.delete_permanent_with_effects(host);
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 3, "Gomamon inherited On Deletion must gain 1 memory");
}
