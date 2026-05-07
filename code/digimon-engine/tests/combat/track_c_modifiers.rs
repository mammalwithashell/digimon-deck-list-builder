//! Track C combat consult-site behavioral tests (2026-05-07).
//!
//! Pins the three modifier wires landed in this PR:
//! - `MayAttackPlayerOnly` (player-scoped) — restricts the controller's
//!   Digimon to attacking the opposing player only. Digimon-target
//!   attacks return `Invalid` while player-target attacks remain legal.
//! - `CannotSwitchAttackTarget` (permanent-scoped, attacker side) —
//!   suppresses Block / retarget paths; the declared target sticks
//!   regardless of available blockers or replacement substitutions.
//! - `CannotBeRedirectedAsAttackTarget` (permanent-scoped, candidate
//!   side) — filters a Digimon out of the Block redirect candidate
//!   list and rejects redirect substitutions targeting it.

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};

fn digimon(card_id: &str, dp: i32) -> digimon_engine::card_data::CardData {
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
    }
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(digimon("ATK", 5000))
        .add_card(digimon("DEF", 5000))
        .add_card(digimon("BLK", 6000))
        .start()
}

// ── MayAttackPlayerOnly ────────────────────────────────────────────────

#[test]
fn may_attack_player_only_blocks_digimon_target_but_allows_player_target() {
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Install a player-scoped MayAttackPlayerOnly modifier on the
    // attacker's controller (P0).
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::MayAttackPlayerOnly,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(
        result,
        AttackResult::Invalid,
        "MayAttackPlayerOnly must reject Digimon-target attacks"
    );

    // Player target remains legal.
    let result_player = r.attack_player(atk, 1, /* vortex */ true);
    assert_ne!(
        result_player,
        AttackResult::Invalid,
        "MayAttackPlayerOnly must NOT reject player-target attacks"
    );
}

#[test]
fn may_attack_player_only_only_affects_named_player() {
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Modifier scoped to player 1 (the defender), not player 0.
    r.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::simple(
            ModifierType::MayAttackPlayerOnly,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );

    let result = r.attack_digimon(atk, def, false);
    assert_ne!(
        result,
        AttackResult::Invalid,
        "MayAttackPlayerOnly on the opposing player must not affect P0's attacks"
    );
}

// ── CannotSwitchAttackTarget ───────────────────────────────────────────

#[test]
fn cannot_switch_attack_target_skips_block_window() {
    // BT24-062 MasterBlimpmon: "[Your Turn] This Digimon's attack target
    // can't change." A Blocker on the defender's side normally installs
    // BlockTiming; with the modifier, Block is skipped entirely and the
    // attack resolves as Digimon-vs-Digimon against the original target.
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    // Pin the attacker's target.
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(
            ModifierType::CannotSwitchAttackTarget,
            0,
            Expiry::Permanent,
            0,
        ),
    );

    let result = r.attack_digimon(atk, def, false);
    // No BlockTiming selection; attack resolves synchronously against DEF.
    assert_ne!(result, AttackResult::InProgress);
    assert_ne!(
        r.current_phase(),
        GamePhase::BlockTiming,
        "CannotSwitchAttackTarget must skip the Block window"
    );
    assert!(r.game.pending_selection.is_none());
}

// (Direct `apply_attack_target_substitution` no-op tests are covered by
// the unit-level guard in `combat.rs::apply_attack_target_substitution`
// itself — the function is `pub(crate)` and not reachable from
// integration tests. The Block-window tests above prove the
// `CannotSwitchAttackTarget` and `CannotBeRedirectedAsAttackTarget`
// gates fire at the only retarget path that is integration-test
// reachable today.)

// ── CannotBeRedirectedAsAttackTarget ───────────────────────────────────

#[test]
fn cannot_be_redirected_filters_block_candidate() {
    // A Digimon with Blocker that ALSO has CannotBeRedirectedAsAttackTarget
    // is a valid blocker for "may declare a block" purposes but cannot
    // become the new attack target via the Block redirect path. With the
    // protected blocker as the only candidate, no BlockTiming installs.
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    r.game.modifiers.add(
        blk,
        ModifierEntry::simple(
            ModifierType::CannotBeRedirectedAsAttackTarget,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let result = r.attack_digimon(atk, def, false);
    assert_ne!(
        result,
        AttackResult::InProgress,
        "no eligible redirect candidate => no BlockTiming"
    );
    assert_ne!(r.current_phase(), GamePhase::BlockTiming);
}

#[test]
fn cannot_be_redirected_does_not_filter_other_block_candidates() {
    // Two Blocker candidates: one protected, one unprotected. The
    // unprotected one must still install BlockTiming and surface as a
    // legal action. The protected one must be filtered out of the
    // valid-action list.
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 5000))
        .add_card(digimon("DEF", 5000))
        .add_card(digimon("BLK_OK", 6000))
        .add_card(digimon("BLK_LOCK", 6000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let blk_ok = r.place_on_field(1, "BLK_OK", Some(0));
    let blk_lock = r.place_on_field(1, "BLK_LOCK", Some(0));
    for h in [blk_ok, blk_lock] {
        r.game
            .modifiers
            .grant_keyword(h, Keyword::Blocker, Expiry::Permanent, 1);
    }
    r.game.modifiers.add(
        blk_lock,
        ModifierEntry::simple(
            ModifierType::CannotBeRedirectedAsAttackTarget,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let result = r.attack_digimon(atk, r.perm_handle(1, 0), false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("BlockTiming must install a pending selection");
    let action_ok = encode_attack(0, blk_ok.index as u16);
    let action_lock = encode_attack(0, blk_lock.index as u16);
    assert!(
        sel.valid_action_ids.contains(&action_ok),
        "unprotected blocker must remain a valid block target"
    );
    assert!(
        !sel.valid_action_ids.contains(&action_lock),
        "protected blocker must be filtered out of the valid block list"
    );
    // The selection is optional (no Collision), so PASS is offered too.
    assert!(sel.is_optional);
    let _ = PASS;
}

