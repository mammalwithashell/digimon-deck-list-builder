use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, ModifierType};
use digimon_engine::TriggerSource;

const PROTECT: &str = r#"
card: TEST-SCOPED-PROTECTION
name: Scoped protection
kind: digimon
level: 4
color: [yellow]
cost: 0
dp: 6000
effects:
  - when: on_play
    process:
      - add_modifier:
          target: source
          modifier: CannotBeDestroyedByEffect
          value: 0
          expiry: end_of_opponents_turn
          opponent_only: true
"#;
const SECURITY: &str = r#"
card: TEST-SECURITY-TAIL
name: Security tail
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - select_security:
          of: you
          filter: {kind: digimon}
          bind_as: chosen
          optional: true
          continue_on_decline: true
          prompt: Choose security
          then:
            - add_to_hand_from_security: {of: you, card: chosen}
            - gain_memory: 2
      - shuffle_security: {of: you}
      - gain_memory: 1
"#;
fn runner(yaml: &str) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("new reusable syntax")
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fire(r: &mut DebugRunner, card: &str) {
    let h = r.place_stack(0, &[card]);
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
#[test]
fn opponent_scoped_modifier_is_explicit_and_clone_safe() {
    let mut r = runner(PROTECT);
    fire(&mut r, "TEST-SCOPED-PROTECTION");
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    for game in [&r.game, &r.game.clone()] {
        let entries = game
            .modifiers
            .get(h, ModifierType::CannotBeDestroyedByEffect);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].cause_filter,
            Some(digimon_engine::replacement::ReplacementCause::OpponentEffect)
        );
    }
}
#[test]
fn optional_security_decline_runs_only_unconditional_tail_and_clones() {
    let mut r = runner(SECURITY);
    fire(&mut r, "TEST-SECURITY-TAIL");
    r.game.player_mut(0).face_up_security.insert(0);
    let parked = r.game.clone();
    for clone in [false, true] {
        if clone {
            r.game = parked.clone();
        }
        let before = r.game.memory;
        let mask = build_action_mask(&r.game, 0);
        assert!(mask[PASS as usize] > 0.5);
        assert_eq!(mask.iter().filter(|v| **v > 0.5).count(), 4);
        r.execute_action(0, PASS).unwrap();
        assert_eq!(r.game.memory, before + 1);
        assert_eq!(r.game.player(0).security.len(), 3);
        assert!(
            r.game.player(0).face_up_security.is_empty(),
            "decline still shuffles"
        );
    }
}

#[test]
fn scoped_protection_blocks_only_opponent_effect_deletion() {
    use digimon_engine::replacement::ReplacementCause as Cause;
    for cause in [
        Cause::OpponentEffect,
        Cause::OwnEffect,
        Cause::Battle,
        Cause::Cost,
    ] {
        let mut r = runner(PROTECT);
        fire(&mut r, "TEST-SCOPED-PROTECTION");
        let h = digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        };
        r.game.delete_permanent_with_cause(h, cause);
        assert_eq!(
            r.game.player(0).battle_area.len(),
            usize::from(cause == Cause::OpponentEffect),
            "cause={cause:?}"
        );
    }
}
#[test]
fn scoped_protection_does_not_block_return_and_expires() {
    let mut r = runner(PROTECT);
    fire(&mut r, "TEST-SCOPED-PROTECTION");
    let h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(r.game.return_to_hand_from_effect(h, 1));
    let mut r = runner(PROTECT);
    fire(&mut r, "TEST-SCOPED-PROTECTION");
    r.end_turn();
    assert!(!r
        .game
        .modifiers
        .get(h, ModifierType::CannotBeDestroyedByEffect)
        .is_empty());
    r.end_turn();
    assert!(r
        .game
        .modifiers
        .get(h, ModifierType::CannotBeDestroyedByEffect)
        .is_empty());
}
#[test]
fn default_modifier_protects_from_own_and_opponent_effects() {
    use digimon_engine::replacement::ReplacementCause as Cause;
    let yaml = PROTECT.replace("          opponent_only: true\n", "");
    for cause in [Cause::OpponentEffect, Cause::OwnEffect] {
        let mut r = runner(&yaml);
        fire(&mut r, "TEST-SCOPED-PROTECTION");
        let h = digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        };
        r.game.delete_permanent_with_cause(h, cause);
        assert_eq!(r.game.player(0).battle_area.len(), 1);
        assert_eq!(
            r.game
                .modifiers
                .get(h, ModifierType::CannotBeDestroyedByEffect)[0]
                .cause_filter,
            None
        );
    }
}
#[test]
fn scoped_filter_and_continuous_modes_cover_new_entries_and_clones() {
    for continuous in [false, true] {
        let yaml = PROTECT
            .replace("target: source", "target: {of: you, kind: digimon}")
            .replace(
                "          opponent_only: true",
                &format!("          opponent_only: true\n          continuous: {continuous}"),
            );
        let mut r = runner(&yaml);
        r.place_stack(0, &["FILL"]);
        fire(&mut r, "TEST-SCOPED-PROTECTION");
        let h = r.place_stack(0, &["FILL"]);
        r.game.tick_declarative_effects();
        let mut game = r.game.clone();
        for _ in 0..2 {
            game.tick_declarative_effects();
            let entries = game
                .modifiers
                .get(h, ModifierType::CannotBeDestroyedByEffect);
            assert_eq!(!entries.is_empty(), continuous);
            if continuous {
                assert!(entries.iter().all(|e| e.cause_filter
                    == Some(digimon_engine::replacement::ReplacementCause::OpponentEffect)));
                game.delete_permanent_with_cause(
                    h,
                    digimon_engine::replacement::ReplacementCause::OpponentEffect,
                );
                assert_eq!(game.player(0).battle_area.len(), 3);
            }
        }
    }
}
#[test]
fn accepted_security_pick_runs_receipted_then_and_tail_once() {
    let mut r = runner(SECURITY);
    fire(&mut r, "TEST-SECURITY-TAIL");
    let picked = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let parked = r.game.clone();
    for clone in [false, true] {
        if clone {
            r.game = parked.clone();
        }
        let before = r.memory();
        let hand = r.game.player(0).hand.len();
        r.execute_action(0, picked).unwrap();
        assert_eq!(r.memory(), before + 3);
        assert_eq!(r.game.player(0).security.len(), 2);
        assert_eq!(r.game.player(0).hand.len(), hand + 1);
    }
}
#[test]
fn no_eligible_security_skips_pick_reward_but_runs_shuffle_tail_once() {
    let yaml = SECURITY.replace("filter: {kind: digimon}", "filter: {kind: tamer}");
    let mut r = runner(&yaml);
    let before = r.memory();
    r.game.player_mut(0).face_up_security.insert(0);
    fire(&mut r, "TEST-SECURITY-TAIL");
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), before + 1);
    assert_eq!(r.game.player(0).security.len(), 3);
    assert!(r.game.player(0).face_up_security.is_empty());
}
#[test]
fn default_optional_security_decline_still_aborts_tail() {
    let yaml = SECURITY.replace("          continue_on_decline: true\n", "");
    let mut r = runner(&yaml);
    fire(&mut r, "TEST-SECURITY-TAIL");
    r.game.player_mut(0).face_up_security.insert(0);
    let before = r.memory();
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.memory(), before);
    assert!(!r.game.player(0).face_up_security.is_empty());
}
#[test]
fn unsupported_decline_and_cause_modes_fail_validation() {
    for yaml in [
        SECURITY.replace("select_security:", "select_hand:"),
        SECURITY.replace("optional: true", "optional: false"),
        SECURITY.replace("optional: true", "optional: true\n          cost: true"),
        PROTECT.replace("CannotBeDestroyedByEffect", "ChangeDp"),
    ] {
        assert!(
            DebugRunner::builder().from_dsl_yaml(&yaml).is_err(),
            "{yaml}"
        );
    }
}

#[test]
fn reused_security_binding_is_cleared_on_decline_and_empty_candidates() {
    for no_candidates in [false, true] {
        let filter = if no_candidates { "tamer" } else { "digimon" };
        let yaml = format!(
            r#"
card: TEST-SECURITY-TAIL
name: Security binding reuse
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - select_security:
          of: you
          filter: {{kind: digimon}}
          bind_as: chosen
          prompt: First pick
      - select_security:
          of: you
          filter: {{kind: {filter}}}
          bind_as: chosen
          optional: true
          continue_on_decline: true
          prompt: Second pick
      - if:
          condition: {{binding_exists: chosen}}
          then:
            - gain_memory: 9
          else:
            - gain_memory: 1
"#
        );
        let mut r = runner(&yaml);
        fire(&mut r, "TEST-SECURITY-TAIL");
        let before = r.memory();
        let pick = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
        r.execute_action(0, pick).unwrap();
        if !no_candidates {
            assert!(r.game.pending_selection.is_some());
            r.game = r.game.clone();
            r.execute_action(0, PASS).unwrap();
        }
        assert_eq!(r.memory(), before + 1, "no_candidates={no_candidates}");
    }
}
#[test]
fn cross_owner_security_and_selecting_override_keep_nested_tail_once() {
    let yaml = r#"
card: TEST-SECURITY-TAIL
name: Opponent security nested tail
kind: tamer
color: [yellow]
cost: 0
effects:
  - when: on_play
    process:
      - as_selecting_player:
          of: opponent
          body:
            - select_security:
                of: opponent
                filter: {kind: digimon}
                bind_as: chosen
                optional: true
                continue_on_decline: true
                prompt: Opponent picks opponent security
                then:
                  - add_to_hand_from_security: {of: opponent, card: chosen}
                  - gain_memory: 2
            - shuffle_security: {of: opponent}
            - gain_memory: 1
      - if:
          condition: {binding_exists: chosen}
          then:
            - gain_memory: 4
          else:
            - gain_memory: 3
"#;
    for decline in [false, true] {
        let mut r = runner(yaml);
        // Keep identities distinct and preserve owner/controller distinction.
        r.game.player_mut(1).face_up_security.insert(0);
        fire(&mut r, "TEST-SECURITY-TAIL");
        let pending = r.game.pending_selection.as_ref().unwrap();
        assert_eq!(pending.selecting_player, 1);
        assert_eq!(pending.zone_owner, Some(1));
        let action = if decline {
            PASS
        } else {
            pending.valid_action_ids[0]
        };
        assert!(build_action_mask(&r.game, 1)[action as usize] > 0.5);
        assert!(build_action_mask(&r.game, 0).iter().all(|v| *v == 0.0));
        let before = r.memory();
        let hand = r.game.player(1).hand.len();
        let parked = r.game.clone();
        for clone in [false, true] {
            if clone {
                r.game = parked.clone();
            }
            r.execute_action(1, action).unwrap();
            assert_eq!(r.memory(), before + if decline { 4 } else { 7 });
            assert_eq!(r.game.player(1).hand.len(), hand + usize::from(!decline));
            assert_eq!(r.game.player(1).security.len(), if decline { 3 } else { 2 });
            assert!(r.game.player(1).face_up_security.is_empty());
        }
    }
}

#[test]
fn scoped_modifier_and_security_tail_round_trip_in_current_pack() {
    let cards = [PROTECT, SECURITY]
        .iter()
        .map(|yaml| {
            let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
            digimon_dsl::compile::compile(&spec).unwrap()
        })
        .collect();
    let pack =
        digimon_dsl::pack::CardPack::from_compiled_cards("scoped-protection-security-tail", cards);
    assert_eq!(pack.manifest.pack_version, digimon_dsl::pack::PACK_FORMAT_VERSION);
    let restored = digimon_dsl::pack::CardPack::from_bytes(&pack.to_bytes().unwrap()).unwrap();
    assert_eq!(restored, pack);
}
