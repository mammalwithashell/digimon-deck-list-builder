//! Occurrence/activation regressions: official rules 13-1-7-1/2/3,
//! 15-4-3/4/5, Q2629 and Q6813. Synthetic effects isolate engine timing;
//! these tests make no DCGO or production BT16-033 readiness claim.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_attack, BREEDING_TARGET, PASS, SECURITY_TARGET},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{EffectTiming, GamePhase, Keyword};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use std::sync::{Arc, Mutex};

type Trace = Arc<Mutex<Vec<String>>>;

fn fighter(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 5;
    card
}

const HARP_SHAPE: &str = r#"
card: CHECK-SOURCE
name: Synthetic Harpymon timing source
kind: digimon
level: 4
color: [yellow, red]
cost: 5
dp: 5000
effects:
  - when: on_security_check
    active_when: { your_turn: true }
    condition: { event_target_is_source: true }
    process:
      - if:
          condition: { security_count: { op: gte, value: 3 } }
          then:
            - gain_memory: 1
      - if:
          condition: { security_count: { op: lte, value: 2 } }
          then:
            - recover: { of: you, count: 1 }
"#;

const CLONE_SECURITY: &str = r#"
card: SEC-CLONE
name: Clone Security fixture
kind: option
color: [red]
cost: 0
effects:
  - when: on_security
    process:
      - select_hand:
          of: you
          bind_as: chosen
          filter: { card_number_is: NEW }
          prompt: Security free play
      - play_from_hand: { of: you, hand_index: chosen, cost_delta: free }
  - when: on_security
    process:
      - gain_memory: 1
"#;

const SHIFTING_OBSERVER: &str = r#"
card: SHIFT-OBSERVER
name: Self-removing observer fixture
kind: digimon
level: 4
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_security_check
    process:
      - delete_permanent: { target: source }
      - add_dp_modifier: { target: event_target, value: 5000, expiry: end_of_turn }
"#;

const PARKED_SHIFTING_OBSERVER: &str = r#"
card: PARKED-SHIFT-OBSERVER
name: Parked self-removing observer fixture
kind: digimon
level: 4
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_security_check
    process:
      - select_own_permanent:
          bind_as: chosen
          filter: { kind: digimon }
          prompt: Continue the captured observer
      - delete_permanent: { target: source }
      - add_dp_modifier: { target: event_target, value: 5000, expiry: end_of_turn }
"#;

const TARGET_BUFF_OBSERVER: &str = r#"
card: TARGET-BUFF-OBSERVER
name: Captured attacker target fixture
kind: digimon
level: 4
color: [red]
cost: 3
dp: 1000
effects:
  - when: on_security_check
    process:
      - add_dp_modifier: { target: event_target, value: 5000, expiry: end_of_turn }
"#;

fn runner(seat: u8, security: &[&str]) -> DebugRunner {
    let other = 1 - seat;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(HARP_SHAPE)
        .expect("valid inline Harpymon-shaped DSL")
        .from_dsl_yaml(CLONE_SECURITY)
        .expect("valid cloneable Security fixture")
        .from_dsl_yaml(TARGET_BUFF_OBSERVER)
        .expect("valid captured attacker buff fixture")
        .from_dsl_yaml(PARKED_SHIFTING_OBSERVER)
        .expect("valid parked source-removal/attacker target fixture")
        .from_dsl_yaml(SHIFTING_OBSERVER)
        .expect("valid source-removal/attacker target fixture")
        .add_card(fighter("INERT", 1000))
        .add_card(fighter("LOSING-SEC", 6000))
        .add_card(fighter("SEC-PLAY", 2000))
        .add_card(fighter("SEC-SHIFT", 2000))
        .add_card(fighter("OBSERVER", 2000))
        .add_card(fighter("FACE-UP", 2000))
        .add_card(fighter("NEW", 2000))
        .add_card(fighter("ALLY", 2000))
        .add_card(fighter("BYSTANDER", 1000))
        .security(seat, &["INERT"; 5])
        .security(other, security)
        .hand(other, &["NEW"])
        .deck(seat, &["INERT"; 10])
        .deck(other, &["INERT"; 10])
        .build();
    r.game.turn_player_idx = seat as usize;
    r.game.memory_pair = (seat, other);
    r.game.memory = 3;
    r.game.turn_count = 2;
    r.game.current_phase = GamePhase::Main;
    r
}

/// Both seats run even if the first fails, so RED records neither hide seat1
/// nor confuse a gauge sign difference with a timing defect.
fn both_seats(mut scenario: impl FnMut(u8)) {
    let mut failures = Vec::new();
    for seat in 0..=1 {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scenario(seat))).is_err() {
            failures.push(seat);
        }
    }
    assert!(
        failures.is_empty(),
        "behavioral failures for attacker seats {failures:?}"
    );
}

fn attack(r: &mut DebugRunner, attacker: PermanentHandle) {
    let action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert_eq!(r.turn_player(), attacker.player);
    assert_eq!(
        build_action_mask(&r.game, attacker.player)[action as usize],
        1.0,
        "fixture attack must be a real legal masked action"
    );
    r.game.decode_action(action, attacker.player);
}

fn assert_prompt_mask(r: &DebugRunner, actor: u8) {
    let sel = r
        .pending_selection()
        .expect("pending player-visible choice");
    assert_eq!(sel.selecting_player, actor);
    let expected: Vec<u16> = {
        let mut ids = sel.valid_action_ids.clone();
        if sel.is_optional && !ids.contains(&PASS) {
            ids.push(PASS);
        }
        ids.sort();
        ids
    };
    for seat in 0..=1 {
        let actual: Vec<u16> = build_action_mask(&r.game, seat)
            .iter()
            .enumerate()
            .filter_map(|(id, &legal)| (legal > 0.5).then_some(id as u16))
            .collect();
        assert_eq!(
            actual,
            if seat == actor {
                expected.clone()
            } else {
                vec![]
            }
        );
    }
}

/// Test choices explicitly select the labelled source, never bypass the mask.
fn choose_source(r: &mut DebugRunner, source: CardHandle) {
    let sel = r.pending_selection().expect("source-labelled TriggerOrder");
    assert_eq!(sel.kind, SelectionKind::TriggerOrder);
    let actor = sel.selecting_player;
    let action = sel
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|entry| entry.source_card == Some(source))
        .expect("requested source offered")
        .action_id;
    assert_prompt_mask(r, actor);
    assert!(
        r.execute_action(1 - actor, action).is_err(),
        "wrong actor must be rejected"
    );
    r.execute_action(actor, action).unwrap();
}

struct Observe {
    timing: EffectTiming,
    label: &'static str,
    trace: Trace,
    self_only: bool,
}
impl CardEffect for Observe {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let label = self.label;
        let trace = self.trace.clone();
        let self_only = self.self_only;
        vec![Effect::on_security_check(card)
            .timing(self.timing)
            .name(label)
            .condition(move |ctx| !self_only || ctx.source_permanent == ctx.attacker())
            .process(move |ctx| {
                let attacking_alive = ctx
                    .game
                    .players
                    .iter()
                    .flat_map(|p| p.battle_area.iter())
                    .any(|p| p.is_attacking);
                trace
                    .lock()
                    .unwrap()
                    .push(format!("{label}:attacker_alive={attacking_alive}"));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

struct SecurityChoice {
    trace: Trace,
}
impl CardEffect for SecurityChoice {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let trace = self.trace.clone();
        vec![Effect::security(card)
            .name("immediate Security hand free-play")
            .process(move |ctx| {
                trace.lock().unwrap().push("security:start".into());
                let player = ctx.player;
                let trace = trace.clone();
                ctx.select_hand(
                    player,
                    "Security: play NEW",
                    false,
                    |game, i| {
                        game.players[player as usize].hand[i].card_id(&game.card_data) == "NEW"
                    },
                    move |ctx, i| {
                        ctx.play_from_hand_free(player, i)
                            .expect("explicit valid 2000DP witness");
                        trace.lock().unwrap().push("security:finished".into());
                    },
                );
            })
            .build()]
    }
}

struct DeleteEarlierAlly;
impl CardEffect for DeleteEarlierAlly {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("Security removes preceding ally")
            .process(|ctx| {
                let attacker = ctx.attacker().expect("attacker during Security");
                ctx.delete_permanent(PermanentHandle {
                    player: attacker.player,
                    index: 0,
                });
            })
            .build()]
    }
}

#[test]
fn security_check_attacking_source_face_down_and_face_up_once() {
    both_seats(|seat| {
        let mut memory_observations = Vec::new();
        for face_up in [false, true] {
            let trace = Trace::default();
            let mut r = runner(seat, &["INERT"]);
            r.register_effect(
                "FACE-UP",
                Arc::new(Observe {
                    timing: EffectTiming::OnCheckFaceUpSecurity,
                    label: "face-up",
                    trace: trace.clone(),
                    self_only: false,
                }),
            );
            let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
            let atk_card = r.top_card(atk);
            r.place_on_field(seat, "CHECK-SOURCE", Some(0)); // nonattacking copy must not gain
            let face_observer = r.place_on_field(seat, "FACE-UP", Some(0));
            let face_card = r.top_card(face_observer);
            if face_up {
                let index = r.game.players[(1 - seat) as usize].security[0].card_index;
                r.game.players[(1 - seat) as usize]
                    .face_up_security
                    .insert(index);
            }
            assert_eq!(r.memory(), 3, "no declaration-only trigger");
            attack(&mut r, atk);
            if r.pending_selection().is_some() {
                choose_source(&mut r, atk_card);
            }
            if r.pending_selection().is_some() {
                choose_source(&mut r, face_card);
            }
            let observed = r.memory();
            println!(
                "seat={seat} face_up={face_up} memory={observed} trace={:?}",
                trace.lock().unwrap()
            );
            assert!(r.pending_selection().is_none());
            assert_eq!(r.security_count(1 - seat), 0);
            assert_eq!(trace.lock().unwrap().len(), usize::from(face_up));
            memory_observations.push((face_up, observed));
        }
        assert_eq!(
            memory_observations,
            vec![(false, 4), (true, 5)],
            "attacker gains exactly once on each face-down/up check; allied copy never gains"
        );
    });
}

#[test]
fn security_check_effect_precedes_losing_security_battle() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["LOSING-SEC"]);
        // The defender-side general observer isolates prebattle order from the
        // independent missing attacker discovery defect.
        r.register_effect(
            "OBSERVER",
            Arc::new(Observe {
                timing: EffectTiming::OnSecurityCheck,
                label: "prebattle-check",
                trace: trace.clone(),
                self_only: false,
            }),
        );
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        r.place_on_field(1 - seat, "OBSERVER", Some(0));
        attack(&mut r, atk);
        let observed = trace.lock().unwrap().clone();
        println!(
            "seat={seat} losing security timeline={observed:?} memory={}",
            r.memory()
        );
        assert_eq!(
            r.battle_area_size(seat),
            0,
            "5000 loses against 6000; no Armor Purge fixture"
        );
        assert_eq!(
            observed,
            vec!["prebattle-check:attacker_alive=true"],
            "check observer processes before battle deletion"
        );
        assert_eq!(
            r.memory(),
            3,
            "attacker +1 and defender +1 remain after battle"
        );
    });
}

#[test]
fn security_check_security_choice_precedes_one_simultaneous_observer_bundle() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect(
            "SEC-PLAY",
            Arc::new(SecurityChoice {
                trace: trace.clone(),
            }),
        );
        r.register_effect(
            "ALLY",
            Arc::new(Observe {
                timing: EffectTiming::OnOpponentSecurityRemoved,
                label: "turn-removal",
                trace: trace.clone(),
                self_only: false,
            }),
        );
        r.register_effect(
            "OBSERVER",
            Arc::new(Observe {
                timing: EffectTiming::OnOwnSecurityRemoved,
                label: "defender-removal",
                trace: trace.clone(),
                self_only: false,
            }),
        );
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let atk_card = r.top_card(atk);
        let ally = r.place_on_field(seat, "ALLY", Some(0));
        let ally_card = r.top_card(ally);
        r.place_on_field(1 - seat, "OBSERVER", Some(0));
        attack(&mut r, atk);
        assert_eq!(
            r.pending_kind(),
            Some(SelectionKind::Hand),
            "Security is the first prompt"
        );
        assert_prompt_mask(&r, 1 - seat);
        assert_eq!(r.memory(), 3);
        assert_eq!(*trace.lock().unwrap(), vec!["security:start"]);
        let action = r.pending_selection().unwrap().valid_action_ids[0];
        assert!(r.execute_action(seat, action).is_err());
        r.execute_action(1 - seat, action).unwrap();
        println!(
            "seat={seat} after Security trace={:?} pending={:?}",
            trace.lock().unwrap(),
            r.pending_selection()
        );
        assert_eq!(
            *trace.lock().unwrap(),
            vec!["security:start", "security:finished"],
            "no check/removal observer activates inside Security or ahead of turn-player choice"
        );
        let sel = r
            .pending_selection()
            .expect("common simultaneous check/removal bundle");
        assert_eq!(sel.kind, SelectionKind::TriggerOrder);
        assert_eq!(sel.selecting_player, seat);
        assert!(
            !sel.is_optional,
            "mandatory attacker effect prevents whole-bundle PASS"
        );
        let mut sources: Vec<_> = sel
            .effect_choices
            .as_ref()
            .unwrap()
            .iter()
            .map(|entry| entry.source_card.unwrap())
            .collect();
        sources.sort_by_key(|card| card.0);
        let mut expected = vec![atk_card, ally_card];
        expected.sort_by_key(|card| card.0);
        assert_eq!(
            sources, expected,
            "both turn-player timings offered by source identity"
        );
        choose_source(&mut r, atk_card);
        if r.pending_selection().is_some() {
            choose_source(&mut r, ally_card);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 4);
    });
}

#[test]
fn security_check_new_security_played_observer_excluded_until_next_check() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["INERT", "SEC-PLAY"]);
        // Remain on the attacking player's memory side despite four erroneous
        // defender activations on RED; isolate eligibility from turn rotation.
        r.game.memory = 6;
        r.register_effect(
            "SEC-PLAY",
            Arc::new(SecurityChoice {
                trace: trace.clone(),
            }),
        );
        for (id, label) in [("OBSERVER", "old"), ("NEW", "new")] {
            r.register_effect(
                id,
                Arc::new(Observe {
                    timing: EffectTiming::OnOwnSecurityRemoved,
                    label,
                    trace: trace.clone(),
                    self_only: false,
                }),
            );
        }
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let old = r.place_on_field(1 - seat, "OBSERVER", Some(0));
        let old_card = r.top_card(old);
        attack(&mut r, atk);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
        assert_prompt_mask(&r, 1 - seat);
        let action = r.pending_selection().unwrap().valid_action_ids[0];
        r.execute_action(1 - seat, action).unwrap();
        let new_card = r.top_card(r.perm_handle(1 - seat, 1));
        // Resolve any incorrectly offered RED bundle explicitly to record the
        // actual first-check behavior without concealing the false eligibility.
        if r.pending_selection().is_some() {
            choose_source(&mut r, old_card);
        }
        if r.pending_selection().is_some() {
            choose_source(&mut r, new_card);
        }
        let first = trace.lock().unwrap().clone();
        r.game.players[seat as usize].battle_area[0].is_suspended = false;
        attack(&mut r, atk);
        assert_eq!(
            r.pending_kind(),
            Some(SelectionKind::TriggerOrder),
            "both old/new eligible on next reveal"
        );
        choose_source(&mut r, old_card);
        if r.pending_selection().is_some() {
            choose_source(&mut r, new_card);
        }
        let final_trace = trace.lock().unwrap().clone();
        println!("seat={seat} first={first:?} complete={final_trace:?}");
        assert_eq!(r.security_count(1 - seat), 0);
        assert_eq!(
            first,
            vec![
                "security:start",
                "security:finished",
                "old:attacker_alive=true"
            ],
            "Security-played observer did not exist when first card was removed"
        );
        assert_eq!(
            final_trace,
            vec![
                "security:start",
                "security:finished",
                "old:attacker_alive=true",
                "old:attacker_alive=true",
                "new:attacker_alive=true"
            ]
        );
    });
}

#[test]
fn security_check_compaction_keeps_attacker_event_and_next_check_identity() {
    both_seats(|seat| {
        let mut r = runner(seat, &["INERT", "SEC-SHIFT"]);
        r.register_effect("SEC-SHIFT", Arc::new(DeleteEarlierAlly));
        r.place_on_field(seat, "ALLY", Some(0));
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let atk_card = r.top_card(atk);
        let bystander = r.place_on_field(seat, "BYSTANDER", Some(0));
        let bystander_card = r.top_card(bystander);
        // Native printed keyword, with a distinct 1000DP bystander at old slot2.
        let index = r
            .game
            .card_data
            .iter()
            .position(|data| data.card_id == "CHECK-SOURCE")
            .unwrap();
        r.game.card_data[index]
            .keywords
            .push(Keyword::SecurityAttackPlus(1));
        let checkpoint = r.event_checkpoint();
        attack(&mut r, atk);
        let reveals: Vec<_> = r
            .events_since(checkpoint)
            .iter()
            .filter_map(|event| match event {
                GameEvent::SecurityReveal { card_id, .. } => Some(card_id.clone()),
                _ => None,
            })
            .collect();
        let live: Vec<_> = r.game.players[seat as usize]
            .battle_area
            .iter()
            .map(|p| p.top_card().handle())
            .collect();
        println!(
            "seat={seat} compaction live={live:?} reveals={reveals:?} memory={}",
            r.memory()
        );
        assert!(
            live.contains(&bystander_card),
            "stale attacker slot must never battle/delete shifted bystander"
        );
        assert!(live.contains(&atk_card));
        assert_eq!(
            reveals,
            vec!["SEC-SHIFT", "INERT"],
            "live shifted attacker completes two checks"
        );
        assert_eq!(r.security_count(1 - seat), 0);
        assert_eq!(
            r.memory(),
            5,
            "attacker-self identity resolves exactly once on each check"
        );
        assert!(r.pending_selection().is_none());
    });
}

#[test]
fn security_check_control_inert_security_real_attack_and_disposal() {
    both_seats(|seat| {
        let mut r = runner(seat, &["INERT"]);
        let atk = r.place_on_field(seat, "ALLY", Some(0));
        attack(&mut r, atk);
        assert_eq!(r.security_count(1 - seat), 0);
        assert_eq!(r.trash_size(1 - seat), 1);
        assert_eq!(r.battle_area_size(seat), 1);
        assert_eq!(r.memory(), 3);
        assert!(r.pending_selection().is_none());
    });
}

struct TransformAndRestore;
impl CardEffect for TransformAndRestore {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("temporarily cover original source")
            .process(|ctx| {
                let atk = ctx.attacker().unwrap();
                let player = ctx.player;
                let top = ctx.game.players[player as usize].hand.remove(0);
                let turn = ctx.game.turn_count;
                ctx.game.players[atk.player as usize].battle_area[atk.index as usize]
                    .digivolve(top, turn);
                assert!(ctx.trash_top_source(atk));
            })
            .build()]
    }
}

#[test]
fn security_check_transform_then_restore_does_not_revive_original_face_trigger() {
    both_seats(|seat| {
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(TransformAndRestore));
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let original = r.top_card(atk);
        attack(&mut r, atk);
        assert_eq!(
            r.top_card(atk),
            original,
            "same physical source restored before activation"
        );
        assert_eq!(
            r.memory(),
            3,
            "losing face effect cancels original pending activation permanently"
        );
    });
}

struct RemoveAttacker;
impl CardEffect for RemoveAttacker {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .process(|ctx| ctx.delete_permanent(ctx.attacker().unwrap()))
            .build()]
    }
}

#[test]
fn security_check_q2629_removed_attacker_no_gain_no_shifted_alias_or_second_reveal() {
    both_seats(|seat| {
        let mut r = runner(seat, &["INERT", "SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(RemoveAttacker));
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let index = r
            .game
            .card_data
            .iter()
            .position(|d| d.card_id == "CHECK-SOURCE")
            .unwrap();
        r.game.card_data[index]
            .keywords
            .push(Keyword::SecurityAttackPlus(1));
        attack(&mut r, atk);
        assert_eq!(r.memory(), 3);
        assert_eq!(r.battle_area_size(seat), 1);
        assert_eq!(r.security_count(1 - seat), 1);
        assert!(r.pending_selection().is_none());
    });
}

struct LeaveReplay;
impl CardEffect for LeaveReplay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .process(|ctx| {
                let player = ctx.player;
                let original = ctx.game.players[player as usize].battle_area[0]
                    .top_card()
                    .handle();
                ctx.return_to_hand(PermanentHandle { player, index: 0 });
                let hand_index = ctx.game.players[player as usize]
                    .hand
                    .iter()
                    .position(|c| c.handle() == original)
                    .unwrap();
                ctx.play_from_hand_free(player, hand_index).unwrap();
            })
            .build()]
    }
}

#[test]
fn security_check_same_physical_observer_leave_replay_is_new_source() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(LeaveReplay));
        r.register_effect(
            "OBSERVER",
            Arc::new(Observe {
                timing: EffectTiming::OnOwnSecurityRemoved,
                label: "original",
                trace: trace.clone(),
                self_only: false,
            }),
        );
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let observer = r.place_on_field(1 - seat, "OBSERVER", Some(0));
        let physical_card = r.top_card(observer);
        attack(&mut r, atk);
        assert_eq!(r.top_card(observer), physical_card);
        assert!(
            trace.lock().unwrap().is_empty(),
            "same handle replay cannot revive earlier occurrence"
        );
        assert_eq!(r.memory(), 4);
    });
}

struct MemorySecurity;
impl CardEffect for MemorySecurity {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .process(|ctx| ctx.game.gain_memory_for_player(1 - ctx.player, 2))
            .build()]
    }
}

struct SeparateGates {
    trace: Trace,
}
impl CardEffect for SeparateGates {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let lapsed = self.trace.clone();
        let invalid = self.trace.clone();
        let live = self.trace.clone();
        vec![
            Effect::on_security_check(card)
                .name("met at the capture, lapsed before activation")
                .trigger_qualifier(|ctx| ctx.game.memory == 3)
                .condition(|ctx| ctx.game.memory >= 5)
                .process(move |_| lapsed.lock().unwrap().push("lapsed".into()))
                .build(),
            Effect::on_security_check(card)
                .name("ineligible occurrence never discovered later")
                .trigger_qualifier(|ctx| ctx.game.memory >= 5)
                .condition(|ctx| ctx.game.memory >= 5)
                .process(move |_| invalid.lock().unwrap().push("invalid".into()))
                .build(),
            Effect::on_security_check(card)
                .name("still met at activation, processing condition read live")
                .trigger_qualifier(|ctx| ctx.game.memory >= 3)
                .condition(|ctx| ctx.game.memory >= 5)
                .process(move |_| live.lock().unwrap().push("live".into()))
                .build(),
        ]
    }
}

/// The trigger qualifier of a captured check observer is decided at the event
/// and re-read when the effect would activate (§15-4-4-5): while the event's
/// referent is on the field the re-read is live, once it has left it reads the
/// referent's snapshot, and the captured event data are kept. The `[Security]`
/// body raises memory 3 -> 5 between the capture and the activation:
/// - qualifier `memory == 3`: met at the capture, no longer met when it would
///   activate, so it can no longer be activated (§15-4-4-5);
/// - qualifier `memory >= 5`: not met at the event, so it never triggered and
///   is not discovered later (§15-8-3-1); a trigger condition met again only
///   triggers again on a new event (§15-4-2-4);
/// - qualifier `memory >= 3`: still met, and its processing condition
///   `memory >= 5` is read live at activation, so it activates.
#[test]
fn security_check_trigger_qualifier_decided_at_capture_rechecked_at_activation() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(MemorySecurity));
        r.register_effect(
            "OBSERVER",
            Arc::new(SeparateGates {
                trace: trace.clone(),
            }),
        );
        let atk = r.place_on_field(seat, "ALLY", Some(0));
        r.place_on_field(seat, "OBSERVER", Some(0));
        attack(&mut r, atk);
        assert_eq!(
            *trace.lock().unwrap(),
            vec!["live"],
            "the lapsed qualifier stops its observer; the never-met one never triggered"
        );
        assert!(
            r.pending_selection().is_none(),
            "no false source/event trigger choice"
        );
        assert_eq!(r.memory(), 5);
    });
}

struct DerivedOnPlay {
    trace: Trace,
}
impl CardEffect for DerivedOnPlay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let trace = self.trace.clone();
        vec![Effect::on_play(card)
            .process(move |_| trace.lock().unwrap().push("derived:on_play".into()))
            .build()]
    }
}

#[test]
fn security_check_derived_on_play_after_security_body_before_older_turn_player_batch() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect(
            "SEC-PLAY",
            Arc::new(SecurityChoice {
                trace: trace.clone(),
            }),
        );
        r.register_effect(
            "NEW",
            Arc::new(DerivedOnPlay {
                trace: trace.clone(),
            }),
        );
        r.register_effect(
            "ALLY",
            Arc::new(Observe {
                timing: EffectTiming::OnOpponentSecurityRemoved,
                label: "older",
                trace: trace.clone(),
                self_only: false,
            }),
        );
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let attacker_card = r.top_card(atk);
        r.place_on_field(seat, "ALLY", Some(0));
        attack(&mut r, atk);
        assert_prompt_mask(&r, 1 - seat);
        let action = r.pending_selection().unwrap().valid_action_ids[0];
        r.execute_action(1 - seat, action).unwrap();
        assert_eq!(
            *trace.lock().unwrap(),
            vec!["security:start", "security:finished", "derived:on_play"]
        );
        assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder));
        assert_prompt_mask(&r, seat);
        choose_source(&mut r, attacker_card);
        assert_eq!(
            *trace.lock().unwrap(),
            vec![
                "security:start",
                "security:finished",
                "derived:on_play",
                "older:attacker_alive=true"
            ]
        );
    });
}

struct PlayMemory;
impl CardEffect for PlayMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn security_check_multi_security_barrier_clone_at_hand_and_trigger_order() {
    both_seats(|seat| {
        let mut r = runner(seat, &["SEC-CLONE"]);
        r.register_effect("NEW", Arc::new(PlayMemory));
        r.register_effect(
            "ALLY",
            Arc::new(Observe {
                timing: EffectTiming::OnOpponentSecurityRemoved,
                label: "ally",
                trace: Trace::default(),
                self_only: false,
            }),
        );
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let atk_card = r.top_card(atk);
        r.place_on_field(seat, "ALLY", Some(0));
        attack(&mut r, atk);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
        assert_prompt_mask(&r, 1 - seat);
        assert_eq!(
            r.memory(),
            3,
            "second immediate clause waits for first choice/body"
        );
        let mut clone_at_hand = DebugRunner::wrap(r.game.clone());
        for game in [&mut r, &mut clone_at_hand] {
            let id = game.pending_selection().unwrap().valid_action_ids[0];
            game.execute_action(1 - seat, id).unwrap();
            assert_eq!(game.pending_kind(), Some(SelectionKind::TriggerOrder));
            assert_prompt_mask(game, seat);
            assert_eq!(
                game.memory(),
                1,
                "Security second clause then derived OnPlay precede original observer batch"
            );
            let sources: Vec<_> = game
                .game
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::MemoryChange {
                        source_card_id: Some(id),
                        ..
                    } => Some(id.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(
                sources,
                vec!["SEC-CLONE", "NEW"],
                "derived activation waits for all immediate bodies"
            );
        }
        let mut clone_at_order = DebugRunner::wrap(r.game.clone());
        for game in [&mut r, &mut clone_at_hand, &mut clone_at_order] {
            choose_source(game, atk_card);
            assert_eq!(game.memory(), 3);
            assert_eq!(game.security_count(1 - seat), 0);
            assert_eq!(game.battle_area_size(1 - seat), 1);
            assert_eq!(game.trash_size(1 - seat), 1);
            assert!(game.pending_selection().is_none());
            assert!(game.game.security_resolution.is_none());
        }
    });
}

#[test]
fn security_check_armor_purge_security_removal_cancels_original_face_clone_resume() {
    both_seats(|seat| {
        let mut r = runner(seat, &["INERT", "SEC-PLAY"]);
        r.force_base_dp("SEC-PLAY", 1000); // promoted ALLY2000 survives this subsequent battle
        r.register_effect("SEC-PLAY", Arc::new(RemoveAttacker));
        let atk = r.place_stack(seat, &["ALLY", "CHECK-SOURCE"]);
        r.game.players[seat as usize].battle_area[atk.index as usize].turn_played = 0;
        let index = r
            .game
            .card_data
            .iter()
            .position(|d| d.card_id == "CHECK-SOURCE")
            .unwrap();
        r.game.card_data[index].keywords.push(Keyword::ArmorPurge);
        attack(&mut r, atk);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
        assert_prompt_mask(&r, seat);
        assert!(r.pending_selection().unwrap().is_optional);
        let mut cloned = DebugRunner::wrap(r.game.clone());
        for game in [&mut r, &mut cloned] {
            let id = game.pending_selection().unwrap().valid_action_ids[0];
            game.execute_action(seat, id).unwrap();
            assert_eq!(game.memory(), 3, "purged original face cannot gain");
            assert_eq!(
                game.game.players[seat as usize].battle_area[0]
                    .top_card()
                    .card_id(&game.game.card_data),
                "ALLY"
            );
            assert_eq!(game.security_count(1 - seat), 1);
            assert_eq!(game.trash_size(seat), 1);
            assert!(game.pending_selection().is_none());
        }
    });
}

struct PlayCheckedSelf;
impl CardEffect for PlayCheckedSelf {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .process(|ctx| ctx.play_pending_security())
            .build()]
    }
}

#[test]
fn security_check_played_checked_digimon_skips_battle_and_double_disposal() {
    both_seats(|seat| {
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.force_base_dp("SEC-PLAY", 8000); // would delete the 5000DP attacker if phantom battle occurs
        r.register_effect("SEC-PLAY", Arc::new(PlayCheckedSelf));
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        attack(&mut r, atk);
        assert_eq!(r.battle_area_size(seat), 1);
        assert_eq!(r.battle_area_size(1 - seat), 1);
        assert_eq!(r.trash_size(1 - seat), 0);
        assert_eq!(r.memory(), 4);
        assert!(r.game.pending_security.is_none());
    });
}

struct CancelSecurityAttack;
impl CardEffect for CancelSecurityAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .process(|ctx| {
                // Exercise the continuation's existing cancelled flag. The
                // public cancel API intentionally rejects Battle; this native
                // fixture does not claim printed Security cancel admission.
                let attack = ctx
                    .game
                    .pending_attack
                    .as_mut()
                    .expect("live native attack");
                assert_eq!(attack.state, digimon_engine::selection::AttackState::Battle);
                attack.cancelled = true;
            })
            .build()]
    }
}

#[test]
fn security_check_cancelled_attack_does_not_reveal_next_card() {
    both_seats(|seat| {
        let mut r = runner(seat, &["INERT", "SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(CancelSecurityAttack));
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let index = r
            .game
            .card_data
            .iter()
            .position(|d| d.card_id == "CHECK-SOURCE")
            .unwrap();
        r.game.card_data[index]
            .keywords
            .push(Keyword::SecurityAttackPlus(1));
        attack(&mut r, atk);
        assert_eq!(
            r.security_count(1 - seat),
            1,
            "cancelled attack cannot perform another actual check"
        );
        assert_eq!(
            r.memory(),
            4,
            "the event already revealed can still process its captured observer"
        );
    });
}

#[test]
fn security_check_live_threshold_recovery_then_memory_on_distinct_checks() {
    both_seats(|seat| {
        let mut r = runner(seat, &["INERT", "INERT"]);
        r.game.players[seat as usize].security.truncate(2);
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let index = r
            .game
            .card_data
            .iter()
            .position(|d| d.card_id == "CHECK-SOURCE")
            .unwrap();
        r.game.card_data[index]
            .keywords
            .push(Keyword::SecurityAttackPlus(1));
        let checkpoint = r.event_checkpoint();
        attack(&mut r, atk);
        assert_eq!(r.security_count(seat), 3, "first check recovers2 to3");
        assert_eq!(r.deck_size(seat), 9, "exactly one recovery");
        assert_eq!(
            r.memory(),
            4,
            "second check gains memory; no memory on first recovery"
        );
        let memory_positions: Vec<_> = r
            .events_since(checkpoint)
            .iter()
            .enumerate()
            .filter_map(|(i, event)| match event {
                GameEvent::MemoryChange {
                    source_card_id: Some(id),
                    ..
                } if id == "CHECK-SOURCE" => Some(i),
                _ => None,
            })
            .collect();
        let reveal_positions: Vec<_> = r
            .events_since(checkpoint)
            .iter()
            .enumerate()
            .filter_map(|(i, event)| matches!(event, GameEvent::SecurityReveal { .. }).then_some(i))
            .collect();
        assert_eq!(reveal_positions.len(), 2);
        assert_eq!(memory_positions.len(), 1);
        assert!(
            memory_positions[0] > reveal_positions[1],
            "memory belongs to the second actual occurrence"
        );
    });
}

struct TransformDefender;
impl CardEffect for TransformDefender {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .process(|ctx| {
                let player = ctx.player;
                let top = ctx.game.players[player as usize].hand.remove(0);
                let turn = ctx.game.turn_count;
                ctx.game.players[player as usize].battle_area[0].digivolve(top, turn);
                assert!(ctx.trash_top_source(PermanentHandle { player, index: 0 }));
            })
            .build()]
    }
}

struct LinkedObserver {
    trace: Trace,
}
impl CardEffect for LinkedObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let trace = self.trace.clone();
        vec![Effect::on_own_security_removed(card)
            .linked()
            .process(move |_| {
                trace.lock().unwrap().push("linked".into());
            })
            .build()]
    }
}

#[test]
fn security_check_linked_observer_retains_effect_when_host_changes_face() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(TransformDefender));
        r.register_effect(
            "OBSERVER",
            Arc::new(LinkedObserver {
                trace: trace.clone(),
            }),
        );
        // A linked card needs a [Link] requirement the host meets, or the rule
        // check trashes it (§17-1-3-2-7) before the check is ever performed.
        r.give_link_requirement("OBSERVER", 0);
        let atk = r.place_on_field(seat, "ALLY", Some(0));
        let host = r.place_on_field(1 - seat, "ALLY", Some(0));
        r.push_linked_owned(host, "OBSERVER", 1 - seat);
        attack(&mut r, atk);
        assert_eq!(
            *trace.lock().unwrap(),
            vec!["linked"],
            "host face loss never cancels the linked card's intact pending effect"
        );
    });
}

#[test]
fn security_check_current_observer_body_compaction_keeps_event_target_on_attacker() {
    both_seats(|seat| {
        let mut r = runner(seat, &["LOSING-SEC"]);
        r.place_on_field(seat, "SHIFT-OBSERVER", Some(0));
        let atk = r.place_on_field(seat, "ALLY", Some(0));
        let attacker_card = r.top_card(atk);
        let bystander = r.place_on_field(seat, "BYSTANDER", Some(0));
        let bystander_card = r.top_card(bystander);
        attack(&mut r, atk);
        let board = &r.game.players[seat as usize].battle_area;
        assert!(
            board.iter().any(|p| p.top_card().handle() == attacker_card),
            "event_target still denotes shifted attacker; buff lets2000 survive security6000"
        );
        let live_atk = r.perm_handle(
            seat,
            board
                .iter()
                .position(|p| p.top_card().handle() == attacker_card)
                .unwrap(),
        );
        let live_bystander = r.perm_handle(
            seat,
            board
                .iter()
                .position(|p| p.top_card().handle() == bystander_card)
                .unwrap(),
        );
        assert_eq!(r.effective_dp(live_atk), Some(7000));
        assert_eq!(
            r.effective_dp(live_bystander),
            Some(1000),
            "shifted bystander never receives event-target modifier"
        );
        assert_eq!(r.security_count(1 - seat), 0);
    });
}

struct OptionalSecurity;
impl CardEffect for OptionalSecurity {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::security(card)
                .optional()
                .needs_outer_optional_prompt()
                .process(|ctx| ctx.gain_memory(1))
                .build(),
            Effect::security(card)
                .process(|ctx| ctx.gain_memory(2))
                .build(),
        ]
    }
}

#[test]
fn security_check_optional_security_barrier_clone_accept_decline() {
    both_seats(|seat| {
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(OptionalSecurity));
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        attack(&mut r, atk);
        assert_prompt_mask(&r, 1 - seat);
        assert!(r.pending_selection().unwrap().is_optional);
        assert_eq!(
            r.memory(),
            3,
            "neither Security body nor captured observer ran before acceptance"
        );
        let mut declined = DebugRunner::wrap(r.game.clone());
        let accept = r.pending_selection().unwrap().valid_action_ids[0];
        r.execute_action(1 - seat, accept).unwrap();
        declined.execute_action(1 - seat, PASS).unwrap();
        assert_eq!(
            r.memory(),
            1,
            "accepted Security bodies spend3, then original observer gains1"
        );
        assert_eq!(
            declined.memory(),
            2,
            "declining first body still resolves second before observer"
        );
        for done in [&r, &declined] {
            assert!(done.pending_selection().is_none());
            assert!(done.game.pending_security.is_none());
            assert_eq!(done.trash_size(1 - seat), 1);
        }
    });
}

struct CostSecurity;
impl CardEffect for CostSecurity {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .optional()
            .activation_cost(|ctx| {
                ctx.gain_memory(1);
                true
            })
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn security_check_optional_activation_cost_is_declinable_before_payment() {
    both_seats(|seat| {
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(CostSecurity));
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        attack(&mut r, atk);
        assert_prompt_mask(&r, 1 - seat);
        assert_eq!(
            r.memory(),
            3,
            "optional cost never pays before player acceptance"
        );
        let mut declined = DebugRunner::wrap(r.game.clone());
        let accept = r.pending_selection().unwrap().valid_action_ids[0];
        r.execute_action(1 - seat, accept).unwrap();
        declined.execute_action(1 - seat, PASS).unwrap();
        assert_eq!(
            r.memory(),
            2,
            "one cost and body, then one captured observer"
        );
        assert_eq!(
            declined.memory(),
            4,
            "declined cost/body untouched; captured observer still activates"
        );
    });
}

struct SecurityTimeline {
    trace: Trace,
}
impl CardEffect for SecurityTimeline {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let trace = self.trace.clone();
        vec![Effect::security(card)
            .process(move |ctx| {
                trace.lock().unwrap().push(format!(
                    "security:pending={}",
                    ctx.game.pending_security.is_some()
                ));
            })
            .build()]
    }
}

#[test]
fn security_check_effect_created_attack_preserves_immediate_security_barrier() {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect(
            "SEC-PLAY",
            Arc::new(SecurityTimeline {
                trace: trace.clone(),
            }),
        );
        r.register_effect(
            "ALLY",
            Arc::new(Observe {
                timing: EffectTiming::OnSecurityCheck,
                label: "observer",
                trace: trace.clone(),
                self_only: true,
            }),
        );
        let atk = r.place_on_field(seat, "ALLY", Some(0));
        let source = r.top_card(atk);
        {
            let mut ctx = digimon_engine::effect_context::EffectContext::new(
                &mut r.game,
                source,
                Some(atk),
                seat,
            );
            ctx.may_attack_now(
                atk,
                digimon_engine::effect_context::AttackTargetRestriction::PlayerOnly,
                false,
                "Effect-created attack",
            )
            .unwrap();
        }
        assert_prompt_mask(&r, seat);
        let action = encode_attack(atk.index as u16, SECURITY_TARGET);
        assert!(r
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .contains(&action));
        r.execute_action(seat, action).unwrap();
        assert_eq!(
            *trace.lock().unwrap(),
            vec!["security:pending=true", "observer:attacker_alive=true"]
        );
        assert!(r.game.pending_security.is_none());
        assert_eq!(r.trash_size(1 - seat), 1);
    });
}

#[test]
fn security_check_parked_observer_clone_compaction_keeps_event_target_on_attacker() {
    both_seats(|seat| {
        let mut r = runner(seat, &["LOSING-SEC"]);
        r.place_on_field(seat, "PARKED-SHIFT-OBSERVER", Some(0));
        let atk = r.place_on_field(seat, "ALLY", Some(0));
        let attacker_card = r.top_card(atk);
        let bystander = r.place_on_field(seat, "BYSTANDER", Some(0));
        let bystander_card = r.top_card(bystander);
        attack(&mut r, atk);
        assert_prompt_mask(&r, seat);
        assert!(
            r.game.pending_security.is_some(),
            "battle waits for the parked observer"
        );
        assert_eq!(r.effective_dp(atk), Some(2000));
        let mut cloned = DebugRunner::wrap(r.game.clone());
        let choice = r.pending_selection().unwrap().valid_action_ids[0];
        for resumed in [&mut r, &mut cloned] {
            resumed.execute_action(seat, choice).unwrap();
            assert!(resumed.pending_selection().is_none());
            assert!(resumed.game.pending_security.is_none());
            let board = &resumed.game.players[seat as usize].battle_area;
            let live_atk = resumed.perm_handle(
                seat,
                board
                    .iter()
                    .position(|p| p.top_card().handle() == attacker_card)
                    .expect("resumed observer buffs shifted attacker before losing battle"),
            );
            let live_bystander = resumed.perm_handle(
                seat,
                board
                    .iter()
                    .position(|p| p.top_card().handle() == bystander_card)
                    .unwrap(),
            );
            assert_eq!(resumed.effective_dp(live_atk), Some(7000));
            assert_eq!(resumed.effective_dp(live_bystander), Some(1000));
            assert_eq!(resumed.security_count(1 - seat), 0);
            assert_eq!(resumed.trash_size(1 - seat), 1);
        }
    });
}

#[derive(Clone, Copy)]
enum SecurityMove {
    Hand,
    Deck,
    Security,
}
struct MoveEarlier {
    destination: SecurityMove,
}
impl CardEffect for MoveEarlier {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let destination = self.destination;
        vec![Effect::security(card)
            .process(move |ctx| {
                let player = match destination {
                    SecurityMove::Security => ctx.attacker().unwrap().player,
                    _ => ctx.player,
                };
                let target = PermanentHandle { player, index: 0 };
                match destination {
                    SecurityMove::Hand => {
                        assert!(ctx.return_to_hand(target).is_some());
                    }
                    SecurityMove::Deck => {
                        assert!(ctx
                            .return_to_deck(target, digimon_engine::enums::StackPosition::Bottom));
                    }
                    SecurityMove::Security => {
                        assert!(ctx.place_permanent_on_security(
                            player,
                            target,
                            digimon_engine::enums::StackPosition::Bottom,
                            false
                        ));
                    }
                }
            })
            .build()]
    }
}

fn security_bounce_surviving_observer(destination: SecurityMove) {
    both_seats(|seat| {
        let trace = Trace::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect("SEC-PLAY", Arc::new(MoveEarlier { destination }));
        for (id, label) in [("ALLY", "leaving"), ("OBSERVER", "survivor")] {
            r.register_effect(
                id,
                Arc::new(Observe {
                    timing: EffectTiming::OnOwnSecurityRemoved,
                    label,
                    trace: trace.clone(),
                    self_only: false,
                }),
            );
        }
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        let attacker_card = r.top_card(atk);
        r.place_on_field(1 - seat, "ALLY", Some(0));
        let survivor = r.place_on_field(1 - seat, "OBSERVER", Some(0));
        let survivor_card = r.top_card(survivor);
        attack(&mut r, atk);
        assert!(
            r.pending_selection().is_none(),
            "one surviving observer requires no ordering pick"
        );
        assert_eq!(
            *trace.lock().unwrap(),
            vec!["survivor:attacker_alive=true"],
            "removed source is cancelled, surviving next-slot source activates exactly once"
        );
        assert_eq!(r.top_card(r.perm_handle(1 - seat, 0)), survivor_card);
        assert_eq!(r.top_card(atk), attacker_card);
        assert_eq!(r.memory(), 3, "one turn-player gain and one defender gain");
        assert!(r.game.pending_security.is_none());
    });
}

#[test]
fn security_check_lifecycle_bounce_to_hand_keeps_next_slot_observer() {
    security_bounce_surviving_observer(SecurityMove::Hand);
}
#[test]
fn security_check_lifecycle_return_to_deck_keeps_next_slot_observer() {
    security_bounce_surviving_observer(SecurityMove::Deck);
}

fn security_placement_keeps_attacker(original_index: u8) {
    both_seats(|seat| {
        let mut r = runner(seat, &["LOSING-SEC", "SEC-PLAY"]);
        r.register_effect(
            "SEC-PLAY",
            Arc::new(MoveEarlier {
                destination: SecurityMove::Security,
            }),
        );
        let leaving = r.place_on_field(seat, "OBSERVER", Some(0));
        let leaving_card = r.top_card(leaving);
        let bystander_before = if original_index == 2 {
            Some(r.place_on_field(seat, "BYSTANDER", Some(0)))
        } else {
            None
        };
        let atk = r.place_on_field(seat, "ALLY", Some(0));
        assert_eq!(atk.index, original_index);
        let attacker_card = r.top_card(atk);
        r.place_on_field(seat, "TARGET-BUFF-OBSERVER", Some(0));
        let bystander =
            bystander_before.unwrap_or_else(|| r.place_on_field(seat, "BYSTANDER", Some(0)));
        let bystander_card = r.top_card(bystander);
        let index = r
            .game
            .card_data
            .iter()
            .position(|d| d.card_id == "ALLY")
            .unwrap();
        r.game.card_data[index]
            .keywords
            .push(Keyword::SecurityAttackPlus(1));
        let checkpoint = r.event_checkpoint();
        attack(&mut r, atk);
        let reveals: Vec<_> = r
            .events_since(checkpoint)
            .iter()
            .filter_map(|event| match event {
                GameEvent::SecurityReveal { card_id, .. } => Some(card_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            reveals,
            vec!["SEC-PLAY", "LOSING-SEC"],
            "one removal preserves the live attacker through exactly two checks"
        );
        let live_atk = r.perm_handle(seat, (original_index - 1) as usize);
        assert_eq!(
            r.top_card(live_atk),
            attacker_card,
            "attacker rekeys exactly once"
        );
        assert_eq!(
            r.effective_dp(live_atk),
            Some(12000),
            "each captured event_target buff reaches the attacker"
        );
        let bystander_index = r.game.players[seat as usize]
            .battle_area
            .iter()
            .position(|p| p.top_card().handle() == bystander_card)
            .unwrap();
        assert_eq!(
            r.effective_dp(r.perm_handle(seat, bystander_index)),
            Some(1000)
        );
        assert!(r.game.players[seat as usize]
            .security
            .iter()
            .any(|c| c.handle() == leaving_card));
        assert!(r.pending_selection().is_none());
        assert!(r.game.pending_security.is_none());
        assert_eq!(r.security_count(1 - seat), 0);
    });
}

#[test]
fn security_check_lifecycle_placement_rekeys_attacker_from_slot_one_once() {
    security_placement_keeps_attacker(1);
}
#[test]
fn security_check_lifecycle_placement_rekeys_attacker_from_slot_two_once() {
    security_placement_keeps_attacker(2);
}

type BreedingHandles = Arc<Mutex<Vec<(Option<PermanentHandle>, Option<PermanentHandle>)>>>;

/// Explicit synthetic [Breeding][Opponent's Turn] eligibility: this does not
/// assert that arbitrary inherited effects can activate in the breeding area.
struct BreedingRemovalObserver {
    seen: BreedingHandles,
}
impl CardEffect for BreedingRemovalObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        let breeding_eligible = |ctx: &digimon_engine::effect_context::EffectReadContext| {
            ctx.player != ctx.game.turn_player()
                && ctx.source_permanent
                    == Some(PermanentHandle {
                        player: ctx.player,
                        index: BREEDING_TARGET as u8,
                    })
                && ctx.source_permanent().is_some()
        };
        vec![Effect::inherited(card)
            .timing(EffectTiming::OnOwnSecurityRemoved)
            .name("explicit breeding security-removal observer")
            // References the breeding area (CR 4.3 §3-4-7-4): only such
            // effects are considered by the breeding-area dispatch.
            .breeding_zone()
            .trigger_qualifier(breeding_eligible)
            .condition(breeding_eligible)
            .optional()
            .needs_outer_optional_prompt()
            .process(move |ctx| {
                seen.lock().unwrap().push((
                    ctx.source_permanent,
                    ctx.game
                        .current_trigger_context
                        .as_ref()
                        .unwrap()
                        .target_permanent,
                ));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

struct RemoveFieldBeforeBreedingObserver {
    queued_handles: BreedingHandles,
}
impl CardEffect for RemoveFieldBeforeBreedingObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let queued_handles = self.queued_handles.clone();
        vec![Effect::security(card)
            .process(move |ctx| {
                assert!(ctx
                    .return_to_hand(PermanentHandle {
                        player: ctx.player,
                        index: 0,
                    })
                    .is_some());
                let observer = ctx
                    .game
                    .effect_queue
                    .iter()
                    .find(|qe| qe.card_id == "OBSERVER")
                    .expect("breeding observer captured before immediate Security");
                queued_handles.lock().unwrap().push((
                    observer.source_permanent,
                    observer.trigger_context.as_ref().unwrap().target_permanent,
                ));
            })
            .build()]
    }
}

#[test]
fn security_check_breeding_observer_keeps_source_and_target_after_field_compaction() {
    both_seats(|seat| {
        let defender = 1 - seat;
        let seen = BreedingHandles::default();
        let queued_handles = BreedingHandles::default();
        let mut r = runner(seat, &["SEC-PLAY"]);
        r.register_effect(
            "OBSERVER",
            Arc::new(BreedingRemovalObserver { seen: seen.clone() }),
        );
        r.register_effect(
            "SEC-PLAY",
            Arc::new(RemoveFieldBeforeBreedingObserver {
                queued_handles: queued_handles.clone(),
            }),
        );
        // Stage the breeding stack before battlefield handles, so fixture
        // placement itself does not compact a live battlefield handle.
        r.place_in_breeding(defender, "FACE-UP");
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "OBSERVER")
            .unwrap();
        let card_index = r.game.next_card_index();
        let source = digimon_engine::card_source::CardSource::new(data_idx, defender, card_index);
        let source_card = source.handle();
        r.game.players[defender as usize]
            .breeding_area
            .as_mut()
            .unwrap()
            .push_under(source);
        let breeding_top = r.game.players[defender as usize]
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .handle();
        let breeding = Some(PermanentHandle {
            player: defender,
            index: BREEDING_TARGET as u8,
        });
        let atk = r.place_on_field(seat, "CHECK-SOURCE", Some(0));
        r.place_on_field(defender, "ALLY", Some(0));
        let survivor = r.place_on_field(defender, "BYSTANDER", Some(0));
        let survivor_card = r.top_card(survivor);
        attack(&mut r, atk);
        println!(
            "seat={seat} queued breeding source/target={:?}",
            queued_handles.lock().unwrap()
        );
        assert_eq!(
            *queued_handles.lock().unwrap(),
            vec![(breeding, breeding)],
            "Security compaction must preserve both breeding source and captured target14"
        );
        let sel = r
            .pending_selection()
            .expect("optional breeding observer survives Security compaction");
        assert_eq!(sel.source_card, source_card);
        assert_eq!(sel.source_permanent, breeding);
        assert!(sel.is_optional);
        assert_eq!(sel.valid_action_ids.len(), 1);
        assert_prompt_mask(&r, defender);
        assert!(seen.lock().unwrap().is_empty(), "body waits for acceptance");
        assert_eq!(
            r.memory(),
            4,
            "only the attacker observer ran before defender acceptance"
        );
        let accept = sel.valid_action_ids[0];
        assert!(r.execute_action(seat, accept).is_err());
        let mut declined = DebugRunner::wrap(r.game.clone());
        declined.execute_action(defender, PASS).unwrap();
        assert!(
            seen.lock().unwrap().is_empty(),
            "declining skips breeding body"
        );
        assert_eq!(declined.memory(), 4);
        assert!(declined.pending_selection().is_none());
        r.execute_action(defender, accept).unwrap();
        assert_eq!(
            *seen.lock().unwrap(),
            vec![(breeding, breeding)],
            "accepted body runs exactly once with source/target14"
        );
        assert_eq!(r.memory(), 3);
        assert!(r.pending_selection().is_none());
        assert!(
            r.execute_action(defender, accept).is_err(),
            "resolved choice cannot run twice"
        );
        assert_eq!(*seen.lock().unwrap(), vec![(breeding, breeding)]);
        for done in [&r, &declined] {
            assert_eq!(done.top_card(done.perm_handle(defender, 0)), survivor_card);
            let stack = done.game.players[defender as usize]
                .breeding_area
                .as_ref()
                .unwrap();
            assert_eq!(stack.top_card().handle(), breeding_top);
            assert!(stack
                .card_sources
                .iter()
                .any(|card| card.handle() == source_card));
            assert_eq!(done.trash_size(defender), 1);
            assert!(done.game.pending_security.is_none());
            assert!(done.game.effect_queue.is_empty());
        }
    });
}
