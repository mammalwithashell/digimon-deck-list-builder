//! Carrier battle-result predicate used by BT25-051 Grizzlymon's inherited draw.
//! Card image and BT25_051.cs inspected 2026-10-22. CR 4.3 §§14-2-1 through
//! 14-2-5 distinguish winning, mutual loss, deletion and security battles.
//! The inline predicate below must follow the battling physical carrier, not
//! another copy of the source card or the occupant of a compacted field slot.

use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, SECURITY_TARGET},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const SOURCE: &str = r#"
card: META-BATTLE-SOURCE
name: Battle-result inherited witness
kind: digimon
color: [green]
level: 4
cost: 4
dp: 4000
effects:
  - scope: inherited
    when: on_any_deletion
    once_per_turn: true
    condition: { source_deleted_battle_opponent: true }
    process:
      - draw: { of: you, count: 1 }
"#;

// This deterministic helper permits repeated legal attacks in the same turn.
// No test resets a consumed OPT or fabricates a battle/deletion result.
const REPEAT_HOST: &str = r#"
card: META-BATTLE-HOST
name: Repeated battle host
kind: digimon
color: [green]
level: 6
cost: 10
dp: 9000
effects:
  - when: when_attacking
    process:
      - unsuspend: { target: source }
"#;

const BLOCKER: &str = r#"
card: META-BATTLE-BLOCKER
name: Battle-result blocking host
kind: digimon
color: [green]
level: 6
cost: 10
dp: 9000
effects:
  - kind: grant_keyword
    keyword: Blocker
"#;

fn digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(dp);
    card
}

fn runner(owner: u8) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SOURCE)
        .unwrap()
        .from_dsl_yaml(REPEAT_HOST)
        .unwrap()
        .from_dsl_yaml(BLOCKER)
        .unwrap()
        .add_card(digimon("META-WEAK", 3000))
        .add_card(digimon("META-EQUAL", 9000))
        .add_card(digimon("META-STRONG", 12000))
        .deck(0, &["META-WEAK"; 30])
        .deck(1, &["META-WEAK"; 30])
        .security(0, &["META-WEAK"; 5])
        .security(1, &["META-WEAK"; 5])
        .memory(8)
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.game.enter_main_phase();
    r
}

fn main_action(r: &mut DebugRunner, owner: u8, action: u16) {
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.turn_player(), owner);
    assert_eq!(
        build_action_mask(&r.game, owner)[action as usize],
        1.0,
        "legal main action {action} for player {owner}, phase {:?}, memory {}",
        r.game.current_phase,
        r.memory()
    );
    r.game.decode_action(action, owner);
}

fn suspended_target(r: &mut DebugRunner, owner: u8, id: &str) -> PermanentHandle {
    let h = r.place_on_field(owner, id, Some(0));
    r.game.players[owner as usize].battle_area[h.index as usize].is_suspended = true;
    h
}

fn attack(r: &mut DebugRunner, attacker: PermanentHandle, defender: PermanentHandle) {
    assert_eq!(attacker.player, 1 - defender.player);
    main_action(
        r,
        attacker.player,
        encode_attack(attacker.index as u16, defender.index as u16),
    );
}

fn settled(r: &DebugRunner) {
    assert!(
        r.game.pending_selection.is_none(),
        "mandatory draw creates no choice"
    );
    assert!(
        r.game.pending_attack.is_none(),
        "attack completely resolved"
    );
}

fn next_main(r: &mut DebugRunner) {
    let old_owner = r.game.turn_player();
    main_action(r, old_owner, PASS);
    let owner = 1 - old_owner;
    assert_eq!(r.game.turn_player(), owner);
    if r.game.current_phase == GamePhase::Breeding {
        main_action(r, owner, PASS);
    }
    assert_eq!(r.game.current_phase, GamePhase::Main);
}

#[test]
fn carrier_battle_win_draws_only_for_its_owner_and_only_once_each_turn() {
    for owner in [0, 1] {
        let mut r = runner(owner);
        let host = r.place_stack(owner, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
        let first = suspended_target(&mut r, 1 - owner, "META-WEAK");
        suspended_target(&mut r, 1 - owner, "META-WEAK");
        let hand = r.hand_size(owner);
        let deck = r.deck_size(owner);
        let other = r.hand_size(1 - owner);
        attack(&mut r, host, first);
        settled(&r);
        assert_eq!(r.hand_size(owner), hand + 1);
        assert_eq!(r.deck_size(owner), deck - 1);
        assert_eq!(r.hand_size(1 - owner), other);
        assert_eq!(r.battle_area_size(1 - owner), 1);
        assert!(
            !r.game.players[owner as usize].battle_area[host.index as usize].is_suspended,
            "the host's when-attacking effect unsuspends its source carrier"
        );
        // The remaining defender moved to slot zero after the first deletion.
        attack(
            &mut r,
            host,
            PermanentHandle {
                player: 1 - owner,
                index: 0,
            },
        );
        settled(&r);
        assert_eq!(r.battle_area_size(1 - owner), 0);
        assert_eq!(
            r.hand_size(owner),
            hand + 1,
            "same physical source already spent OPT"
        );

        next_main(&mut r);
        next_main(&mut r);
        let next = suspended_target(&mut r, 1 - owner, "META-WEAK");
        let hand = r.hand_size(owner); // Exclude the normal draw-phase draw.
        attack(&mut r, host, next);
        settled(&r);
        assert_eq!(
            r.hand_size(owner),
            hand + 1,
            "OPT resets through real turn advancement"
        );
    }
}

#[test]
fn separate_physical_source_copies_have_separate_opt_and_do_not_observe_allies() {
    let mut r = runner(0);
    let first = r.place_stack(0, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
    let second = r.place_stack(0, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
    r.place_on_field(0, "META-BATTLE-SOURCE", Some(0)); // Top-card scope is inactive.
    suspended_target(&mut r, 1, "META-WEAK");
    suspended_target(&mut r, 1, "META-WEAK");
    let hand = r.hand_size(0);
    attack(
        &mut r,
        first,
        PermanentHandle {
            player: 1,
            index: 0,
        },
    );
    settled(&r);
    assert_eq!(
        r.hand_size(0),
        hand + 1,
        "other copy must not observe an ally's battle win"
    );
    attack(
        &mut r,
        second,
        PermanentHandle {
            player: 1,
            index: 0,
        },
    );
    settled(&r);
    assert_eq!(
        r.hand_size(0),
        hand + 2,
        "second physical source still has its own OPT"
    );
}

#[test]
fn blocking_carrier_wins_on_opponents_turn_with_two_real_block_choices() {
    for defending_owner in [0, 1] {
        for chosen_index in [0_u16, 1] {
            let attacking_owner = 1 - defending_owner;
            let mut r = runner(attacking_owner);
            let attacker = r.place_on_field(attacking_owner, "META-WEAK", Some(0));
            r.place_stack(
                defending_owner,
                &["META-BATTLE-SOURCE", "META-BATTLE-BLOCKER"],
            );
            r.place_stack(
                defending_owner,
                &["META-BATTLE-SOURCE", "META-BATTLE-BLOCKER"],
            );
            let hand = r.hand_size(defending_owner);
            let attacker_hand = r.hand_size(attacking_owner);
            let security = r.security_count(defending_owner);
            main_action(
                &mut r,
                attacking_owner,
                encode_attack(attacker.index as u16, SECURITY_TARGET),
            );
            assert_eq!(r.game.current_phase, GamePhase::BlockTiming);
            let s = r
                .game
                .pending_selection
                .as_ref()
                .expect("two blockers offer a choice");
            assert_eq!(s.selecting_player, defending_owner);
            assert!(s.is_optional);
            assert_eq!(s.valid_action_ids, vec![100, 101]);
            let mask = build_action_mask(&r.game, defending_owner);
            for (i, value) in mask.iter().enumerate() {
                assert_eq!(
                    *value,
                    if [100, 101, PASS].contains(&(i as u16)) {
                        1.0
                    } else {
                        0.0
                    },
                    "block action {i}"
                );
            }
            assert!(build_action_mask(&r.game, attacking_owner)
                .iter()
                .all(|v| *v == 0.0));
            r.execute_action(defending_owner, 100 + chosen_index)
                .unwrap();
            settled(&r);
            assert_eq!(r.battle_area_size(attacking_owner), 0);
            assert_eq!(
                r.hand_size(defending_owner),
                hand + 1,
                "only chosen carrier wins"
            );
            assert_eq!(r.hand_size(attacking_owner), attacker_hand);
            assert_eq!(r.security_count(defending_owner), security);
            assert!(
                r.game.players[defending_owner as usize].battle_area[chosen_index as usize]
                    .is_suspended
            );
            assert!(
                !r.game.players[defending_owner as usize].battle_area[(1 - chosen_index) as usize]
                    .is_suspended
            );
        }
    }
}

#[test]
fn declining_block_does_not_award_a_battle_win_to_idle_carriers() {
    let mut r = runner(0);
    let attacker = r.place_on_field(0, "META-EQUAL", Some(0));
    r.place_stack(1, &["META-BATTLE-SOURCE", "META-BATTLE-BLOCKER"]);
    r.place_stack(1, &["META-BATTLE-SOURCE", "META-BATTLE-BLOCKER"]);
    let hand = r.hand_size(1);
    main_action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    assert_eq!(build_action_mask(&r.game, 1)[PASS as usize], 1.0);
    r.execute_action(1, PASS).unwrap();
    settled(&r);
    assert_eq!(r.hand_size(1), hand);
    assert_eq!(r.security_count(1), 4);
    assert_eq!(r.battle_area_size(0), 1);
}

#[test]
fn ties_and_losses_never_produce_a_carrier_battle_win() {
    for owner in [0, 1] {
        for (enemy, remaining_opponents) in [("META-EQUAL", 0), ("META-STRONG", 1)] {
            let mut r = runner(owner);
            let host = r.place_stack(owner, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
            let opponent = suspended_target(&mut r, 1 - owner, enemy);
            let hand = r.hand_size(owner);
            attack(&mut r, host, opponent);
            settled(&r);
            assert_eq!(r.battle_area_size(owner), 0);
            assert_eq!(r.battle_area_size(1 - owner), remaining_opponents);
            assert_eq!(
                r.hand_size(owner),
                hand,
                "both lose a tie; a loss is not a win"
            );
        }
    }
}

#[test]
fn security_battle_does_not_satisfy_battle_opponent_deletion() {
    for owner in [0, 1] {
        let mut r = runner(owner);
        let host = r.place_stack(owner, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
        let hand = r.hand_size(owner);
        main_action(
            &mut r,
            owner,
            encode_attack(host.index as u16, SECURITY_TARGET),
        );
        settled(&r);
        assert_eq!(r.security_count(1 - owner), 4);
        assert_eq!(
            r.hand_size(owner),
            hand,
            "security Digimon are not deleted in battle"
        );
        assert_eq!(r.battle_area_size(owner), 1);
    }
}

#[test]
fn nonbattle_deletion_does_not_satisfy_predicate_or_spend_opt() {
    for cause in [
        ReplacementCause::OwnEffect,
        ReplacementCause::OpponentEffect,
        ReplacementCause::Cost,
    ] {
        let mut r = runner(0);
        let host = r.place_stack(0, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
        let other = suspended_target(&mut r, 1, "META-WEAK");
        let hand = r.hand_size(0);
        r.game.delete_permanent_with_cause(other, cause);
        settled(&r);
        assert_eq!(r.battle_area_size(1), 0);
        assert_eq!(
            r.hand_size(0),
            hand,
            "nonbattle deletion cannot satisfy battle cause"
        );
        let real_opponent = suspended_target(&mut r, 1, "META-WEAK");
        attack(&mut r, host, real_opponent);
        settled(&r);
        assert_eq!(
            r.hand_size(0),
            hand + 1,
            "failed predicate leaves OPT unspent"
        );
    }
}

#[test]
fn battle_opponent_and_source_identity_survive_field_compaction() {
    let mut r = runner(0);
    let prefix = r.place_on_field(0, "META-WEAK", Some(0));
    let old_host = r.place_stack(0, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
    let host_card = r.top_card(old_host);
    r.place_stack(0, &["META-BATTLE-SOURCE", "META-BATTLE-HOST"]);
    r.game
        .delete_permanent_with_cause(prefix, ReplacementCause::OwnEffect);
    let host = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(r.top_card(host), host_card);
    let opponent = suspended_target(&mut r, 1, "META-WEAK");
    let neighbor = r.place_on_field(1, "META-STRONG", Some(0));
    let neighbor_card = r.top_card(neighbor);
    let hand = r.hand_size(0);
    attack(&mut r, host, opponent);
    settled(&r);
    assert_eq!(r.hand_size(0), hand + 1);
    assert_eq!(
        r.top_card(PermanentHandle {
            player: 1,
            index: 0
        }),
        neighbor_card,
        "the deleted opponent's slot now holds a distinct non-battling Digimon"
    );
}
