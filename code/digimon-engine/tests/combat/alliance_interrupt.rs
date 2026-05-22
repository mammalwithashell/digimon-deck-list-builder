//! Alliance interrupt tests.
//!
//! Exercises the PR5 `try_enter_alliance` path:
//!   - no Alliance allies present → window is skipped, attack proceeds
//!   - Alliance ally present → AllianceTiming prompt installed for the
//!     attacker's controller
//!   - declared ally: attacker gains +ally_DP, ally is suspended, +1 SA
//!   - declined: attack proceeds to Counter/Block with no changes
//!   - Alliance modifiers expire at end of attack
//!   - Vortex skips Alliance too

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};

fn dgmn(card_id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    digimon_engine::card_data::CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

#[test]
fn no_alliance_allies_skips_window() {
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 5000))
        .add_card(dgmn("DEF", 3000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.attack_digimon(atk, def, false);
    // No Alliance keyword, no Blocker keyword → synchronous resolution.
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn alliance_ally_installs_prompt() {
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 5000))
        .add_card(dgmn("DEF", 3000))
        .add_card(dgmn("ALLY", 4000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(ally, Keyword::Alliance, Expiry::Permanent, 0);

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::AllianceTiming);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("AllianceTiming installs a selection");
    assert_eq!(sel.selecting_player, 0, "attacker's controller chooses");
    assert!(sel.is_optional);
    // The ally is at field index 1 (atk is at index 0).
    assert_eq!(sel.valid_action_ids, vec![encode_attack(0, 1)]);
}

#[test]
fn alliance_declared_buffs_attacker_and_suspends_ally() {
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 4000))
        .add_card(dgmn("DEF", 6000)) // stronger without ally
        .add_card(dgmn("ALLY", 3000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(ally, Keyword::Alliance, Expiry::Permanent, 0);

    r.attack_digimon(atk, def, false);
    let ally_action = encode_attack(0, 1);
    r.game
        .resolve_selection(0, ally_action)
        .expect("declare Alliance");

    // ATK 4000 + ALLY 3000 = 7000 > DEF 6000 → attacker wins.
    assert!(r.game.pending_attack.is_none());
    // DEF was deleted, ATK + ALLY remain on field 0.
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(r.battle_area_size(1), 0);
    // Ally was suspended.
    assert!(r.game.players[0].battle_area[1].is_suspended);
    // Alliance modifiers expired at end of attack.
    assert_eq!(r.game.modifiers.sum(atk, ModifierType::ChangeDp), 0);
    assert_eq!(
        r.game
            .modifiers
            .sum(atk, ModifierType::SecurityAttackChange),
        0
    );
}

#[test]
fn alliance_declined_preserves_board_state() {
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 5000))
        .add_card(dgmn("DEF", 3000))
        .add_card(dgmn("ALLY", 3000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(ally, Keyword::Alliance, Expiry::Permanent, 0);

    r.attack_digimon(atk, def, false);
    r.game.resolve_selection(0, PASS).expect("decline alliance");

    // ATK still wins on its own (5000 > 3000).
    assert!(r.game.pending_attack.is_none());
    // Ally wasn't suspended.
    assert!(!r.game.players[0].battle_area[1].is_suspended);
}

#[test]
fn vortex_bypasses_alliance() {
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 5000))
        .add_card(dgmn("DEF", 3000))
        .add_card(dgmn("ALLY", 4000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(ally, Keyword::Alliance, Expiry::Permanent, 0);

    let result = r.attack_digimon(atk, def, /* vortex = */ true);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn suspended_alliance_ally_is_not_a_candidate() {
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 5000))
        .add_card(dgmn("DEF", 3000))
        .add_card(dgmn("ALLY", 4000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(ally, Keyword::Alliance, Expiry::Permanent, 0);
    r.game.players[0].battle_area[ally.index as usize].is_suspended = true;

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn alliance_then_block_sequence() {
    // Alliance first, then blocker — both windows run in order.
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 3000))
        .add_card(dgmn("DEF", 3000))
        .add_card(dgmn("ALLY", 2000))
        .add_card(dgmn("BLK", 9000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(ally, Keyword::Alliance, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    // First prompt: Alliance for P0.
    r.attack_digimon(atk, def, false);
    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(sel.selecting_player, 0);
    assert_eq!(r.current_phase(), GamePhase::AllianceTiming);

    // P0 declines alliance.
    r.game.resolve_selection(0, PASS).expect("decline alliance");

    // Second prompt: Block for P1.
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("BlockTiming installed after Alliance declined");
    assert_eq!(sel.selecting_player, 1);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);

    // P1 declines block.
    r.game.resolve_selection(1, PASS).expect("decline block");

    // Resolution: ATK 3000 vs DEF 3000 → mutual destruction.
    assert!(r.game.pending_attack.is_none());
    // ATK + DEF gone, ALLY + BLK remain.
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.battle_area_size(1), 1);
}
