//! CR 4.3 11-2-4: immediate effect attack offers cannot nest declarations.
use digimon_engine::{
    action::{build_action_mask, space::PASS},
    combat::AttackResult,
    debug_runner::{make_test_card, DebugRunner},
    enums::GamePhase,
    selection::{AttackState, AttackTarget, PendingAttack},
};
const YAML: &str = r#"
card: TEST-EFFECT-ATTACK-GUARD
name: Effect attack guard
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - may_attack_now:
          attacker: source
          targets: player
          optional: true
          without_suspending: false
          ignore_summoning_sickness: false
"#;
fn setup(player: u8, without_suspending: bool, ignore_sickness: bool) -> DebugRunner {
    let yaml = YAML.replace("without_suspending: false", &format!("without_suspending: {without_suspending}"))
        .replace("ignore_summoning_sickness: false", &format!("ignore_summoning_sickness: {ignore_sickness}"));
    let mut r = DebugRunner::builder().from_dsl_yaml(&yaml).unwrap()
        .add_card(make_test_card("TEST-OTHER-ATTACKER", "Other attacker"))
        .add_card(make_test_card("TEST-SECURITY", "Security"))
        .security(0, &["TEST-SECURITY"; 5]).security(1, &["TEST-SECURITY"; 5]).start();
    r.skip_mulligan();
    r.game.turn_count = 3;
    r.game.turn_player_idx = player as usize;
    r.place_on_field(player, "TEST-EFFECT-ATTACK-GUARD", Some(0));
    r.place_on_field(player, "TEST-OTHER-ATTACKER", Some(0));
    r
}
#[test]
fn effect_attack_modes_offer_existing_legal_choices_without_an_attack() {
    for player in [0, 1] {
        for (without, ignore) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut r = setup(player, without, ignore);
            r.fire_on_play(player, 0);
            let selection = r.pending_selection().expect("legal immediate attack choice");
            let mask = build_action_mask(&r.game, player);
            assert!(selection.valid_action_ids.iter().any(|id| *id != PASS && mask[*id as usize] > 0.0));
            assert!(mask[PASS as usize] > 0.0);
        }
    }
}
#[test]
fn effect_attack_modes_never_offer_nested_declarations_including_end_of_attack() {
    for player in [0, 1] {
        for (without, ignore) in [(false, false), (true, false), (false, true), (true, true)] {
            for state in [AttackState::Declared, AttackState::EndOfAttackDrain(AttackResult::SecurityCheckSurvived)] {
                let mut r = setup(player, without, ignore);
                let attacker = digimon_engine::permanent::PermanentHandle { player, index: 1 };
                r.game.pending_attack = Some(PendingAttack {
                    attacker,
                    original_target: AttackTarget::Player(1 - player),
                    effective_target: AttackTarget::Player(1 - player),
                    effective_target_removed: false,
                    is_blocked: false,
                    blocker: None,
                    is_vortex: false,
                    is_overclock: false,
                    declaration_committed: true,
                    cancelled: false,
                    battle_occurred: false,
                    battle_defender_deleted: false,
                    effect_battle_piercing_pending: false,
                    security_check_started: false,
                    return_phase: GamePhase::Main,
                    state,
                    counter_depth: 0,
                });
                r.fire_on_play(player, 0);
                assert!(r.pending_selection().is_none(), "CR 11-2-4: no nested attack choice, seat {player}, modes {without}/{ignore}, state {state:?}");
                let pending = r.game.pending_attack.as_ref().unwrap();
                assert_eq!(pending.attacker, attacker);
                assert_eq!(pending.state, state);
                assert_eq!(r.game.digimon_attacks_this_turn[player as usize], 0);
                assert!(!r.game.players[player as usize].battle_area[0].is_suspended);
            }
        }
    }
}
