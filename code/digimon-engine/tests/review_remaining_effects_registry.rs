//! Independent production-registration probes; no manual DSL registration.
//! Optional full-data run binds the original 4,483-row R4 source byte for byte.
use digimon_engine::action::{build_action_mask, PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::SelectionKind;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

fn original_data() -> HashMap<String, CardData> {
    let fixture = include_str!("fixtures/focused_remaining_effects_r4.json");
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/focused_remaining_effects_r4.provenance.json"
    ))
    .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture.as_bytes())),
        provenance["fixture_sha256"].as_str().unwrap()
    );
    let text = match std::env::var("FOCUSED_REMAINING_EFFECTS_R4_FULL") {
        Ok(path) => {
            let text = std::fs::read_to_string(path).expect("full original R4");
            assert_eq!(
                format!("{:x}", Sha256::digest(text.as_bytes())),
                provenance["original_runtime_sha256"].as_str().unwrap()
            );
            let full: serde_json::Value = serde_json::from_str(&text).unwrap();
            let subset: serde_json::Value = serde_json::from_str(fixture).unwrap();
            assert_eq!(
                full.as_object().unwrap().len() as u64,
                provenance["original_runtime_record_count"]
                    .as_u64()
                    .unwrap()
            );
            for (id, row) in subset.as_object().unwrap() {
                assert_eq!(&full[id], row, "original row: {id}");
            }
            eprintln!("production-registry probe uses original full 4483-row R4");
            text
        }
        Err(std::env::VarError::NotPresent) => fixture.to_owned(),
        Err(error) => panic!("invalid full R4 path: {error}"),
    };
    CardData::load_from_str(&text).unwrap()
}

fn act(runner: &mut DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0
    );
    // DebugRunner::execute_action is pending-selection-only; the normal decoder
    // handles both ordinary play and the subsequent masked selections.
    runner.game.decode_action(action, player);
}

#[test]
fn ordinary_registry_dispatches_spinomon_live_turn_aura() {
    let data = original_data();
    for player in 0..2 {
        let mut runner = DebugRunner::builder().with_card_data(data.clone()).build();
        assert!(runner.game.effect_registry.get("BT4-018").is_some());
        runner.set_first_player(player);
        runner.set_turn(2);
        runner.set_phase(GamePhase::Main);
        let own = runner.place_on_field(player, "BT4-018", Some(0));
        let other = runner.place_on_field(1 - player, "BT4-018", Some(0));
        assert_eq!(runner.effective_dp(own), Some(13000));
        assert_eq!(runner.effective_dp(other), Some(10000));
    }
}

#[test]
fn ordinary_registry_dispatches_impmon_discard_then_own_trash_return() {
    let data = original_data();
    for player in 0..2 {
        let mut target = make_test_card("TEST-REGISTRY-LORD", "Registry Lord");
        target.traits = vec!["Seven Great Demon Lords".into()];
        let mut runner = DebugRunner::builder()
            .with_card_data(data.clone())
            .add_card(target)
            .hand(player, &["BT4-018", "BT6-068"])
            .deck(0, &["BT4-018"; 8])
            .deck(1, &["BT4-018"; 8])
            .build();
        assert!(runner.game.effect_registry.get("BT6-068").is_some());
        runner.set_first_player(player);
        runner.set_turn(2);
        runner.set_phase(GamePhase::Main);
        runner.game.set_memory(10);
        runner.inject_trash(player, "TEST-REGISTRY-LORD");
        runner.inject_trash(1 - player, "TEST-REGISTRY-LORD");
        act(&mut runner, player, 1);
        assert_eq!(runner.memory(), 7);
        let selection = runner.pending_selection().unwrap();
        assert_eq!(selection.kind, SelectionKind::Hand);
        assert!(selection.is_optional);
        assert_eq!(build_action_mask(&runner.game, player)[PASS as usize], 1.0);
        act(&mut runner, player, 0);
        let selection = runner.pending_selection().unwrap();
        assert_eq!(selection.kind, SelectionKind::Trash);
        assert!(!selection.is_optional);
        assert_eq!(selection.valid_action_ids, vec![TRASH_EFFECT_START]);
        assert_eq!(build_action_mask(&runner.game, player)[PASS as usize], 0.0);
        assert!(build_action_mask(&runner.game, 1 - player)
            .iter()
            .all(|&x| x == 0.0));
        act(&mut runner, player, TRASH_EFFECT_START);
        let hand: Vec<_> = runner.game.players[player as usize]
            .hand
            .iter()
            .map(|card| card.card_id(&runner.game.card_data))
            .collect();
        assert_eq!(hand, vec!["TEST-REGISTRY-LORD"]);
        assert_eq!(runner.game.players[(1 - player) as usize].trash.len(), 1);
        assert!(runner.pending_selection().is_none());
    }
}
