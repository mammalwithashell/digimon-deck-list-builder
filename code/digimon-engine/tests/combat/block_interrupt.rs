//! Combat state-machine tests — Block interrupt path.
//!
//! Exercises the PR4 split of `attack_digimon` / `attack_player` into
//! `begin_attack` → `advance_pending_attack` → `resolve_pending_battle`:
//!   - no blockers → attack resolves synchronously (behavioral parity with
//!     the pre-state-machine engine; covered by `combat_scenarios.rs` too)
//!   - blocker candidates present → BlockTiming selection installed, mask
//!     emits blocker bits + PASS
//!   - block declined → attack resolves against the original target
//!   - block declared → attack redirects to the blocker; Digimon battle
//!     runs against the blocker, not the original target
//!   - player attack + blocker → security loop does NOT run; battle runs
//!     against the blocker instead
//!   - `is_attacking` flag set during flight, cleared by cleanup
//!   - Vortex short-circuits the block window
//!   - wrong-player cannot resolve block selection

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword};

fn strong_digimon(card_id: &str, dp: i32) -> digimon_engine::card_data::CardData {
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
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn runner_with_attacker_vs(defender_dp: i32, attacker_dp: i32) -> DebugRunner {
    DebugRunner::builder()
        .add_card(strong_digimon("ATK", attacker_dp))
        .add_card(strong_digimon("DEF", defender_dp))
        .add_card(strong_digimon("BLK", 9000))
        .start()
}

/// Baseline: attack against a defender with no Blocker keyword runs
/// synchronously and returns a terminal result.
#[test]
fn no_blockers_attack_resolves_synchronously() {
    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_attack.is_none(), "pending_attack cleaned up");
    assert!(r.game.pending_selection.is_none());
}

/// Blocker candidate present → BlockTiming selection installed, phase
/// flipped, mask emits blocker bits + PASS for the defender.
#[test]
fn blocker_candidate_installs_blocktiming_selection() {
    let mut r = runner_with_attacker_vs(5000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    let result = r.attack_digimon(atk, r.perm_handle(1, 0), false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("BlockTiming installs a PendingSelection");
    assert_eq!(sel.selecting_player, 1, "defender declares blockers");
    assert!(sel.is_optional, "Block is always a may-trigger");
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, 1)],
        "only the Blocker-keyword slot (field index 1) is legal"
    );

    let mask = build_action_mask(&r.game, 1);
    assert_eq!(mask[encode_attack(0, 1) as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);
    // Attacker's controller sees an all-zero mask.
    let attacker_mask = build_action_mask(&r.game, 0);
    let legal: Vec<usize> = attacker_mask
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    assert!(
        legal.is_empty(),
        "attacker sees no legal action during BlockTiming, got {:?}",
        legal
    );
}

/// Block declined → attack resumes against the original Digimon target.
#[test]
fn block_declined_resolves_against_original_target() {
    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_digimon(atk, def, false);
    // Defender passes on blocking.
    r.game
        .resolve_selection(1, PASS)
        .expect("PASS on BlockTiming is legal");

    assert!(r.game.pending_attack.is_none(), "attack cleaned up");
    assert!(r.game.pending_selection.is_none());
    // DEF (3000 DP) was deleted; BLK survived.
    assert_eq!(
        r.battle_area_size(1),
        1,
        "only BLK remains on defender's field"
    );
}

/// Block declared → attack redirects to the blocker; battle runs attacker
/// vs blocker, not attacker vs original target.
#[test]
fn block_declared_redirects_battle_to_blocker() {
    // ATK: 5000, DEF: 3000 (weaker), BLK: 9000 (stronger). If block fires,
    // attacker loses to BLK; if not, attacker wins over DEF.
    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_digimon(atk, r.perm_handle(1, 0), false);
    let block_action = encode_attack(0, 1);
    r.game
        .resolve_selection(1, block_action)
        .expect("declaring a valid blocker must succeed");

    assert!(r.game.pending_attack.is_none());
    // ATK (5000) lost to BLK (9000). DEF survives. BLK survives.
    assert_eq!(r.battle_area_size(0), 0, "attacker deleted by blocker");
    assert_eq!(r.battle_area_size(1), 2, "DEF and BLK both survive");
}

/// Player attack with a blocker → security loop does NOT run; the attack
/// redirects to a Digimon battle against the blocker.
#[test]
fn player_attack_with_blocker_redirects_to_blocker_battle() {
    let mut r = runner_with_attacker_vs(/* def_dp unused */ 0, 9000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    let sec_before = r.security_count(1);

    // Attack P1 directly. ATK=9000 vs BLK=9000 → mutual destruction.
    r.attack_player(atk, 1, false);
    let block_action = encode_attack(0, 0);
    r.game
        .resolve_selection(1, block_action)
        .expect("block on player attack");

    assert_eq!(
        r.security_count(1),
        sec_before,
        "security was NOT checked because the blocker intercepted"
    );
    // Mutual destruction: both ATK and BLK are trashed.
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.battle_area_size(1), 0);
}

/// Player attack declining a block → proceeds into the security loop.
#[test]
fn player_attack_declining_block_runs_security_loop() {
    let mut r = DebugRunner::builder()
        .add_card(strong_digimon("ATK", 9000))
        .add_card(strong_digimon("BLK", 9000))
        .add_card(strong_digimon("SEC", 1))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    let sec_before = r.security_count(1);

    r.attack_player(atk, 1, false);
    r.game.resolve_selection(1, PASS).expect("decline block");

    assert!(r.game.pending_attack.is_none());
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "one security card revealed when block declined"
    );
}

/// `is_attacking` is live during the attack flight and cleared by cleanup.
#[test]
fn is_attacking_flag_lifecycle() {
    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    // Pre-attack: no one is attacking.
    assert!(!r.game.players[0].battle_area[atk.index as usize].is_attacking);

    r.attack_digimon(atk, def, false);
    // Paused on BlockTiming. Attacker is mid-flight, flag is live.
    assert!(
        r.game.players[0].battle_area[atk.index as usize].is_attacking,
        "is_attacking is true while the attack is parked on BlockTiming"
    );

    r.game.resolve_selection(1, PASS).expect("decline block");
    // Attack terminated. Flag cleared — and the attacker is still on the
    // field because DEF (3000) < ATK (5000) so ATK won.
    assert!(!r.game.players[0].battle_area[atk.index as usize].is_attacking);
}

/// Vortex attacks bypass the Block window (per rules).
#[test]
fn vortex_bypasses_blocktiming() {
    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    // Mark vortex = true. No block prompt — synchronous resolution.
    let result = r.attack_digimon(atk, def, /* vortex = */ true);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_attack.is_none());
}

/// Only the defending player can resolve a block selection.
#[test]
fn wrong_player_cannot_resolve_block() {
    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_digimon(atk, def, false);
    use digimon_engine::selection::SelectionError;
    let err = r
        .game
        .resolve_selection(0, PASS)
        .expect_err("P0 not selector");
    assert_eq!(err, SelectionError::WrongPlayer);
}

/// Collision on the attacker grants Blocker to every unsuspended opponent
/// Digimon for this attack — a BlockTiming prompt installs even when the
/// defender has no Blocker-keyword permanents.
#[test]
fn collision_attacker_expands_blocker_pool_to_all_defenders() {
    use digimon_engine::action::space::encode_attack;

    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def_a = r.place_on_field(1, "DEF", Some(0));
    let _def_b = r.place_on_field(1, "DEF", Some(0));
    let _def_c = r.place_on_field(1, "DEF", Some(0));
    // No Blocker keyword anywhere — but attacker has Collision.
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);

    let result = r.attack_digimon(atk, def_a, false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Collision installs a BlockTiming selection");
    // All three defender Digimon are candidates.
    assert_eq!(
        sel.valid_action_ids,
        vec![
            encode_attack(0, 0),
            encode_attack(0, 1),
            encode_attack(0, 2)
        ],
    );
}

/// Collision still respects the standard disqualifiers (suspended,
/// non-Digimon) — only *unsuspended opponent Digimon* gain the virtual
/// Blocker keyword.
#[test]
fn collision_still_skips_suspended_defenders() {
    use digimon_engine::action::space::encode_attack;

    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def_a = r.place_on_field(1, "DEF", Some(0));
    let _def_b = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);
    // Suspend the second defender.
    r.game.players[1].battle_area[1].is_suspended = true;

    r.attack_digimon(atk, def_a, false);
    let sel = r.game.pending_selection.as_ref().unwrap();
    // Only the unsuspended defender remains.
    assert_eq!(sel.valid_action_ids, vec![encode_attack(0, 0)]);
}

/// Collision + existing Blocker don't double-count — each opponent Digimon
/// appears exactly once in the candidate list even when both paths
/// (native Blocker keyword + Collision grant) would qualify it.
#[test]
fn collision_with_blocker_keyword_does_not_double_list() {
    use digimon_engine::action::space::encode_attack;

    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_digimon(atk, def, false);
    let sel = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, 0), encode_attack(0, 1)],
        "each defender emitted once despite dual qualification"
    );
}

/// Suspended Blocker-keyword Digimon are NOT valid blocker candidates.
#[test]
fn suspended_blocker_is_not_a_candidate() {
    let mut r = runner_with_attacker_vs(3000, 5000);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    // Suspend the blocker.
    r.game.players[1].battle_area[blk.index as usize].is_suspended = true;

    // No valid blocker candidates → attack runs synchronously.
    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
}
