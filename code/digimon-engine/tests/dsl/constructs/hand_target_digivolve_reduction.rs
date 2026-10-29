//! A target card's in-hand evolution reducer is distinct from a play reducer.
//! The qualifying sources belong to the actual evolving base, not a neighbor.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, BREEDING_TARGET},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, CardSourceRef, CostDelta, Expiry, GamePhase, ModifierType, PlaySource,
};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;

const CARD: &str = r#"
card: TEST-HAND-EVOLUTION-REDUCER
name: Hand evolution reducer
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 7000
alt_paths:
  - kind: digivolve
    from: {kind: digimon, level_eq: 4, color_is: yellow}
    cost: 3
effects:
  - kind: cost_reduction
    when_digivolving_this: true
    condition:
      all_of:
        - cost_target_from_hand: true
        - cost_target_permanent:
            kind: digimon
            source_count: {filter: {kind: tamer}, at_least: 1}
    amount: 2
"#;

const EFFECT_HOST: &str = r#"
card: TEST-HAND-BASE
name: Evolution base
kind: digimon
level: 4
color: [yellow]
cost: 0
dp: 6000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter:
            all_of:
              - card_number_is: TEST-HAND-EVOLUTION-REDUCER
              - can_digivolve_onto_with_cost: {target: source, cost: printed}
          prompt: Choose a payable hand evolution
      - effect_initiated_digivolve: {target: source, from_hand: result, cost: printed}
"#;

fn setup(
    player: u8,
    tamer_source: bool,
    yaml: &str,
    in_trash: bool,
) -> (DebugRunner, PermanentHandle) {
    let mut base = make_test_card("TEST-HAND-BASE", "Evolution base");
    base.level = Some(4);
    base.colors = vec![CardColor::Yellow];
    base.dp = Some(6000);
    let mut tamer = make_test_card("TEST-HAND-TAMER", "Source Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Yellow];
    let mut other = make_test_card("TEST-OTHER-EVOLUTION", "Different incoming card");
    other.level = Some(5);
    other.colors = vec![CardColor::Yellow];
    other.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 4,
        memory_cost: 3,
    }];
    let builder = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("explicit target-hosted evolution reducer syntax")
        .from_dsl_yaml(EFFECT_HOST)
        .unwrap()
        .add_card(other)
        .add_card(base)
        .add_card(tamer)
        .add_card(make_test_card("TEST-HAND-FILL", "Filler"))
        .deck(0, &["TEST-HAND-FILL"; 20])
        .deck(1, &["TEST-HAND-FILL"; 20]);
    let builder = builder.hand(player, &["TEST-HAND-EVOLUTION-REDUCER"]);
    let mut runner = builder.start();
    if in_trash {
        let card = runner.game.players[player as usize].hand.remove(0);
        runner.game.players[player as usize].trash.push(card);
    }
    runner.skip_mulligan();
    runner.set_first_player(player);
    runner.set_phase(GamePhase::Main);
    runner.game.set_memory(10);
    let host = if tamer_source {
        runner.place_stack(player, &["TEST-HAND-TAMER", "TEST-HAND-BASE"])
    } else {
        runner.place_stack(player, &["TEST-HAND-BASE"])
    };
    (runner, host)
}

fn native(runner: &mut DebugRunner, host: PermanentHandle) {
    let action = encode_digivolve(0, host.index as u16);
    assert!(build_action_mask(&runner.game, host.player)[action as usize] > 0.5);
    runner.game.decode_action(action, host.player);
    runner.auto_resolve().unwrap();
    let permanent = if host.index == BREEDING_TARGET as u8 {
        runner.game.players[host.player as usize]
            .breeding_area
            .as_ref()
            .unwrap()
    } else {
        &runner.game.players[host.player as usize].battle_area[host.index as usize]
    };
    assert_eq!(
        permanent.top_card().card_id(&runner.game.card_data),
        "TEST-HAND-EVOLUTION-REDUCER"
    );
}

#[test]
fn hand_target_digivolve_reduction_reads_actual_base_for_both_seats() {
    for player in [0, 1] {
        for has_source in [false, true] {
            let (mut r, h) = setup(player, has_source, CARD, false);
            native(&mut r, h);
            assert_eq!(r.memory(), if has_source { 9 } else { 7 });
        }
    }
}

#[test]
fn hand_target_digivolve_reduction_ignores_other_permanents_sources() {
    let (mut r, h) = setup(0, false, CARD, false);
    r.place_stack(0, &["TEST-HAND-TAMER", "TEST-HAND-BASE"]);
    r.place_on_field(0, "TEST-HAND-TAMER", None);
    native(&mut r, h);
    assert_eq!(r.memory(), 7);
}

#[test]
fn hand_target_digivolve_reduction_can_read_breeding_base_without_field_observer() {
    let (mut r, mut h) = setup(0, true, CARD, false);
    let base = r.game.players[0].battle_area.remove(h.index as usize);
    r.game.players[0].breeding_area = Some(base);
    h.index = BREEDING_TARGET as u8;
    native(&mut r, h);
    assert_eq!(r.memory(), 9);
}

#[test]
fn hand_target_digivolve_reduction_obeys_reduction_prohibitions() {
    for player in [0, 1] {
        for (owner, kind) in [
            (player, ModifierType::CannotReduceDigivolveCost),
            (1 - player, ModifierType::OpponentCannotReduceDigivolveCost),
        ] {
            let (mut r, h) = setup(player, true, CARD, false);
            r.game.modifiers.add_player_modifier(
                owner,
                PlayerModifierEntry::simple(kind, 0, Expiry::Permanent, None, 0),
            );
            native(&mut r, h);
            assert_eq!(r.memory(), 7);
        }
    }
}

#[test]
fn hand_target_digivolve_reduction_preserves_hand_origin_for_effect_routes() {
    for from_trash in [false, true] {
        let (mut r, h) = setup(0, true, CARD, from_trash);
        let source = if from_trash {
            CardSourceRef::Trash(0, 0)
        } else {
            CardSourceRef::Hand(0, 0)
        };
        assert!(r.game.effect_initiated_digivolve_from_source(
            0,
            source,
            h,
            CostDelta::Reduce(0),
            false,
            PlaySource::ByEffect
        ));
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), if from_trash { 7 } else { 9 });
    }
}

#[test]
fn hand_target_digivolve_reduction_does_not_discount_playing_the_card() {
    let (mut r, _) = setup(0, true, CARD, false);
    r.game.decode_action(0, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 3, "play cost 7 cannot use an evolution reducer");
}

#[test]
fn hand_target_digivolve_reduction_never_borrows_play_only_reducer() {
    let yaml = CARD.replace("when_digivolving_this: true", "when_playing_this: true")
        .replace("    condition:\n      all_of:\n        - cost_target_from_hand: true\n        - cost_target_permanent:\n            kind: digimon\n            source_count: {filter: {kind: tamer}, at_least: 1}\n", "");
    let (mut r, h) = setup(0, true, &yaml, false);
    native(&mut r, h);
    assert_eq!(r.memory(), 7);
}

#[test]
fn hand_target_digivolve_reduction_field_copy_cannot_double_the_hand_reducer() {
    let (mut r, h) = setup(0, true, CARD, false);
    r.place_stack(0, &["TEST-HAND-TAMER", "TEST-HAND-EVOLUTION-REDUCER"]);
    native(&mut r, h);
    assert_eq!(
        r.memory(),
        9,
        "only the physical incoming hand card reduces"
    );
}

#[test]
fn hand_target_digivolve_reduction_field_card_cannot_reduce_a_different_incoming_card() {
    let (mut r, h) = setup(0, true, CARD, false);
    r.place_stack(0, &["TEST-HAND-TAMER", "TEST-HAND-EVOLUTION-REDUCER"]);
    r.game
        .stage_inject_card(0, "TEST-OTHER-EVOLUTION", "hand")
        .unwrap();
    let action = encode_digivolve(1, h.index as u16);
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.game.decode_action(action, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 7);
}

#[test]
fn hand_target_digivolve_reduction_base_predicate_is_false_outside_cost_context() {
    use digimon_dsl::compiled::CompiledPredicate;
    use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
    use digimon_engine::effect_context::EffectReadContext;
    let (r, h) = setup(0, true, CARD, false);
    let pred = CompiledPredicate {
        cost_target_permanent: Some(Box::new(CompiledPredicate::default())),
        ..Default::default()
    };
    let ctx = EffectReadContext::new(&r.game, r.top_card(h), None, 0);
    assert!(!eval_predicate(&pred, &ctx, PredicateSubject::None));
}

#[test]
fn hand_target_digivolve_reduction_flag_itself_does_not_host_a_trash_card() {
    let yaml = CARD.replace("        - cost_target_from_hand: true\n", "");
    let (mut r, h) = setup(0, true, &yaml, true);
    assert!(r.game.effect_initiated_digivolve_from_source(
        0,
        CardSourceRef::Trash(0, 0),
        h,
        CostDelta::Reduce(0),
        false,
        PlaySource::ByEffect
    ));
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 7);
}

#[test]
fn hand_target_digivolve_reduction_rejects_conflicting_cost_host_flags() {
    let yaml = CARD.replace(
        "    when_digivolving_this: true",
        "    when_digivolving_this: true\n    when_playing_this: true",
    );
    let result = DebugRunner::builder().from_dsl_yaml(&yaml);
    assert!(result.err().unwrap().contains("mutually exclusive"));
}

#[test]
fn hand_target_digivolve_reduction_native_mask_queries_preserve_hand_state() {
    for qualifies in [false, true] {
        let (mut r, h) = setup(0, qualifies, CARD, false);
        r.game.set_memory(5);
        let action = encode_digivolve(0, h.index as u16);
        for _ in 0..2 {
            assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
            assert_eq!(r.memory(), 5);
            assert_eq!(r.game.players[0].hand.len(), 1);
        }
    }
}

#[test]
fn hand_target_digivolve_reduction_effect_choice_uses_reduced_affordability() {
    use digimon_engine::{enums::EffectTiming, TriggerSource};
    for qualifies in [false, true] {
        let (mut r, h) = setup(0, qualifies, CARD, false);
        r.game.set_memory(-9);
        r.game
            .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
        r.game.drain_effect_queue();
        if qualifies {
            assert!(
                r.pending_selection_view().is_some(),
                "reduced cost 1 is payable during the effect"
            );
            assert!(build_action_mask(&r.game, 0)[0] > 0.5);
            assert_eq!(r.memory(), -9, "eligibility query cannot pay memory");
            r.execute_action(0, 0).unwrap();
            r.auto_resolve().unwrap();
            assert_eq!(
                r.game.players[0].battle_area[h.index as usize]
                    .top_card()
                    .card_id(&r.game.card_data),
                "TEST-HAND-EVOLUTION-REDUCER"
            );
            let own_memory = if r.turn_player() == 0 {
                r.memory()
            } else {
                -r.memory()
            };
            assert_eq!(own_memory, -10);
        } else {
            assert!(
                r.pending_selection_view().is_none(),
                "printed cost 3 is not payable at -9"
            );
            assert_eq!(r.game.players[0].hand.len(), 1);
            assert_eq!(r.memory(), -9);
        }
    }
}

#[test]
fn hand_target_digivolve_reduction_clamps_payment_at_zero() {
    let yaml = CARD.replace("    amount: 2", "    amount: 5");
    let (mut r, h) = setup(0, true, &yaml, false);
    native(&mut r, h);
    assert_eq!(r.memory(), 10);
}
