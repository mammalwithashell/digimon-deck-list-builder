use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{EffectTiming, Expiry, Keyword, TriggerSource};

fn fighter(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

fn battle_yaml(optional: bool) -> String {
    format!(
        r#"
card: TST-BATTLE
name: Battle Step Test
kind: digimon
level: 6
color: [green]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: battle_target
          filter: {{ kind: digimon }}
          prompt: "Choose battle target"
          optional: {optional}
      - battle:
          attacker: this
          defender: battle_target
"#
    )
}

#[test]
fn effect_battle_deletes_defender_without_piercing_security_check() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&battle_yaml(false))
        .expect("inline DSL compiles")
        .add_card(fighter("DEF", 3000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let attacker = runner.place_on_field(0, "TST-BATTLE", Some(0));
    runner.place_on_field(1, "DEF", Some(0));
    runner
        .game
        .modifiers
        .grant_keyword(attacker, Keyword::Piercing, Expiry::Permanent, 0);
    let security_before = runner.security_count(1);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(attacker),
    );
    runner.game.drain_effect_queue();

    let (selecting_player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("battle target selection");
        assert!(!pending.is_optional);
        assert_eq!(pending.valid_action_ids.len(), 1);
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, action_id)
        .expect("selection resolves");

    assert_eq!(runner.battle_area_size(1), 0, "defender should be deleted");
    assert_eq!(
        runner.security_count(1),
        security_before,
        "effect battle must not continue into Piercing security checks"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "effect battle must not leave an attack pending"
    );
}

#[test]
fn effect_battle_optional_target_decline_leaves_defender_in_play() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&battle_yaml(true))
        .expect("inline DSL compiles")
        .add_card(fighter("DEF", 3000))
        .start();

    let attacker = runner.place_on_field(0, "TST-BATTLE", Some(0));
    runner.place_on_field(1, "DEF", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(attacker),
    );
    runner.game.drain_effect_queue();

    let selecting_player = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("optional battle target selection");
        assert!(pending.is_optional);
        pending.selecting_player
    };
    runner
        .execute_action(selecting_player, PASS)
        .expect("decline selection");

    assert_eq!(
        runner.battle_area_size(1),
        1,
        "declining the optional selection should skip battle"
    );
    assert!(runner.game.pending_attack.is_none());
}
