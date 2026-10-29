//! A granted end-of-turn attack must execute every target offered by its mask.
//! EX12-057 prints Paishu as a Digimon token (CR 4.3 section 4-21-1).
use digimon_engine::{
    action::{build_action_mask, encode_attack},
    card_data::CardData,
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardKind, Expiry, GamePhase, ModifierType},
    modifiers::{ModifierEntry, ModifierPayload},
    runners::replay::ReplayRunner,
    serialization::to_ui_json,
};

fn boundary(kind: CardKind, transform: bool) -> DebugRunner {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.dp = Some(8000);
    let mut target = make_test_card("TARGET", "Target");
    target.card_kind = kind;
    target.dp = Some(1000);
    let mut r = DebugRunner::builder()
        .add_card(attacker)
        .add_card(target)
        .deck(0, &["ATTACKER"; 12])
        .deck(1, &["ATTACKER"; 12])
        .security(0, &["ATTACKER"; 3])
        .security(1, &["ATTACKER"; 3])
        .memory(-1)
        .start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let target = r.place_on_field(1, "TARGET", Some(0));
    r.game.players[1].battle_area[0].is_suspended = true;
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, 0),
    );
    if transform {
        r.game.modifiers.add(
            target,
            ModifierEntry::simple(ModifierType::TreatAsDigimon, 0, Expiry::EndOfTurn, 1)
                .with_payload(ModifierPayload::AdditiveSynthIdentity {
                    kind: CardKind::Digimon,
                    level: 3,
                    colors: vec![],
                    traits: vec![],
                    dp: 1000,
                }),
        );
    }
    r.set_phase(GamePhase::EndOfTurnAction);
    r
}

#[test]
fn granted_end_of_turn_attack_executes_effective_digimon_targets() {
    for (kind, transform) in [
        (CardKind::Token, false),
        (CardKind::DigiEgg, false),
        (CardKind::Tamer, true),
        (CardKind::Digimon, false),
    ] {
        let mut r = boundary(kind, transform);
        let action = encode_attack(0, 0);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize],
            1.0,
            "{kind:?}"
        );
        let before = to_ui_json(&r.game);
        let mut cloned = r.game.clone();
        r.game.decode_action(action, 0);
        cloned.decode_action(action, 0);
        assert!(
            to_ui_json(&r.game) != before,
            "legal attack on {kind:?} must execute"
        );
        assert_eq!(to_ui_json(&r.game), to_ui_json(&cloned));
        assert!(r.game.players[0].battle_area[0].is_suspended);
    }
}

#[test]
fn granted_end_of_turn_attack_excludes_untransformed_non_digimon() {
    for kind in [CardKind::Tamer, CardKind::Option] {
        let mut r = boundary(kind, false);
        let action = encode_attack(0, 0);
        assert_eq!(build_action_mask(&r.game, 0)[action as usize], 0.0);
        let before = to_ui_json(&r.game);
        r.game.decode_action(action, 0);
        assert_eq!(to_ui_json(&r.game), before);
    }
}

#[test]
fn recorded_monarchlizamon_attack_on_paishu_token_progresses() {
    // Unchanged diagnostic recording: fixed-continuation window 4, parent
    // anchor game 786, seed 44523624. Its first stalled declaration is row 38.
    let recording: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/training/glowing-dawn-sw-eot-token-attack.json"
    ))
    .unwrap();
    let cards = CardData::load_from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/cards.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut replay = ReplayRunner::new(recording.clone(), &cards, true).unwrap();
    for row in recording["actions"].as_array().unwrap() {
        if row["phase"] == "Mulligan" {
            continue;
        }
        let action = row["action_id"].as_u64().unwrap() as u16;
        let player = row["player_id"].as_u64().unwrap() as u8 - 1;
        assert_eq!(
            build_action_mask(&replay.game, player)[action as usize],
            1.0,
            "recorded row {} must remain legal",
            row["step"]
        );
        if row["step"] == 38 {
            assert_eq!(replay.game.current_phase, GamePhase::EndOfTurnAction);
            assert_eq!(
                replay.game.card_data[replay.game.players[0].battle_area[1].top_card().data_index]
                    .card_id,
                "BT25-057"
            );
            assert_eq!(
                replay.game.card_data[replay.game.players[1].battle_area[1].top_card().data_index]
                    .card_id,
                "TOKEN_PAISHU"
            );
            assert_eq!(action, encode_attack(1, 1));
            let before = to_ui_json(&replay.game);
            let mut cloned = replay.game.clone();
            replay.game.decode_action(action, player);
            cloned.decode_action(action, player);
            assert!(
                to_ui_json(&replay.game) != before,
                "recorded legal attack must not loop"
            );
            assert_eq!(to_ui_json(&replay.game), to_ui_json(&cloned));
            assert!(replay.game.players[0].battle_area[1].is_suspended);
            return;
        }
        let step = replay.step();
        assert!(
            step.divergences.is_empty(),
            "recorded row {}: {:?}",
            row["step"],
            step.divergences
        );
    }
    panic!("missing recorded stalled attack");
}
