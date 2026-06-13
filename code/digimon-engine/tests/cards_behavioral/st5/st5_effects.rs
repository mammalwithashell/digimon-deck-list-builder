//! ST5 mid/top-end effects: keyword grants, Security Attack, and Digi-Burst.

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn vanilla(id: &str, name: &str, dp: i32, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.dp = Some(dp);
    card.level = Some(level);
    card
}

#[test]
fn st5_09_when_digivolving_grants_blocker_to_one_own_digimon() {
    let ally = vanilla("ST5-EFFECTS-ALLY", "ST5 Effects Ally", 3000, 4);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-09")
        .expect("ST5-09 YAML parses")
        .add_card(ally)
        .build();
    let metalgreymon = runner.place_on_field(0, "ST5-09", Some(0));
    let ally = runner.place_on_field(0, "ST5-EFFECTS-ALLY", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(metalgreymon),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let pick = runner
        .pending_selection_view()
        .expect("ST5-09 target selection")
        .valid_action_ids[1];
    runner
        .execute_action(0, pick)
        .expect("select ally for ST5-09 Blocker grant");

    assert!(runner.game.has_keyword(ally, Keyword::Blocker));
    runner.end_turn();
    assert!(
        runner.game.has_keyword(ally, Keyword::Blocker),
        "Blocker lasts through the opponent's turn"
    );
    runner.end_turn();
    assert!(
        !runner.game.has_keyword(ally, Keyword::Blocker),
        "Blocker expires at the end of the opponent's turn"
    );
}

#[test]
fn st5_12_when_digivolving_grants_reboot_to_up_to_two_own_digimon() {
    let ally_a = vanilla("ST5-REBOOT-A", "ST5 Reboot A", 3000, 4);
    let ally_b = vanilla("ST5-REBOOT-B", "ST5 Reboot B", 3000, 4);
    let ally_c = vanilla("ST5-REBOOT-C", "ST5 Reboot C", 3000, 4);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-12")
        .expect("ST5-12 YAML parses")
        .add_card(ally_a)
        .add_card(ally_b)
        .add_card(ally_c)
        .build();
    let machinedramon = runner.place_on_field(0, "ST5-12", Some(0));
    let ally_a = runner.place_on_field(0, "ST5-REBOOT-A", Some(0));
    let ally_b = runner.place_on_field(0, "ST5-REBOOT-B", Some(0));
    let ally_c = runner.place_on_field(0, "ST5-REBOOT-C", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(machinedramon),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    );
    let first = runner.pending_selection_view().unwrap().valid_action_ids[1];
    runner
        .execute_action(0, first)
        .expect("pick first Reboot target");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    );
    let second = runner.pending_selection_view().unwrap().valid_action_ids[1];
    runner
        .execute_action(0, second)
        .expect("pick second Reboot target");
    runner.auto_resolve().expect("finish ST5-12 selection");

    assert!(runner.game.has_keyword(ally_a, Keyword::Reboot));
    assert!(runner.game.has_keyword(ally_b, Keyword::Reboot));
    assert!(!runner.game.has_keyword(ally_c, Keyword::Reboot));
}

#[test]
fn st5_13_has_printed_security_attack_plus_one() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-13")
        .expect("ST5-13 YAML parses")
        .build();
    let blitzgreymon = runner.place_on_field(0, "ST5-13", Some(0));

    assert!(runner
        .game
        .has_keyword(blitzgreymon, Keyword::SecurityAttackPlus(1)));
    assert_eq!(runner.game.security_attack_keyword_bonus(blitzgreymon), 1);
}

#[test]
fn st5_13_digi_burst_two_trashes_sources_and_buffs_one_own_digimon() {
    let source_a = vanilla("ST5-BURST-A", "ST5 Burst A", 1000, 3);
    let source_b = vanilla("ST5-BURST-B", "ST5 Burst B", 1000, 4);
    let target = vanilla("ST5-BURST-TARGET", "ST5 Burst Target", 5000, 5);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-13")
        .expect("ST5-13 YAML parses")
        .add_card(source_a)
        .add_card(source_b)
        .add_card(target)
        .build();
    let blitzgreymon = runner.place_stack(0, &["ST5-BURST-A", "ST5-BURST-B", "ST5-13"]);
    let target = runner.place_on_field(0, "ST5-BURST-TARGET", Some(0));

    assert!(
        runner
            .game
            .activate_field_main(0, blitzgreymon.index as usize),
        "ST5-13 Main Digi-Burst effect should activate"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        })
    );
    runner
        .execute_action(
            0,
            encode_source_select(blitzgreymon.index as u16, 0).expect("first source pick"),
        )
        .expect("pick first source");
    runner
        .execute_action(
            0,
            encode_source_select(blitzgreymon.index as u16, 1).expect("second source pick"),
        )
        .expect("pick second source");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let target_pick = runner.pending_selection_view().unwrap().valid_action_ids[1];
    runner
        .execute_action(0, target_pick)
        .expect("select DP buff target");

    assert_eq!(
        runner.game.player(0).battle_area[blitzgreymon.index as usize]
            .card_sources
            .len(),
        1,
        "Digi-Burst 2 trashes both selected sources"
    );
    assert_eq!(runner.game.player(0).trash.len(), 2);
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        4000
    );

    runner.end_turn();
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        4000,
        "DP buff lasts through the opponent's turn"
    );
    runner.end_turn();
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        0,
        "DP buff expires at the end of the opponent's turn"
    );
}

#[test]
fn st5_12_reboot_grant_can_be_declined_without_targets() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-12")
        .expect("ST5-12 YAML parses")
        .build();
    let machinedramon = runner.place_on_field(0, "ST5-12", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(machinedramon),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    );
    assert!(
        runner
            .pending_selection_view()
            .expect("ST5-12 optional-zero selection")
            .is_optional,
        "up to 2 means the player may choose zero targets"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline Reboot grant");
    assert!(!runner.game.has_keyword(machinedramon, Keyword::Reboot));
}
