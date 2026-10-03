use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, Keyword};

fn fighter(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

#[derive(Default)]
struct TimingCounts {
    on_attack: u32,
    when_attacking: u32,
    end_of_attack: u32,
    end_of_battle: u32,
}

struct TimingWitness(Arc<Mutex<TimingCounts>>);

impl CardEffect for TimingWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let on_attack = self.0.clone();
        let when_attacking = self.0.clone();
        let end_of_attack = self.0.clone();
        let end_of_battle = self.0.clone();

        vec![
            Effect::on_attack(card)
                .name("count OnAttack")
                .process(move |_ctx| {
                    on_attack.lock().unwrap().on_attack += 1;
                })
                .build(),
            Effect::when_attacking(card)
                .name("count WhenAttacking")
                .process(move |_ctx| {
                    when_attacking.lock().unwrap().when_attacking += 1;
                })
                .build(),
            Effect::end_of_attack(card)
                .name("count EndOfAttack")
                .process(move |_ctx| {
                    end_of_attack.lock().unwrap().end_of_attack += 1;
                })
                .build(),
            Effect::end_of_battle(card)
                .name("count EndOfBattle")
                .process(move |_ctx| {
                    end_of_battle.lock().unwrap().end_of_battle += 1;
                })
                .build(),
        ]
    }
}

#[test]
fn direct_battle_deletes_loser_but_does_not_trigger_piercing() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 12000))
        .add_card(fighter("DEF", 3000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);

    let security_before = r.security_count(1);

    let result = r.battle_digimon(atk, def);

    assert_eq!(result, AttackResult::AttackerWins);
    assert_eq!(r.battle_area_size(1), 0, "defender should be deleted");
    assert_eq!(
        r.security_count(1),
        security_before,
        "direct effect battles must not continue into Piercing security checks"
    );
    assert!(
        r.game.pending_attack.is_none(),
        "direct effect battles must not create a PendingAttack"
    );
}

#[test]
fn direct_battle_fires_end_of_battle_but_no_attack_timings() {
    let counts = Arc::new(Mutex::new(TimingCounts::default()));

    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 12000))
        .add_card(fighter("DEF", 3000))
        .add_card(fighter("WITNESS", 5000))
        .start();
    r.register_effect("WITNESS", Arc::new(TimingWitness(counts.clone())));

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _witness = r.place_on_field(0, "WITNESS", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.battle_digimon(atk, def);

    assert_eq!(result, AttackResult::AttackerWins);
    let counts = counts.lock().unwrap();
    assert_eq!(counts.on_attack, 0, "direct battle must not fire OnAttack");
    assert_eq!(
        counts.when_attacking, 0,
        "direct battle must not fire WhenAttacking"
    );
    assert_eq!(
        counts.end_of_attack, 0,
        "direct battle must not fire EndOfAttack"
    );
    assert_eq!(
        counts.end_of_battle, 1,
        "direct battle should still fire EndOfBattle"
    );
}

#[test]
fn direct_battle_rejects_same_controller_targets() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 12000))
        .add_card(fighter("ALLY", 3000))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));

    let result = r.battle_digimon(atk, ally);

    assert_eq!(result, AttackResult::Invalid);
    assert_eq!(r.battle_area_size(0), 2, "same-controller battle rejected");
    assert_eq!(r.trash_size(0), 0, "no same-controller permanent deleted");
}

// ─── G-ENGINE-BATTLE-COMPARE-SOURCE-COUNT (2026-10-01) ──────────────────────
// `battle_digimon_with(.., BattleComparison::DigivolutionCards)` compares the
// two Digimon's digivolution-card counts instead of DP for THIS battle
// (EX13-076 Imperialdramon: Paladin Mode). DCGO `IBattle.CompareStats`:
// `HasIceclad || CompareDigivolutionCards` → source-count difference.

use digimon_engine::combat::BattleComparison;

#[test]
fn source_count_battle_more_sources_wins_despite_lower_dp() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 3000))
        .add_card(fighter("DEF", 15000))
        .add_card(fighter("SRC", 1000))
        .start();
    let atk = r.place_stack(0, &["SRC", "SRC", "ATK"]);
    let def = r.place_stack(1, &["SRC", "DEF"]);

    let result = r.battle_digimon_with(atk, def, BattleComparison::DigivolutionCards);

    assert_eq!(result, AttackResult::AttackerWins);
    assert_eq!(r.battle_area_size(1), 0, "fewer digivolution cards loses");
    assert_eq!(r.battle_area_size(0), 1, "more digivolution cards survives");
}

#[test]
fn source_count_battle_fewer_sources_loses_despite_higher_dp() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 15000))
        .add_card(fighter("DEF", 3000))
        .add_card(fighter("SRC", 1000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_stack(1, &["SRC", "DEF"]);

    let result = r.battle_digimon_with(atk, def, BattleComparison::DigivolutionCards);

    assert_eq!(result, AttackResult::DefenderWins);
    assert_eq!(r.battle_area_size(0), 0, "0 digivolution cards loses to 1");
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn source_count_battle_equal_counts_delete_both() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 15000))
        .add_card(fighter("DEF", 3000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.battle_digimon_with(atk, def, BattleComparison::DigivolutionCards);

    assert_eq!(result, AttackResult::MutualDestruction);
    assert_eq!(r.battle_area_size(0), 0, "tie (0 vs 0) deletes both");
    assert_eq!(r.battle_area_size(1), 0, "tie (0 vs 0) deletes both");
}

#[test]
fn dp_comparison_is_unchanged_by_default() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 3000))
        .add_card(fighter("DEF", 15000))
        .add_card(fighter("SRC", 1000))
        .start();
    let atk = r.place_stack(0, &["SRC", "SRC", "ATK"]);
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.battle_digimon_with(atk, def, BattleComparison::Dp);

    assert_eq!(result, AttackResult::DefenderWins, "DP battle: 3000 < 15000");
    assert_eq!(r.battle_area_size(0), 0);
}

#[test]
fn source_count_battle_still_fires_end_of_battle() {
    let counts = Arc::new(Mutex::new(TimingCounts::default()));
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 3000))
        .add_card(fighter("DEF", 15000))
        .add_card(fighter("SRC", 1000))
        .add_card(fighter("WITNESS", 5000))
        .start();
    r.register_effect("WITNESS", Arc::new(TimingWitness(counts.clone())));
    let atk = r.place_stack(0, &["SRC", "ATK"]);
    let _witness = r.place_on_field(0, "WITNESS", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let result = r.battle_digimon_with(atk, def, BattleComparison::DigivolutionCards);

    assert_eq!(result, AttackResult::AttackerWins);
    assert_eq!(counts.lock().unwrap().end_of_battle, 1);
}

#[test]
fn tie_battle_with_evade_declined_deletes_both() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 5000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Evade, Expiry::Permanent, 0);

    let _ = r.battle_digimon_with(atk, def, BattleComparison::DigivolutionCards);
    let v = r.pending_selection_view().expect("<Evade> prompt");
    r.execute_action(v.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 0, "attacker deleted");
    assert_eq!(r.battle_area_size(1), 0, "defender deleted");
}

#[test]
fn tie_battle_with_evade_accepted_still_deletes_the_other() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 5000))
        .start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Evade, Expiry::Permanent, 0);

    let _ = r.battle_digimon(atk, def);
    let v = r.pending_selection_view().expect("<Evade> prompt");
    let accept = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != digimon_engine::action::space::PASS)
        .unwrap();
    r.execute_action(v.selecting_player, accept).expect("accept");
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 1, "attacker evaded");
    assert_eq!(r.battle_area_size(1), 0, "defender still deleted by the tie");
}
