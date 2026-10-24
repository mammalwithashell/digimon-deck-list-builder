use super::*;
use digimon_engine::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

fn fixture(actor: PlayerId, own: bool, duplicate: bool, only_second: bool) -> Game {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .add_card(make_test_card("PICK", "Pick"))
        .add_card(make_test_card("OTHER", "Other"))
        .deck(0, &["OTHER"; 12])
        .deck(1, &["OTHER"; 12])
        .start();
    runner.skip_mulligan();
    runner.game.enter_main_phase();
    let source = runner.place_on_field(actor, "SOURCE", Some(0));
    runner.place_on_field(actor, "PICK", Some(0));
    runner.place_on_field(1 - actor, "OTHER", Some(0));
    runner.place_on_field(1 - actor, "PICK", Some(0));
    let target_owner = if own { actor } else { 1 - actor };
    if duplicate {
        runner.place_on_field(target_owner, "PICK", Some(0));
    }
    let card = runner.game.player(actor).battle_area[0].top_card().handle();
    let mut ctx = EffectContext::new(&mut runner.game, card, Some(source), actor);
    let filter = move |_game: &Game, handle: digimon_engine::PermanentHandle| {
        !only_second || handle.index == 2
    };
    if own {
        ctx.select_own_permanent("Pick a friendly permanent", false, filter, |ctx, target| {
            ctx.suspend(target);
        });
    } else {
        ctx.select_opponent_permanent(
            "Pick an opposing permanent",
            false,
            filter,
            |ctx, target| {
                ctx.suspend(target);
            },
        );
    }
    runner.game
}

fn cards(id: &str) -> SelectPayload {
    SelectPayload::Cards {
        ids: vec![id.into()],
        ordinal: None,
        trigger: None,
        trigger_not: None,
    }
}

#[test]
fn shared_field_identity_executes_the_selected_owner_for_both_actors() {
    for actor in [0, 1] {
        for own in [false, true] {
            let mut game = fixture(actor, own, false, false);
            let target_owner = if own { actor } else { 1 - actor };
            let (row, wire) = build_selection_row(&game, 7, actor, &cards("PICK"), None).unwrap();
            assert_eq!(row.actor, actor);
            assert_eq!(wire.card_ids, ["PICK"]);
            assert!(wire.permanent_targets.is_empty());
            let action = resolve_next(&game, &row, 0)
                .expect("shared identity must lower")
                .unwrap();
            assert_eq!(action, ATTACK_START + 1);
            game.decode_action(action, actor);
            assert!(game.player(target_owner).battle_area[1].is_suspended);
            assert!(!game.player(1 - target_owner).battle_area[1].is_suspended);
        }
    }
}

#[test]
fn field_identity_ignores_duplicate_cards_outside_the_legal_candidates() {
    for actor in [0, 1] {
        let game = fixture(actor, false, true, true);
        let (row, wire) = build_selection_row(&game, 7, actor, &cards("PICK"), None).unwrap();
        assert_eq!(
            resolve_next(&game, &row, 0).unwrap(),
            Some(ATTACK_START + 2)
        );
        assert_eq!(wire.card_ids, ["PICK"]);
    }
}

#[test]
fn field_identity_refuses_ambiguous_legal_duplicates() {
    let game = fixture(0, false, true, false);
    let error = build_selection_row(&game, 7, 0, &cards("PICK"), None).unwrap_err();
    assert!(error.contains("ambiguous"), "{error}");
}

#[test]
fn field_identity_refuses_missing_or_masked_out_cards() {
    let game = fixture(0, false, true, true);
    for id in ["ABSENT", "OTHER"] {
        let error = build_selection_row(&game, 7, 0, &cards(id), None).unwrap_err();
        assert!(error.contains("no legal field candidate"), "{error}");
    }
}

#[test]
fn field_identity_keeps_the_existing_decider_guard() {
    let game = fixture(1, false, false, false);
    let error = build_selection_row(&game, 7, 0, &cards("PICK"), None).unwrap_err();
    assert!(error.contains("belongs to player 1"), "{error}");
}

#[test]
fn any_field_identity_keeps_absolute_owner_and_rejects_both_side_duplicates() {
    let mut game = fixture(0, false, false, false);
    let pending = game.pending_selection.as_mut().unwrap();
    pending.kind = SelectionKind::AnyField;
    pending.valid_action_ids = vec![ATTACK_START, ATTACK_START + TARGETS_PER_ATTACKER + 1];
    let (row, _) = build_selection_row(&game, 7, 0, &cards("PICK"), None).unwrap();
    let target = row
        .targets
        .as_ref()
        .expect("identity must bind a physical local target")[0];
    assert_eq!((target.player, target.frame), (1, 1));
    assert_eq!(
        resolve_next(&game, &row, 0).unwrap(),
        Some(ATTACK_START + TARGETS_PER_ATTACKER + 1)
    );

    game.pending_selection
        .as_mut()
        .unwrap()
        .valid_action_ids
        .push(ATTACK_START + 1);
    let error = build_selection_row(&game, 7, 0, &cards("PICK"), None).unwrap_err();
    assert!(error.contains("ambiguous"), "{error}");
}

fn delegated_fixture(controller: PlayerId, own: bool) -> Game {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SOURCE", "Source"))
        .add_card(make_test_card("OTHER", "Other"))
        .add_card(make_test_card("REAL", "Real target"))
        .add_card(make_test_card("DECOY", "Wrong-board decoy"))
        .deck(0, &["OTHER"; 12])
        .deck(1, &["OTHER"; 12])
        .start();
    runner.skip_mulligan();
    runner.game.enter_main_phase();
    let source = runner.place_on_field(controller, "SOURCE", Some(0));
    runner.place_on_field(1 - controller, "OTHER", Some(0));
    let target_owner = if own { controller } else { 1 - controller };
    runner.place_on_field(target_owner, "REAL", Some(0));
    runner.place_on_field(1 - target_owner, "DECOY", Some(0));
    let card = runner.game.player(controller).battle_area[0]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, card, Some(source), controller);
    if own {
        ctx.as_selecting_player(1 - controller)
            .select_own_permanent(
                "Opponent chooses from the controller's field",
                false,
                |_, handle| handle.index == 1,
                |ctx, target| ctx.suspend(target),
            );
    } else {
        ctx.as_selecting_player(1 - controller)
            .select_opponent_permanent(
                "Opponent chooses from their own field",
                false,
                |_, handle| handle.index == 1,
                |ctx, target| ctx.suspend(target),
            );
    }
    runner.game
}

fn assert_delegated_identity_executes_real_target(own: bool) {
    for controller in [0, 1] {
        let mut game = delegated_fixture(controller, own);
        let chooser = 1 - controller;
        let target_owner = if own { controller } else { chooser };
        assert_eq!(
            game.pending_selection.as_ref().unwrap().selecting_player,
            chooser
        );
        let (row, wire) = build_selection_row(&game, 7, chooser, &cards("REAL"), None)
            .expect("delegated card pick must use the actual target owner");
        assert_eq!(row.actor, chooser);
        assert_eq!(wire.card_ids, ["REAL"]);
        let target = row.targets.as_ref().unwrap()[0];
        assert_eq!((target.player, target.frame), (target_owner, 1));
        let action = resolve_next(&game, &row, 0).unwrap().unwrap();
        assert_eq!(action, ATTACK_START + 1);
        game.decode_action(action, chooser);
        assert!(game.player(target_owner).battle_area[1].is_suspended);
        assert!(!game.player(1 - target_owner).battle_area[1].is_suspended);
    }
}

#[test]
fn delegated_own_field_identity_executes_the_controller_board_for_both_seats() {
    assert_delegated_identity_executes_real_target(true);
}

#[test]
fn delegated_opponent_field_identity_executes_the_target_board_for_both_seats() {
    assert_delegated_identity_executes_real_target(false);
}

#[test]
fn delegated_field_identity_rejects_the_other_board_decoy() {
    for controller in [0, 1] {
        for own in [false, true] {
            let game = delegated_fixture(controller, own);
            let error = build_selection_row(&game, 7, 1 - controller, &cards("DECOY"), None)
                .expect_err("same legal slot on the wrong board is not a candidate");
            assert!(error.contains("no legal field candidate"), "{error}");
        }
    }
}

#[test]
fn relative_field_identity_without_owner_metadata_is_rejected() {
    for actor in [0, 1] {
        for own in [false, true] {
            let mut game = fixture(actor, own, false, false);
            game.pending_selection.as_mut().unwrap().zone_owner = None;
            let error = build_selection_row(&game, 7, actor, &cards("PICK"), None)
                .expect_err("unresolved field ownership must never be guessed from the actor");
            assert!(error.contains("ownership unavailable"), "{error}");
        }
    }
}

#[test]
fn relative_field_identity_rejects_an_absolute_band_for_another_owner() {
    let mut game = fixture(1, false, false, false);
    let pending = game.pending_selection.as_mut().unwrap();
    assert_eq!(pending.zone_owner, Some(0));
    pending.valid_action_ids = vec![ATTACK_START + TARGETS_PER_ATTACKER + 1];
    let error = build_selection_row(&game, 7, 1, &cards("PICK"), None)
        .expect_err("an absolute band for player 1 cannot select from player 0's field");
    assert!(error.contains("no legal field candidate"), "{error}");
}

fn combat_fixture(attacker_player: PlayerId, alliance: bool) -> DebugRunner {
    use digimon_engine::enums::{Expiry, Keyword};

    let mut blocker = make_test_card("BLOCKER", "Blocker");
    blocker.dp = Some(5000);
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("ALLY", "Ally"))
        .add_card(make_test_card("OTHER", "Other"))
        .add_card(blocker)
        .deck(0, &["OTHER"; 12])
        .deck(1, &["OTHER"; 12])
        .security(0, &["OTHER"; 5])
        .security(1, &["OTHER"; 5])
        .start();
    runner.skip_mulligan();
    runner.set_first_player(attacker_player);
    runner.set_turn(3);
    runner.game.enter_main_phase();
    let attacker = runner.place_on_field(attacker_player, "ATTACKER", Some(0));
    runner.place_on_field(attacker_player, "ALLY", Some(0));
    runner.place_on_field(1 - attacker_player, "OTHER", Some(0));
    let blocker = runner.place_on_field(1 - attacker_player, "BLOCKER", Some(0));
    if alliance {
        runner.game.modifiers.grant_keyword(
            attacker,
            Keyword::Alliance,
            Expiry::Permanent,
            attacker_player,
        );
    } else {
        runner.game.modifiers.grant_keyword(
            blocker,
            Keyword::Blocker,
            Expiry::Permanent,
            1 - attacker_player,
        );
    }
    let attack = digimon_engine::action::space::encode_attack(
        0,
        digimon_engine::action::space::SECURITY_TARGET,
    );
    assert_eq!(
        digimon_engine::action::mask::build_action_mask(&runner.game, attacker_player)
            [attack as usize],
        1.0
    );
    runner.game.decode_action(attack, attacker_player);
    runner
}

#[test]
fn alliance_field_identity_uses_the_actual_ally_owner() {
    for attacker in [0, 1] {
        let mut runner = combat_fixture(attacker, true);
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Alliance prompt");
        assert_eq!(pending.zone_owner, Some(attacker));
        assert_eq!(pending.prompt, "Declare an ally (Alliance)");
        let (row, wire) =
            build_selection_row(&runner.game, 7, attacker, &cards("ALLY"), None).unwrap();
        assert_eq!(row.targets.as_ref().unwrap()[0].player, attacker);
        assert_eq!(wire.card_ids, ["ALLY"]);
        let action = resolve_next(&runner.game, &row, 0).unwrap().unwrap();
        runner.game.decode_action(action, attacker);
        assert!(runner.game.player(attacker).battle_area[1].is_suspended);
    }
}

#[test]
fn blocker_field_identity_uses_defender_even_with_attacker_provenance() {
    for attacker in [0, 1] {
        let mut runner = combat_fixture(attacker, false);
        let defender = 1 - attacker;
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Blocker prompt");
        assert_eq!(pending.zone_owner, Some(defender));
        assert_eq!(pending.source_permanent.unwrap().player, attacker);
        assert_eq!(pending.prompt, "Declare a blocker");
        let (row, wire) =
            build_selection_row(&runner.game, 7, defender, &cards("BLOCKER"), None).unwrap();
        assert_eq!(row.targets.as_ref().unwrap()[0].player, defender);
        assert_eq!(wire.card_ids, ["BLOCKER"]);
        let action = resolve_next(&runner.game, &row, 0).unwrap().unwrap();
        runner.game.decode_action(action, defender);
        assert!(runner.game.player(defender).battle_area[1].is_suspended);
    }
}

#[test]
fn raid_field_identity_uses_the_opponent_board_for_both_seats() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::enums::{Expiry, Keyword};

    for attacker_player in [0, 1] {
        let defender = 1 - attacker_player;
        let mut attacker_card = make_test_card("ATTACKER", "Attacker");
        attacker_card.dp = Some(9000);
        let mut high = make_test_card("HIGH", "Highest DP target");
        high.dp = Some(5000);
        let mut runner = DebugRunner::builder()
            .add_card(attacker_card)
            .add_card(high)
            .add_card(make_test_card("OTHER", "Other"))
            .add_card(make_test_card("DECOY", "Wrong-board decoy"))
            .deck(0, &["OTHER"; 12])
            .deck(1, &["OTHER"; 12])
            .security(0, &["OTHER"; 5])
            .security(1, &["OTHER"; 5])
            .start();
        runner.skip_mulligan();
        runner.set_first_player(attacker_player);
        runner.set_turn(3);
        runner.game.enter_main_phase();
        let attacker = runner.place_on_field(attacker_player, "ATTACKER", Some(0));
        runner.place_on_field(attacker_player, "DECOY", Some(0));
        runner.place_on_field(defender, "OTHER", Some(0));
        runner.place_on_field(defender, "HIGH", Some(0));
        runner.game.modifiers.grant_keyword(
            attacker,
            Keyword::Raid,
            Expiry::Permanent,
            attacker_player,
        );
        let attack = encode_attack(0, SECURITY_TARGET);
        assert_eq!(
            digimon_engine::action::mask::build_action_mask(&runner.game, attacker_player)
                [attack as usize],
            1.0
        );
        runner.game.decode_action(attack, attacker_player);

        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("native Raid prompt");
        assert_eq!(pending.kind, SelectionKind::OppField);
        assert_eq!(pending.zone_owner, Some(defender));
        assert_eq!(pending.selecting_player, attacker_player);
        assert_eq!(pending.source_permanent, Some(attacker));
        assert_eq!(pending.valid_action_ids, [encode_attack(0, 1)]);
        assert!(
            build_selection_row(&runner.game, 7, attacker_player, &cards("DECOY"), None).is_err()
        );
        let (row, wire) =
            build_selection_row(&runner.game, 7, attacker_player, &cards("HIGH"), None).unwrap();
        let target = row.targets.as_ref().unwrap()[0];
        assert_eq!((target.player, target.frame), (defender, 1));
        assert_eq!(wire.card_ids, ["HIGH"]);
        let action = resolve_next(&runner.game, &row, 0).unwrap().unwrap();
        assert_eq!(action, encode_attack(0, 1));
        runner.game.decode_action(action, attacker_player);
        assert_eq!(runner.game.player(defender).security.len(), 5);
        assert_eq!(runner.game.player(defender).battle_area.len(), 1);
        assert!(runner
            .game
            .player(defender)
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "HIGH"));
        assert_eq!(
            runner.game.player(attacker_player).battle_area[1]
                .top_card()
                .card_id(&runner.game.card_data),
            "DECOY"
        );
    }
}

#[test]
fn overclock_field_identity_pays_from_the_carrier_board_for_both_seats() {
    use digimon_engine::action::space::{encode_attack, PASS};
    use digimon_engine::enums::{Expiry, Keyword};

    for owner in [0, 1] {
        let mut carrier_card = make_test_card("CARRIER", "Overclock carrier");
        carrier_card.dp = Some(5000);
        let mut runner = DebugRunner::builder()
            .add_card(carrier_card)
            .add_card(make_test_card("SACRIFICE", "Sacrifice"))
            .add_card(make_test_card("OTHER", "Other"))
            .add_card(make_test_card("DECOY", "Wrong-board decoy"))
            .deck(0, &["OTHER"; 12])
            .deck(1, &["OTHER"; 12])
            .security(0, &["OTHER"; 5])
            .security(1, &["OTHER"; 5])
            .start();
        runner.skip_mulligan();
        runner.set_first_player(owner);
        runner.set_turn(3);
        runner.game.enter_main_phase();
        let carrier = runner.place_on_field(owner, "CARRIER", Some(0));
        runner.place_on_field(owner, "SACRIFICE", Some(0));
        runner.place_on_field(1 - owner, "OTHER", Some(0));
        runner.place_on_field(1 - owner, "DECOY", Some(0));
        runner
            .game
            .modifiers
            .grant_keyword(carrier, Keyword::Overclock, Expiry::EndOfTurn, owner);
        assert_eq!(
            digimon_engine::action::mask::build_action_mask(&runner.game, owner)[PASS as usize],
            1.0
        );
        runner.game.decode_action(PASS, owner);

        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("native Overclock prompt");
        assert_eq!(pending.kind, SelectionKind::OwnField);
        assert_eq!(pending.zone_owner, Some(owner));
        assert_eq!(pending.selecting_player, owner);
        assert_eq!(pending.source_permanent, Some(carrier));
        assert_eq!(pending.valid_action_ids, [encode_attack(0, 1)]);
        assert!(build_selection_row(&runner.game, 7, owner, &cards("DECOY"), None).is_err());
        let (row, wire) =
            build_selection_row(&runner.game, 7, owner, &cards("SACRIFICE"), None).unwrap();
        let target = row.targets.as_ref().unwrap()[0];
        assert_eq!((target.player, target.frame), (owner, 1));
        assert_eq!(wire.card_ids, ["SACRIFICE"]);
        let action = resolve_next(&runner.game, &row, 0).unwrap().unwrap();
        assert_eq!(action, encode_attack(0, 1));
        runner.game.decode_action(action, owner);
        assert!(runner
            .game
            .player(owner)
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "SACRIFICE"));
        assert_eq!(runner.game.player(owner).battle_area.len(), 1);
        assert_eq!(
            runner.game.player(1 - owner).battle_area[1]
                .top_card()
                .card_id(&runner.game.card_data),
            "DECOY"
        );
        assert_eq!(runner.game.player(1 - owner).security.len(), 4);
    }
}

fn dsl_field_fixture(actor: PlayerId, yaml: &str, target_owner: PlayerId) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("field-prompt fixture compiles")
        .add_card(make_test_card("FIRST", "First target"))
        .add_card(make_test_card("SECOND", "Second target"))
        .add_card(make_test_card("DECOY", "Wrong-board decoy"))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(actor, &["FIELD-SOURCE"])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.set_first_player(actor);
    runner.set_turn(3);
    runner.game.enter_main_phase();
    runner.place_on_field(target_owner, "FIRST", Some(0));
    runner.place_on_field(target_owner, "SECOND", Some(0));
    runner.place_on_field(1 - target_owner, "DECOY", Some(0));
    let play = digimon_engine::action::space::PLAY_HAND_START;
    assert_eq!(
        digimon_engine::action::mask::build_action_mask(&runner.game, actor)[play as usize],
        1.0
    );
    runner.game.decode_action(play, actor);
    runner
}

#[test]
fn count_capped_field_identity_keeps_owner_on_first_and_resumed_picks() {
    for actor in [0, 1] {
        for own in [false, true] {
            let owner = if own { actor } else { 1 - actor };
            let yaml = format!(
                r#"
card: FIELD-SOURCE
name: Field source
kind: digimon
color: [white]
level: 6
cost: 1
dp: 12000
effects:
  - when: on_play
    process:
      - select_count_capped_multi:
          of: {}
          zone: battle_area
          min: 2
          max: 2
          filter: {{ level_eq: 3 }}
          bind_as: picked
          prompt: Pick two field targets
          optional_zero: false
      - gain_memory: 1
"#,
                if own { "you" } else { "opponent" }
            );
            let mut runner = dsl_field_fixture(actor, &yaml, owner);
            let memory_before = runner.game.memory;
            for (frame, id) in [(0, "FIRST"), (1, "SECOND")] {
                let pending = runner
                    .game
                    .pending_selection
                    .as_ref()
                    .expect("first or resumed field pick");
                assert_eq!(pending.zone_owner, Some(owner));
                assert_eq!(pending.selecting_player, actor);
                assert!(
                    build_selection_row(&runner.game, 7, actor, &cards("DECOY"), None).is_err()
                );
                let (row, wire) =
                    build_selection_row(&runner.game, 7, actor, &cards(id), None).unwrap();
                let target = row.targets.as_ref().unwrap()[0];
                assert_eq!((target.player, target.frame), (owner, frame));
                assert_eq!(wire.card_ids, [id]);
                let action = resolve_next(&runner.game, &row, 0).unwrap().unwrap();
                runner.game.decode_action(action, actor);
            }
            assert!(runner.game.pending_selection.is_none());
            assert_eq!(
                runner.game.memory,
                memory_before + 1,
                "both picks complete the actual effect tail"
            );
        }
    }
}

#[test]
fn per_color_field_identity_resolves_existing_encoded_candidates_for_both_seats() {
    let yaml = r#"
card: FIELD-SOURCE
name: Field source
kind: digimon
color: [white]
level: 6
cost: 1
dp: 12000
effects:
  - when: on_play
    process:
      - delete_one_per_opponent_color:
          prompt: "Delete 1 {color} Digimon"
"#;
    for actor in [0, 1] {
        let owner = 1 - actor;
        let mut runner = dsl_field_fixture(actor, yaml, owner);
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("per-color field prompt");
        assert_eq!(pending.zone_owner, Some(owner));
        assert_eq!(pending.kind, SelectionKind::OppField);
        assert_eq!(pending.selecting_player, actor);
        let expected = digimon_engine::action::space::encode_attack(owner as u16, 1);
        assert!(pending.valid_action_ids.contains(&expected));
        assert!(build_selection_row(&runner.game, 7, actor, &cards("DECOY"), None).is_err());
        let (row, wire) =
            build_selection_row(&runner.game, 7, actor, &cards("SECOND"), None).unwrap();
        let target = row.targets.as_ref().unwrap()[0];
        assert_eq!((target.player, target.frame), (owner, 1));
        assert_eq!(wire.card_ids, ["SECOND"]);
        let action = resolve_next(&runner.game, &row, 0).unwrap().unwrap();
        assert_eq!(
            action, expected,
            "preserve the producer's existing action encoding"
        );
        runner.game.decode_action(action, actor);
        assert!(runner.game.pending_selection.is_none());
        assert_eq!(runner.game.player(owner).battle_area.len(), 1);
        assert_eq!(
            runner.game.player(owner).battle_area[0]
                .top_card()
                .card_id(&runner.game.card_data),
            "FIRST"
        );
        assert_eq!(
            runner.game.player(actor).battle_area[0]
                .top_card()
                .card_id(&runner.game.card_data),
            "DECOY"
        );
    }
}
