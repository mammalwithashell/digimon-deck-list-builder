//! A granted end-of-turn attack must execute every target offered by its mask.
//! EX12-057 prints Paishu as a Digimon token (CR 4.3 section 4-21-1).
use digimon_engine::{
    action::{build_action_mask, encode_attack},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardKind, Expiry, GamePhase, ModifierType},
    modifiers::{ModifierEntry, ModifierPayload},
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

/// Real-card version of a Glowing Dawn training stall (game 786, seed
/// 44523624): MonarchLizamon (BT25-057) holding an end-of-turn attack grant
/// attacks a suspended Paishu token (EX12-057's token; CR 4.3 4-21-1). The
/// game recording this test used to replay is unreachable since a DUAL card
/// checked from security battles with its Digimon DP (CR 4.3 4-6-1; engine fix
/// 716c6e291): on its turn 4 the token now loses a security battle to a
/// checked BT25-057 and is deleted, so the stalled attack never comes. The board below is the recorded one at that attack.
#[test]
fn monarchlizamon_end_of_turn_attack_on_paishu_token_progresses() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-057")
        .expect("BT25-057")
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(-1)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, "FILLER", Some(0));
    let monarch = r.place_on_field(0, "BT25-057", Some(0));
    r.place_on_field(1, "FILLER", Some(0));
    let paishu = r.place_on_field(1, "TOKEN_PAISHU", Some(0));
    assert_eq!((monarch.index, paishu.index), (1, 1), "recorded slots");
    assert_eq!(
        r.game.card_data[r.game.players[1].battle_area[1].top_card().data_index].card_kind,
        CardKind::Token
    );
    r.game.players[1].battle_area[1].is_suspended = true;
    r.game.modifiers.add(
        monarch,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, 0),
    );
    r.set_phase(GamePhase::EndOfTurnAction);

    let action = encode_attack(1, 1);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    let before = to_ui_json(&r.game);
    let mut cloned = r.game.clone();
    r.game.decode_action(action, 0);
    cloned.decode_action(action, 0);
    assert!(to_ui_json(&r.game) != before, "legal attack must not loop");
    assert_eq!(to_ui_json(&r.game), to_ui_json(&cloned));
    assert!(r.game.players[0].battle_area[1].is_suspended);
}
