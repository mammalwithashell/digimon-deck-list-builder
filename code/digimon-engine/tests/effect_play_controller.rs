//! Effect-play restrictions follow the causing effect controller (Q4663/Q4664/Q6238).
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, ModifierType};

fn audit_runner() -> DebugRunner {
    use digimon_engine::enums::{CardColor, GamePhase};
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../cards/bt8/BT8-097.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/ex5/EX5-060.yaml"))
        .unwrap();
    for (id, kind, dp, color) in [
        ("A-RED", CardKind::Digimon, 7000, CardColor::Red),
        ("A-BLUE", CardKind::Digimon, 7000, CardColor::Blue),
        ("A-LOW", CardKind::Digimon, 6000, CardColor::Red),
        ("A-TAMER", CardKind::Tamer, 1000, CardColor::Blue),
        ("A-OPTION", CardKind::Option, 1000, CardColor::Blue),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.dp = Some(dp);
        c.level = Some(3);
        c.colors = vec![color];
        c.play_cost = 1;
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, &["BT8-097", "EX5-060", "A-LOW"])
        .hand(1, &["A-LOW", "A-TAMER"])
        .deck(0, &["A-RED"; 12])
        .deck(1, &["A-RED"; 12])
        .security(0, &["BT8-097"])
        .security(1, &["A-RED"; 4])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.inject_trash(1, "A-LOW");
    r.game.current_phase = GamePhase::Main;
    r
}

#[test]
fn bt8_097_q4663_own_effect_can_play_opponents_digimon() {
    let mut r = audit_runner();
    r.place_on_field(0, "A-RED", None);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
    r.game.decode_action(0, 0);
    assert!(r
        .game
        .modifiers
        .player_has(1, ModifierType::CannotPlayDigimonByEffect));
    r.game.play_from_hand(0, 0).unwrap(); // My Dragomon, opponent's trash.
    let v = r
        .pending_selection_view()
        .expect("opponent chooses trash Digimon");
    assert_eq!(v.selecting_player, 1);
    let a = v.valid_action_ids[0];
    assert!(digimon_engine::action::build_action_mask(&r.game, 1)[a as usize] > 0.0);
    let mut cloned = r.game.clone();
    cloned.decode_action(a, 1);
    assert_eq!(
        cloned.players[1].battle_area.len(),
        1,
        "cloned pending choice retains effect controller"
    );
    r.execute_action(1, a).unwrap();
    assert_eq!(
        r.battle_area_size(1),
        1,
        "Q4663: Crimson Blaze blocks the opponent's effects, not my Dragomon playing their Digimon"
    );
}

#[test]
fn bt8_097_q6238_prevents_effect_play_into_breeding() {
    let mut r = audit_runner();
    r.place_on_field(0, "A-RED", None);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[0] > 0.0);
    r.game.decode_action(0, 0);
    assert!(
        !r.game.play_to_breeding_from_hand(1, 0),
        "Q6238: opponent cannot play a Digimon into breeding by effect"
    );
    assert!(r.game.players[1].breeding_area.is_none());
    assert_eq!(r.game.players[1].hand.len(), 2);
}

#[test]
fn q4664_opponents_effect_cannot_play_my_digimon() {
    let mut r = audit_runner();
    r.place_on_field(0, "A-RED", None);
    r.game.decode_action(0, 0);
    let dragomon = r.game.players[0].hand.remove(0);
    r.game.players[1].hand.push(dragomon);
    r.inject_trash(0, "A-LOW");
    r.game.memory = 0;
    r.game.play_from_hand(1, 2).unwrap();
    assert!(
        r.pending_selection().is_none(),
        "Q4664: a prohibited effect-play candidate must not be selectable"
    );
    let cloned = r.game.clone();
    for game in [&r.game, &cloned] {
        assert_eq!(
            game.players[0].battle_area.len(),
            1,
            "blocked opponent effect must not play my Digimon"
        );
        assert!(game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&game.card_data) == "A-LOW"));
    }
}

#[test]
fn explicit_effect_context_controls_hand_trash_and_breeding_gates() {
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::{CostDelta, PlaySource};
    for destination in [0, 1] {
        for controller in [0, 1] {
            for zone in ["hand", "trash", "breeding", "token"] {
                let mut r = audit_runner();
                r.place_on_field(0, "A-RED", None);
                r.game.decode_action(0, 0);
                r.inject_trash(0, "A-LOW");
                let index = r.game.players[destination as usize]
                    .hand
                    .iter()
                    .position(|c| c.card_id(&r.game.card_data) == "A-LOW")
                    .unwrap();
                let source = r.game.players[0].battle_area[0].top_card().handle();
                // A direct context may be invoked while another cause is scoped.
                r.game
                    .set_effect_source_player_for_test(Some(1 - controller));
                let played = {
                    let mut ctx = EffectContext::new(&mut r.game, source, None, controller);
                    match zone {
                        "hand" => ctx.play_from_hand_free(destination, index).is_some(),
                        "trash" => {
                            let index = ctx.game.players[destination as usize]
                                .trash
                                .iter()
                                .position(|c| c.card_id(&ctx.game.card_data) == "A-LOW")
                                .unwrap();
                            ctx.play_from_trash_with_cost(destination, index, CostDelta::Free)
                                .is_some()
                        }
                        "token" => ctx.play_token(destination, "familiar").is_some(),
                        _ => ctx.play_to_breeding_from_hand(destination, index),
                    }
                };
                assert_eq!(
                    played,
                    controller == 0,
                    "controller {controller}, destination {destination}, zone {zone}"
                );
                // A later Game call must see the restored surrounding cause.
                let index = r.game.players[1]
                    .hand
                    .iter()
                    .position(|c| c.card_id(&r.game.card_data) == "A-TAMER")
                    .unwrap();
                assert!(
                    r.game
                        .play_from_hand_with_cost(1, index, CostDelta::Free, PlaySource::ByEffect)
                        .is_some(),
                    "Digimon restriction does not block Tamers"
                );
                r.inject_trash(0, "A-LOW");
                let index = r.game.players[0].trash.len() - 1;
                let result = r.game.play_from_trash_with_cost(
                    0,
                    index,
                    CostDelta::Free,
                    PlaySource::ByEffect,
                );
                assert_eq!(
                    result.is_some(),
                    controller == 1,
                    "effect controller scope is restored even after failed play"
                );
            }
        }
    }
}
