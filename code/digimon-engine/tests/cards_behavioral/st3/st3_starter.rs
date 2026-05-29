//! ST3 Starter Deck Heaven's Yellow.
//!
//! Coverage focuses on the mechanics unique to the starter: DP-zero deletion
//! observers, yellow DP reduction, recovery, blocker memory loss, Tamer/security
//! DP support, and Security Attack reduction.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::compiled;

const ST3_IDS: [&str; 16] = [
    "ST3-01", "ST3-02", "ST3-03", "ST3-04", "ST3-05", "ST3-06", "ST3-07", "ST3-08",
    "ST3-09", "ST3-10", "ST3-11", "ST3-12", "ST3-13", "ST3-14", "ST3-15", "ST3-16",
];

const EVENT_TARGET_DP_PROBE: &str = r#"
card: TEST-DP-ZERO-OBSERVER
name: DP Zero Observer
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 1000
traits: [Test]
effects:
  - scope: inherited
    when: on_any_deletion
    condition:
      all_of:
        - event_target_kind: digimon
        - event_target_owner: opponent
        - event_target_dp_lte: 0
    process:
      - gain_memory: 1
"#;

fn yellow_digimon(id: &str, dp: i32, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = level as u16 + 1;
    card.colors = vec![CardColor::Yellow];
    card
}

fn yellow_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.play_cost = 2;
    card.colors = vec![CardColor::Yellow];
    card
}

fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::PlayerBattleAreaAttack {
            player: handle.player,
            attacker: handle,
            card: runner.game.players[handle.player as usize].battle_area[handle.index as usize]
                .top_card()
                .handle(),
        },
    );
    runner.game.drain_effect_queue();
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

#[test]
fn event_target_dp_lte_probe_parses_for_dp_zero_deletion_observers() {
    DebugRunner::builder()
        .from_dsl_yaml(EVENT_TARGET_DP_PROBE)
        .expect("event_target_dp_lte must be accepted for DP-zero deletion observers");
}

#[test]
fn all_st3_cards_are_in_the_embedded_dsl_pack() {
    for card_id in ST3_IDS {
        let card = compiled(card_id);
        assert_eq!(card.card, card_id, "{card_id} must compile from YAML");
    }
}

#[test]
fn st3_vanilla_cards_keep_printed_metadata_and_no_effects() {
    let vanilla = [
        ("ST3-02", CompiledCardKind::Digimon, Some(3), Some(2), Some(3000)),
        ("ST3-03", CompiledCardKind::Digimon, Some(3), Some(3), Some(4000)),
        ("ST3-06", CompiledCardKind::Digimon, Some(4), Some(4), Some(5000)),
        ("ST3-10", CompiledCardKind::Digimon, Some(6), Some(10), Some(12000)),
    ];

    for (card_id, kind, level, cost, dp) in vanilla {
        let card = compiled(card_id);
        assert_eq!(card.kind, kind, "{card_id} kind");
        assert_eq!(card.level, level, "{card_id} level");
        assert_eq!(card.cost, cost, "{card_id} cost");
        assert_eq!(card.dp, dp, "{card_id} DP");
        assert!(
            card.effects.is_empty(),
            "{card_id} is vanilla and must not gain effect clauses"
        );
    }
}

#[test]
fn st3_04_inherited_gains_memory_only_for_opponent_dp_zero_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-04")
        .expect("ST3-04 in pack")
        .dsl_card("ST3-14")
        .expect("ST3-14 in pack")
        .add_card(yellow_digimon("CARRIER", 4000, 4))
        .add_card(yellow_digimon("OPP-2000", 2000, 3))
        .hand(0, &["ST3-14"])
        .memory(0)
        .start();

    runner.place_stack(0, &["ST3-04", "CARRIER"]);
    runner.place_on_field(1, "OPP-2000", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("ST3-14 selection resolves");

    assert_eq!(
        runner.memory(),
        1,
        "Patamon inherited effect must gain 1 memory after an opponent Digimon is deleted at 0 DP"
    );
    assert_eq!(runner.battle_area_size(1), 0, "opponent Digimon deleted");
}

#[test]
fn st3_01_inherited_buffs_carrier_after_opponent_dp_zero_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-01")
        .expect("ST3-01 in pack")
        .dsl_card("ST3-14")
        .expect("ST3-14 in pack")
        .add_card(yellow_digimon("CARRIER", 4000, 4))
        .add_card(yellow_digimon("OPP-2000", 2000, 3))
        .hand(0, &["ST3-14"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["ST3-01", "CARRIER"]);
    runner.place_on_field(1, "OPP-2000", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("ST3-14 selection resolves");

    assert_eq!(
        runner.effective_dp(carrier),
        Some(5000),
        "Tokomon inherited effect gives the carrier +1000 DP for the turn"
    );
}

#[test]
fn st3_05_inherited_when_attacking_gains_memory_at_four_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-05")
        .expect("ST3-05 in pack")
        .dsl_card("ST3-02")
        .expect("ST3-02 in pack")
        .add_card(yellow_digimon("CARRIER", 4000, 4))
        .security(0, &["ST3-02", "ST3-02", "ST3-02", "ST3-02"])
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["ST3-05", "CARRIER"]);

    fire_when_attacking(&mut runner, carrier);

    assert_eq!(runner.memory(), 1, "Angemon inherited effect gains 1 memory");
}

#[test]
fn st3_07_has_blocker_and_loses_two_memory_when_attacking() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-07")
        .expect("ST3-07 in pack")
        .memory(5)
        .start();
    let unimon = runner.place_on_field(0, "ST3-07", Some(0));

    assert!(
        runner.game.has_keyword(unimon, Keyword::Blocker),
        "Unimon must have printed Blocker"
    );
    fire_when_attacking(&mut runner, unimon);
    assert_eq!(runner.memory(), 3, "Unimon loses exactly 2 memory");
}

#[test]
fn st3_08_inherited_when_attacking_reduces_one_opponent_digimon_by_1000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-08")
        .expect("ST3-08 in pack")
        .add_card(yellow_digimon("CARRIER", 7000, 6))
        .add_card(yellow_digimon("OPP", 4000, 4))
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["ST3-08", "CARRIER"]);
    let target = runner.place_on_field(1, "OPP", Some(0));

    fire_when_attacking(&mut runner, carrier);
    runner.auto_resolve().expect("MagnaAngemon target selection");

    assert_eq!(runner.effective_dp(target), Some(3000));
}

#[test]
fn st3_09_when_digivolving_recovers_at_three_or_less_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-09")
        .expect("ST3-09 in pack")
        .add_card(yellow_digimon("BASE", 4000, 4))
        .add_card(yellow_digimon("FILL", 3000, 3))
        .security(0, &["FILL", "FILL", "FILL"])
        .deck(0, &["FILL", "FILL"])
        .memory(0)
        .start();
    let angewomon = runner.place_stack(0, &["BASE", "ST3-09"]);

    fire_when_digivolving(&mut runner, angewomon);
    runner.auto_resolve().expect("Recovery resolves");

    assert_eq!(
        runner.security_count(0),
        4,
        "Angewomon must recover +1 when controller has 3 or less security"
    );
}

#[test]
fn st3_11_when_attacking_reduces_one_opponent_digimon_by_4000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-11")
        .expect("ST3-11 in pack")
        .add_card(yellow_digimon("OPP", 7000, 5))
        .memory(0)
        .start();
    let seraphimon = runner.place_on_field(0, "ST3-11", Some(0));
    let target = runner.place_on_field(1, "OPP", Some(0));

    fire_when_attacking(&mut runner, seraphimon);
    runner.auto_resolve().expect("Seraphimon target selection");

    assert_eq!(runner.effective_dp(target), Some(3000));
}

#[test]
fn st3_12_tamer_buffs_own_security_digimon_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-12")
        .expect("ST3-12 in pack")
        .add_card(yellow_digimon("ATK", 8000, 5))
        .add_card(yellow_digimon("SEC", 7000, 4))
        .security(1, &["SEC"])
        .start();
    let atk = runner.place_on_field(0, "ATK", Some(0));
    runner.place_on_field(1, "ST3-12", Some(0));
    runner.game.turn_player_idx = 0;

    let result = runner.attack_player(atk, 1, false);

    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "T.K. Takaishi should raise P1's 7000-DP security Digimon to 9000 during P0's turn"
    );
}

#[test]
fn st3_13_main_buffs_one_own_digimon_by_3000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-13")
        .expect("ST3-13 in pack")
        .add_card(yellow_digimon("ALLY", 4000, 4))
        .hand(0, &["ST3-13"])
        .memory(0)
        .start();
    let ally = runner.place_on_field(0, "ALLY", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("Heaven's Gate target selection");

    assert_eq!(runner.effective_dp(ally), Some(7000));
}

#[test]
fn st3_13_security_buffs_own_security_digimon_for_the_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-13")
        .expect("ST3-13 in pack")
        .add_card(yellow_digimon("ATK-A", 8000, 5))
        .add_card(yellow_digimon("ATK-B", 8000, 5))
        .add_card(yellow_digimon("SEC", 4000, 4))
        .security(1, &["SEC", "ST3-13"])
        .start();
    let first_attacker = runner.place_on_field(0, "ATK-A", Some(0));
    let second_attacker = runner.place_on_field(0, "ATK-B", Some(0));

    let first = runner.attack_player(first_attacker, 1, false);
    runner.auto_resolve().expect("Heaven's Gate security resolves");
    assert_eq!(
        runner.game.defender_security_dp_adjustment(1),
        5000,
        "Heaven's Gate security should install a +5000 Security Digimon DP modifier"
    );
    let second = runner.attack_player(second_attacker, 1, false);

    assert_eq!(first, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        second,
        AttackResult::AttackerDeletedBySecurity,
        "Heaven's Gate security should raise the next P1 security Digimon by +5000 for the turn"
    );
}

#[test]
fn st3_15_main_reduces_selected_security_attack_by_three() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-15")
        .expect("ST3-15 in pack")
        .add_card(yellow_digimon("OPP", 6000, 5))
        .hand(0, &["ST3-15"])
        .memory(0)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("Holy Flame target selection");

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(opp, ModifierType::SecurityAttackChange),
        -3,
        "Holy Flame should install SecurityAttackChange -3 on the selected Digimon"
    );
}

#[test]
fn st3_16_security_activates_main_dp_reduction() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST3-16")
        .expect("ST3-16 in pack")
        .add_card(yellow_digimon("ATK", 8000, 5))
        .add_card(yellow_digimon("OPP", 10000, 6))
        .security(1, &["ST3-16"])
        .start();
    let _opp = runner.place_on_field(0, "OPP", Some(0));
    let atk = runner.place_on_field(0, "ATK", Some(0));

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::InProgress);
    runner.auto_resolve().expect("Seven Heavens security target selection");

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "Seven Heavens security must activate the -10000 DP main effect and delete a 0-DP Digimon"
    );
}

#[test]
fn st3_12_security_clause_plays_tamer_without_paying_cost() {
    let card = compiled("ST3-12");
    let security = card
        .effects
        .iter()
        .find_map(|effect| match effect {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("ST3-12 must have a security play clause");

    assert!(security.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlayFromSecurity { .. }
    )));
}

#[test]
fn st3_12_field_dp_bonus_lowers_as_face_up_security_dp_aura() {
    let card = compiled("ST3-12");
    let aura = card
        .effects
        .iter()
        .find_map(|effect| match effect {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                ..
            }) => Some((scope, dp_modifier)),
            _ => None,
        })
        .expect("ST3-12 must have a declarative aura for Security Digimon DP");

    assert_eq!(*aura.0, CompiledScope::FaceUp);
    assert_eq!(*aura.1, Some(2000));
}

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = yellow_tamer("T");
}
