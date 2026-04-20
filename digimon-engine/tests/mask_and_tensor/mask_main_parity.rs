//! Main-phase action-mask parity tests.
//!
//! Each test in this file locks in a specific §4.x behavior gap from
//! RUST_PYTHON_PARITY.md so that regressions are caught immediately.
//!
//! §4.2 — Option card color requirement
//! §4.3 — Blitz attack exception (memory < 0)
//! §4.4 — Raid + CAN_ATTACK_UNSUSPENDED target rule (unsuspended targets)

use digimon_engine::action::{build_action_mask, encode_attack, DNA_DIGIVOLVE_START, SECURITY_TARGET};
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType};

// ─── Card factories ────────────────────────────────────────────────────

fn make_option(id: &str, color: CardColor) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn make_digimon(id: &str, color: CardColor) -> CardData {
    make_digimon_dp(id, color, 4000)
}

fn make_digimon_dp(id: &str, color: CardColor, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn make_tamer(id: &str, color: CardColor) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 4,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── §4.2 Option color requirement ────────────────────────────────────

/// An Option card must be masked out (mask[0] == 0.0) when the player has
/// no Digimon or Tamer of a matching color on the field. Walks through the
/// empty-field, wrong-color, and matching-color transitions in one test so
/// a regression at any stage is caught immediately.
#[test]
fn mask_option_requires_matching_color_on_field() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("OPT-R", CardColor::Red))
        .add_card(make_digimon("BLUE-MON", CardColor::Blue))
        .add_card(make_digimon("RED-MON", CardColor::Red))
        .hand(0, &["OPT-R"])
        .start();

    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Empty field → no matching color → masked out.
    let mask_no_field = build_action_mask(&r.game, 0);
    assert_eq!(
        mask_no_field[0], 0.0,
        "Option with empty field must be masked out"
    );

    // Wrong-color Digimon on field → still masked out.
    r.place_on_field(0, "BLUE-MON", Some(0));
    let mask_wrong_color = build_action_mask(&r.game, 0);
    assert_eq!(
        mask_wrong_color[0], 0.0,
        "Blue Digimon does not satisfy Red Option color requirement"
    );

    // Matching-color Digimon on field → unmasked.
    r.place_on_field(0, "RED-MON", Some(0));
    let mask_match = build_action_mask(&r.game, 0);
    assert_eq!(
        mask_match[0], 1.0,
        "Red Digimon on field should unmask Red Option"
    );
}

/// A Tamer of the matching color also satisfies the Option color requirement.
#[test]
fn mask_option_color_check_accepts_tamer() {
    // Red Option in hand; Red Tamer on field — should satisfy the requirement.
    let mut r = DebugRunner::builder()
        .add_card(make_option("OPT-R", CardColor::Red))
        .add_card(make_tamer("TAMER-R", CardColor::Red))
        .hand(0, &["OPT-R"])
        .start();

    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Place a Red Tamer on field — correct color, Tamer type.
    r.place_on_field(0, "TAMER-R", Some(0));

    let mask = build_action_mask(&r.game, 0);

    assert_eq!(
        mask[0], 1.0,
        "Option with matching-color Tamer on field must be playable"
    );
}

// ─── §4.3 Blitz under negative memory ──────────────────────────────────

/// A Digimon that digivolved this turn and has modifier-granted Blitz can
/// attack even when memory < 0 (§4.3 parity with Python's action_mask.py
/// lines 107-114).
#[test]
fn mask_blitz_can_attack_under_negative_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .add_card(make_digimon("DEF", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.place_on_field(opp, "DEF", Some(0));

    // Simulate "digivolved this turn" on the attacker, and suspend the
    // opponent so there's a Digimon target available.
    r.game.players[tp as usize].battle_area[attacker.index as usize]
        .turn_digivolved = r.game.turn_count;
    r.game.players[opp as usize].battle_area[0].is_suspended = true;

    r.game.enter_main_phase();
    r.game.set_memory(-3);

    // Baseline: no Blitz → no attack bit while memory<0.
    let mask_no_blitz = build_action_mask(&r.game, tp);
    let sec_bit = encode_attack(attacker.index as u16, SECURITY_TARGET) as usize;
    let dig_bit = encode_attack(attacker.index as u16, 0) as usize;
    assert_eq!(mask_no_blitz[sec_bit], 0.0, "no Blitz + memory<0 → no security attack");
    assert_eq!(mask_no_blitz[dig_bit], 0.0, "no Blitz + memory<0 → no digimon attack");

    // Grant Blitz → both attack bits should light up.
    r.game.modifiers.grant_keyword(
        attacker, Keyword::Blitz, Expiry::EndOfTurn, tp,
    );
    let mask_blitz = build_action_mask(&r.game, tp);
    assert_eq!(mask_blitz[sec_bit], 1.0, "Blitz + digivolved this turn → security");
    assert_eq!(mask_blitz[dig_bit], 1.0, "Blitz + digivolved this turn → digimon");
}

/// Blitz only exempts memory<0 when the attacker digivolved *this* turn.
/// A Blitz Digimon that wasn't freshly digivolved still can't attack.
#[test]
fn mask_blitz_without_digivolving_does_not_attack_under_negative_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .add_card(make_digimon("DEF", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[0].is_suspended = true;

    r.game.enter_main_phase();
    r.game.set_memory(-3);
    r.game.modifiers.grant_keyword(
        attacker, Keyword::Blitz, Expiry::EndOfTurn, tp,
    );
    // NOTE: no turn_digivolved assignment — attacker was placed as-is.

    let mask = build_action_mask(&r.game, tp);
    let sec_bit = encode_attack(attacker.index as u16, SECURITY_TARGET) as usize;
    assert_eq!(
        mask[sec_bit], 0.0,
        "Blitz without this-turn digivolve must not unlock attack under memory<0"
    );
}

// ─── §4.4 Raid + CAN_ATTACK_UNSUSPENDED target rules ───────────────────

/// With Raid, an attacker can target unsuspended enemies tied for the
/// highest effective DP. Lower-DP unsuspended enemies remain off-limits.
#[test]
fn mask_raid_targets_highest_dp_unsuspended() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_dp("ATK", CardColor::Red, 5000))
        .add_card(make_digimon_dp("WEAK", CardColor::Blue, 2000))
        .add_card(make_digimon_dp("STRONG", CardColor::Blue, 7000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let weak_idx = r.place_on_field(opp, "WEAK", Some(0)).index;
    let strong_idx = r.place_on_field(opp, "STRONG", Some(0)).index;
    // Both enemies unsuspended.
    for p in &mut r.game.players[opp as usize].battle_area {
        p.is_suspended = false;
    }

    r.game.enter_main_phase();
    r.game.set_memory(3);

    // Baseline: no Raid → unsuspended targets stay off-limits.
    let weak_bit = encode_attack(attacker.index as u16, weak_idx as u16) as usize;
    let strong_bit = encode_attack(attacker.index as u16, strong_idx as u16) as usize;
    let baseline = build_action_mask(&r.game, tp);
    assert_eq!(baseline[weak_bit], 0.0, "no Raid → unsuspended weak is off-limits");
    assert_eq!(baseline[strong_bit], 0.0, "no Raid → unsuspended strong is off-limits");

    // Grant Raid → only the highest-DP unsuspended target (STRONG) lights up.
    r.game.modifiers.grant_keyword(
        attacker, Keyword::Raid, Expiry::EndOfTurn, tp,
    );
    let raid_mask = build_action_mask(&r.game, tp);
    assert_eq!(raid_mask[strong_bit], 1.0, "Raid → highest-DP unsuspended is attackable");
    assert_eq!(raid_mask[weak_bit], 0.0, "Raid → lower-DP unsuspended is NOT attackable");
}

/// When multiple unsuspended enemies share the highest DP, Raid allows
/// attacking any of them (matches Python's `any(... == max(...))`).
#[test]
fn mask_raid_allows_all_tied_for_highest() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_dp("ATK", CardColor::Red, 5000))
        .add_card(make_digimon_dp("TWIN", CardColor::Blue, 4000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let twin_a = r.place_on_field(opp, "TWIN", Some(0)).index;
    let twin_b = r.place_on_field(opp, "TWIN", Some(0)).index;
    for p in &mut r.game.players[opp as usize].battle_area {
        p.is_suspended = false;
    }

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.modifiers.grant_keyword(
        attacker, Keyword::Raid, Expiry::EndOfTurn, tp,
    );
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, twin_a as u16) as usize], 1.0,
        "Raid → first tied-for-max target is attackable",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, twin_b as u16) as usize], 1.0,
        "Raid → second tied-for-max target is also attackable",
    );
}

/// CAN_ATTACK_UNSUSPENDED modifier broadens attack targets to every
/// unsuspended enemy regardless of DP.
#[test]
fn mask_can_attack_unsuspended_modifier_allows_all_unsuspended() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_dp("ATK", CardColor::Red, 5000))
        .add_card(make_digimon_dp("WEAK", CardColor::Blue, 2000))
        .add_card(make_digimon_dp("STRONG", CardColor::Blue, 7000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let weak_idx = r.place_on_field(opp, "WEAK", Some(0)).index;
    let strong_idx = r.place_on_field(opp, "STRONG", Some(0)).index;
    for p in &mut r.game.players[opp as usize].battle_area {
        p.is_suspended = false;
    }

    r.game.enter_main_phase();
    r.game.set_memory(3);
    r.game.modifiers.add(
        attacker,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CanAttackUnsuspended,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: tp,
        },
    );

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, weak_idx as u16) as usize], 1.0,
        "CAN_ATTACK_UNSUSPENDED → lower-DP unsuspended is attackable",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, strong_idx as u16) as usize], 1.0,
        "CAN_ATTACK_UNSUSPENDED → higher-DP unsuspended is attackable",
    );
}

// ─── §4.5 DNA Digivolve mask (range 63-92) ─────────────────────────────

/// A DNA-digivolve card (Digimon with `dna_costs`) is playable via mask bit
/// `DNA_DIGIVOLVE_START + hand_index` when two permanents on the battle
/// area satisfy any of its DnaCost entries in either ordering.
fn make_digimon_level(id: &str, color: CardColor, level: u8) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(4000),
        play_cost: 5,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// A Digimon hand card carrying DNA costs — used as the evolution target.
fn make_dna_digimon(
    id: &str,
    color: CardColor,
    dna_costs: Vec<DnaCost>,
) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(6000),
        play_cost: 7,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs,
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn req(level: u8, color: CardColor) -> DnaRequirement {
    DnaRequirement {
        level,
        card_color: Some(color),
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

#[test]
fn mask_dna_digivolve_emits_when_valid_pair_exists() {
    // DNA card needs Red Lv3 + Blue Lv3. Field has both → mask bit set.
    let dna_costs = vec![DnaCost {
        requirement1: req(3, CardColor::Red),
        requirement2: req(3, CardColor::Blue),
        memory_cost: 0,
    }];
    let mut r = DebugRunner::builder()
        .add_card(make_dna_digimon("DNA-X", CardColor::Purple, dna_costs))
        .add_card(make_digimon_level("RED-MON", CardColor::Red, 3))
        .add_card(make_digimon_level("BLUE-MON", CardColor::Blue, 3))
        .hand(0, &["DNA-X"])
        .start();

    r.game.set_memory(3);
    r.game.enter_main_phase();
    r.place_on_field(0, "RED-MON", Some(0));
    r.place_on_field(0, "BLUE-MON", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[DNA_DIGIVOLVE_START as usize], 1.0,
        "DNA-X is in hand slot 0 → bit DNA_DIGIVOLVE_START + 0 should be 1",
    );
}

#[test]
fn mask_dna_digivolve_accepts_either_ordering() {
    // Same requirement, but place Blue BEFORE Red in the battle_area.
    // Validator must try both orderings.
    let dna_costs = vec![DnaCost {
        requirement1: req(3, CardColor::Red),
        requirement2: req(3, CardColor::Blue),
        memory_cost: 0,
    }];
    let mut r = DebugRunner::builder()
        .add_card(make_dna_digimon("DNA-X", CardColor::Purple, dna_costs))
        .add_card(make_digimon_level("RED-MON", CardColor::Red, 3))
        .add_card(make_digimon_level("BLUE-MON", CardColor::Blue, 3))
        .hand(0, &["DNA-X"])
        .start();

    r.game.set_memory(3);
    r.game.enter_main_phase();
    r.place_on_field(0, "BLUE-MON", Some(0)); // Blue first
    r.place_on_field(0, "RED-MON", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[DNA_DIGIVOLVE_START as usize], 1.0,
        "reversed battle_area order must still satisfy the DNA pair",
    );
}

#[test]
fn mask_dna_digivolve_rejects_when_no_pair() {
    // Needs Red + Blue; field has two Reds. No valid pair.
    let dna_costs = vec![DnaCost {
        requirement1: req(3, CardColor::Red),
        requirement2: req(3, CardColor::Blue),
        memory_cost: 0,
    }];
    let mut r = DebugRunner::builder()
        .add_card(make_dna_digimon("DNA-X", CardColor::Purple, dna_costs))
        .add_card(make_digimon_level("RED-A", CardColor::Red, 3))
        .add_card(make_digimon_level("RED-B", CardColor::Red, 3))
        .hand(0, &["DNA-X"])
        .start();

    r.game.set_memory(3);
    r.game.enter_main_phase();
    r.place_on_field(0, "RED-A", Some(0));
    r.place_on_field(0, "RED-B", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[DNA_DIGIVOLVE_START as usize], 0.0,
        "no Red+Blue pair → mask bit must stay off",
    );
}

#[test]
fn mask_dna_digivolve_does_not_gate_on_memory() {
    // Python's DNA mask section (action_mask.py:161-166) emits the bit
    // whenever a valid pair exists, regardless of memory. Memory is
    // validated at action-execution time. Locks in the Python-parity
    // behavior for Rust — at negative memory with a valid pair, the bit
    // still fires.
    let dna_costs = vec![DnaCost {
        requirement1: req(3, CardColor::Red),
        requirement2: req(3, CardColor::Blue),
        memory_cost: 5,
    }];
    let mut r = DebugRunner::builder()
        .add_card(make_dna_digimon("DNA-X", CardColor::Purple, dna_costs))
        .add_card(make_digimon_level("RED-MON", CardColor::Red, 3))
        .add_card(make_digimon_level("BLUE-MON", CardColor::Blue, 3))
        .hand(0, &["DNA-X"])
        .start();

    r.game.set_memory(-8);
    r.game.enter_main_phase();
    r.place_on_field(0, "RED-MON", Some(0));
    r.place_on_field(0, "BLUE-MON", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[DNA_DIGIVOLVE_START as usize], 1.0,
        "DNA mask emits bit regardless of memory — Python parity",
    );
}

#[test]
fn mask_dna_digivolve_skips_cards_without_dna_costs() {
    // A regular Digimon in hand (no dna_costs) must never emit a DNA bit.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_level("REGULAR", CardColor::Red, 5))
        .add_card(make_digimon_level("RED-MON", CardColor::Red, 3))
        .add_card(make_digimon_level("BLUE-MON", CardColor::Blue, 3))
        .hand(0, &["REGULAR"])
        .start();

    r.game.set_memory(3);
    r.game.enter_main_phase();
    r.place_on_field(0, "RED-MON", Some(0));
    r.place_on_field(0, "BLUE-MON", Some(0));

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[DNA_DIGIVOLVE_START as usize], 0.0,
        "Digimon without dna_costs must not emit the DNA bit even if a pair exists",
    );
}

// ─── §4.7c CANNOT_PLAY_FROM_HAND ───────────────────────────────────────

/// If any permanent has an active CannotPlayFromHand modifier, every
/// hand-play bit (0-29) is suppressed. Unconditional semantics — Rust
/// doesn't carry Python's context discriminants (§4.7x).
#[test]
fn mask_cannot_play_from_hand_suppresses_all_hand_bits() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_level("BLOCKER", CardColor::Red, 3))
        .add_card(make_digimon_level("HAND-A", CardColor::Red, 3))
        .add_card(make_digimon_level("HAND-B", CardColor::Blue, 3))
        .hand(0, &["HAND-A", "HAND-B"])
        .start();

    let blocker = r.place_on_field(0, "BLOCKER", Some(0));
    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Baseline: both hand cards are playable (affordable + Digimon kind).
    let mask_baseline = build_action_mask(&r.game, 0);
    assert_eq!(mask_baseline[0], 1.0, "HAND-A should be playable without the modifier");
    assert_eq!(mask_baseline[1], 1.0, "HAND-B should be playable without the modifier");

    // Grant CannotPlayFromHand to the blocker.
    r.game.modifiers.add(
        blocker,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CannotPlayFromHand,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: 0,
        },
    );

    let mask_blocked = build_action_mask(&r.game, 0);
    assert_eq!(mask_blocked[0], 0.0, "HAND-A suppressed while modifier is active");
    assert_eq!(mask_blocked[1], 0.0, "HAND-B suppressed while modifier is active");
}

// ─── §4.7b CANNOT_DIGIVOLVE ────────────────────────────────────────────

/// A permanent with an active CannotDigivolve modifier must not have any
/// digivolve bit emitted, regardless of evo-cost match.
#[test]
fn mask_cannot_digivolve_suppresses_digivolve_bits_on_base() {
    use digimon_engine::action::encode_digivolve;

    // Hand card: Red Lv4 with Red Lv3 evo_cost.
    let mut evo_card = make_digimon_level("EVO-RED", CardColor::Red, 4);
    evo_card.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 0, // Red
        level: 3,
        memory_cost: 1,
    }];

    let base_card = make_digimon_level("BASE-RED", CardColor::Red, 3);

    let mut r = DebugRunner::builder()
        .add_card(evo_card)
        .add_card(base_card)
        .hand(0, &["EVO-RED"])
        .start();

    let base = r.place_on_field(0, "BASE-RED", Some(0));
    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Baseline: evo bit to base[0] is emitted (evo_cost matches).
    let baseline = build_action_mask(&r.game, 0);
    let evo_bit = encode_digivolve(0 as u16, base.index as u16) as usize;
    assert_eq!(baseline[evo_bit], 1.0, "baseline digivolve bit should be set");

    // Grant CannotDigivolve to the base.
    r.game.modifiers.add(
        base,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CannotDigivolve,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: 0,
        },
    );

    let blocked = build_action_mask(&r.game, 0);
    assert_eq!(
        blocked[evo_bit], 0.0,
        "CannotDigivolve on base must suppress the digivolve bit onto it",
    );
}

// ─── §4.7a CANNOT_ATTACK_TARGET ────────────────────────────────────────

/// An enemy permanent with CannotAttackTarget active must not have any
/// attacker-vs-target bit emitted. Security-attack bit is orthogonal.
#[test]
fn mask_cannot_attack_target_suppresses_digimon_attack_bit() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon_level("ATK", CardColor::Red, 4))
        .add_card(make_digimon_level("DEF", CardColor::Blue, 3))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    r.game.set_memory(3);
    r.game.enter_main_phase();

    // Baseline: attack bit to defender is emitted (suspended).
    let baseline = build_action_mask(&r.game, tp);
    let atk_bit = encode_attack(attacker.index as u16, defender.index as u16) as usize;
    let sec_bit = encode_attack(attacker.index as u16, SECURITY_TARGET) as usize;
    assert_eq!(baseline[atk_bit], 1.0, "baseline: suspended defender is attackable");
    assert_eq!(baseline[sec_bit], 1.0, "baseline: security is attackable");

    // Grant CannotAttackTarget to defender.
    r.game.modifiers.add(
        defender,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::CannotAttackTarget,
            value: 1,
            expiry: Expiry::EndOfTurn,
            source_player: opp,
        },
    );

    let blocked = build_action_mask(&r.game, tp);
    assert_eq!(
        blocked[atk_bit], 0.0,
        "CannotAttackTarget on defender must suppress the digimon-attack bit",
    );
    assert_eq!(
        blocked[sec_bit], 1.0,
        "security attack bit is orthogonal to CannotAttackTarget",
    );
}
