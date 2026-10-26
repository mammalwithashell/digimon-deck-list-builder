//! Bounded timing audit of the frozen 13-archetype training/evaluation pool.
//! EX12-034/036/057 card faces and DCGO each require an own DIGIMON play.
//! CR 4.3 15-8-3-1/4: the trigger condition must actually occur.
//! These tests use the production play path and legal prompt actions.

use digimon_engine::{
    action::{build_action_mask, space::PASS},
    card_data::CardData,
    debug_runner::DebugRunner,
    enums::GamePhase,
};

fn setup(observer: &str) -> DebugRunner {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut data = CardData::load_from_str(&raw).unwrap();
    let mut builder = DebugRunner::builder().dsl_card(observer).unwrap();
    // Production metadata. DarkTyrannomon is an effect-free play/deck control.
    for id in [observer, "BT24-084", "BT1-019", "EX13-045"] {
        builder = builder.add_card(data.remove(id).unwrap());
    }
    let mut runner = builder
        .deck(0, &["BT1-019"; 15])
        .deck(1, &["BT1-019"; 15])
        .memory(10)
        .start();
    runner.set_phase(GamePhase::Main);
    runner.place_on_field(0, observer, Some(0));
    runner.place_on_field(1, "EX13-045", Some(0));
    runner
}

fn play(runner: &mut DebugRunner, player: u8, card: &str) {
    let hand = runner.add_to_hand(player, card);
    assert!(
        runner.play(player, hand).is_some(),
        "fixture play must succeed"
    );
}

fn finish(runner: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let action = *view
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("mandatory observer target");
        assert_eq!(
            build_action_mask(&runner.game, view.selecting_player)[action as usize],
            1.0
        );
        runner
            .execute_action(view.selecting_player, action)
            .unwrap();
    }
    panic!("unexpected repeated prompt");
}

fn tamer_does_not_trigger(observer: &str) {
    let mut runner = setup(observer);
    play(&mut runner, 0, "BT24-084");
    assert!(
        runner.pending_selection().is_none(),
        "{observer}: Tamer play must not trigger the Digimon-play clause: {:?}",
        runner.pending_selection_view()
    );
    // A nonmatching event must not consume the once-per-turn allowance.
    play(&mut runner, 0, "BT1-019");
    assert!(
        runner.pending_selection().is_some(),
        "{observer}: later Digimon play must still trigger"
    );
    finish(&mut runner);
}

fn own_digimon_triggers(observer: &str) {
    let mut runner = setup(observer);
    play(&mut runner, 0, "BT1-019");
    assert!(
        runner.pending_selection().is_some(),
        "{observer}: own Digimon play must trigger"
    );
    finish(&mut runner);
}

fn opponent_digimon_does_not_trigger(observer: &str) {
    let mut runner = setup(observer);
    runner.set_first_player(1);
    play(&mut runner, 1, "BT1-019");
    assert!(
        runner.pending_selection().is_none(),
        "{observer}: opponent Digimon play must not trigger: {:?}",
        runner.pending_selection_view()
    );
}

#[test]
fn erlangmon_ignores_tamer_play() {
    tamer_does_not_trigger("EX12-034");
}
#[test]
fn ryugumon_ignores_tamer_play() {
    tamer_does_not_trigger("EX12-036");
}
#[test]
fn takutoumon_ignores_tamer_play() {
    tamer_does_not_trigger("EX12-057");
}
#[test]
fn erlangmon_observes_own_digimon_play() {
    own_digimon_triggers("EX12-034");
}
#[test]
fn ryugumon_observes_own_digimon_play() {
    own_digimon_triggers("EX12-036");
}
#[test]
fn takutoumon_observes_own_digimon_play() {
    own_digimon_triggers("EX12-057");
}
#[test]
fn erlangmon_ignores_opponent_digimon_play() {
    opponent_digimon_does_not_trigger("EX12-034");
}
#[test]
fn ryugumon_ignores_opponent_digimon_play() {
    opponent_digimon_does_not_trigger("EX12-036");
}
#[test]
fn takutoumon_ignores_opponent_digimon_play() {
    opponent_digimon_does_not_trigger("EX12-057");
}

fn firamon_event_fixture(event_card: &str, player: u8) -> DebugRunner {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut data = CardData::load_from_str(&raw).unwrap();
    let mut builder = DebugRunner::builder().dsl_card("BT25-013").unwrap();
    for id in ["BT25-013", "BT25-017", "BT1-019", event_card] {
        if let Some(card) = data.remove(id) {
            builder = builder.add_card(card);
        }
    }
    let mut runner = builder
        .deck(0, &["BT1-019"; 15])
        .deck(1, &["BT1-019"; 15])
        .memory(10)
        .start();
    runner.set_phase(GamePhase::Main);
    runner.place_on_field(0, "BT25-013", Some(0));
    runner.add_to_hand(0, "BT25-017");
    let played = runner.place_on_field(player, event_card, None);
    // Suppressing the entrant's own On Play leaves observer broadcasts intact.
    runner.fire_play_event_triggers(player, played.index as usize, true, true);
    runner
}

#[test]
fn firamon_ignores_own_blue_tamer_play() {
    let runner = firamon_event_fixture("ST24-13", 0);
    assert!(
        runner.pending_selection().is_none(),
        "a blue Tamer is not a blue Digimon: {:?}",
        runner.pending_selection_view()
    );
}

#[test]
fn firamon_observes_own_blue_digimon_play() {
    let mut runner = firamon_event_fixture("BT1-029", 0);
    assert!(
        runner.pending_selection().is_some(),
        "own blue Digimon must offer Flaremon"
    );
    finish(&mut runner);
    assert_eq!(
        runner.game.players[0].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "BT25-017"
    );
}

#[test]
fn firamon_ignores_opponent_blue_digimon_play() {
    let runner = firamon_event_fixture("BT1-029", 1);
    assert!(runner.pending_selection().is_none());
}

fn start_turn_delay_fixture(with_asuna: bool) -> DebugRunner {
    // LM-027 + BT24-088 coexist in a frozen Chronomon training list.
    // CR 4.3 15-4-3-1/2/4/5: simultaneous own effects share the next-effect choice.
    // Both card faces and DCGO register this at Start of Your Turn.
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut data = CardData::load_from_str(&raw).unwrap();
    let mut builder = DebugRunner::builder()
        .dsl_card("LM-027")
        .unwrap()
        .dsl_card("BT24-088")
        .unwrap();
    for id in ["LM-027", "BT24-088", "BT1-019", "BT26-008"] {
        builder = builder.add_card(data.remove(id).unwrap());
    }
    let mut runner = builder
        .deck(0, &["BT1-019"; 15])
        .deck(1, &["BT1-019"; 15])
        .memory(10)
        .start();
    runner.set_phase(GamePhase::Main);
    if with_asuna {
        runner.place_on_field(0, "BT24-088", Some(0));
    }
    let delay = runner.place_on_field(0, "LM-027", Some(0));
    runner.game.players[0].battle_area[delay.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner: 0,
            trash_on_turn: 4,
            trigger: digimon_engine::enums::DelayTrigger::StartOfYourNextTurn,
            placed_on_turn: 0,
        };
    runner.place_on_field(1, "BT1-019", Some(0));
    for id in ["BT1-019", "BT26-008"] {
        let hand = runner.add_to_hand(0, id);
        let card = runner.game.players[0].hand.remove(hand);
        runner.game.players[0].trash.push(card);
    }
    runner.set_turn(3);
    runner.set_first_player(1);
    runner.game.memory = -2;
    runner.end_turn();
    runner
}

#[test]
fn start_turn_scramble_delay_alone_is_due() {
    let runner = start_turn_delay_fixture(false);
    assert_eq!(runner.game.turn_count, 4);
    let view = runner
        .pending_selection_view()
        .expect("eligible Red Scramble must offer its delay");
    assert_eq!(
        runner
            .game
            .card_data
            .iter()
            .find(|c| c.card_id == "LM-027")
            .unwrap()
            .card_name,
        "Red Scramble"
    );
    assert!(view.is_optional, "Delay must retain the player's decline");
}

#[test]
fn start_turn_scramble_delay_and_asuna_share_trigger_order() {
    let runner = start_turn_delay_fixture(true);
    let view = runner
        .pending_selection_view()
        .expect("both start-turn effects must be pending");
    assert_eq!(view.kind, digimon_engine::selection::SelectionKind::TriggerOrder,
        "player must choose Red Scramble OR Asuna first, rather than being forced into Asuna: {view:?}");
    assert!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count() >= 2,
        "both start-turn effects must be offered: {view:?}"
    );
}

#[test]
fn start_turn_delay_decline_reschedules_and_reoffers_after_clone() {
    let mut runner = start_turn_delay_fixture(true);
    let mut cloned = start_turn_delay_fixture(true);
    cloned.game = runner.game.clone();
    for game in [&mut runner, &mut cloned] {
        let view = game.pending_selection_view().unwrap();
        assert_eq!(
            view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
            2
        );
        assert_eq!(build_action_mask(&game.game, 0)[PASS as usize], 1.0);
        game.execute_action(0, PASS).unwrap();
        let delay = game.game.players[0]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&game.game.card_data) == "LM-027")
            .unwrap();
        assert!(matches!(
            delay.option_state,
            digimon_engine::permanent::OptionState::Delayed {
                trash_on_turn: 6,
                ..
            }
        ));
        // Stage the next opponent turn, then cross the real turn boundary.
        game.set_phase(GamePhase::Main);
        game.set_turn(5);
        game.set_first_player(1);
        game.game.memory = -2;
        game.end_turn();
        let view = game
            .pending_selection_view()
            .expect("declined Delay must return next start");
        assert_eq!(
            view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
            2
        );
    }
}

#[test]
fn start_turn_delay_or_asuna_can_be_selected_first_from_cloned_pool() {
    let original = start_turn_delay_fixture(true);
    for expected in ["LM-027", "BT24-088"] {
        let mut runner = start_turn_delay_fixture(true);
        runner.game = original.game.clone();
        let view = runner.pending_selection_view().unwrap();
        let choice = view
            .effect_choices
            .unwrap()
            .into_iter()
            .find(|c| c.label.contains(expected))
            .expect("both clauses are offered");
        assert_eq!(
            build_action_mask(&runner.game, 0)[choice.action_id as usize],
            1.0
        );
        runner.execute_action(0, choice.action_id).unwrap();
        // Selection has entered the chosen effect's processing, not its sibling.
        let prompt = runner
            .pending_selection_view()
            .expect("chosen effect has a player decision");
        assert_eq!(prompt.source_card, choice.source_card.unwrap());
    }
}

fn start_turn_bacchus_fixture(previous_turn_use: bool) -> DebugRunner {
    // BT25-077 Bacchusmon: [All Turns][OPT] observes any Digimon play.
    // BT24-088 Asuna plays a TS Digimon during the next Start of Your Turn.
    // CR 4.3 15-14-1-5-1 resets uses when the turn changes, before this event.
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut data = CardData::load_from_str(&raw).unwrap();
    let mut builder = DebugRunner::builder()
        .dsl_card("BT25-077")
        .unwrap()
        .dsl_card("BT24-088")
        .unwrap();
    for id in ["BT25-077", "BT24-088", "BT25-030", "BT1-019"] {
        builder = builder.add_card(data.remove(id).unwrap());
    }
    let mut runner = builder
        .deck(0, &["BT1-019"; 15])
        .deck(1, &["BT1-019"; 15])
        .memory(10)
        .start();
    runner.set_phase(GamePhase::Main);
    runner.place_on_field(0, "BT25-077", Some(0));
    runner.place_on_field(0, "BT24-088", Some(0));
    runner.place_on_field(1, "BT1-019", Some(0));
    runner.set_turn(3);
    runner.set_first_player(1);
    if previous_turn_use {
        runner.game.memory = 10;
        play(&mut runner, 1, "BT1-019");
        finish(&mut runner);
        assert_eq!(bacchus_uses(&runner), 1, "prior-turn positive control");
    }
    let hand = runner.add_to_hand(0, "BT25-030");
    let card = runner.game.players[0].hand.remove(hand);
    runner.game.players[0].trash.push(card);
    runner.game.memory = -2;
    runner.end_turn();
    finish(&mut runner);
    assert_eq!(runner.game.turn_count, 4);
    assert_eq!(runner.game.current_phase, GamePhase::Main);
    runner
}

fn bacchus_uses(runner: &DebugRunner) -> u8 {
    let permanent = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT25-077")
        .unwrap();
    let card = permanent.top_card().handle();
    let effects = runner.game.effects_for_card("BT25-077", card).unwrap();
    let (slot, effect) = effects
        .iter()
        .enumerate()
        .find(|(_, e)| e.timing == digimon_engine::enums::EffectTiming::OnEnterFieldAnyone)
        .unwrap();
    permanent.activation_count(card, effect.shared_opt_group.unwrap_or(slot as u8))
}

#[test]
fn prior_turn_opt_use_resets_before_asuna_start_turn_play() {
    let runner = start_turn_bacchus_fixture(true);
    assert_eq!(runner.game.players[1].battle_area.len(), 1,
        "new-turn Bacchus observer must delete one of the two opposing Digimon after Asuna plays Elecmon");
    assert_eq!(bacchus_uses(&runner), 1, "the new-turn use remains spent");
}

#[test]
fn start_turn_opt_use_stays_spent_when_main_phase_begins() {
    let mut runner = start_turn_bacchus_fixture(false);
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "fresh Bacchus observer must resolve during the start window"
    );
    assert_eq!(
        bacchus_uses(&runner),
        1,
        "advancing to Main must not clear a use from this turn"
    );
    runner.game.memory = 10;
    play(&mut runner, 0, "BT1-019");
    assert!(
        runner.pending_selection().is_none(),
        "a second play in this turn cannot reuse Bacchus"
    );
}

struct StartTurnEnemyChange {
    remove: bool,
}

impl digimon_engine::effect::CardEffect for StartTurnEnemyChange {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let remove = self.remove;
        vec![digimon_engine::effect::Effect::start_of_your_turn(card)
            .name("Change the opponent board before the Scramble condition is checked")
            .process(move |ctx| {
                if remove {
                    ctx.delete_permanent(digimon_engine::permanent::PermanentHandle {
                        player: 1,
                        index: 0,
                    });
                } else {
                    ctx.play_from_hand_free(1, 0);
                }
            })
            .build()]
    }
}

fn scramble_processing_condition_fixture(remove: bool) -> DebugRunner {
    // Real Red Scramble, synthetic sibling body to isolate processing vs
    // trigger conditions (CR 4.3 15-6-1, 15-8-3-9-1/2).
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut data = CardData::load_from_str(&raw).unwrap();
    let mut sibling =
        digimon_engine::debug_runner::make_test_card("AUDIT-START", "Audit start sibling");
    sibling.card_kind = digimon_engine::enums::CardKind::Tamer;
    sibling.dp = None;
    sibling.level = None;
    let mut builder = DebugRunner::builder()
        .dsl_card("LM-027")
        .unwrap()
        .add_card(sibling);
    for id in ["LM-027", "BT1-019"] {
        builder = builder.add_card(data.remove(id).unwrap());
    }
    let mut runner = builder
        .deck(0, &["BT1-019"; 15])
        .deck(1, &["BT1-019"; 15])
        .memory(10)
        .start();
    runner.register_effect(
        "AUDIT-START",
        std::sync::Arc::new(StartTurnEnemyChange { remove }),
    );
    runner.set_phase(GamePhase::Main);
    runner.place_on_field(0, "AUDIT-START", Some(0));
    let delay = runner.place_on_field(0, "LM-027", Some(0));
    runner.game.players[0].battle_area[delay.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner: 0,
            trash_on_turn: 4,
            trigger: digimon_engine::enums::DelayTrigger::StartOfYourNextTurn,
            placed_on_turn: 0,
        };
    if remove {
        runner.place_on_field(1, "BT1-019", Some(0));
    } else {
        runner.add_to_hand(1, "BT1-019");
    }
    let hand = runner.add_to_hand(0, "BT1-019");
    let card = runner.game.players[0].hand.remove(hand);
    runner.game.players[0].trash.push(card);
    runner.set_turn(3);
    runner.set_first_player(1);
    runner.game.memory = -2;
    runner.end_turn();
    if remove {
        let view = runner.pending_selection_view().unwrap();
        let action = view
            .effect_choices
            .unwrap()
            .iter()
            .find(|c| c.label.contains("AUDIT-START"))
            .unwrap()
            .action_id;
        runner.execute_action(0, action).unwrap();
    }
    runner
}

#[test]
fn start_turn_sibling_can_enable_scramble_processing_condition() {
    let mut runner = scramble_processing_condition_fixture(false);
    let view = runner.pending_selection_view().expect(
        "Scramble must remain pending until its sibling can enable its processing condition",
    );
    assert!(view.is_optional);
    finish(&mut runner);
    assert!(!runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "LM-027"));
}

#[test]
fn start_turn_sibling_can_disable_scramble_before_activation() {
    let runner = scramble_processing_condition_fixture(true);
    assert!(runner.pending_selection().is_none());
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "LM-027"));
    assert_eq!(runner.game.current_phase, GamePhase::Main);
}

struct NativeDelayReplacesCarrier {
    park: bool,
}

impl digimon_engine::effect::CardEffect for NativeDelayReplacesCarrier {
    fn effects(
        &self,
        card: digimon_engine::card_source::CardHandle,
    ) -> Vec<digimon_engine::effect::Effect> {
        let park = self.park;
        vec![digimon_engine::effect::Effect::on_play(card)
            .name("Native Delay reseats its physical Option as a new permanent")
            .delay(digimon_engine::enums::DelayTrigger::StartOfYourNextTurn)
            .process(move |ctx| {
                // Public native card API: the physical card survives, but the
                // old permanent has left and a newly placed one now exists.
                let returned = ctx
                    .bounce_self()
                    .expect("native body returns its old carrier");
                let hand = ctx
                    .game
                    .player(ctx.player)
                    .hand
                    .iter()
                    .position(|c| c.handle() == returned)
                    .unwrap();
                let played = ctx
                    .game
                    .use_option_from_hand_without_paying_cost(ctx.player, hand);
                assert!(
                    !matches!(played, digimon_engine::selection::OptionPlayResult::Invalid),
                    "native effect reuses returned Option"
                );
                assert!(
                    ctx.game.player(ctx.player).battle_area.iter().any(|p| !p
                        .card_sources
                        .is_empty()
                        && p.top_card().handle() == card
                        && matches!(
                            p.option_state,
                            digimon_engine::permanent::OptionState::Delayed {
                                placed_on_turn: 4,
                                ..
                            }
                        )),
                    "native re-placement must create its new carrier before cleanup: {:?}",
                    ctx.game.player(ctx.player).battle_area
                );
                if park {
                    ctx.select_own_permanent("continue native body", false, |_, _| true, |_, _| {});
                    // This witness has no work after the choice. Keep that
                    // continuation as cloneable data, not a closure callback.
                    ctx.game.pending_selection_resume =
                        Some(digimon_engine::resume::ResumeStack::new());
                }
            })
            .build()]
    }
}

fn native_delay_new_incarnation_fixture(
    park: bool,
) -> (DebugRunner, digimon_engine::card_source::CardHandle) {
    let mut option =
        digimon_engine::debug_runner::make_test_card("AUDIT-DELAY", "Audit native Delay");
    option.card_kind = digimon_engine::enums::CardKind::Option;
    option.level = None;
    option.dp = None;
    let filler = digimon_engine::debug_runner::make_test_card("AUDIT-FILLER", "Audit filler");
    let mut runner = DebugRunner::builder()
        .add_card(option)
        .add_card(filler)
        .deck(0, &["AUDIT-FILLER"; 15])
        .deck(1, &["AUDIT-FILLER"; 15])
        .memory(10)
        .start();
    runner.register_effect(
        "AUDIT-DELAY",
        std::sync::Arc::new(NativeDelayReplacesCarrier { park }),
    );
    runner.set_phase(GamePhase::Main);
    let old = runner.place_on_field(0, "AUDIT-DELAY", Some(0));
    let physical_card = runner.game.players[0].battle_area[old.index as usize]
        .top_card()
        .handle();
    runner.game.players[0].battle_area[old.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner: 0,
            trash_on_turn: 4,
            trigger: digimon_engine::enums::DelayTrigger::StartOfYourNextTurn,
            placed_on_turn: 0,
        };
    runner.place_on_field(0, "AUDIT-FILLER", Some(0));
    runner.set_turn(3);
    runner.set_first_player(1);
    runner.game.memory = -2;
    runner.end_turn();
    (runner, physical_card)
}

fn assert_new_delay_survives(
    runner: &DebugRunner,
    physical_card: digimon_engine::card_source::CardHandle,
) {
    let new = runner.game.players[0].battle_area.iter().find(|p| p.top_card().handle() == physical_card)
        .unwrap_or_else(|| panic!("cleanup of an old native Delay must not trash a new incarnation of the same physical card; field={:?}, trash={:?}, events={:?}", runner.game.players[0].battle_area, runner.game.players[0].trash, runner.game.events));
    assert!(matches!(
        new.option_state,
        digimon_engine::permanent::OptionState::Delayed {
            placed_on_turn: 4,
            trash_on_turn: 6,
            ..
        }
    ));
    assert_eq!(runner.game.current_phase, GamePhase::Main);
}

#[test]
fn native_start_delay_cleanup_does_not_delete_replaced_same_card() {
    let (runner, physical_card) = native_delay_new_incarnation_fixture(false);
    assert_new_delay_survives(&runner, physical_card);
}

#[test]
fn native_start_delay_parked_cleanup_preserves_new_incarnation_after_clone() {
    let (mut runner, physical_card) = native_delay_new_incarnation_fixture(true);
    assert!(runner.pending_selection().is_some());
    let (mut cloned, _) = native_delay_new_incarnation_fixture(true);
    cloned.game = runner.game.clone();
    for branch in [&mut runner, &mut cloned] {
        finish(branch);
        assert_new_delay_survives(branch, physical_card);
    }
}
