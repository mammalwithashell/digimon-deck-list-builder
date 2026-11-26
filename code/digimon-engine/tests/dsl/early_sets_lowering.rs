//! Behavioral lowering tests for constructs used in ST1–ST6 and BT1–BT4:
//! - `predicate:stack_size_gte`
//! - `timing:on_use_option`
//! - `timing:on_block`

use digimon_engine::action::{build_action_mask, encode_attack, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};

#[test]
fn stack_size_gte_predicate_gates_aura_on_total_stack_cards() {
    const AURA_TAMER: &str = r#"
card: TEST-STACK-GTE-TAMER
name: Stack Gte Tamer
kind: tamer
color: [red]
cost: 4
effects:
  - text: "[Your Turn] All of your red Digimon with 3 or more digivolution cards gain <Security A. +1>."
    kind: aura
    active_when: { your_turn: true }
    target:
      owner: you
      zone: [battle_area]
      kind: digimon
      color_is: red
      stack_size_gte: 4
    grant_keyword: { keyword: SecurityAttackPlus, value: 1 }
"#;
    let mut red3 = make_test_card("RED-LV3", "Red Lv3");
    red3.colors = vec![CardColor::Red];
    red3.level = Some(3);
    let mut red4 = make_test_card("RED-LV4", "Red Lv4");
    red4.colors = vec![CardColor::Red];
    red4.level = Some(4);
    let mut red5 = make_test_card("RED-LV5", "Red Lv5");
    red5.colors = vec![CardColor::Red];
    red5.level = Some(5);
    let mut red6 = make_test_card("RED-LV6", "Red Lv6");
    red6.colors = vec![CardColor::Red];
    red6.level = Some(6);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(AURA_TAMER)
        .expect("tamer yaml compiles")
        .add_card(red3)
        .add_card(red4)
        .add_card(red5)
        .add_card(red6)
        .deck(0, &["RED-LV3"; 6])
        .deck(1, &["RED-LV3"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.place_on_field(0, "TEST-STACK-GTE-TAMER", Some(0));
    let small = r.place_stack(0, &["RED-LV3", "RED-LV4", "RED-LV5"]); // stack_size = 3
    let big = r.place_stack(0, &["RED-LV3", "RED-LV4", "RED-LV5", "RED-LV6"]); // stack_size = 4
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(small, Keyword::SecurityAttackPlus(1)),
        "stack_size 3 must not satisfy stack_size_gte: 4"
    );
    assert!(
        r.game.has_keyword(big, Keyword::SecurityAttackPlus(1)),
        "stack_size 4 must satisfy stack_size_gte: 4"
    );
}

#[test]
fn on_use_option_timing_triggers_after_option_resolves() {
    const OBSERVER: &str = r#"
card: TEST-ON-USE-OPT-OBS
name: Option Observer
kind: tamer
color: [purple]
cost: 2
effects:
  - text: "[All Turns] When a player uses an Option card, gain 1 memory."
    when: on_use_option
    active_when: { all_turns: true }
    process:
      - gain_memory: 1
"#;
    const SIMPLE_OPT: &str = r#"
card: TEST-SIMPLE-OPT
name: Simple Opt
kind: option
color: [purple]
cost: 1
effects:
  - text: "[Main] <Draw 1>."
    when: main_from_hand
    process:
      - draw: { of: you, count: 1 }
"#;
    let mut purp = make_test_card("PURP-LV3", "Purp Lv3");
    purp.colors = vec![CardColor::Purple];
    purp.level = Some(3);
    purp.card_kind = CardKind::Digimon;

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(OBSERVER)
        .expect("observer yaml")
        .from_dsl_yaml(SIMPLE_OPT)
        .expect("option yaml")
        .add_card(purp)
        .deck(0, &["PURP-LV3"; 6])
        .deck(1, &["PURP-LV3"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.place_on_field(0, "TEST-ON-USE-OPT-OBS", Some(0));
    let hand_idx = r.add_to_hand(0, "TEST-SIMPLE-OPT");
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[hand_idx], 1.0);
    r.game.decode_action(hand_idx as u16, 0);

    // Paid 1 for option, gained 1 from on_use_option => net 5 memory, and drew 1.
    assert_eq!(r.memory(), 5);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn on_block_timing_fires_when_attacker_is_blocked() {
    const BLOCKED_MON: &str = r#"
card: TEST-ON-BLOCK-MON
name: Blocked Mon
kind: digimon
level: 5
color: [red]
cost: 7
dp: 8000
effects:
  - text: "[Your Turn] When this Digimon is blocked, gain 3 memory."
    when: on_block
    active_when: { your_turn: true }
    condition: { event_permanent_is_source: true }
    process:
      - gain_memory: 3
"#;
    const BLOCKER_MON: &str = r#"
card: TEST-BLOCKER-MON
name: Blocker Mon
kind: digimon
level: 4
color: [black]
cost: 5
dp: 5000
effects:
  - text: "<Blocker>"
    kind: grant_keyword
    keyword: Blocker
"#;
    let filler = make_test_card("FILLER", "Filler");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(BLOCKED_MON)
        .expect("blocked mon yaml")
        .from_dsl_yaml(BLOCKER_MON)
        .expect("blocker yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 2])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.turn_count = 2;
    r.game.set_memory(2);
    r.place_on_field(0, "TEST-ON-BLOCK-MON", Some(0));
    r.place_on_field(1, "TEST-BLOCKER-MON", Some(0));

    let atk = encode_attack(0, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[atk as usize], 1.0);
    r.game.decode_action(atk, 0);

    // Opponent chooses to block with slot 0 (action 100 in BlockTiming).
    assert_eq!(r.game.current_phase, GamePhase::BlockTiming);
    let blk_mask = build_action_mask(&r.game, 1);
    let blk_action = blk_mask
        .iter()
        .enumerate()
        .find(|&(idx, &v)| idx != 62 && v > 0.5)
        .map(|(idx, _)| idx as u16)
        .expect("blocker must have a legal block action");
    r.game.decode_action(blk_action, 1);

    assert_eq!(r.memory(), 5, "on_block must grant +3 memory (2 -> 5)");
}

#[test]
fn digivolved_this_turn_counts_battle_area_and_excludes_breeding() {
    const EGG: &str = r#"
card: TEST-DIGI-TURN-EGG
name: Digi Turn Egg
kind: digi_egg
level: 2
color: [green]
effects:
  - text: "[When Attacking] If you've digivolved this turn, this Digimon gets +1000 DP for the turn."
    scope: inherited
    when: when_attacking
    condition: { digivolved_this_turn: you }
    process:
      - add_dp_modifier: { target: source, value: 1000, expiry: end_of_turn }
"#;
    let mut grn3 = make_test_card("GRN-LV3", "Grn Lv3");
    grn3.colors = vec![CardColor::Green];
    grn3.level = Some(3);
    grn3.dp = Some(3000);
    grn3.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 3,
        level: 2,
        memory_cost: 0,
    }];
    let mut grn4 = make_test_card("GRN-LV4", "Grn Lv4");
    grn4.colors = vec![CardColor::Green];
    grn4.level = Some(4);
    grn4.dp = Some(5000);
    grn4.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 3,
        level: 3,
        memory_cost: 2,
    }];

    // Case 1: Only digivolved in breeding area -> Q870: does NOT count.
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(EGG)
        .expect("egg yaml")
        .add_card(grn3.clone())
        .add_card(grn4.clone())
        .deck(0, &["GRN-LV3"; 6])
        .deck(1, &["GRN-LV3"; 6])
        .security(1, &["GRN-LV3"; 2])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.place_in_breeding(0, "TEST-DIGI-TURN-EGG");
    let hand_lv3 = r.add_to_hand(0, "GRN-LV3");
    let breed_evo = digimon_engine::action::encode_digivolve(
        hand_lv3 as u16,
        digimon_engine::action::BREEDING_TARGET,
    );
    r.game.decode_action(breed_evo, 0);
    assert_eq!(r.game.digivolutions_this_turn[0], 0);

    let attacker = r.place_stack(0, &["TEST-DIGI-TURN-EGG", "GRN-LV3"]);
    r.game.decode_action(encode_attack(0, SECURITY_TARGET), 0);
    assert_eq!(r.dp_of(attacker), Some(3000), "breeding evo must not trigger +1000 DP");

    // Case 2: Digivolve in battle area -> counts and triggers +1000 DP.
    let mut r2 = DebugRunner::builder()
        .from_dsl_yaml(EGG)
        .expect("egg yaml")
        .add_card(grn3)
        .add_card(grn4)
        .deck(0, &["GRN-LV3"; 6])
        .deck(1, &["GRN-LV3"; 6])
        .security(1, &["GRN-LV3"; 2])
        .build();
    r2.set_phase(GamePhase::Main);
    r2.game.turn_count = 2;
    r2.game.set_memory(5);
    let attacker2 = r2.place_field_stack(0, &["TEST-DIGI-TURN-EGG", "GRN-LV3"], false, 0);
    let hand_lv4 = r2.add_to_hand(0, "GRN-LV4");
    let field_evo = digimon_engine::action::encode_digivolve(hand_lv4 as u16, 0);
    r2.game.decode_action(field_evo, 0);
    assert_eq!(r2.game.digivolutions_this_turn[0], 1);

    r2.game.decode_action(encode_attack(0, SECURITY_TARGET), 0);
    assert_eq!(r2.dp_of(attacker2), Some(6000), "battle-area evo triggers +1000 DP (5000 -> 6000)");
}

#[test]
fn cannot_be_blocked_by_no_sources_skips_sourceless_blockers_only() {
    const UNBLOCKABLE_NO_SRC: &str = r#"
card: TEST-UNBLK-NO-SRC
name: Unblockable No Src
kind: digimon
level: 5
color: [blue]
cost: 6
dp: 7000
effects:
  - text: "[Your Turn] This Digimon can't be blocked by your opponent's Digimon with no digivolution cards."
    kind: aura
    active_when: { your_turn: true }
    target: { is_source_permanent: true }
    modifier: CannotBeBlockedByNoSources
"#;
    const BLOCKER: &str = r#"
card: TEST-BLK-UNIT
name: Blocker Unit
kind: digimon
level: 4
color: [black]
cost: 5
dp: 5000
effects:
  - text: "<Blocker>"
    kind: grant_keyword
    keyword: Blocker
"#;
    let filler = make_test_card("FILLER", "Filler");

    // Opponent has only a sourceless blocker -> BlockTiming must NOT open.
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(UNBLOCKABLE_NO_SRC)
        .expect("attacker yaml")
        .from_dsl_yaml(BLOCKER)
        .expect("blocker yaml")
        .add_card(filler.clone())
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 2])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.turn_count = 2;
    r.place_on_field(0, "TEST-UNBLK-NO-SRC", Some(0));
    r.place_on_field(1, "TEST-BLK-UNIT", Some(0));
    r.game.tick_declarative_effects();
    r.game.decode_action(encode_attack(0, SECURITY_TARGET), 0);
    assert_ne!(
        r.game.current_phase,
        GamePhase::BlockTiming,
        "sourceless blocker must not be able to block"
    );

    // Opponent has a sourced blocker (1 digivolution card) -> BlockTiming MUST open.
    let mut r2 = DebugRunner::builder()
        .from_dsl_yaml(UNBLOCKABLE_NO_SRC)
        .expect("attacker yaml")
        .from_dsl_yaml(BLOCKER)
        .expect("blocker yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 2])
        .build();
    r2.set_phase(GamePhase::Main);
    r2.game.turn_count = 2;
    r2.place_on_field(0, "TEST-UNBLK-NO-SRC", Some(0));
    r2.place_stack(1, &["FILLER", "TEST-BLK-UNIT"]);
    r2.game.tick_declarative_effects();
    r2.game.decode_action(encode_attack(0, SECURITY_TARGET), 0);
    assert_eq!(
        r2.game.current_phase,
        GamePhase::BlockTiming,
        "sourced blocker must still be able to block"
    );
}

#[test]
fn name_matches_binding_deletes_all_same_name_opponent_digimon() {
    const SWEEPER: &str = r#"
card: TEST-NAME-SWEEPER
name: Name Sweeper
kind: digimon
level: 7
color: [white]
cost: 15
dp: 15000
effects:
  - text: "[On Play] Choose 1 of your opponent's Digimon. Delete all of your opponent's Digimon that share a name with it."
    when: on_play
    process:
      - select_opponent_permanent:
          optional: false
          filter: { kind: digimon }
          bind_as: chosen
          prompt: "Choose 1 of your opponent's Digimon"
      - delete_all_permanents:
          over: { of: opponent, zone: [battle_area], kind: digimon, name_matches_binding: chosen }
"#;
    let agu = make_test_card("AGU", "Agumon");
    let gabu = make_test_card("GABU", "Gabumon");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SWEEPER)
        .expect("sweeper yaml")
        .add_card(agu)
        .add_card(gabu)
        .deck(0, &["AGU"; 6])
        .deck(1, &["AGU"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(15);
    r.place_on_field(1, "AGU", Some(0));
    r.place_on_field(1, "AGU", Some(0));
    r.place_on_field(1, "GABU", Some(0));

    let hand_idx = r.add_to_hand(0, "TEST-NAME-SWEEPER");
    r.game.decode_action(hand_idx as u16, 0);
    // Pick slot 0 (Agumon)
    let first_action = r.pending_selection().unwrap().valid_action_ids[0];
    r.game.decode_action(first_action, 0);

    // Both Agumons deleted, Gabumon survives!
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "GABU"
    );
}

#[test]
fn arm_digivolve_cost_reducer_respects_from_level_and_to_level() {
    const POTATO: &str = r#"
card: TEST-LV5-LV6-REDUCER
name: Lv5 To Lv6 Reducer
kind: option
color: [green]
cost: 0
effects:
  - text: "[Main] Next green Lv.5->Lv.6 digivolve costs 4 less."
    when: main
    process:
      - arm_digivolve_cost_reducer:
          amount: 4
          single_fire: true
          target_color: green
          from_level: 5
          to_level: 6
          suspend_cost: false
"#;
    let mut g4 = make_test_card("G4", "Green4");
    g4.colors = vec![CardColor::Green];
    g4.level = Some(4);
    let mut g5 = make_test_card("G5", "Green5");
    g5.colors = vec![CardColor::Green];
    g5.level = Some(5);
    g5.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 3,
        level: 4,
        memory_cost: 3,
    }];
    let mut g6 = make_test_card("G6", "Green6");
    g6.colors = vec![CardColor::Green];
    g6.level = Some(6);
    g6.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 3,
        level: 5,
        memory_cost: 4,
    }];

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(POTATO)
        .expect("potato yaml")
        .add_card(g4)
        .add_card(g5)
        .add_card(g6)
        .deck(0, &["G4"; 6])
        .deck(1, &["G4"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.place_on_field(0, "G4", Some(0));

    let opt = r.add_to_hand(0, "TEST-LV5-LV6-REDUCER");
    r.game.decode_action(opt as u16, 0);
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);

    // Lv.4 -> Lv.5 must NOT trigger or consume the reducer (pays full 3 memory: 5 -> 2).
    let h5 = r.add_to_hand(0, "G5");
    r.game
        .decode_action(digimon_engine::action::encode_digivolve(h5 as u16, 0), 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);

    // Lv.5 -> Lv.6 MUST trigger and consume the reducer (cost 4 - 4 = 0: stays at 2 memory).
    let h6 = r.add_to_hand(0, "G6");
    r.game
        .decode_action(digimon_engine::action::encode_digivolve(h6 as u16, 0), 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 0);
}

#[test]
fn in_phase_gates_on_unsuspend_to_main_vs_unsuspend_phase() {
    const MAIN_UNSUSPEND_MON: &str = r#"
card: TEST-MAIN-UNSUSPEND
name: Main Unsuspend Mon
kind: digimon
level: 5
color: [blue]
cost: 7
dp: 7000
effects:
  - text: "[Your Turn] [Once Per Turn] When this Digimon becomes unsuspended during your main phase, gain 1 memory."
    when: on_unsuspend
    active_when: { your_turn: true }
    once_per_turn: true
    trigger_if: { event_permanent_is_source: true }
    condition: { in_phase: main }
    process:
      - gain_memory: 1
"#;
    let filler = make_test_card("FILLER", "Filler");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(MAIN_UNSUSPEND_MON)
        .expect("mon yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .build();
    r.set_phase(GamePhase::Unsuspend);
    r.game.set_memory(2);
    let h = r.place_field_stack(0, &["TEST-MAIN-UNSUSPEND"], true, 0);
    r.game.unsuspend(h);
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 2, "unsuspend during Unsuspend phase must not fire in_phase: main");

    // Suspend again, enter Main phase, unsuspend -> must fire +1 memory.
    r.set_phase(GamePhase::Main);
    r.game.suspend(h);
    r.game.unsuspend(h);
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 3, "unsuspend during Main phase must fire in_phase: main");
}

#[test]
fn attack_target_matches_checks_defender_highest_dp_during_when_attacking() {
    const BWG: &str = r#"
card: TEST-ATK-TGT-BWG
name: Test BWG
kind: digimon
level: 6
color: [black]
cost: 13
dp: 12000
effects:
  - text: "[When Attacking] When you attack your opponent's Digimon with the highest DP, unsuspend this Digimon."
    when: when_attacking
    condition:
      attack_target_matches:
        kind: digimon
        dp_eq: { aggregate: { selector: highest_dp, scope: opponent } }
    process:
      - unsuspend: { target: source }
"#;
    let mut small = make_test_card("OPP-SMALL", "Opp Small");
    small.dp = Some(4000);
    let mut big = make_test_card("OPP-BIG", "Opp Big");
    big.dp = Some(9000);

    // Attacking the lower-DP opponent Digimon -> stays suspended.
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(BWG)
        .expect("bwg yaml")
        .add_card(small.clone())
        .add_card(big.clone())
        .deck(0, &["OPP-SMALL"; 6])
        .deck(1, &["OPP-SMALL"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.turn_count = 2;
    let atk = r.place_on_field(0, "TEST-ATK-TGT-BWG", Some(0));
    r.place_field_stack(1, &["OPP-SMALL"], true, 0);
    r.place_field_stack(1, &["OPP-BIG"], true, 0);
    r.game.decode_action(encode_attack(0, 0), 0);
    assert!(r.game.players[0].battle_area[atk.index as usize].is_suspended);

    // Attacking the highest-DP opponent Digimon -> unsuspends!
    let mut r2 = DebugRunner::builder()
        .from_dsl_yaml(BWG)
        .expect("bwg yaml")
        .add_card(small)
        .add_card(big)
        .deck(0, &["OPP-SMALL"; 6])
        .deck(1, &["OPP-SMALL"; 6])
        .build();
    r2.set_phase(GamePhase::Main);
    r2.game.turn_count = 2;
    let atk2 = r2.place_on_field(0, "TEST-ATK-TGT-BWG", Some(0));
    r2.place_field_stack(1, &["OPP-SMALL"], true, 0);
    r2.place_field_stack(1, &["OPP-BIG"], true, 0);
    r2.game.decode_action(encode_attack(0, 1), 0);
    assert!(
        !r2.game.players[0].battle_area[atk2.index as usize].is_suspended,
        "attacking opponent's highest-DP Digimon must unsuspend attacker"
    );
}

#[test]
fn event_target_name_matches_source_triggers_only_on_same_name_ally_play() {
    const KERAMON: &str = r#"
card: TEST-NAME-MATCH-KERA
name: Keramon
kind: digimon
level: 3
color: [black]
cost: 3
dp: 2000
effects:
  - text: "[Your Turn] When you play another Digimon with the same name as this Digimon, gain 1 memory."
    scope: inherited
    when: on_ally_played
    active_when: { your_turn: true }
    trigger_if:
      event_target_owner: you
      event_target_kind: digimon
      event_target_is_source: false
      event_target_name_matches_source: true
    process:
      - gain_memory: 1
"#;
    let mut diabo = make_test_card("DIABO", "Diaboromon");
    diabo.play_cost = 2;
    let mut other = make_test_card("OTHER", "Agumon");
    other.play_cost = 2;

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(KERAMON)
        .expect("keramon yaml")
        .add_card(diabo)
        .add_card(other)
        .deck(0, &["OTHER"; 6])
        .deck(1, &["OTHER"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.place_stack(0, &["TEST-NAME-MATCH-KERA", "DIABO"]);

    // Playing a differently named Digimon costs 2 -> memory 3 (no trigger).
    let h_other = r.add_to_hand(0, "OTHER");
    r.game.decode_action(h_other as u16, 0);
    assert_eq!(r.memory(), 3);

    // Playing another Diaboromon costs 2 and triggers +1 memory -> memory 2.
    let h_same = r.add_to_hand(0, "DIABO");
    r.game.decode_action(h_same as u16, 0);
    assert_eq!(r.memory(), 2);
}

#[test]
fn aura_modifier_colors_adds_color_for_rules_during_your_turn() {
    const DUAL_MON: &str = r#"
card: TEST-ADD-COLOR-MON
name: Add Color Mon
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
effects:
  - text: "[Your Turn] This Digimon is also treated as yellow."
    kind: aura
    active_when: { your_turn: true }
    target: { is_source_permanent: true }
    modifier: AddColor
    modifier_colors: [yellow]
"#;
    let filler = make_test_card("FILLER", "Filler");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DUAL_MON)
        .expect("mon yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    let h = r.place_on_field(0, "TEST-ADD-COLOR-MON", Some(0));
    r.game.tick_declarative_effects();
    let colors = r.game.players[0].battle_area[h.index as usize].colors_for_rules(
        &r.game.card_data,
        &r.game.modifiers,
        h,
    );
    assert!(colors.contains(&CardColor::Red));
    assert!(colors.contains(&CardColor::Yellow));
}

#[test]
fn may_add_top_security_to_hand_cost_aborts_tail_on_decline_or_empty() {
    const LOP: &str = r#"
card: TEST-LOP-COST
name: Test Lop
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 2000
effects:
  - text: "[On Play] Look at top security. You may add it to hand to <Draw 1>."
    when: on_play
    process:
      - may_add_top_security_to_hand: { of: you, cost: true }
      - draw: { of: you, count: 1 }
"#;
    let filler = make_test_card("FILLER", "Filler");

    // Case 1: Accept -> adds security card + draws 1 (hand +2, sec -1).
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(LOP)
        .expect("lop yaml")
        .add_card(filler.clone())
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 2])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    let h = r.add_to_hand(0, "TEST-LOP-COST");
    r.game.decode_action(h as u16, 0);
    let pick = r.pending_selection().unwrap().valid_action_ids[0];
    r.game.decode_action(pick, 0);
    assert_eq!(r.security_count(0), 1);
    assert_eq!(r.hand_size(0), 2);

    // Case 2: Decline -> does NOT add security and does NOT draw.
    let mut r2 = DebugRunner::builder()
        .from_dsl_yaml(LOP)
        .expect("lop yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 2])
        .build();
    r2.set_phase(GamePhase::Main);
    r2.game.set_memory(5);
    let h2 = r2.add_to_hand(0, "TEST-LOP-COST");
    r2.game.decode_action(h2 as u16, 0);
    r2.game.decode_action(digimon_engine::action::PASS, 0);
    assert_eq!(r2.security_count(0), 2);
    assert_eq!(r2.hand_size(0), 0);
}

#[test]
fn event_deleted_digimon_count_scales_memory_on_simultaneous_deletion() {
    const MALO: &str = r#"
card: TEST-MALO-OBS
name: Test Malo
kind: digimon
level: 6
color: [purple]
cost: 13
dp: 12000
effects:
  - text: "[All Turns] When another Digimon is deleted, gain 1 memory for each Digimon deleted."
    when: on_any_deletion
    active_when: { all_turns: true }
    trigger_if:
      event_target_kind: digimon
      event_target_is_source: false
    process:
      - gain_memory_fn:
          formula:
            base: 0
            per: event_deleted_digimon_count
            delta: 1
"#;
    let filler = make_test_card("FILLER", "Filler");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(MALO)
        .expect("malo yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(1);
    r.place_on_field(0, "TEST-MALO-OBS", Some(0));
    let v1 = r.place_on_field(1, "FILLER", Some(0));
    let v2 = r.place_on_field(1, "FILLER", Some(0));
    let _ = r.game.delete_permanents_batch(
        vec![v1, v2],
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 3, "2 simultaneous deletions must grant +2 memory (1 -> 3)");
}

#[test]
fn digisorption_may_suspend_opponent_allows_suspending_opponent_digimon_once_per_turn() {
    const CERES: &str = r#"
card: TEST-CERES-MON
name: Test Ceres
kind: digimon
level: 6
color: [green]
cost: 12
dp: 12000
effects:
  - text: "<Digisorption -3>"
    kind: grant_keyword
    keyword: Digisorption
    value: 3
  - text: "[Your Turn][OPT] <Digisorption> may suspend opponent's Digimon instead."
    kind: aura
    active_when: { your_turn: true }
    target: { is_source_permanent: true }
    modifier: DigisorptionMaySuspendOpponent
"#;
    let mut g5 = make_test_card("G5", "Green5");
    g5.colors = vec![CardColor::Green];
    g5.level = Some(5);
    let mut ceres_evo = make_test_card("OPP-MON", "Opp Mon");
    ceres_evo.colors = vec![CardColor::Blue];
    ceres_evo.level = Some(4);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(CERES)
        .expect("ceres yaml")
        .add_card(g5)
        .add_card(ceres_evo)
        .deck(0, &["G5"; 6])
        .deck(1, &["OPP-MON"; 6])
        .build();
    // Give TEST-CERES-MON an evo cost from green Lv.5 (cost 5)
    if let Some(cd) = r
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "TEST-CERES-MON")
    {
        cd.evo_costs = vec![digimon_engine::card_data::EvoCost {
            card_color: 3,
            level: 5,
            memory_cost: 5,
        }];
    }
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.place_on_field(0, "TEST-CERES-MON", Some(0)); // already in play (aura active)
    r.place_on_field(0, "G5", Some(0)); // slot 1: digivolve base
    let opp = r.place_on_field(1, "OPP-MON", Some(0));
    r.game.tick_declarative_effects();

    let h = r.add_to_hand(0, "TEST-CERES-MON");
    r.game
        .decode_action(digimon_engine::action::encode_digivolve(h as u16, 1), 0);
    // Accept Digisorption choice (first choice in EffectChoice)
    if let Some(sel) = r.pending_selection() {
        let accept = sel.valid_action_ids[0];
        r.game.decode_action(accept, 0);
    }
    // Now suspend prompt must include opponent slot 0 (encode_attack(1, 0))
    let opp_action = encode_attack(1, 0);
    assert!(
        r.pending_selection()
            .expect("suspend prompt")
            .valid_action_ids
            .contains(&opp_action),
        "DigisorptionMaySuspendOpponent must expose opponent Digimon in suspend prompt"
    );
    r.game.decode_action(opp_action, 0);
    assert!(
        r.game.players[1].battle_area[opp.index as usize].is_suspended,
        "opponent Digimon must be suspended"
    );
    // Cost was 5 - 3 = 2 -> memory 5 -> 3
    assert_eq!(r.memory(), 3);
}

#[test]
fn event_cause_digi_burst_returns_trashed_source_only_on_digi_burst() {
    const ROOKIE: &str = r#"
card: TEST-DB-RETURN
name: Test Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - text: "[Your Turn] When this card is trashed due to activating this Digimon's <Digi-Burst>, return this card to its owner's hand."
    scope: inherited
    when: on_digivolution_card_trashed
    active_when: { your_turn: true }
    condition:
      trashed_source_card_id_is: TEST-DB-RETURN
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_cause: digi_burst
    process:
      - add_to_hand_from_trash: { of: you, card: self }
"#;
    const BURSTER: &str = r#"
card: TEST-DB-HOST
name: Test Burster
kind: digimon
level: 5
color: [red]
cost: 7
dp: 7000
effects:
  - text: "[Main] <Digi-Burst 1>: Gain 1 memory."
    when: main_on_field
    process:
      - digi_burst:
          count: 1
          bind_as: burst_sources
          prompt: "Choose 1 digivolution card to trash for <Digi-Burst 1>"
      - gain_memory: 1
"#;
    let filler = make_test_card("FILLER", "Filler");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(ROOKIE)
        .expect("rookie yaml")
        .from_dsl_yaml(BURSTER)
        .expect("burster yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(2);
    r.place_stack(0, &["TEST-DB-RETURN", "TEST-DB-HOST"]);
    let hand_before = r.game.players[0].hand.len();

    // Activate [Main] Digi-Burst 1 on slot 0.
    let mask = build_action_mask(&r.game, 0);
    let main_act = (digimon_engine::action::FIELD_EFFECT_START
        ..digimon_engine::action::FIELD_EFFECT_END)
        .find(|&a| mask[a as usize] > 0.5)
        .expect("Digi-Burst main action must be legal");
    r.game.decode_action(main_act, 0);
    // Select the TEST-DB-RETURN source card to trash for Digi-Burst.
    if let Some(sel) = r.pending_selection() {
        let pick = sel.valid_action_ids[0];
        r.game.decode_action(pick, 0);
    }
    assert_eq!(r.memory(), 3, "Digi-Burst body must resolve (+1 memory)");
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before + 1,
        "TEST-DB-RETURN must return from trash to hand when trashed by Digi-Burst"
    );
    assert!(
        r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "TEST-DB-RETURN"),
        "TEST-DB-RETURN must be in owner's hand"
    );
    assert!(
        r.game.players[0].trash.is_empty(),
        "TEST-DB-RETURN must no longer remain in trash"
    );
}

#[test]
fn event_played_from_trash_grants_rush_only_to_trash_played_digimon() {
    const ANUBIS: &str = r#"
card: TEST-ANUBIS
name: Test Anubismon
kind: digimon
level: 6
color: [purple]
cost: 11
dp: 10000
effects:
  - text: "[When Digivolving] You may play 1 level 3 Digimon card from your trash without paying its memory cost."
    when: when_digivolving
    optional: true
    condition:
      count_gte:
        filter:
          of: you
          zone: [trash]
          kind: digimon
          level_eq: 3
        n: 1
    process:
      - select_trash:
          of: you
          optional: true
          bind_as: trash_pick
          filter: { kind: digimon, level_eq: 3 }
          prompt: "Choose 1 level 3 Digimon card from your trash to play"
      - play_from_trash_free: { of: you, hand_index: trash_pick }

  - text: "[Your Turn] When you play Digimon from your trash, those Digimon gain <Rush> for the turn."
    when: on_ally_played
    active_when: { your_turn: true }
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_kind: digimon
        - event_played_from_trash: true
    process:
      - grant_keyword:
          target: event_target
          keyword: Rush
          expiry: end_of_turn
"#;
    let mut p5 = make_test_card("P5", "Purp5");
    p5.colors = vec![CardColor::Purple];
    p5.level = Some(5);
    let mut p3 = make_test_card("P3", "Purp3");
    p3.colors = vec![CardColor::Purple];
    p3.level = Some(3);
    p3.play_cost = 2;

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(ANUBIS)
        .expect("anubis yaml")
        .add_card(p5)
        .add_card(p3)
        .deck(0, &["P3"; 6])
        .deck(1, &["P3"; 6])
        .build();
    if let Some(cd) = r
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == "TEST-ANUBIS")
    {
        cd.evo_costs = vec![digimon_engine::card_data::EvoCost {
            card_color: 6,
            level: 5,
            memory_cost: 3,
        }];
    }
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.place_on_field(0, "P5", Some(0));
    let trash_idx = r.add_to_hand(0, "P3");
    let trash_card = r.game.players[0].hand.remove(trash_idx);
    r.game.players[0].put_in_trash(trash_card);

    // Digivolve P5 -> TEST-ANUBIS, which plays P3 from trash.
    let h_anubis = r.add_to_hand(0, "TEST-ANUBIS");
    r.game
        .decode_action(digimon_engine::action::encode_digivolve(h_anubis as u16, 0), 0);
    // Select P3 from trash (first non-pass valid action).
    if let Some(sel) = r.pending_selection() {
        let pick = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .expect("trash pick");
        r.game.decode_action(pick, 0);
    }
    let from_trash = digimon_engine::permanent::PermanentHandle { player: 0, index: 1 };
    assert!(
        r.game.has_keyword(from_trash, Keyword::Rush),
        "Digimon played from trash must gain Rush from event_played_from_trash"
    );

    // Playing a Digimon from hand must NOT gain Rush.
    let h_hand = r.add_to_hand(0, "P3");
    r.game.decode_action(h_hand as u16, 0);
    let from_hand = digimon_engine::permanent::PermanentHandle { player: 0, index: 2 };
    assert!(
        !r.game.has_keyword(from_hand, Keyword::Rush),
        "Digimon played from hand must NOT gain Rush"
    );
}

#[test]
fn attack_target_binding_ref_deletes_attacked_digimon_with_no_sources() {
    const OPT: &str = r#"
card: TEST-DRAG-DEPTHS
name: Test Drag Depths
kind: option
color: [blue]
cost: 1
effects:
  - text: '[Main] All of your Digimon gain "[Your Turn] When attacking an opponent''s Digimon with no digivolution cards, delete that Digimon." for the turn.'
    when: main_from_hand
    process:
      - grant_triggered_effect:
          dynamic: true
          target: { of: you, zone: [battle_area], kind: digimon }
          timing: when_attacking
          expiry: end_of_turn
          body:
            - if:
                condition:
                  all_of:
                    - your_turn: true
                    - attack_target_matches:
                        kind: digimon
                        materials_count_lte: 0
                then:
                  - delete_permanent: { target: attack_target }
"#;
    let mut blue3 = make_test_card("B3", "Blue3");
    blue3.colors = vec![CardColor::Blue];
    blue3.dp = Some(2000);
    let mut big = make_test_card("OPP-BIG", "Opp Big");
    big.dp = Some(12000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(OPT)
        .expect("opt yaml")
        .add_card(blue3)
        .add_card(big)
        .deck(0, &["B3"; 6])
        .deck(1, &["OPP-BIG"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.turn_count = 2;
    r.game.set_memory(5);
    r.place_on_field(0, "B3", Some(0));
    r.place_field_stack(1, &["OPP-BIG"], true, 0);

    let h = r.add_to_hand(0, "TEST-DRAG-DEPTHS");
    r.game.decode_action(h as u16, 0);

    // Weak B3 attacks 12000 DP OPP-BIG (no sources) -> deleted by when_attacking before battle!
    r.game.decode_action(encode_attack(0, 0), 0);
    assert!(
        r.game.players[1].battle_area.is_empty(),
        "attack_target with no sources must be deleted by when_attacking"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "attacker must survive because defender was deleted before battle"
    );
}

#[test]
fn can_only_digivolve_into_name_containing_restricts_digivolve_mask() {
    const LUCEMON: &str = r#"
card: TEST-LUCEMON
name: Lucemon
kind: digimon
level: 3
color: [yellow]
cost: 13
dp: 10000
effects:
  - text: "[All Turns] This Digimon can only digivolve into Digimon cards with [Lucemon] in their name."
    kind: aura
    active_when: { all_turns: true }
    target: { is_source_permanent: true }
    modifier: CanOnlyDigivolveIntoNameContaining
    modifier_name: "Lucemon"
"#;
    let mut non_luce = make_test_card("YEL-LV4", "Angemon");
    non_luce.colors = vec![CardColor::Yellow];
    non_luce.level = Some(4);
    non_luce.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 2,
    }];
    let mut luce_fm = make_test_card("LUCE-FM", "Lucemon: Chaos Mode");
    luce_fm.colors = vec![CardColor::Yellow];
    luce_fm.level = Some(5);
    luce_fm.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 4,
    }];

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(LUCEMON)
        .expect("lucemon yaml")
        .add_card(non_luce)
        .add_card(luce_fm)
        .deck(0, &["YEL-LV4"; 6])
        .deck(1, &["YEL-LV4"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(5);
    r.place_on_field(0, "TEST-LUCEMON", Some(0));
    r.game.tick_declarative_effects();

    let h_non = r.add_to_hand(0, "YEL-LV4");
    let h_luce = r.add_to_hand(0, "LUCE-FM");
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[digimon_engine::action::encode_digivolve(h_non as u16, 0) as usize],
        0.0,
        "Lucemon must NOT be able to digivolve into Angemon"
    );
    assert_eq!(
        mask[digimon_engine::action::encode_digivolve(h_luce as u16, 0) as usize],
        1.0,
        "Lucemon MUST be able to digivolve into Lucemon: Chaos Mode"
    );
}

#[test]
fn own_memory_gte_gates_aura_on_turn_player_memory() {
    const MON: &str = r#"
card: TEST-MEM-GTE
name: Mem Gte Mon
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
effects:
  - text: "[Your Turn] While you have 3 or more memory, this Digimon gets +2000 DP."
    kind: aura
    active_when:
      all_of:
        - your_turn: true
        - own_memory_gte: 3
    target: { is_source_permanent: true }
    dp_modifier: 2000
"#;
    let filler = make_test_card("FILLER", "Filler");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(MON)
        .expect("mon yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    let h = r.place_on_field(0, "TEST-MEM-GTE", Some(0));
    r.game.set_memory(2);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(5000));
    r.game.set_memory(3);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(7000));
}

#[test]
fn stack_size_lte_filters_opponent_digimon_by_total_stack_cards() {
    const MON: &str = r#"
card: TEST-STACK-LTE
name: Stack Lte Mon
kind: digimon
level: 5
color: [blue]
cost: 6
dp: 6000
effects:
  - text: "[On Play] Delete 1 opponent Digimon with 1 or fewer digivolution cards (stack_size <= 2)."
    when: on_play
    process:
      - select_opponent_permanent:
          optional: false
          bind_as: victim
          filter: { kind: digimon, stack_size_lte: 2 }
          prompt: "Delete 1 opponent Digimon with stack_size <= 2"
      - delete_permanent: { target: victim }
"#;
    let filler = make_test_card("FILLER", "Filler");
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(MON)
        .expect("mon yaml")
        .add_card(filler)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.set_memory(10);
    r.place_stack(1, &["FILLER", "FILLER"]); // stack_size = 2 (valid)
    r.place_stack(1, &["FILLER", "FILLER", "FILLER"]); // stack_size = 3 (invalid)
    let h = r.add_to_hand(0, "TEST-STACK-LTE");
    r.game.decode_action(h as u16, 0);
    let sel = r.pending_selection().expect("selection must open for stack_size=2 only");
    assert_eq!(sel.valid_action_ids.len(), 1);
    let pick = sel.valid_action_ids[0];
    r.game.decode_action(pick, 0);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 3);
}

#[test]
fn name_matches_source_checks_other_digimon_with_same_name() {
    const EGG: &str = r#"
card: TEST-NAME-SRC-EGG
name: Test Pagumon
kind: digi_egg
level: 2
color: [purple]
effects:
  - text: "[Your Turn] While you have another Digimon with the same name as this Digimon, this Digimon gets +1000 DP."
    scope: inherited
    kind: aura
    active_when:
      all_of:
        - your_turn: true
        - any_permanent:
            of: you
            zone: [battle_area]
            kind: digimon
            is_source_permanent: false
            name_matches_source: true
    target: { is_source_permanent: true }
    dp_modifier: 1000
"#;
    let mut agu = make_test_card("AGU", "Agumon");
    agu.dp = Some(2000);
    let mut gabu = make_test_card("GABU", "Gabumon");
    gabu.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(EGG)
        .expect("egg yaml")
        .add_card(agu)
        .add_card(gabu)
        .deck(0, &["AGU"; 6])
        .deck(1, &["AGU"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    let host = r.place_stack(0, &["TEST-NAME-SRC-EGG", "AGU"]);
    r.place_on_field(0, "GABU", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(2000));

    r.place_on_field(0, "AGU", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(3000));
}

#[test]
fn event_battle_surviving_deleter_fires_when_ally_wins_battle() {
    const TAMER: &str = r#"
card: TEST-KEN-OBS
name: Test Ken
kind: tamer
color: [green]
cost: 4
effects:
  - text: "[Your Turn] When one of your Digimon deletes an opponent's Digimon in battle and survives, suspend this Tamer to gain 1 memory."
    when: on_ally_won_battle
    active_when: { your_turn: true }
    trigger_if:
      event_battle_surviving_deleter:
        owner: you
    process:
      - gain_memory: 1
"#;
    let mut big = make_test_card("BIG", "Big");
    big.dp = Some(9000);
    let mut small = make_test_card("SMALL", "Small");
    small.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(TAMER)
        .expect("tamer yaml")
        .add_card(big)
        .add_card(small)
        .deck(0, &["BIG"; 6])
        .deck(1, &["SMALL"; 6])
        .build();
    r.set_phase(GamePhase::Main);
    r.game.turn_count = 2;
    r.game.set_memory(2);
    r.place_on_field(0, "TEST-KEN-OBS", Some(0));
    r.place_on_field(0, "BIG", Some(0));
    r.place_field_stack(1, &["SMALL"], true, 0);

    r.game.decode_action(encode_attack(1, 0), 0);
    assert_eq!(r.memory(), 3, "surviving battle deleter must trigger +1 memory");
}



