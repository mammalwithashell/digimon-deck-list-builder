use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack};
use digimon_engine::{CardKind, EffectTiming, GamePhase, Keyword, TriggerSource};

fn digimon(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

fn tamer(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn progress_digimon(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut card = digimon(id, dp);
    card.keywords = vec![Keyword::Progress];
    card
}

#[test]
fn selector_lowest_dp_only_offers_lowest_current_candidates() {
    let yaml = r#"
card: TST-LOWEST
name: Lowest Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          selector: lowest_dp
          prompt: "Delete lowest DP"
      - delete_permanent: { target: tgt }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(digimon("LOW", 3000))
        .add_card(digimon("HIGH", 9000))
        .start();
    let source = runner.place_on_field(0, "TST-LOWEST", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
    let (selecting_player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().expect("selection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only LOW should be offered"
        );
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, action_id)
        .expect("selection resolves");
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "HIGH"
    );
}

#[test]
fn selector_highest_dp_recomputes_after_prior_suspend_step() {
    let yaml = r#"
card: TST-HIGHEST-SUSPENDED
name: Highest Suspended Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: first
          filter: { kind: digimon }
          selector: lowest_dp
          prompt: "Suspend a Digimon"
      - suspend: { target: first }
      - select_opponent_permanent:
          bind_as: second
          filter: { kind: digimon, is_suspended: true }
          selector: highest_dp
          prompt: "Bottom-deck highest suspended"
      - return_to_deck: { target: second, position: bottom, include_sources: true }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(digimon("LOW", 3000))
        .add_card(digimon("HIGH", 9000))
        .start();
    let source = runner.place_on_field(0, "TST-HIGHEST-SUSPENDED", Some(0));
    runner.place_on_field(0, "LOW", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
    let (selecting_player, first_action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("first selection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only LOW should be offered"
        );
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, first_action)
        .expect("first selection resolves");
    let (selecting_player, second_action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("second selection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only the now-suspended LOW Digimon should be offered"
        );
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, second_action)
        .expect("second selection resolves");
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "HIGH"
    );
}

#[test]
fn dp_lte_selection_sees_same_effect_dp_modifier() {
    let yaml = r#"
card: TST-DP-THEN-DELETE
name: DP Then Delete Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: reduced
          filter: { kind: digimon }
          prompt: "Choose DP reduction target"
      - add_dp_modifier:
          target: reduced
          value: -3000
          expiry: end_of_turn
      - select_opponent_permanent:
          bind_as: deleted
          filter: { kind: digimon, dp_lte: 3000 }
          prompt: "Delete 3000 DP or less"
      - delete_permanent: { target: deleted }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(digimon("NATURAL-LOW", 3000))
        .add_card(digimon("REDUCED-LOW", 6000))
        .add_card(digimon("TOO-HIGH", 9000))
        .start();
    let source = runner.place_on_field(0, "TST-DP-THEN-DELETE", Some(0));
    let natural_low = runner.place_on_field(1, "NATURAL-LOW", Some(0));
    let reduced_low = runner.place_on_field(1, "REDUCED-LOW", Some(0));
    let too_high = runner.place_on_field(1, "TOO-HIGH", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();

    let reduce_action = digimon_engine::action::space::encode_attack(0, reduced_low.index as u16);
    let (selecting_player, first_actions) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("first selection should park");
        (pending.selecting_player, pending.valid_action_ids.clone())
    };
    assert!(first_actions.contains(&reduce_action));
    runner
        .execute_action(selecting_player, reduce_action)
        .expect("first selection resolves");

    assert_eq!(
        runner.effective_dp(reduced_low),
        Some(3000),
        "the first selection's DP modifier must apply before the second selection is installed"
    );

    let natural_low_action =
        digimon_engine::action::space::encode_attack(0, natural_low.index as u16);
    let reduced_low_action =
        digimon_engine::action::space::encode_attack(0, reduced_low.index as u16);
    let too_high_action = digimon_engine::action::space::encode_attack(0, too_high.index as u16);
    let second_actions = runner
        .game
        .pending_selection
        .as_ref()
        .expect("second selection should park")
        .valid_action_ids
        .clone();

    assert!(
        second_actions.contains(&natural_low_action),
        "a naturally 3000 DP target should remain legal"
    );
    assert!(
        second_actions.contains(&reduced_low_action),
        "a 6000 DP target reduced by the same effect to 3000 DP should be legal"
    );
    assert!(
        !second_actions.contains(&too_high_action),
        "an unreduced 9000 DP target should remain illegal"
    );
}

#[test]
fn selector_with_only_no_dp_candidates_installs_no_selection() {
    let yaml = r#"
card: TST-NO-DP
name: No DP Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: tamer }
          selector: lowest_dp
          prompt: "Choose lowest DP Tamer"
      - delete_permanent: { target: tgt }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(tamer("TAMER"))
        .start();
    let source = runner.place_on_field(0, "TST-NO-DP", Some(0));
    runner.place_on_field(1, "TAMER", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "selector with no effective-DP candidates should not offer no-DP targets"
    );
}

#[test]
fn selector_extreme_ignores_progress_excluded_opponent_targets() {
    let yaml = r#"
card: TST-PROGRESS-GATE
name: Progress Gate Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          selector: lowest_dp
          prompt: "Choose legal lowest DP"
      - suspend: { target: tgt }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(progress_digimon("LOW-PROGRESS", 3000))
        .add_card(digimon("HIGH-LEGAL", 9000))
        .start();
    let source = runner.place_on_field(1, "TST-PROGRESS-GATE", Some(0));
    let excluded_low = runner.place_on_field(0, "LOW-PROGRESS", Some(0));
    runner.place_on_field(0, "HIGH-LEGAL", Some(0));
    runner.game.pending_attack = Some(PendingAttack {
        attacker: excluded_low,
        original_target: AttackTarget::Player(1),
        effective_target: AttackTarget::Player(1),
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        battle_defender_deleted: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("legal high-DP target should still be offered");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "Progress-excluded low-DP target should not define the DP extreme"
    );
}

/// G-PLAY-COST-AGGREGATE: the `lowest_play_cost` field selector on a
/// `select_*_permanent` step restricts the candidate set to the
/// permanent(s) with the lowest printed play cost — mirroring `lowest_dp`
/// but keyed on play cost rather than effective DP. Consumed by EX4-073
/// clause C ("delete 1 opponent Digimon/Tamer with the lowest play cost").
#[test]
fn selector_lowest_play_cost_only_offers_cheapest_candidates() {
    fn cost_digimon(id: &str, play_cost: u16) -> digimon_engine::CardData {
        let mut card = make_test_card(id, id);
        card.dp = Some(5000);
        card.play_cost = play_cost;
        card
    }

    let yaml = r#"
card: TST-LOWCOST
name: Lowest Cost Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          selector: lowest_play_cost
          prompt: "Delete lowest play cost"
      - delete_permanent: { target: tgt }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(cost_digimon("CHEAP", 2))
        .add_card(cost_digimon("PRICEY", 9))
        .start();
    let source = runner.place_on_field(0, "TST-LOWCOST", Some(0));
    runner.place_on_field(1, "CHEAP", Some(0));
    runner.place_on_field(1, "PRICEY", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
    let (selecting_player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().expect("selection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "only CHEAP (play cost 2) should be offered, not PRICEY (9)"
        );
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, action_id)
        .expect("selection resolves");
    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.players[1].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "PRICEY",
        "the cheapest opponent Digimon (CHEAP) must have been the deleted target"
    );
}

#[test]
fn selector_lowest_material_count_preserves_ties_for_fewest_sources() {
    let yaml = r#"
card: TST-FEWEST-SOURCES
name: Fewest Sources Test
kind: digimon
level: 6
color: [red]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          selector: lowest_material_count
          prompt: "Delete fewest sources"
      - delete_permanent: { target: tgt }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .add_card(digimon("ZERO-A", 3000))
        .add_card(digimon("ZERO-B", 4000))
        .add_card(digimon("STACKED-TOP", 9000))
        .add_card(digimon("STACKED-SRC", 1000))
        .start();
    let source = runner.place_on_field(0, "TST-FEWEST-SOURCES", Some(0));
    let zero_a = runner.place_on_field(1, "ZERO-A", Some(0));
    let zero_b = runner.place_on_field(1, "ZERO-B", Some(0));
    let stacked = runner.place_stack(1, &["STACKED-SRC", "STACKED-TOP"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();

    let pending = runner.game.pending_selection.as_ref().expect("selection");
    let zero_a_action = digimon_engine::action::space::encode_attack(0, zero_a.index as u16);
    let zero_b_action = digimon_engine::action::space::encode_attack(0, zero_b.index as u16);
    let stacked_action = digimon_engine::action::space::encode_attack(0, stacked.index as u16);
    assert!(
        pending.valid_action_ids.contains(&zero_a_action),
        "first zero-source target should be legal"
    );
    assert!(
        pending.valid_action_ids.contains(&zero_b_action),
        "tied zero-source target should also be legal"
    );
    assert!(
        !pending.valid_action_ids.contains(&stacked_action),
        "target with one source must be excluded"
    );
}
