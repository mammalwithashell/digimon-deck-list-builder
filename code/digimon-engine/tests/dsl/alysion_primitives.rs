//! Behavioral tests for reusable DSL/engine primitives added for the ALYSION
//! card wave (ST25/ST26/ED01/SD01/SD02/BD01/BD02):
//! - `ModifierType::BattleCompareDigivolutionCards`
//! - `ModifierType::CanAttackUnsuspendedWithFewerSources`
//! - `PredicateSpec::event_target_materials_count_lte`
//! - `AttackTargetSpec::DigimonFewerSources` + `can_attack_unsuspended` on
//!   `force_attack` / `may_attack_now` / `can_attack`
//! - Non-numeric `SourceDp` / `BindingDp` deletion caps unaffected by
//!   `ChangeDPDeleteEffectMaxDP` (Q7566 / Q7568).

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::replacement::ReplacementCause;

#[test]
fn battle_compare_digivolution_cards_modifier_wins_on_sources_not_against_security() {
    let yaml = r#"
card: TEST-CMP
name: Comparemon
kind: digimon
level: 6
color: [blue]
cost: 12
dp: 2000
effects:
  - kind: aura
    target: {}
    active_when: { all_turns: true }
    modifier: BattleCompareDigivolutionCards
    modifier_value: 1
"#;
    let mut big = make_test_card("BIG-SEC", "Bigmon");
    big.dp = Some(15000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(big)
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(1, &["BIG-SEC"])
        .memory(5)
        .start();
    r.game.enter_main_phase();

    // Attacker has 2000 DP and 2 digivolution cards; defender has 15000 DP and 1 digivolution card.
    let attacker = r.place_stack(0, &["FILLER", "FILLER", "TEST-CMP"]);
    let defender = r.place_stack(1, &["FILLER", "BIG-SEC"]);
    r.game.players[1].battle_area[defender.index as usize].is_suspended = true;

    r.game.decode_action(encode_attack(attacker.index as u16, defender.index as u16), 0);
    r.auto_resolve().unwrap();

    // Comparemon wins on digivolution cards (2 > 1) despite lower DP (2000 < 15000).
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.battle_area_size(1), 0);

    // Against a Security Digimon (15000 DP), DP is compared normally, so Comparemon loses.
    r.game.players[0].battle_area[0].is_suspended = false;
    r.game.decode_action(encode_attack(0, SECURITY_TARGET), 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(0), 0);
}

#[test]
fn can_attack_unsuspended_with_fewer_sources_filters_targets_strictly() {
    let yaml = r#"
card: TEST-FEWER
name: Fewermon
kind: digimon
level: 6
color: [blue]
cost: 12
dp: 10000
effects:
  - kind: aura
    target: {}
    active_when: { your_turn: true }
    modifier: CanAttackUnsuspendedWithFewerSources
    modifier_value: 1
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(5)
        .start();
    r.game.enter_main_phase();

    let attacker = r.place_stack(0, &["FILLER", "FILLER", "TEST-FEWER"]); // 2 sources
    let fewer = r.place_stack(1, &["FILLER", "FILLER"]); // 1 source (unsuspended)
    let equal = r.place_stack(1, &["FILLER", "FILLER", "FILLER"]); // 2 sources (unsuspended)
    let more = r.place_stack(1, &["FILLER", "FILLER", "FILLER", "FILLER"]); // 3 sources (unsuspended)

    r.game.tick_declarative_effects();
    let mask = build_action_mask(&r.game, 0);
    assert!(mask[encode_attack(attacker.index as u16, fewer.index as u16) as usize] > 0.5);
    assert!(mask[encode_attack(attacker.index as u16, equal.index as u16) as usize] < 0.5);
    assert!(mask[encode_attack(attacker.index as u16, more.index as u16) as usize] < 0.5);
}

#[test]
fn event_target_materials_count_lte_gates_deletion_observer() {
    let yaml = r#"
card: TEST-OBS
name: Observemon
kind: digimon
level: 6
color: [blue]
cost: 12
dp: 12000
effects:
  - when: on_any_deletion
    once_per_turn: true
    active_when: { all_turns: true }
    trigger_if:
      all_of:
        - event_target_owner: opponent
        - event_target_kind: digimon
        - event_target_materials_count_lte:
            formula:
              source_material_count: {}
    process:
      - trash_top_security: { of: opponent }
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(1, &["FILLER"; 3])
        .memory(5)
        .start();
    r.game.enter_main_phase();

    let _obs = r.place_stack(0, &["FILLER", "TEST-OBS"]); // 1 source
    let opp_more = r.place_stack(1, &["FILLER", "FILLER", "FILLER"]); // 2 sources -> no trigger
    let opp_eq = r.place_stack(1, &["FILLER", "FILLER"]); // 1 source -> triggers

    r.game.delete_permanents_batch(vec![opp_more], ReplacementCause::OpponentEffect);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(1), 3);

    // After opp_more was deleted at index 0, opp_eq shifted to index 0.
    let opp_eq_now = digimon_engine::permanent::PermanentHandle { player: 1, index: 0 };
    let _ = opp_eq;
    r.game.delete_permanents_batch(vec![opp_eq_now], ReplacementCause::OpponentEffect);
    r.auto_resolve().unwrap();
    assert_eq!(r.security_count(1), 2);
}

#[test]
fn force_attack_digimon_fewer_sources_with_unsuspended_option() {
    let yaml = r#"
card: TEST-FORCE
name: ForceOption
kind: option
color: [blue]
cost: 1
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          optional: false
          bind_as: attacker
          filter:
            kind: digimon
            can_attack:
              targets: digimon_fewer_sources
              can_attack_unsuspended: true
          prompt: "Choose attacker"
      - force_attack:
          attacker: attacker
          targets: digimon_fewer_sources
          can_attack_unsuspended: true
          prompt: "Attack opponent Digimon with fewer sources"
"#;
    let mut blue_mon = make_test_card("BLUE-MON", "BlueMon");
    blue_mon.colors = vec![CardColor::Blue];
    blue_mon.dp = Some(10000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(blue_mon)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-FORCE"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(5)
        .start();
    r.game.enter_main_phase();

    let _small = r.place_stack(0, &["BLUE-MON"]); // 0 sources -> cannot attack anyone with fewer
    let big = r.place_stack(0, &["FILLER", "FILLER", "BLUE-MON"]); // 2 sources -> only valid attacker
    let opp_fewer = r.place_stack(1, &["FILLER", "FILLER"]); // 1 source (unsuspended)
    let _opp_eq = r.place_stack(1, &["FILLER", "FILLER", "FILLER"]); // 2 sources (unsuspended)

    r.game.decode_action(PLAY_HAND_START, 0);
    // Only `big` should be offered as attacker.
    let sel = r.pending_selection().expect("attacker prompt");
    assert_eq!(sel.valid_action_ids.len(), 1);
    r.choose_prompt_card("BLUE-MON", None).unwrap();

    // Target prompt: ONLY `opp_fewer` is valid (not `opp_eq`, not player/security).
    let sel2 = r.pending_selection().expect("target prompt");
    assert_eq!(
        sel2.valid_action_ids,
        vec![encode_attack(big.index as u16, opp_fewer.index as u16)]
    );
    let act = sel2.valid_action_ids[0];
    r.game.decode_action(act, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn non_numeric_source_dp_deletion_cap_is_not_boosted_by_change_dp_delete_max() {
    let board_maxer_yaml = r#"
card: TEST-BMAX
name: BoardMaxer
kind: tamer
color: [red]
cost: 3
effects:
  - kind: aura
    active_when: { all_turns: true }
    target: { of: you, zone: [battle_area], kind: digimon }
    modifier: ChangeDPDeleteEffectMaxDP
    modifier_value: 2000
"#;
    let ref_deleter_yaml = r#"
card: TEST-REF-DEL
name: RefDeletermon
kind: digimon
level: 4
color: [red]
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          optional: false
          bind_as: victim
          filter: { kind: digimon, dp_lte: { formula: { source_dp: {} } } }
          prompt: "Delete 1 opponent Digimon with DP <= this Digimon's DP"
      - delete_permanent: { target: victim }
"#;
    let mut opp_6k = make_test_card("OPP-6K", "Opp6k");
    opp_6k.dp = Some(6000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(board_maxer_yaml)
        .unwrap()
        .from_dsl_yaml(ref_deleter_yaml)
        .unwrap()
        .add_card(opp_6k)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-REF-DEL"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    r.game.enter_main_phase();

    r.place_stack(0, &["TEST-BMAX"]);
    r.place_stack(1, &["OPP-6K"]); // 6000 DP > 5000 DP (non-numeric ref must NOT be boosted!)
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn is_event_host_predicate_matches_opponent_digimon_on_source_trash() {
    let observer_yaml = r#"
card: TEST-HOST-OBS
name: HostObsmon
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: on_digivolution_card_trashed
    once_per_turn: true
    active_when: { all_turns: true }
    trigger_if:
      all_of:
        - any_permanent:
            of: opponent
            zone: [battle_area]
            kind: digimon
            is_event_host: true
        - any_of:
            - event_cause: own_effect
            - event_cause: opponent_effect
            - event_cause: cost
    process:
      - draw: { of: you, count: 1 }
"#;
    let stripper_yaml = r#"
card: TEST-STRIP
name: Stripmon
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          optional: false
          bind_as: victim
          filter: { kind: digimon }
          prompt: "Trash bottom source"
      - trash_bottom_sources: { target: victim, count: 1 }
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(observer_yaml)
        .unwrap()
        .from_dsl_yaml(stripper_yaml)
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-STRIP"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    r.game.enter_main_phase();

    r.place_stack(0, &["TEST-HOST-OBS"]);
    r.place_stack(1, &["FILLER", "FILLER"]);
    let hand_before = r.hand_size(0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r.choose_prompt_card("FILLER", None).unwrap();
    // -1 played +1 drawn via is_event_host observer
    assert_eq!(r.hand_size(0), hand_before);
}

#[test]
fn of_permanent_predicate_checks_bound_permanent_dp_threshold() {
    let yaml = r#"
card: TEST-OF-PERM
name: OfPermmon
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          optional: false
          bind_as: boosted
          filter: { kind: digimon }
          prompt: "Choose Digimon to boost"
      - add_dp_modifier: { target: boosted, value: 3000, expiry: end_of_turn }
      - if:
          condition:
            any_permanent:
              of: you
              zone: [battle_area]
              of_permanent: boosted
              dp_gte: 12000
          then:
            - add_modifier:
                target: boosted
                modifier: SecurityAttackChange
                value: 1
                expiry: end_of_turn
"#;
    let mut big = make_test_card("BIG-10K", "Big10k");
    big.dp = Some(10000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(big)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-OF-PERM"])
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .memory(10)
        .start();
    r.game.enter_main_phase();

    let h = r.place_stack(0, &["BIG-10K"]);
    r.game.decode_action(PLAY_HAND_START, 0);
    r.choose_prompt_card("BIG-10K", None).unwrap();
    assert_eq!(r.effective_dp(h), Some(13000));
    assert_eq!(r.game.effective_security_strike(h), 2);
}
