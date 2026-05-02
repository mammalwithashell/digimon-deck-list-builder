use std::path::PathBuf;

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::{parse_printed_keywords, CardData};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase, Keyword};

fn cards_json_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data/cards.json")
}

fn real_cards() -> std::collections::HashMap<String, CardData> {
    CardData::load_from_file(&cards_json_path()).expect("real cards.json must load")
}

fn runner_with_real_cards() -> DebugRunner {
    DebugRunner::builder().with_card_data(real_cards()).start()
}

fn printed_keywords(card_id: &str) -> Vec<Keyword> {
    let cards = real_cards();
    let card = cards.get(card_id).expect("fixture card must exist");
    parse_printed_keywords(&card.effect_text, &card.inherited_text, &card.security_text)
}

#[test]
fn collision_removes_block_pass_when_blocker_exists() {
    let mut r = runner_with_real_cards();
    let attacker = r.place_on_field(0, "EX10-034", Some(0));
    let _blocker = r.place_on_field(1, "BT5-008", Some(0));

    assert!(r.game.has_keyword(attacker, Keyword::Collision));

    r.game
        .decode_action(encode_attack(0, SECURITY_TARGET), attacker.player);
    let mask = build_action_mask(&r.game, 1);

    assert_eq!(
        mask[PASS as usize], 0.0,
        "Collision makes block mandatory when a legal blocker exists"
    );
}

#[test]
fn piercing_after_digimon_battle_checks_security_once() {
    assert!(
        printed_keywords("BT1-022").contains(&Keyword::Piercing),
        "fixture must carry printed Piercing"
    );

    let mut r = runner_with_real_cards();
    let attacker = r.place_on_field(0, "BT1-022", Some(0));
    let _defender = r.place_on_field(1, "BT5-008", Some(0));
    r.game.players[1]
        .security
        .push(digimon_engine::card_source::CardSource::new(
            r.game
                .card_data
                .iter()
                .position(|c| c.card_id == "BT1-010")
                .expect("BT1-010 fixture exists"),
            1,
            9000,
        ));
    r.game.players[1]
        .security
        .push(digimon_engine::card_source::CardSource::new(
            r.game
                .card_data
                .iter()
                .position(|c| c.card_id == "BT1-011")
                .expect("BT1-011 fixture exists"),
            1,
            9001,
        ));

    assert!(r.game.has_keyword(attacker, Keyword::Piercing));

    r.game.decode_action(encode_attack(0, 0), attacker.player);

    assert_eq!(r.game.player(1).security.len(), 1);
}

#[test]
fn reboot_unsuspends_during_opponents_unsuspend_phase_once() {
    assert!(
        printed_keywords("BT2-063").contains(&Keyword::Reboot),
        "fixture must carry printed Reboot"
    );

    let mut r = runner_with_real_cards();
    let handle = r.place_on_field(0, "BT2-063", Some(0));
    let _filler = r.place_on_field(1, "BT5-008", Some(0));
    assert!(r.game.has_keyword(handle, Keyword::Reboot));

    r.game.suspend(handle);
    r.game.end_turn();

    assert!(!r.game.player(0).battle_area[0].is_suspended);
}

#[test]
fn retaliation_deletes_battle_opponent_when_deleted_in_battle() {
    let mut r = runner_with_real_cards();
    let attacker = r.place_on_field(0, "BT2-074", Some(0));
    let _defender = r.place_on_field(1, "BT1-084", Some(0));

    assert!(r.game.has_keyword(attacker, Keyword::Retaliation));

    r.game.decode_action(encode_attack(0, 0), attacker.player);

    assert!(r.game.player(0).battle_area.is_empty());
    assert!(
        r.game.player(1).battle_area.is_empty(),
        "Retaliation deletes the battled opponent Digimon"
    );
}

#[test]
fn collision_decode_rejects_block_decline_when_mandatory() {
    let mut r = runner_with_real_cards();
    let attacker = r.place_on_field(0, "EX10-034", Some(0));
    let _blocker = r.place_on_field(1, "BT5-008", Some(0));

    r.game
        .decode_action(encode_attack(0, SECURITY_TARGET), attacker.player);
    r.game.decode_action(PASS, 1);

    assert!(
        r.game.pending_attack.is_some(),
        "declining a mandatory block should not advance combat"
    );
}

#[test]
fn group6_fixture_ids_still_carry_expected_printed_keywords() {
    assert!(printed_keywords("EX10-034").contains(&Keyword::Collision));
    assert!(printed_keywords("BT1-022").contains(&Keyword::Piercing));
    assert!(printed_keywords("BT2-063").contains(&Keyword::Reboot));
    assert!(printed_keywords("BT2-074").contains(&Keyword::Retaliation));
}

#[test]
fn buried_face_security_attack_keyword_does_not_count_as_inherited() {
    let mut source = make_test_card("TST-FACE-SA", "Face Security Attack");
    source.effect_text =
        "＜Security A. +1＞ (This Digimon checks 1 additional security card.)".to_string();
    source.keywords = parse_printed_keywords(&source.effect_text, "", "");

    let attacker = make_test_card("TST-TOP-NO-SA", "Top No Security Attack");
    let mut r = DebugRunner::builder()
        .add_card(source)
        .add_card(attacker)
        .build();
    let handle = r.place_stack(0, &["TST-FACE-SA", "TST-TOP-NO-SA"]);

    assert_eq!(
        r.game.security_attack_keyword_bonus(handle),
        0,
        "face text on buried sources must not become inherited Security Attack"
    );
}

#[test]
fn buried_inherited_security_attack_keyword_counts() {
    let mut source = make_test_card("TST-INHERITED-SA", "Inherited Security Attack");
    source.inherited_text =
        "＜Security A. +1＞ (This Digimon checks 1 additional security card.)".to_string();
    source.keywords = parse_printed_keywords("", &source.inherited_text, "");

    let attacker = make_test_card("TST-TOP-INHERITS-SA", "Top Inherits Security Attack");
    let mut r = DebugRunner::builder()
        .add_card(source)
        .add_card(attacker)
        .build();
    let handle = r.place_stack(0, &["TST-INHERITED-SA", "TST-TOP-INHERITS-SA"]);

    assert_eq!(r.game.security_attack_keyword_bonus(handle), 1);
}

#[test]
fn decode_action_does_not_auto_fire_material_save() {
    let mut carrier = make_test_card("TST-MATERIAL-SAVE", "Material Save Carrier");
    carrier.keywords = vec![Keyword::MaterialSave(1)];

    let source = make_test_card("TST-MATERIAL-SOURCE", "Material Source");
    let mut tamer = make_test_card("TST-TAMER", "Helpful Tamer");
    tamer.card_kind = CardKind::Tamer;

    let mut r = DebugRunner::builder()
        .add_card(source)
        .add_card(carrier)
        .add_card(tamer)
        .build();
    r.place_stack(0, &["TST-MATERIAL-SOURCE", "TST-MATERIAL-SAVE"]);
    r.place_on_field(0, "TST-TAMER", Some(0));
    r.game.current_phase = GamePhase::Main;

    r.game.decode_action(PASS, 0);

    assert!(
        r.game.pending_selection.is_none(),
        "declarative refresh must not execute active keyword processes"
    );
}
