//! BT27-024: official English card image and category 522040 (2026-11-22).
//! Mandatory cross-Digimon source choices = 2 * floor(opponent hand / 4),
//! followed independently by a sourceless opposing Digimon suspend lock.
use digimon_engine::action::{mask::build_action_mask, space::PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlaySource};

const ID: &str = "BT27-024";
fn card(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.colors = vec![color];
    c.dp = Some(if id == "F" { 1000 } else { 12000 });
    c
}
fn base() -> DebugRunnerBuilder {
    let mut squad = card("SQUAD", 3, CardColor::Red);
    squad.traits = vec!["DATA SQUAD".into()];
    let mut tamer = card("TAMER", 3, CardColor::Blue);
    tamer.card_kind = CardKind::Tamer;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT27-024 production DSL")
        .add_card(card("F", 3, CardColor::Blue))
        .add_card(card("RED", 3, CardColor::Red))
        .add_card(card("TOP", 5, CardColor::Blue))
        .add_card(squad)
        .add_card(tamer)
        .deck(0, &["F"; 20])
        .deck(1, &["F"; 20])
        .memory(10)
}
fn choose(r: &mut DebugRunner, which: usize) {
    let v = r
        .pending_selection_view()
        .expect("mandatory player-visible choice");
    assert!(!r.pending_is_optional());
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    r.execute_action(0, v.valid_action_ids[which]).unwrap();
}
#[test]
fn bt27_024_hand_bucket_boundaries_and_cross_stack_choice() {
    for hand in [0, 3, 4, 7, 8, 9, 12] {
        let mut r = base().hand(0, &[ID]).hand(1, &vec!["F"; hand]).start();
        let a = r.place_stack(1, &["F", "F", "F", "TOP"]);
        let b = r.place_stack(1, &["F", "F", "F", "TOP"]);
        r.play(0, 0);
        let expected = (hand / 4 * 2).min(6);
        for i in 0..expected {
            choose(&mut r, if i == 0 { 5 } else { 0 });
        }
        assert_eq!(r.game.players[1].trash.len(), expected, "hand={hand}");
        assert_eq!(
            r.game.players[1].battle_area[a.index as usize]
                .card_sources
                .len()
                + r.game.players[1].battle_area[b.index as usize]
                    .card_sources
                    .len(),
            8 - expected
        );
        if expected == 2 {
            assert_eq!(
                r.game.players[1].battle_area[a.index as usize]
                    .card_sources
                    .len(),
                3
            );
            assert_eq!(
                r.game.players[1].battle_area[b.index as usize]
                    .card_sources
                    .len(),
                3
            );
        }
        if r.pending_selection().is_some() {
            choose(&mut r, 0);
        }
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt27_024_clamps_short_supply_and_then_locks_newly_empty_target() {
    let mut r = base().hand(0, &[ID]).hand(1, &["F"; 8]).start();
    let a = r.place_stack(1, &["F", "TOP"]);
    r.play(0, 0);
    choose(&mut r, 0);
    assert_eq!(r.game.players[1].trash.len(), 1);
    choose(&mut r, 0);
    assert!(r.game.modifiers.has(a, ModifierType::CannotSuspend));
}
#[test]
fn bt27_024_filters_source_hosts_and_owners() {
    let mut r = base().hand(0, &[ID]).hand(1, &["F"; 4]).start();
    r.place_stack(0, &["F", "TOP"]);
    r.place_stack(1, &["F", "TAMER"]);
    r.place_stack(1, &["F", "F", "TOP"]);
    r.play(0, 0);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    choose(&mut r, 1);
    choose(&mut r, 0);
    choose(&mut r, 0);
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 2);
}
#[test]
fn bt27_024_zero_bucket_still_locks_and_filters_targets() {
    let mut r = base().hand(0, &[ID]).hand(1, &["F"; 3]).start();
    r.place_stack(0, &["TOP"]);
    r.place_stack(1, &["F", "TOP"]);
    r.place_stack(1, &["TAMER"]);
    let a = r.place_stack(1, &["TOP"]);
    let b = r.place_stack(1, &["TOP"]);
    r.play(0, 0);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    choose(&mut r, 1);
    assert!(!r.game.modifiers.has(a, ModifierType::CannotSuspend));
    assert!(r.game.modifiers.has(b, ModifierType::CannotSuspend));
    assert_eq!(r.game.players[1].trash.len(), 0);
}
#[test]
fn bt27_024_no_opponent_digimon_no_choice() {
    let mut r = base().hand(0, &[ID]).hand(1, &["F"; 8]).start();
    r.place_stack(1, &["F", "TAMER"]);
    r.play(0, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].trash.len(), 0);
}
#[test]
fn bt27_024_actual_standard_and_trait_evolution_fire_effect() {
    for source in ["F", "SQUAD"] {
        let mut r = base().hand(0, &[ID]).hand(1, &["F"; 4]).start();
        r.place_stack(0, &[source]);
        r.place_stack(1, &["F", "F", "TOP"]);
        assert!(r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
        assert_eq!(r.memory(), 8);
        choose(&mut r, 0);
        choose(&mut r, 0);
        choose(&mut r, 0);
        assert_eq!(r.game.players[1].trash.len(), 2);
    }
}
#[test]
fn bt27_024_wrong_color_without_trait_cannot_evolve() {
    let mut r = base().hand(0, &[ID]).start();
    r.place_stack(0, &["RED"]);
    assert!(!r.game.digivolve_from_hand(0, 0, 0, PlaySource::ByHand));
    assert_eq!(r.memory(), 10);
}
#[test]
fn bt27_024_jamming_survives_security_but_not_digimon_battle() {
    let mut r = base().security(1, &["TOP"]).start();
    r.set_turn(2);
    let a = r.place_field_stack(0, &[ID], false, 0);
    r.attack_player(a, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.security_count(1), 0);
    let mut r = base().start();
    r.set_turn(2);
    let a = r.place_field_stack(0, &[ID], false, 0);
    let b = r.place_field_stack(1, &["TOP"], true, 0);
    r.attack_digimon(a, b, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 0);
}
#[test]
fn bt27_024_inherited_draw_is_both_players_once_per_turn() {
    let mut r = base().security(1, &["F"; 4]).start();
    r.set_turn(2);
    let a = r.place_field_stack(0, &[ID, "TOP"], false, 0);
    r.attack_player(a, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        (r.game.players[0].hand.len(), r.game.players[1].hand.len()),
        (1, 1)
    );
    r.game.players[0].battle_area[0].is_suspended = false;
    r.attack_player(a, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        (r.game.players[0].hand.len(), r.game.players[1].hand.len()),
        (1, 1)
    );
}
#[test]
fn bt27_024_face_up_attack_does_not_draw() {
    let mut r = base().security(1, &["TOP"]).start();
    r.set_turn(2);
    let a = r.place_field_stack(0, &[ID], false, 0);
    r.attack_player(a, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(
        (r.game.players[0].hand.len(), r.game.players[1].hand.len()),
        (0, 0)
    );
}
#[test]
fn bt27_024_lock_survives_own_turn_end_blocks_attack_and_expires_at_opponent_end() {
    let mut r = base().hand(0, &[ID]).security(0, &["TOP"; 2]).start();
    r.set_turn(2);
    let b = r.place_field_stack(1, &["TOP"], false, 0);
    r.play(0, 0);
    choose(&mut r, 0);
    r.end_turn();
    r.auto_resolve().unwrap();
    assert!(r.game.modifiers.has(b, ModifierType::CannotSuspend));
    assert!(matches!(
        r.attack_player(b, 0, false),
        digimon_engine::combat::AttackResult::Invalid
    ));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert!(!r.game.modifiers.has(b, ModifierType::CannotSuspend));
}

#[test]
fn bt27_024_own_hand_size_does_not_increase_source_trash() {
    let mut r = base()
        .hand(0, &[ID, "F", "F", "F", "F", "F", "F", "F"])
        .hand(1, &["F"; 3])
        .start();
    r.place_stack(1, &["F", "F", "TOP"]);
    r.play(0, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].trash.len(), 0);
}
#[test]
fn bt27_024_metadata_matches_official_image() {
    let r = base().start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.name, "Gaogamon");
    assert_eq!(c.level, Some(4));
    assert_eq!(c.dp, Some(5000));
    assert!(c.traits.contains(&"DATA SQUAD".into()));
    assert!(c.traits.contains(&"Beast".into()));
}
