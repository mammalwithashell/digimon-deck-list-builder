//! Rush / Vortex summoning-sickness exemption tests
//! (§2.1 in RUST_PYTHON_PARITY.md).
//!
//! A Digimon with the Rush keyword — granted via modifier — can attack on
//! the turn it was played. A Digimon invoked via the <Vortex> end-of-turn
//! mechanic is exempt at the call site without needing a persistent
//! keyword. Without either, freshly-played Digimon are summoning-sick and
//! can't attack.
//!
//! Scope note: only *modifier-granted* Rush is checked here. Native Rush
//! from a card's static keyword list isn't wired yet — see §2.1b in the
//! parity doc (requires `CardData.keywords` / effect-listing infra).

use digimon_engine::action::{build_action_mask, encode_attack, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword};

fn fighter(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(dp),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

#[test]
fn freshly_played_without_rush_cannot_attack() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    // Place both on field with turn_played = current turn (summoning sick).
    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);

    assert!(
        !r.game.can_attack(attacker, false),
        "fresh non-Rush digimon must not attack"
    );
    let result = r.attack_digimon(attacker, defender, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::Invalid);
    assert_eq!(r.battle_area_size(1), 1, "defender survived");
}

#[test]
fn freshly_played_with_rush_can_attack() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);

    // Grant Rush for the turn.
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);

    assert!(
        r.game.can_attack(attacker, false),
        "Rush-granted digimon must be able to attack on the turn played"
    );
    let result = r.attack_digimon(attacker, defender, false);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::AttackerWins,
        "5000 DP attacker beats 3000 DP defender"
    );
    assert_eq!(r.battle_area_size(1), 0, "defender deleted");
}

#[test]
fn rush_does_not_override_suspended_state() {
    // Rush exempts summoning sickness but not the general "must be unsuspended"
    // requirement — a suspended permanent still cannot attack.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    // Suspend the attacker manually.
    r.game.players[0].battle_area[attacker.index as usize].is_suspended = true;

    assert!(
        !r.game.can_attack(attacker, false),
        "suspended attacker cannot attack, even with Rush"
    );
    let result = r.attack_digimon(attacker, defender, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::Invalid);
}

#[test]
fn freshly_played_with_vortex_can_attack() {
    // A <Vortex> end-of-turn attack bypasses summoning sickness without
    // needing a persistent keyword — matches Python's
    // `can_attack(is_vortex=True)` and `resolve_attack(is_vortex=True)`.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);

    assert!(
        !r.game.can_attack(attacker, false),
        "without vortex, fresh non-Rush digimon cannot attack"
    );
    assert!(
        r.game.can_attack(attacker, true),
        "with vortex=true, fresh digimon bypass summoning sickness"
    );

    let result = r.attack_digimon(attacker, defender, true);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::AttackerWins,
        "vortex attack resolves combat normally"
    );
    assert_eq!(
        r.battle_area_size(1),
        0,
        "defender deleted by vortex attack"
    );
}

#[test]
fn vortex_does_not_override_suspended_state() {
    // Vortex exempts summoning sickness but not the general "must be
    // unsuspended" requirement — a suspended permanent still cannot attack.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);
    r.game.players[0].battle_area[attacker.index as usize].is_suspended = true;

    assert!(
        !r.game.can_attack(attacker, true),
        "suspended attacker cannot attack, even with vortex"
    );
    let result = r.attack_digimon(attacker, defender, true);
    assert_eq!(result, digimon_engine::combat::AttackResult::Invalid);
}

#[test]
fn mask_allows_rush_granted_attack_on_turn_played() {
    // The Main-phase attack mask must reflect modifier-granted Rush —
    // without this, a policy sees the action as illegal even though
    // `can_attack` would permit it.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    // Build the mask for whichever side is the current turn player —
    // attacks are only enumerated while memory is on your side of the
    // seesaw, which is the turn player's.
    let turn_player = r.game.turn_player();
    let opp = 1 - turn_player;
    let attacker = r.place_on_field(turn_player, "ATK", None);
    let _defender = r.place_on_field(opp, "DEF", None);

    // `begin_turn` leaves us in Breeding; advance to Main so the attack-mask
    // branch runs.
    r.game.enter_main_phase();
    assert!(
        r.game.memory >= 0,
        "turn player should have non-negative memory"
    );

    // Sanity: without Rush, the attack bit is off.
    let mask_no_rush = build_action_mask(&r.game, turn_player);
    let security_bit = encode_attack(attacker.index as u16, SECURITY_TARGET) as usize;
    assert_eq!(
        mask_no_rush[security_bit], 0.0,
        "fresh non-Rush attacker should be masked out"
    );

    // Grant Rush and re-check.
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    let mask_with_rush = build_action_mask(&r.game, turn_player);
    assert_eq!(
        mask_with_rush[security_bit], 1.0,
        "Rush-granted attacker should have security attack unmasked"
    );
}
