use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::SelectionKind;
use digimon_engine::selection::TriggerSource;

struct PayCostSelectsSourceThenRunsProcess {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for PayCostSelectsSourceThenRunsProcess {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("pay cost selects source, then process")
            .pay_cost(move |ctx| {
                cost_log.lock().unwrap().push("pay_cost");
                ctx.select_own_sources(
                    "Trash 1 source to pay this effect cost",
                    1,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC")
                    },
                    move |ctx, refs| {
                        assert_eq!(refs.len(), 1);
                        assert!(ctx.game.trash_source_ref(refs[0]));
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("process");
                ctx.gain_memory(3);
            })
            .build()]
    }
}

struct PayCostSelectionDeclines {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for PayCostSelectionDeclines {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("pay cost selection declines")
            .pay_cost_fn(move |ctx| {
                cost_log.lock().unwrap().push("pay_cost");
                ctx.select_own_sources(
                    "Choose source, then decline pay cost",
                    1,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC")
                    },
                    move |ctx, refs| {
                        assert_eq!(refs.len(), 1);
                        ctx.decline_pending_pay_cost();
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("process");
                ctx.gain_memory(3);
            })
            .build()]
    }
}

struct OptionalPayCostSelectsSource {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for OptionalPayCostSelectsSource {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("optional pay cost selects source")
            .pay_cost(move |ctx| {
                ctx.select_own_sources(
                    "Optionally trash 1 source to pay this effect cost",
                    0,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC")
                    },
                    {
                        let cost_log = cost_log.clone();
                        move |ctx, refs| match refs.as_slice() {
                            [] => {
                                cost_log.lock().unwrap().push("decline_cost");
                                ctx.decline_pending_pay_cost();
                            }
                            [source] => {
                                cost_log.lock().unwrap().push("accept_cost");
                                assert!(ctx.game.trash_source_ref(*source));
                            }
                            _ => panic!("optional pay cost should choose at most 1 source"),
                        }
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("process");
                ctx.gain_memory(3);
            })
            .build()]
    }
}

struct InnerPayCostSelectsSource {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for InnerPayCostSelectsSource {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::end_of_your_turn(card)
            .name("inner pay cost selects source")
            .pay_cost_fn(move |ctx| {
                cost_log.lock().unwrap().push("inner_pay_cost");
                ctx.select_own_sources(
                    "Trash inner source to pay cost",
                    1,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC-B")
                    },
                    move |ctx, refs| {
                        assert_eq!(refs.len(), 1);
                        assert!(ctx.game.trash_source_ref(refs[0]));
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("inner_process");
                ctx.gain_memory(5);
            })
            .build()]
    }
}

struct InnerOptionalPayCostDeclines {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for InnerOptionalPayCostDeclines {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::end_of_your_turn(card)
            .name("inner optional pay cost may decline")
            .pay_cost_fn(move |ctx| {
                ctx.select_own_sources(
                    "Optionally trash inner source to pay cost",
                    0,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC-B")
                    },
                    {
                        let cost_log = cost_log.clone();
                        move |ctx, refs| match refs.as_slice() {
                            [] => {
                                cost_log.lock().unwrap().push("inner_decline_cost");
                                ctx.decline_pending_pay_cost();
                            }
                            [source] => {
                                cost_log.lock().unwrap().push("inner_accept_cost");
                                assert!(ctx.game.trash_source_ref(*source));
                            }
                            _ => panic!("inner optional cost should choose at most 1 source"),
                        }
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("inner_process");
                ctx.gain_memory(5);
            })
            .build()]
    }
}

struct OuterPayCostCallbackDrainsInnerQueue {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for OuterPayCostCallbackDrainsInnerQueue {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let callback_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("outer pay cost selection drains inner queue")
            .pay_cost_fn(move |ctx| {
                cost_log.lock().unwrap().push("outer_pay_cost");
                let callback_log = callback_log.clone();
                ctx.select_own_sources(
                    "Trash outer source to pay cost",
                    1,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC-A")
                    },
                    move |ctx, refs| {
                        assert_eq!(refs.len(), 1);
                        assert!(ctx.game.trash_source_ref(refs[0]));
                        callback_log.lock().unwrap().push("outer_callback");
                        ctx.game.enqueue_triggered(
                            EffectTiming::EndOfYourTurn,
                            TriggerSource::PlayerBattleArea(0),
                        );
                        ctx.game.drain_effect_queue();
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("outer_process");
                ctx.gain_memory(3);
            })
            .build()]
    }
}

struct OncePerTurnPayCostSlotConsumedBeforeResume {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for OncePerTurnPayCostSlotConsumedBeforeResume {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("once per turn pay cost selection")
            .once_per_turn()
            .pay_cost_fn(move |ctx| {
                cost_log.lock().unwrap().push("pay_cost");
                ctx.select_own_sources(
                    "Consume OPT before pay-cost resumes",
                    1,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC")
                    },
                    move |ctx, refs| {
                        assert_eq!(refs.len(), 1);
                        if let Some(source) = ctx.source_permanent {
                            let source_card = ctx.source_card;
                            ctx.game.player_mut(source.player).battle_area[source.index as usize]
                                .record_activation(source_card, 0);
                        }
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("process");
                ctx.gain_memory(3);
            })
            .build()]
    }
}

struct PayCostCallbackRemovesSourceBeforeResume {
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for PayCostCallbackRemovesSourceBeforeResume {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let cost_log = self.log.clone();
        let process_log = self.log.clone();
        vec![Effect::on_play(card)
            .name("pay cost removes source before resume")
            .pay_cost_fn(move |ctx| {
                cost_log.lock().unwrap().push("pay_cost");
                ctx.select_own_sources(
                    "Remove source permanent before pay-cost resumes",
                    1,
                    1,
                    |game, source_ref| {
                        game.card_data_for_handle(source_ref.card)
                            .is_some_and(|data| data.card_id == "SRC")
                    },
                    move |ctx, refs| {
                        assert_eq!(refs.len(), 1);
                        if let Some(source) = ctx.source_permanent {
                            ctx.game
                                .player_mut(source.player)
                                .battle_area
                                .remove(source.index as usize);
                        }
                    },
                );
                true
            })
            .process(move |ctx| {
                process_log.lock().unwrap().push("process");
                ctx.gain_memory(3);
            })
            .build()]
    }
}

#[test]
fn triggered_pay_cost_source_selection_parks_process_until_selection_resolves() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(PayCostSelectsSourceThenRunsProcess { log: log.clone() }),
    );

    let host = r.place_stack(0, &["SRC", "HOST"]);

    let memory_before = r.memory();
    r.play(0, 0);

    assert_eq!(*log.lock().unwrap(), vec!["pay_cost"]);
    assert!(r.game.pending_pay_cost_effect.is_some());
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("source selection installed");
    assert!(matches!(
        pending.kind,
        SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0
        }
    ));
    assert_eq!(r.memory(), memory_before - 3);

    let action = encode_source_select(host.index as u16, 0).expect("source selection action");
    r.game
        .resolve_selection(0, action)
        .expect("source-selection action resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(*log.lock().unwrap(), vec!["pay_cost", "process"]);
    assert_eq!(r.memory(), memory_before);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn mandatory_pay_cost_selection_rejects_pass() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(PayCostSelectsSourceThenRunsProcess { log }),
    );

    r.place_stack(0, &["SRC", "HOST"]);
    r.play(0, 0);

    let err = r
        .game
        .resolve_selection(0, PASS)
        .expect_err("mandatory pay-cost source selection must reject PASS");
    assert_eq!(format!("{err:?}"), "InvalidAction");
    assert!(r.game.pending_selection.is_some());
    assert!(r.game.pending_pay_cost_effect.is_some());
}

#[test]
fn pay_cost_selection_decline_discards_parked_process() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(PayCostSelectionDeclines { log: log.clone() }),
    );

    let host = r.place_stack(0, &["SRC", "HOST"]);
    let memory_before = r.memory();
    r.play(0, 0);

    assert_eq!(*log.lock().unwrap(), vec!["pay_cost"]);
    assert!(r.game.pending_pay_cost_effect.is_some());

    let action = encode_source_select(host.index as u16, 0).expect("source selection action");
    r.game
        .resolve_selection(0, action)
        .expect("source-selection action resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(*log.lock().unwrap(), vec!["pay_cost"]);
    assert_eq!(r.memory(), memory_before - 3);
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn optional_pay_cost_decline_skips_process() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(OptionalPayCostSelectsSource { log: log.clone() }),
    );

    r.place_stack(0, &["SRC", "HOST"]);
    let memory_before = r.memory();
    r.play(0, 0);

    assert!(r.game.pending_pay_cost_effect.is_some());
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional source selection installed");
    assert!(matches!(
        pending.kind,
        SelectionKind::SourceMulti {
            min: 0,
            max: 1,
            picked: 0
        }
    ));
    assert!(pending.is_optional);

    r.game
        .resolve_selection(0, PASS)
        .expect("optional pay-cost PASS resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(*log.lock().unwrap(), vec!["decline_cost"]);
    assert_eq!(r.memory(), memory_before - 3);
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn optional_pay_cost_accept_pays_cost_and_resumes_process() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(OptionalPayCostSelectsSource { log: log.clone() }),
    );

    let host = r.place_stack(0, &["SRC", "HOST"]);
    let memory_before = r.memory();
    r.play(0, 0);

    let action = encode_source_select(host.index as u16, 0).expect("source selection action");
    r.game
        .resolve_selection(0, action)
        .expect("optional pay-cost source selection resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(*log.lock().unwrap(), vec!["accept_cost", "process"]);
    assert_eq!(r.memory(), memory_before);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn nested_pay_cost_selection_does_not_overwrite_outer_continuation() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST-A", "Host A"))
        .add_card(make_test_card("HOST-B", "Host B"))
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("INNER", "Inner"))
        .add_card(make_test_card("OUTER", "Outer"))
        .hand(0, &["OUTER"])
        .memory(5)
        .start();
    r.register_effect(
        "INNER",
        Arc::new(InnerPayCostSelectsSource { log: log.clone() }),
    );
    r.register_effect(
        "OUTER",
        Arc::new(OuterPayCostCallbackDrainsInnerQueue { log: log.clone() }),
    );

    let outer_host = r.place_stack(0, &["SRC-A", "HOST-A"]);
    let inner_host = r.place_stack(0, &["SRC-B", "HOST-B"]);
    r.place_on_field(0, "INNER", Some(0));

    let memory_before = r.memory();
    r.play(0, 0);

    let outer_action =
        encode_source_select(outer_host.index as u16, 0).expect("outer source action");
    r.game
        .resolve_selection(0, outer_action)
        .expect("outer source-selection action resolves");

    assert_eq!(
        *log.lock().unwrap(),
        vec!["outer_pay_cost", "outer_callback", "inner_pay_cost"]
    );
    assert!(r.game.pending_selection.is_some());
    assert!(r.game.pending_pay_cost_effect.is_some());

    let inner_action =
        encode_source_select(inner_host.index as u16, 0).expect("inner source action");
    r.game
        .resolve_selection(0, inner_action)
        .expect("inner source-selection action resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "outer_pay_cost",
            "outer_callback",
            "inner_pay_cost",
            "inner_process",
            "outer_process"
        ]
    );
    assert_eq!(r.memory(), memory_before - 3 + 5 + 3);
    assert_eq!(r.trash_size(0), 2);
}

#[test]
fn nested_optional_pay_cost_decline_skips_active_inner_and_resumes_outer() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST-A", "Host A"))
        .add_card(make_test_card("HOST-B", "Host B"))
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("SRC-B", "Source B"))
        .add_card(make_test_card("INNER", "Inner"))
        .add_card(make_test_card("OUTER", "Outer"))
        .hand(0, &["OUTER"])
        .memory(5)
        .start();
    r.register_effect(
        "INNER",
        Arc::new(InnerOptionalPayCostDeclines { log: log.clone() }),
    );
    r.register_effect(
        "OUTER",
        Arc::new(OuterPayCostCallbackDrainsInnerQueue { log: log.clone() }),
    );

    let outer_host = r.place_stack(0, &["SRC-A", "HOST-A"]);
    r.place_stack(0, &["SRC-B", "HOST-B"]);
    r.place_on_field(0, "INNER", Some(0));

    let memory_before = r.memory();
    r.play(0, 0);

    let outer_action =
        encode_source_select(outer_host.index as u16, 0).expect("outer source action");
    r.game
        .resolve_selection(0, outer_action)
        .expect("outer source-selection action resolves");

    assert_eq!(
        *log.lock().unwrap(),
        vec!["outer_pay_cost", "outer_callback"]
    );
    assert!(r.game.pending_selection.is_some());
    assert!(r.game.pending_pay_cost_effect.is_some());
    assert_eq!(r.game.pending_pay_cost_stack.len(), 1);

    r.game
        .resolve_selection(0, PASS)
        .expect("inner optional pay-cost PASS resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert!(r.game.pending_pay_cost_stack.is_empty());
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "outer_pay_cost",
            "outer_callback",
            "inner_decline_cost",
            "outer_process"
        ]
    );
    assert_eq!(r.memory(), memory_before - 3 + 3);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn resumed_pay_cost_process_rechecks_once_per_turn_slot() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(OncePerTurnPayCostSlotConsumedBeforeResume { log: log.clone() }),
    );

    let host = r.place_stack(0, &["SRC", "HOST"]);
    let memory_before = r.memory();
    r.play(0, 0);

    let action = encode_source_select(host.index as u16, 0).expect("source selection action");
    r.game
        .resolve_selection(0, action)
        .expect("source-selection action resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(*log.lock().unwrap(), vec!["pay_cost"]);
    assert_eq!(r.memory(), memory_before - 3);
}

#[test]
fn resumed_pay_cost_process_rechecks_source_liveness() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("HOST", "Host"))
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PAYSEL", "PaySel"))
        .hand(0, &["PAYSEL"])
        .memory(10)
        .start();
    r.register_effect(
        "PAYSEL",
        Arc::new(PayCostCallbackRemovesSourceBeforeResume { log: log.clone() }),
    );

    let host = r.place_stack(0, &["SRC", "HOST"]);
    let memory_before = r.memory();
    r.play(0, 0);

    let action = encode_source_select(host.index as u16, 0).expect("source selection action");
    r.game
        .resolve_selection(0, action)
        .expect("source-selection action resolves");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_pay_cost_effect.is_none());
    assert_eq!(*log.lock().unwrap(), vec!["pay_cost"]);
    assert_eq!(r.memory(), memory_before - 3);
}
