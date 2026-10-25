//! BT7-038 JetSilphymon. Official observation
//! 8aea3fc7ee8b95499bdbd7db002d4825b4485eae4ca8490d953c89f782526f85.
//! Production-card regressions, including the shared hand-hosted reducer.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, BREEDING_TARGET},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardColor, CardKind, CardSourceRef, CostDelta, Expiry, ModifierType, PlaySource,
};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn setup(with_tamer: bool, hybrid_base: bool) -> DebugRunner {
    let mut base = make_test_card("TEST-JET-BASE", "Jet base");
    base.level = Some(4);
    base.colors = vec![CardColor::Yellow];
    base.dp = Some(6000);
    base.traits = if hybrid_base {
        vec!["Hybrid".into()]
    } else {
        vec!["Warrior".into()]
    };
    let mut tamer = make_test_card("TEST-JET-TAMER", "Jet Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Yellow];
    let mut runner = DebugRunner::builder()
        .dsl_card("BT7-038")
        .expect("BT7-038 production YAML")
        .add_card(base)
        .add_card(tamer)
        .add_card(make_test_card("TEST-JET-FILL", "Filler"))
        .hand(0, &["BT7-038"])
        .deck(0, &["TEST-JET-FILL"; 20])
        .deck(1, &["TEST-JET-FILL"; 20])
        .memory(10)
        .start();
    if with_tamer {
        runner.place_stack(0, &["TEST-JET-TAMER", "TEST-JET-BASE"]);
    } else {
        runner.place_stack(0, &["TEST-JET-BASE"]);
    }
    runner
}

fn native(runner: &mut DebugRunner, field: u16) {
    let action = encode_digivolve(0, field);
    assert!(build_action_mask(&runner.game, 0)[action as usize] > 0.5);
    runner.game.decode_action(action, 0);
    assert!(
        runner.pending_selection_view().is_none(),
        "mandatory static reduction has no hidden choice"
    );
    runner.auto_resolve().expect("resolve digivolution");
}

#[test]
fn bt7_038_hand_cost_reduction_requires_a_tamer_under_the_actual_base() {
    for with_tamer in [false, true] {
        let mut r = setup(with_tamer, true);
        native(&mut r, 0);
        assert_eq!(r.memory(), if with_tamer { 9 } else { 7 });
        assert_eq!(r.game.players[0].security.len(), 1);
    }
}

#[test]
fn bt7_038_neighbor_sources_and_top_tamers_do_not_reduce_cost() {
    let mut r = setup(false, true);
    r.place_stack(0, &["TEST-JET-TAMER", "TEST-JET-BASE"]);
    r.place_stack(0, &["TEST-JET-TAMER"]);
    native(&mut r, 0);
    assert_eq!(r.memory(), 7);
}

#[test]
fn bt7_038_reduction_obeys_both_players_cost_reduction_prohibitions() {
    for (owner, kind) in [
        (0, ModifierType::CannotReduceDigivolveCost),
        (1, ModifierType::OpponentCannotReduceDigivolveCost),
    ] {
        let mut r = setup(true, true);
        r.game.modifiers.add_player_modifier(
            owner,
            PlayerModifierEntry::simple(kind, 0, Expiry::Permanent, None, 0),
        );
        native(&mut r, 0);
        assert_eq!(r.memory(), 7);
    }
}

#[test]
fn bt7_038_effect_evolution_from_trash_is_not_discounted() {
    let mut r = setup(true, true);
    let card = r.game.players[0].hand.remove(0);
    r.game.players[0].trash.push(card);
    assert!(r.game.effect_initiated_digivolve_from_source(
        0,
        CardSourceRef::Trash(0, 0),
        PermanentHandle {
            player: 0,
            index: 0
        },
        CostDelta::Reduce(0),
        false,
        PlaySource::ByEffect
    ));
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[0].security.len(), 1);
}

#[test]
fn bt7_038_playing_from_hand_is_not_reduced_and_does_not_recover() {
    let mut r = setup(true, true);
    assert!(build_action_mask(&r.game, 0)[0] > 0.5);
    r.game.decode_action(0, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 3);
    assert_eq!(r.game.players[0].security.len(), 0);
}

#[test]
fn bt7_038_recovery_requires_a_hybrid_source_not_its_own_hybrid_top() {
    for hybrid_base in [false, true] {
        let mut r = setup(false, hybrid_base);
        native(&mut r, 0);
        assert_eq!(r.game.players[0].security.len(), usize::from(hybrid_base));
        assert_eq!(
            r.game.players[0].hand.len(),
            1,
            "ordinary digivolution bonus draw remains"
        );
    }
}

#[test]
fn bt7_038_breeding_evolution_uses_hand_reducer_but_never_activates_recovery() {
    let mut r = setup(true, true);
    let base = r.game.players[0].battle_area.remove(0);
    r.game.players[0].breeding_area = Some(base);
    native(&mut r, BREEDING_TARGET);
    assert_eq!(r.memory(), 9);
    assert_eq!(r.game.players[0].security.len(), 0);
}

#[test]
fn bt7_038_printed_circle_rejects_wrong_color_level_and_tamer() {
    for shape in [0, 1, 2] {
        let mut r = setup(false, false);
        let index = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-JET-BASE")
            .unwrap();
        match shape {
            0 => r.game.card_data[index].colors = vec![CardColor::Red],
            1 => r.game.card_data[index].level = Some(3),
            _ => {
                r.game.card_data[index].card_kind = CardKind::Tamer;
                r.game.card_data[index].level = None;
            }
        }
        assert_eq!(
            build_action_mask(&r.game, 0)[encode_digivolve(0, 0) as usize],
            0.0
        );
    }
}

#[test]
fn bt7_038_field_copy_cannot_double_the_incoming_hand_card_reduction() {
    let mut r = setup(true, true);
    r.place_stack(0, &["TEST-JET-TAMER", "BT7-038"]);
    native(&mut r, 0);
    assert_eq!(r.memory(), 9);
}
