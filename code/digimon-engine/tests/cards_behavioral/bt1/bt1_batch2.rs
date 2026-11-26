//! Behavioral tests for BT1-031 through BT1-060 (30 cards).

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

// ─── BT1-031 Monmon ──────────────────────────────────────────────────────────

#[test]
fn bt1_031_monmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-031")
        .expect("BT1-031 loads")
        .add_card(make_digimon("OPP-ATK", "OppAtk", CardColor::Red, 3, 2000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let mon = r.place_on_field(1, "BT1-031", Some(0));
    assert!(r.game.has_keyword(mon, Keyword::Blocker));

    let _atk = r.place_on_field(0, "OPP-ATK", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert_eq!(r.current_phase(), GamePhase::BlockTiming, "Monmon must open BlockTiming");
}

// ─── BT1-032 Frigimon ────────────────────────────────────────────────────────

#[test]
fn bt1_032_frigimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-032")
        .expect("BT1-032 loads")
        .add_card(make_digimon("BIG-SEC", "BigSec", CardColor::Red, 6, 12000))
        .security(1, &["BIG-SEC", "BIG-SEC"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let frig = r.place_on_field(0, "BT1-032", Some(0));
    assert!(r.game.has_keyword(frig, Keyword::Jamming));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert_eq!(r.battle_area_size(0), 1, "Jamming survives 12000 DP security");
}

// ─── BT1-033 Dolphmon ────────────────────────────────────────────────────────

#[test]
fn bt1_033_dolphmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-033")
        .expect("BT1-033 loads")
        .add_card(make_digimon("BLUE-HOST", "BlueHost", CardColor::Blue, 5, 6000))
        .add_card(make_digimon("OPP-BARE", "OppBare", CardColor::Red, 3, 2000))
        .memory(5)
        .start();
    r.game.turn_count = 1;

    let host = r.place_stack(0, &["BT1-033", "BLUE-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(6000), "no opponent sourceless Digimon");

    r.place_on_field(1, "OPP-BARE", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(7000), "Dolphmon grants +1000 DP");
}

// ─── BT1-034 Ikkakumon ───────────────────────────────────────────────────────

#[test]
fn bt1_034_ikkakumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-034")
        .expect("BT1-034 loads")
        .dsl_card("BT1-072")
        .expect("BT1-072 (Woodmon, <Blocker>) loads")
        .add_card(make_digimon("BLUE-HOST", "BlueHost", CardColor::Blue, 5, 7000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let host = r.place_field_stack(0, &["BT1-034", "BLUE-HOST"], false, 0);
    let _blk = r.place_on_field(1, "BT1-072", Some(0));
    r.game.tick_declarative_effects();

    assert!(r
        .game
        .modifiers
        .has(host, ModifierType::CannotBeBlockedByNoSources));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert_ne!(
        r.current_phase(),
        GamePhase::BlockTiming,
        "sourceless blocker cannot block Ikkakumon host"
    );
    assert_eq!(r.security_count(1), 1, "attack proceeds directly to security");
}

// ─── BT1-035 Leomon ──────────────────────────────────────────────────────────

#[test]
fn bt1_035_leomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-035")
        .expect("BT1-035 loads")
        .memory(1)
        .start();

    let leo = r.place_on_field(0, "BT1-035", Some(0));
    r.game.delete_permanent_with_effects(leo);
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 3, "Leomon On Deletion must gain 2 memory");
}

// ─── BT1-036 Garurumon ───────────────────────────────────────────────────────

#[test]
fn bt1_036_garurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-036")
        .expect("BT1-036 loads")
        .add_card(make_digimon("ALLY", "Ally", CardColor::Blue, 3, 3000))
        .memory(8)
        .start();

    let ally = r.place_field_stack(0, &["ALLY"], true, 0);
    assert!(r.game.players[0].battle_area[0].is_suspended);

    let h = r.add_to_hand(0, "BT1-036");
    r.play(0, h).expect("BT1-036 plays");
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "Garurumon On Play must unsuspend selected own Digimon"
    );
    let _ = ally;
}

// ─── BT1-037 Gorillamon (Vanilla) ────────────────────────────────────────────

#[test]
fn bt1_037_gorillamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-037")
        .expect("BT1-037 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-037").unwrap();
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-037");
    r.play(0, h).expect("Gorillamon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT1-038 Monzaemon (Vanilla) ─────────────────────────────────────────────

#[test]
fn bt1_038_monzaemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-038")
        .expect("BT1-038 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-038").unwrap();
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-038");
    r.play(0, h).expect("Monzaemon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT1-039 Cerberusmon ─────────────────────────────────────────────────────

#[test]
fn bt1_039_cerberusmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-039")
        .expect("BT1-039 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .hand(0, &["FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _cerb = r.place_on_field(0, "BT1-039", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some(), "optional trash-3-to-unsuspend must prompt");
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 0, "3 cards trashed from hand");
    assert_eq!(r.trash_size(0), 3);
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "Cerberusmon must unsuspend after trashing 3 cards"
    );
}

// ─── BT1-040 WereGarurumon ───────────────────────────────────────────────────

#[test]
fn bt1_040_weregarurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-040")
        .expect("BT1-040 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(2)
        .start();
    r.game.turn_count = 2;

    let _wg = r.place_on_field(0, "BT1-040", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 5, "When Attacking gains 3 memory");
    r.game.fire_end_of_your_turn(0);
    assert_eq!(r.memory(), 2, "End of turn delayed trigger loses 3 memory");
}

// ─── BT1-041 Zudomon ─────────────────────────────────────────────────────────

#[test]
fn bt1_041_zudomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-041")
        .expect("BT1-041 loads")
        .add_card(make_digimon("BLUE-LV6", "BlueLv6", CardColor::Blue, 6, 11000))
        .add_card(make_digimon("OPP-BARE", "OppBare", CardColor::Red, 3, 2000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(8)
        .start();
    r.game.turn_count = 2;

    // On Play: Draw 2
    let h = r.add_to_hand(0, "BT1-041");
    r.play(0, h).expect("Zudomon plays");
    assert_eq!(r.hand_size(0), 2, "Zudomon On Play draws 2");

    // Inherited When Attacking: +1 memory if opponent has sourceless Digimon
    let _host = r.place_field_stack(0, &["BT1-041", "BLUE-LV6"], false, 0);
    let _opp = r.place_on_field(1, "OPP-BARE", Some(0));
    let mem_before = r.memory();
    act(&mut r, 0, encode_attack(1, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert_eq!(
        r.memory(),
        mem_before + 1,
        "Zudomon inherited gains 1 memory when opponent has sourceless Digimon"
    );
}

// ─── BT1-042 LoaderLeomon (Vanilla) ──────────────────────────────────────────

#[test]
fn bt1_042_loaderleomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-042")
        .expect("BT1-042 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-042").unwrap();
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(10000));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-042");
    r.play(0, h).expect("LoaderLeomon plays");
    assert_eq!(r.memory(), 1);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(10000));
}

// ─── BT1-043 SaberLeomon ─────────────────────────────────────────────────────

#[test]
fn bt1_043_saberleomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-043")
        .expect("BT1-043 loads")
        .add_card(make_digimon("BLUE-LV5", "BlueLv5", CardColor::Blue, 5, 7000))
        .add_card(make_digimon("SRC", "Src", CardColor::Red, 3, 2000))
        .add_card(make_digimon("OPP-TOP", "OppTop", CardColor::Red, 6, 10000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    set_evo_cost(&mut r, "BT1-043", 1, 5, 3); // Blue = 1, Lv5 -> cost 3
    r.game.turn_count = 2;

    let _host = r.place_on_field(0, "BLUE-LV5", Some(0));
    let _opp = r.place_stack(1, &["SRC", "SRC", "SRC", "SRC", "OPP-TOP"]);
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 5);

    let h = r.add_to_hand(0, "BT1-043");
    act(&mut r, 0, encode_digivolve(h as u16, 0));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(
        r.game.players[1].battle_area[0].card_sources.len(),
        1,
        "SaberLeomon must trash 4 bottom digivolution cards"
    );
    assert_eq!(r.trash_size(1), 4);
}

// ─── BT1-044 MetalGarurumon ──────────────────────────────────────────────────

#[test]
fn bt1_044_metalgarurumon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-044")
        .expect("BT1-044 loads")
        .add_card(make_digimon("LV4-SRC", "Lv4Src", CardColor::Blue, 4, 4000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Blue, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _mg = r.place_field_stack(0, &["LV4-SRC", "BT1-044"], false, 0);
    let mem_before = r.memory();

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some(), "must prompt to pick Lv4 or lower source");
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(0), 2, "Lv4 source played as a new Digimon");
    assert_eq!(r.memory(), mem_before, "played without paying memory cost");
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}

// ─── BT1-045 Tsukaimon (Vanilla) ─────────────────────────────────────────────

#[test]
fn bt1_045_tsukaimon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-045")
        .expect("BT1-045 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-045").unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(2));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-045");
    r.play(0, h).expect("Tsukaimon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(3000));
}

// ─── BT1-046 Kudamon ─────────────────────────────────────────────────────────

#[test]
fn bt1_046_kudamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-046")
        .expect("BT1-046 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .hand(0, &["FILLER", "FILLER"])
        .deck(0, &["FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let _kud = r.place_on_field(0, "BT1-046", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 3, "Kudamon draws 1 when attacking with <= 4 cards in hand");
}

// ─── BT1-047 Tinkermon (Vanilla) ─────────────────────────────────────────────

#[test]
fn bt1_047_tinkermon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-047")
        .expect("BT1-047 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-047").unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-047");
    r.play(0, h).expect("Tinkermon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(3000));
}

// ─── BT1-048 Patamon ─────────────────────────────────────────────────────────

#[test]
fn bt1_048_patamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-048")
        .expect("BT1-048 loads")
        .add_card(make_tamer("YEL-TAMER", "YelTamer", CardColor::Yellow))
        .add_card(make_tamer("RED-TAMER", "RedTamer", CardColor::Red))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        // Last 4 elements are top 4 of deck (contains both YEL-TAMERs).
        .deck(0, &["FILLER", "FILLER", "RED-TAMER", "YEL-TAMER", "YEL-TAMER"])
        .memory(5)
        .start();

    let h = r.add_to_hand(0, "BT1-048");
    r.play(0, h).expect("Patamon plays");
    resolve_first_pending(&mut r);

    assert_eq!(r.hand_size(0), 2, "Patamon adds all yellow Tamers among top 4");
    assert_eq!(r.deck_size(0), 3, "non-yellow Tamer + filler bottomed + 1 unrevealed");
}

// ─── BT1-049 Labramon ────────────────────────────────────────────────────────

#[test]
fn bt1_049_labramon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-049")
        .expect("BT1-049 loads")
        .dsl_card("BT1-055")
        .expect("BT1-055 (Angemon -3000 DP) loads")
        .add_card(make_digimon("YEL-HOST", "YelHost", CardColor::Yellow, 4, 5000))
        .add_card(make_digimon("OPP-WEAK", "OppWeak", CardColor::Red, 3, 2000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .memory(8)
        .start();
    r.game.turn_count = 1;

    let _host = r.place_stack(0, &["BT1-049", "YEL-HOST"]);
    let _opp = r.place_on_field(1, "OPP-WEAK", Some(0));

    // Play BT1-055 Angemon to give OPP-WEAK -3000 DP -> deleted by 0 DP rule.
    let h = r.add_to_hand(0, "BT1-055");
    r.play(0, h).expect("Angemon plays");
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(1), 0, "2000 DP opp deleted by -3000 DP");
    assert_eq!(
        r.hand_size(0),
        1,
        "Labramon inherited draws 1 when opponent Digimon is deleted by 0 DP"
    );
}

// ─── BT1-050 Liollmon (Vanilla) ──────────────────────────────────────────────

#[test]
fn bt1_050_liollmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-050")
        .expect("BT1-050 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-050").unwrap();
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-050");
    r.play(0, h).expect("Liollmon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(4000));
}

// ─── BT1-051 Reppamon (Vanilla) ──────────────────────────────────────────────

#[test]
fn bt1_051_reppamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-051")
        .expect("BT1-051 loads")
        .memory(5)
        .start();

    let card = r.compiled_card("BT1-051").unwrap();
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-051");
    r.play(0, h).expect("Reppamon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(4000));
}

// ─── BT1-052 Seasarmon ───────────────────────────────────────────────────────

#[test]
fn bt1_052_seasarmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-052")
        .expect("BT1-052 loads")
        .add_card(make_digimon("BIG-SEC", "BigSec", CardColor::Red, 6, 12000))
        .security(1, &["BIG-SEC", "BIG-SEC"])
        .memory(5)
        .start();
    r.game.turn_count = 2;

    let sea = r.place_on_field(0, "BT1-052", Some(0));
    assert!(r.game.has_keyword(sea, Keyword::Jamming));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);
    assert_eq!(r.battle_area_size(0), 1, "Seasarmon Jamming survives security battle");
}

// ─── BT1-053 Darcmon ─────────────────────────────────────────────────────────

#[test]
fn bt1_053_darcmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-053")
        .expect("BT1-053 loads")
        .add_card(make_digimon("YEL-LV3", "YelLv3", CardColor::Yellow, 3, 2000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .memory(8)
        .start();
    r.game.turn_count = 1;

    let _darc = r.place_field_stack(0, &["BT1-053"], true, 0);
    let h = r.add_to_hand(0, "YEL-LV3");
    r.play(0, h).expect("YelLv3 plays");
    resolve_first_pending(&mut r);

    assert_eq!(
        r.hand_size(0),
        1,
        "suspended Darcmon draws 1 when playing a level 3 yellow Digimon"
    );
}

// ─── BT1-054 Liamon ──────────────────────────────────────────────────────────

#[test]
fn bt1_054_liamon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-054")
        .expect("BT1-054 loads")
        .add_card(make_digimon("OPP-DIGI", "OppDigi", CardColor::Red, 4, 6000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(3)
        .start();
    r.game.turn_count = 2;

    let _lia = r.place_on_field(0, "BT1-054", Some(0));
    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));

    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(
        r.effective_dp(opp),
        Some(4000),
        "Liamon gives opponent Digimon -2000 DP when attacking with 3+ memory"
    );
}

// ─── BT1-055 Angemon ─────────────────────────────────────────────────────────

#[test]
fn bt1_055_angemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-055")
        .expect("BT1-055 loads")
        .add_card(make_digimon("OPP-DIGI", "OppDigi", CardColor::Red, 4, 5000))
        .memory(8)
        .start();

    let opp = r.place_on_field(1, "OPP-DIGI", Some(0));
    let h = r.add_to_hand(0, "BT1-055");
    r.play(0, h).expect("Angemon plays");
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.effective_dp(opp), Some(2000), "Angemon On Play gives -3000 DP");
}

// ─── BT1-056 Petermon ────────────────────────────────────────────────────────

#[test]
fn bt1_056_petermon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-056")
        .expect("BT1-056 loads")
        .dsl_card("BT1-047")
        .expect("BT1-047 (Tinkermon) loads")
        .memory(8)
        .start();

    add_to_trash(&mut r, 0, "BT1-047");
    let h = r.add_to_hand(0, "BT1-056");
    r.play(0, h).expect("Petermon plays");
    assert!(r.pending_selection().is_some());
    resolve_first_pending(&mut r);

    assert_eq!(r.battle_area_size(0), 2, "Petermon plays Tinkermon free from trash");
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(r.memory(), 3, "only paid Petermon's cost 5");
}

// ─── BT1-057 Sirenmon (Vanilla) ──────────────────────────────────────────────

#[test]
fn bt1_057_sirenmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-057")
        .expect("BT1-057 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-057").unwrap();
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(6000));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-057");
    r.play(0, h).expect("Sirenmon plays");
    assert_eq!(r.memory(), 3);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(6000));
}

// ─── BT1-058 Chirinmon ───────────────────────────────────────────────────────

#[test]
fn bt1_058_chirinmon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-058")
        .expect("BT1-058 loads")
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .security(1, &["FILLER", "FILLER"])
        .memory(2)
        .start();
    r.game.turn_count = 2;

    let _chi = r.place_on_field(0, "BT1-058", Some(0));
    act(&mut r, 0, encode_attack(0, SECURITY_TARGET));
    resolve_first_pending(&mut r);

    assert_eq!(r.memory(), 5, "When Attacking gains 3 memory");
    r.game.fire_end_of_your_turn(0);
    assert_eq!(r.memory(), 2, "End of turn loses 3 memory");
}

// ─── BT1-059 Piximon (Vanilla) ───────────────────────────────────────────────

#[test]
fn bt1_059_piximon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-059")
        .expect("BT1-059 loads")
        .memory(8)
        .start();

    let card = r.compiled_card("BT1-059").unwrap();
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(9000));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert!(card.effects.is_empty());

    let h = r.add_to_hand(0, "BT1-059");
    r.play(0, h).expect("Piximon plays");
    assert_eq!(r.memory(), 2);
    assert_eq!(r.effective_dp(r.perm_handle(0, 0)), Some(9000));
}

// ─── BT1-060 MagnaAngemon ────────────────────────────────────────────────────

#[test]
fn bt1_060_magnaangemon() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT1-060")
        .expect("BT1-060 loads")
        .add_card(make_digimon("YEL-LV6", "YelLv6", CardColor::Yellow, 6, 10000))
        .add_card(make_digimon("FILLER", "Filler", CardColor::Yellow, 3, 1000))
        .deck(0, &["FILLER", "FILLER"])
        .security(0, &["FILLER"; 5])
        .memory(8)
        .start();
    r.game.turn_count = 1;

    // On Play: Recovery +1 (Deck) -> security 5 -> 6
    let h = r.add_to_hand(0, "BT1-060");
    r.play(0, h).expect("MagnaAngemon plays");
    assert_eq!(r.security_count(0), 6, "On Play Recovery +1 raises security 5 -> 6");

    // Inherited Your Turn aura: +1000 DP per 3 security -> 6 security => +2000 DP
    let host = r.place_stack(0, &["BT1-060", "YEL-LV6"]);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.effective_dp(host),
        Some(12000),
        "MagnaAngemon inherited grants +2000 DP at 6 security"
    );
}
