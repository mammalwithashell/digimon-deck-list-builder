use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    decode_attack, encode_attack, HAND_EFFECT_START, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::PlayerId;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-083";

fn matching_digimon(card_id: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.traits = vec![trait_name.to_string()];
    card
}

fn filler_digimon(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

fn hand_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn hand_contains(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

fn resolve_until_hand_pick(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("BT21-083 should keep a pending selection while paying its stash cost");

        if view.kind == SelectionKind::Hand {
            let hand_slot = hand_index(runner, player, card_id) as u16;
            let action = [PLAY_HAND_START + hand_slot, HAND_EFFECT_START + hand_slot]
                .into_iter()
                .find(|action| view.valid_action_ids.contains(action))
                .unwrap_or(HAND_EFFECT_START + hand_slot);
            assert!(
                view.valid_action_ids.contains(&action),
                "{card_id} should be a legal hand-selection action; view={view:?}"
            );
            runner
                .execute_action(player, action)
                .expect("select hand card for BT21-083");
            return;
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .unwrap_or_else(|| panic!("BT21-083 prompt had no non-PASS action: {view:?}"));
        runner
            .execute_action(player, action)
            .expect("advance BT21-083 pending prompt");
    }

    panic!("BT21-083 never installed a hand selection for {card_id}");
}

fn resolve_until_attack_prompt(runner: &mut DebugRunner, player: PlayerId, attacker_index: u8) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("BT21-083 should keep a pending selection while paying attack cost");

        if view.kind == SelectionKind::Target {
            let attack_actions: Vec<u16> = view
                .valid_action_ids
                .iter()
                .copied()
                .filter(|&action| action != PASS)
                .collect();
            assert!(
                !attack_actions.is_empty(),
                "BT21-083 should expose at least one attack target: {view:?}"
            );
            for action in attack_actions {
                let (actual_attacker, _) = decode_attack(action);
                assert_eq!(
                    actual_attacker as u8, attacker_index,
                    "BT21-083 must grant the attack to the just-played or just-digivolved Digimon"
                );
            }
            return;
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .unwrap_or_else(|| panic!("BT21-083 prompt had no non-PASS action: {view:?}"));
        runner
            .execute_action(player, action)
            .expect("advance BT21-083 pending prompt");
    }

    panic!("BT21-083 never installed an optional attack prompt");
}

#[test]
fn bt21_083_start_main_places_matching_hand_card_under_self_draws_and_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-083 YAML loads")
        .add_card(matching_digimon("XH-HAND", "Xros Heart"))
        .add_card(filler_digimon("NO-TRAIT"))
        .add_card(filler_digimon("DRAW-CARD"))
        .hand(0, &["XH-HAND", "NO-TRAIT"])
        .deck(0, &["DRAW-CARD"])
        .memory(2)
        .start();

    let taiki = runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);
    let stack_before = runner.game.players[0].battle_area[taiki.index as usize]
        .card_sources
        .len();

    runner.game.enter_main_phase();
    resolve_until_hand_pick(&mut runner, 0, "XH-HAND");
    runner
        .auto_resolve()
        .expect("finish BT21-083 start-main effect");

    let taiki_stack = &runner.game.players[0].battle_area[taiki.index as usize].card_sources;
    assert_eq!(
        taiki_stack.len(),
        stack_before + 1,
        "BT21-083 should place exactly one matching hand card under itself"
    );
    assert!(
        taiki_stack
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "XH-HAND"),
        "the selected Xros Heart Digimon should be under BT21-083"
    );
    assert!(
        !hand_contains(&runner, 0, "XH-HAND"),
        "the selected card must leave hand"
    );
    assert!(
        hand_contains(&runner, 0, "NO-TRAIT"),
        "non-matching hand cards must remain untouched"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "BT21-083 should draw 1 after placing the card"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "BT21-083 should gain 1 memory after placing the card"
    );
}

#[test]
fn bt21_083_played_matching_digimon_suspends_tamer_and_installs_optional_attack_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-083 YAML loads")
        .add_card(matching_digimon("HERO-PLAY", "Hero"))
        .add_card(filler_digimon("SECURITY-CARD"))
        .hand(0, &["HERO-PLAY"])
        .security(1, &["SECURITY-CARD"])
        .memory(10)
        .start();

    let taiki = runner.place_on_field(0, CARD_ID, Some(0));
    let played_index = runner.play(0, 0).expect("play matching Hero Digimon") as u8;
    let opponent_security_before = runner.game.players[1].security.len();

    resolve_until_attack_prompt(&mut runner, 0, played_index);

    assert!(
        runner.game.players[0].battle_area[taiki.index as usize].is_suspended,
        "BT21-083 must suspend itself as the printed cost before granting the attack"
    );

    let attack_action = encode_attack(played_index as u16, SECURITY_TARGET);
    let pending = runner
        .pending_selection_view()
        .expect("BT21-083 should leave the optional attack prompt pending");
    assert!(
        pending.is_optional,
        "BT21-083 says the Digimon may attack, so the attack prompt must be optional"
    );
    assert!(
        pending.valid_action_ids.contains(&attack_action),
        "the just-played matching Digimon should be able to attack the player"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "declining the optional BT21-083 attack must be legal"
    );
    assert_eq!(
        mask[attack_action as usize], 1.0,
        "the attack target must be exposed through the action mask"
    );

    runner
        .execute_action(0, PASS)
        .expect("decline BT21-083 optional attack");
    assert!(runner.game.pending_attack.is_none());
    assert_eq!(
        runner.game.players[1].security.len(),
        opponent_security_before,
        "declining the optional attack should not check security"
    );
}
