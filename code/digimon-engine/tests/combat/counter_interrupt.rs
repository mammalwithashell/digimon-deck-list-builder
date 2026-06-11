//! Counter interrupt tests — blast digivolve path through
//! `try_enter_counter` and `execute_blast_digivolve`.
//!
//! Scenarios covered:
//!   - no blast candidates → synchronous attack
//!   - blast card but no valid digivolve target (level/color mismatch) → skip
//!   - valid (hand, field) pair → CounterTiming prompt installed
//!   - mask for defender emits the `encode_digivolve` bits + PASS
//!   - attacker sees empty mask during CounterTiming
//!   - decline: card stays in hand, attack advances to BlockOpen
//!   - declare: card stack-pushes, `WhenDigivolving` fires (+1 memory via
//!     the pilot), attack continues against the original target
//!   - attacker deleted during Counter → state machine skips BlockOpen/Battle
//!     and lands in Cleanup; no AttackerWins is claimed
//!   - Vortex bypasses CounterTiming
//!   - Counter → Block sequence (declare Counter then Block fires next)
//!   - Player-target attack skips Counter entirely (Python parity)
//!   - wrong player cannot resolve counter selection

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{decode_digivolve, encode_digivolve, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword};

/// Vanilla Digimon fixture: `level` + `dp` + a single `Red` color.
fn dgmn(card_id: &str, level: u8, dp: i32) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
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

/// Digimon fixture with a specific evo_cost — needed so `can_digivolve`
/// accepts a blast card when digivolving from a base of the given color/level.
fn blast_card(card_id: &str, level: u8, base_level: u8, base_color: u8) -> CardData {
    let mut card = dgmn(card_id, level, 5000);
    card.evo_costs.push(EvoCost {
        card_color: base_color,
        level: base_level,
        memory_cost: 0,
    });
    card
}

fn native_blast_card(card_id: &str, level: u8, base_level: u8, base_color: u8) -> CardData {
    let mut card = blast_card(card_id, level, base_level, base_color);
    card.keywords.push(Keyword::BlastDigivolve);
    card
}

/// Build the runner with a blast-capable defender and a fresh attacker
/// already on the field.
fn scenario_with_blast_in_hand(
    blast_id: &str,
    base_level: u8,
    base_color: u8,
    attacker_dp: i32,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = DebugRunner::builder()
        // TEST-013 / TEST-014 behave as blast-digivolve candidates thanks
        // to the `.blast_digivolve()` flag in the registered effect.
        .add_card(blast_card(blast_id, base_level + 1, base_level, base_color))
        .add_card(dgmn("ATK", 4, attacker_dp))
        .add_card(dgmn("BASE", base_level, 3000))
        .hand(1, &[blast_id])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    // Defender base Digimon (blast target) on field.
    let _base = r.place_on_field(1, "BASE", Some(0));
    (r, atk)
}

// ───────────────────────────────────────────────────────────────────────────

#[test]
fn no_blast_candidates_attack_resolves_synchronously() {
    let mut r = DebugRunner::builder()
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("DEF", 4, 3000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn blast_card_without_valid_target_skips_counter() {
    // Blast wants base_level=5 Red, but only a level-3 Red is on field →
    // can_digivolve returns false → no prompt installed.
    let mut r = DebugRunner::builder()
        .add_card(blast_card("TEST-013", 6, 5, 0 /* Red */))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        .hand(1, &["TEST-013"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _base = r.place_on_field(1, "BASE", Some(0));

    let result = r.attack_digimon(atk, r.perm_handle(1, 0), false);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn counter_prompt_installs_with_valid_pair() {
    let (mut r, _atk) = scenario_with_blast_in_hand("TEST-013", 3, 0, 5000);
    let target = r.perm_handle(1, 0);
    let atk = r.perm_handle(0, 0);

    let result = r.attack_digimon(atk, target, false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("CounterTiming installs a selection");
    assert_eq!(sel.selecting_player, 1, "defender chooses");
    assert!(sel.is_optional);
    // Only pair: hand_idx 0 × field_idx 0.
    assert_eq!(sel.valid_action_ids, vec![encode_digivolve(0, 0)]);
}

/// Q18 (G-BLAST-DIGIVOLVE-IMMUNITY) — a base Digimon immune to ALL Digimon
/// effects, INCLUDING its own controller's (a bare `CannotBeAffected` = the
/// `Any` filter), cannot `<Blast Digivolve>` — Blast Digivolve is itself a
/// Digimon effect. With the immunity installed, the otherwise-valid
/// (hand, field) blast pair is suppressed at counter-candidate collection, so
/// no Counter prompt installs and the attack resolves synchronously. (Models
/// Quantumon LM-020's "isn't affected by any Digimon's effects, including its
/// own".)
#[test]
fn blast_target_immune_to_own_effects_is_not_a_counter_candidate() {
    let (mut r, _atk) = scenario_with_blast_in_hand("TEST-013", 3, 0, 5000);
    let target = r.perm_handle(1, 0);
    let atk = r.perm_handle(0, 0);

    // Sanity: without immunity this pair WOULD install a Counter prompt
    // (asserted by `counter_prompt_installs_with_valid_pair`). Install an
    // unconditional `CannotBeAffected` (Any) on the blast base.
    r.game.modifiers.add(
        target,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotBeAffected,
            0,
            Expiry::Permanent,
            target.player,
        ),
    );

    let result = r.attack_digimon(atk, target, false);
    assert_eq!(
        result,
        AttackResult::AttackerWins,
        "immune base is not a blast candidate → no Counter window, attack resolves synchronously"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no Counter prompt should install for an effect-immune blast base"
    );
}

#[test]
fn native_blast_digivolve_keyword_installs_counter_candidate() {
    let mut r = DebugRunner::builder()
        .add_card(native_blast_card("NATIVE-BLAST", 4, 3, 0 /* Red */))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        .hand(1, &["NATIVE-BLAST"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let target = r.place_on_field(1, "BASE", Some(0));

    let result = r.attack_digimon(atk, target, false);

    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("native BlastDigivolve keyword should install Counter selection");
    assert_eq!(sel.valid_action_ids, vec![encode_digivolve(0, 0)]);
}

#[test]
fn counter_mask_emits_digivolve_bits_plus_pass_for_defender() {
    let (mut r, atk) = scenario_with_blast_in_hand("TEST-013", 3, 0, 5000);
    r.attack_digimon(atk, r.perm_handle(1, 0), false);

    let mask = build_action_mask(&r.game, 1);
    assert_eq!(mask[encode_digivolve(0, 0) as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);
    let legal_for_attacker: Vec<usize> = build_action_mask(&r.game, 0)
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    assert!(
        legal_for_attacker.is_empty(),
        "attacker's mask must be empty during CounterTiming, got {:?}",
        legal_for_attacker
    );
}

#[test]
fn counter_declined_advances_to_block_and_preserves_hand() {
    let (mut r, atk) = scenario_with_blast_in_hand("TEST-013", 3, 0, 5000);
    let hand_before = r.hand_size(1);
    r.attack_digimon(atk, r.perm_handle(1, 0), false);
    r.game
        .resolve_selection(1, PASS)
        .expect("PASS declines counter");

    // No Blocker keyword → BlockOpen auto-advances to Battle.
    assert!(r.game.pending_attack.is_none(), "attack terminated");
    assert_eq!(r.hand_size(1), hand_before, "blast card stays in hand");
    // ATK 5000 > BASE 3000 → attacker wins, BASE deleted.
    assert_eq!(r.battle_area_size(1), 0);
}

#[test]
fn counter_declared_stacks_card_and_fires_when_digivolving() {
    // Use a weaker attacker (3000) so the post-blast permanent (5000) wins
    // the battle outright — this lets us inspect the stacked state after
    // the attack terminates.
    let (mut r, atk) = scenario_with_blast_in_hand("TEST-013", 3, 0, /* atk_dp = */ 3000);
    let hand_before = r.hand_size(1);
    let memory_before = r.memory();
    let target = r.perm_handle(1, 0);

    r.attack_digimon(atk, target, false);
    r.game
        .resolve_selection(1, encode_digivolve(0, 0))
        .expect("declare blast");

    assert!(r.game.pending_attack.is_none(), "attack terminated");
    // Card moved hand → stack (still present because stacked permanent
    // won the battle and survived).
    assert_eq!(r.hand_size(1), hand_before - 1);
    assert_eq!(
        r.game.players[1].battle_area[0].card_sources.len(),
        2,
        "blast card stacked on base — digivolution stack size = 2"
    );
    // WhenDigivolving fired — pilot grants +1 memory to the defender,
    // moving the gauge to -1 from P0's perspective. With the attack fully
    // resolved, P0's turn ends (post-attack `check_turn_end`) and the
    // gauge seesaws to +1 for the new active player P1.
    assert_eq!(
        r.turn_player(),
        1,
        "memory crossed to the defender's side, so P0's turn ends"
    );
    assert_eq!(r.memory(), memory_before + 1);
    // Attacker deleted by defender's stacked Digimon.
    assert_eq!(r.battle_area_size(0), 0);
}

#[test]
fn counter_that_deletes_attacker_skips_block_and_battle() {
    // TEST-014's WhenDigivolving deletes the attacker. After blast resolves,
    // advance should land in Cleanup; ATK is already gone so no battle runs.
    let mut r = DebugRunner::builder()
        .add_card(blast_card("TEST-014", 4, 3, 0 /* Red */))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        .hand(1, &["TEST-014"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let target = r.place_on_field(1, "BASE", Some(0));

    r.attack_digimon(atk, target, false);
    r.game
        .resolve_selection(1, encode_digivolve(0, 0))
        .expect("declare blast with attacker-delete payload");

    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.battle_area_size(0), 0, "attacker deleted by blast");
    assert_eq!(
        r.battle_area_size(1),
        1,
        "defender keeps the stacked blast permanent"
    );
}

#[test]
fn vortex_bypasses_counter() {
    let (mut r, atk) = scenario_with_blast_in_hand("TEST-013", 3, 0, 5000);
    let result = r.attack_digimon(atk, r.perm_handle(1, 0), /* vortex = */ true);
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn counter_then_block_sequence() {
    // Defender has both a blast card in hand AND a Blocker Digimon on field.
    // Declining Counter → BlockTiming prompt fires next.
    let mut r = DebugRunner::builder()
        .add_card(blast_card("TEST-013", 4, 3, 0))
        .add_card(dgmn("ATK", 4, 5000))
        .add_card(dgmn("BASE", 3, 3000))
        .add_card(dgmn("BLK", 4, 9000))
        .hand(1, &["TEST-013"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let base = r.place_on_field(1, "BASE", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_digimon(atk, base, false);
    // First prompt: CounterTiming.
    assert_eq!(r.current_phase(), GamePhase::CounterTiming);

    r.game.resolve_selection(1, PASS).expect("decline counter");

    // Second prompt: BlockTiming.
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("BlockTiming installed after Counter declined");
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    assert_eq!(sel.selecting_player, 1);
}

#[test]
fn player_target_attack_skips_counter() {
    let mut r = DebugRunner::builder()
        .add_card(blast_card("TEST-013", 4, 3, 0))
        .add_card(dgmn("ATK", 4, 9000))
        .add_card(dgmn("BASE", 3, 3000))
        .add_card(dgmn("SEC", 4, 1))
        .hand(1, &["TEST-013"])
        .security(1, &["SEC"])
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _base = r.place_on_field(1, "BASE", Some(0));

    // Player-target attack — Counter should be skipped (Python parity).
    r.attack_player(atk, 1, false);

    // Either synchronously completed (no blockers either) or parked on
    // BlockOpen. Either way the current phase is NOT CounterTiming.
    assert_ne!(
        r.current_phase(),
        GamePhase::CounterTiming,
        "Player-target attacks skip Counter"
    );
    assert!(
        r.game.pending_attack.is_none() || r.current_phase() == GamePhase::BlockTiming,
        "expected attack to be done or parked on Block"
    );
}

#[test]
fn wrong_player_cannot_resolve_counter() {
    let (mut r, atk) = scenario_with_blast_in_hand("TEST-013", 3, 0, 5000);
    r.attack_digimon(atk, r.perm_handle(1, 0), false);

    use digimon_engine::selection::SelectionError;
    let err = r
        .game
        .resolve_selection(0, PASS)
        .expect_err("attacker may not answer the defender's counter prompt");
    assert_eq!(err, SelectionError::WrongPlayer);
}

#[test]
fn encode_decode_digivolve_round_trip_for_action_ids() {
    // Sanity: confirm that the action ID shape we install matches the
    // decode expected by the callback.
    let action = encode_digivolve(0, 0);
    let (h, f) = decode_digivolve(action);
    assert_eq!(h, 0);
    assert_eq!(f, 0);
    let action2 = encode_digivolve(5, 3);
    let (h2, f2) = decode_digivolve(action2);
    assert_eq!(h2, 5);
    assert_eq!(f2, 3);
}
