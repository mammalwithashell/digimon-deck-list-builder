//! BT27-089: official Bandai image and Q7881-7886 checked 2026-11-22.
//! Placement is bottom/face-down; the After protection needs suspend payment,
//! but still happens when optional deck placement is declined.
use digimon_engine::{
    action::{mask::build_action_mask, space::PASS},
    card_data::CardData,
    debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder},
    enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType},
    modifiers::ModifierEntry,
    permanent::PermanentHandle,
};
const ID: &str = "BT27-089";
fn card(id: &str, name: &str, kind: CardKind) -> CardData {
    let mut c = make_test_card_with_level(id, name, 4);
    c.card_kind = kind;
    c.colors = vec![CardColor::Blue];
    c.dp = Some(if id == "BIG" { 15000 } else { 1000 });
    c
}
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("new Thomas YAML required")
        .dsl_card("BT27-019")
        .unwrap()
        .add_card(card("GAO", "Gaomon", CardKind::Digimon))
        .add_card(card("MACH", "MachGaogamon", CardKind::Digimon))
        .add_card(card("OPT", "Gaogamon", CardKind::Option))
        .add_card(card("F", "Filler", CardKind::Digimon))
        .add_card(card("BIG", "Big", CardKind::Digimon))
        .deck(0, &["F"; 20])
        .deck(1, &["F"; 20])
        .security(1, &["F", "F"])
        .memory(5)
}
fn setup() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = base().start();
    r.set_first_player(0);
    let t = r.place_stack(0, &["F", ID]);
    let g = r.place_on_field(0, "MACH", Some(0));
    (r, t, g)
}
fn pick(r: &mut DebugRunner, i: usize) {
    let v = r.pending_selection_view().expect("real choice");
    let a = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .nth(i)
        .unwrap();
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.execute_action(0, a).unwrap();
}
fn effect_bounce(r: &mut DebugRunner, p: u8) {
    let h = r.place_on_field(1, "F", Some(0));
    assert!(r.game.return_to_hand_from_effect(h, p));
    r.game.drain_effect_queue();
}
fn pay(r: &mut DebugRunner, place: bool) {
    r.accept_optional_trigger().expect("may pay suspension");
    r.execute_branch(if place { 0 } else { 1 })
        .expect("independent deck placement choice");
    pick(r, 0);
}
#[test]
fn bt27_089_identity_and_real_play() {
    let mut r = base().hand(0, &[ID]).start();
    r.play(0, 0).unwrap();
    assert_eq!(r.memory(), 2);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.compiled_card(ID).unwrap().cost, Some(3));
}
#[test]
fn bt27_089_start_main_hand_filter_bottom_payment_draw_memory() {
    for idx in [0, 1] {
        let mut r = base().hand(0, &["GAO", "MACH", "OPT", "F"]).start();
        r.set_first_player(0);
        let t = r.place_stack(0, &["F", ID]);
        let d = r.deck_size(0);
        let m = r.memory();
        r.game.enter_main_phase();
        let v = r.pending_selection_view().unwrap();
        assert!(r.pending_is_optional());
        assert_eq!(v.valid_action_ids.iter().filter(|a| **a != PASS).count(), 2);
        pick(&mut r, idx);
        let s = &r.game.players[0].battle_area[t.index as usize].card_sources;
        assert_eq!(
            s[0].card_id(&r.game.card_data),
            if idx == 0 { "GAO" } else { "MACH" }
        );
        assert!(s[0].face_down);
        assert_eq!(s[1].card_id(&r.game.card_data), "F");
        assert_eq!(r.memory(), m + 1);
        assert_eq!(r.deck_size(0), d - 1);
    }
}
#[test]
fn bt27_089_start_main_decline_or_no_eligible_no_gain() {
    for eligible in [true, false] {
        let mut r = base()
            .hand(0, if eligible { &["GAO"] } else { &["OPT"] })
            .start();
        r.set_first_player(0);
        let t = r.place_on_field(0, ID, Some(0));
        let d = r.deck_size(0);
        r.game.enter_main_phase();
        if eligible {
            r.execute_action(0, PASS).unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 5);
        assert_eq!(r.deck_size(0), d);
        assert_eq!(
            r.game.players[0].battle_area[t.index as usize]
                .card_sources
                .len(),
            1
        );
    }
}
#[test]
fn bt27_089_effect_bounce_either_controller_places_bottom_and_protects() {
    for p in [0, 1] {
        let (mut r, t, g) = setup();
        let d = r.deck_size(0);
        effect_bounce(&mut r, p);
        pay(&mut r, true);
        let s = &r.game.players[0].battle_area[t.index as usize];
        assert!(s.is_suspended);
        assert_eq!(s.card_sources.len(), 3);
        assert!(s.card_sources[0].face_down);
        assert!(!s.card_sources[1].face_down);
        assert_eq!(r.deck_size(0), d - 1);
        assert!(r
            .game
            .modifiers
            .has(g, ModifierType::CannotBeDestroyedByBattle));
    }
}
#[test]
fn bt27_089_declining_placement_still_protects_after_suspension() {
    let (mut r, t, g) = setup();
    let d = r.deck_size(0);
    effect_bounce(&mut r, 0);
    pay(&mut r, false);
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert_eq!(r.deck_size(0), d);
    assert!(r
        .game
        .modifiers
        .has(g, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn bt27_089_declining_payment_no_after_protection() {
    let (mut r, t, g) = setup();
    effect_bounce(&mut r, 0);
    r.decline_optional_trigger().unwrap();
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
    assert!(!r
        .game
        .modifiers
        .has(g, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn bt27_089_suspended_or_cannot_suspend_cannot_pay() {
    for locked in [false, true] {
        let (mut r, t, g) = setup();
        if locked {
            r.game.modifiers.add(
                t,
                ModifierEntry::simple(ModifierType::CannotSuspend, 1, Expiry::Permanent, 1),
            );
        } else {
            r.game.players[0].battle_area[t.index as usize].is_suspended = true;
        }
        effect_bounce(&mut r, 0);
        if r.pending_selection().is_some() {
            r.accept_optional_trigger().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert!(!r
            .game
            .modifiers
            .has(g, ModifierType::CannotBeDestroyedByBattle));
    }
}
#[test]
fn bt27_089_own_hand_and_rule_draw_do_not_trigger() {
    let (mut r, t, _) = setup();
    let h = r.place_on_field(0, "F", Some(0));
    r.game.return_to_hand_from_effect(h, 0);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    let before = r.hand_size(1);
    r.end_turn();
    assert_eq!(r.hand_size(1), before + 1);
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[0].battle_area[t.index as usize].is_suspended);
}
#[test]
fn bt27_089_actual_inherited_effect_draw_triggers() {
    let (mut r, t, _) = setup();
    let h = r.place_stack(0, &["BT27-019", "MACH"]);
    let before = r.hand_size(1);
    r.attack_player(h, 1, false);
    assert_eq!(r.hand_size(1), before + 1);
    r.accept_optional_trigger().unwrap();
    r.execute_branch(1).unwrap();
    pick(&mut r, 0);
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
}
#[test]
fn bt27_089_protection_filter_and_battle_not_effect_expiry() {
    let (mut r, _, g) = setup();
    r.place_on_field(0, "GAO", Some(0));
    r.place_on_field(1, "MACH", Some(0));
    effect_bounce(&mut r, 0);
    r.accept_optional_trigger().unwrap();
    r.execute_branch(1).unwrap();
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        1
    );
    pick(&mut r, 0);
    let enemy = r.place_on_field(1, "BIG", Some(0));
    r.battle_digimon(g, enemy);
    assert!(r
        .game
        .modifiers
        .has(g, ModifierType::CannotBeDestroyedByBattle));
    r.game.delete_permanent_with_effects(g);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    let (mut r, _, g) = setup();
    effect_bounce(&mut r, 0);
    pay(&mut r, false);
    r.end_turn();
    assert!(!r
        .game
        .modifiers
        .has(g, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn bt27_089_not_once_per_turn() {
    let (mut r, t, _) = setup();
    effect_bounce(&mut r, 0);
    pay(&mut r, false);
    r.game.players[0].battle_area[t.index as usize].is_suspended = false;
    effect_bounce(&mut r, 0);
    assert!(r.pending_selection().is_some());
    r.decline_optional_trigger().unwrap();
}
#[test]
fn bt27_089_security_plays_without_paying() {
    let mut r = base().security(0, &[ID]).start();
    r.set_first_player(0);
    // A legal pass supplies memory to the new active player.
    r.pass_turn();
    r.set_phase(GamePhase::Main);
    assert_eq!(r.game.turn_player(), 1);
    let a = r.place_on_field(1, "BIG", Some(0));
    let m = r.memory();
    r.attack_player(a, 0, false);
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == ID));
    assert_eq!(r.memory(), m);
    assert_eq!(r.game.turn_player(), 1);
}

#[test]
fn bt27_089_protection_survives_stronger_security_battle() {
    let mut r = base().security(1, &["BIG"]).start();
    r.set_first_player(0);
    r.place_on_field(0, ID, Some(0));
    let g = r.place_on_field(0, "MACH", Some(0));
    effect_bounce(&mut r, 0);
    pay(&mut r, false);
    r.attack_player(g, 1, false);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
}
#[test]
fn bt27_089_opponent_turn_effect_still_triggers() {
    let (mut r, t, _) = setup();
    // Raw end_turn at positive memory immediately re-ends the new turn
    // after the effect, unsuspending Thomas on the next own turn.
    r.pass_turn();
    r.set_phase(GamePhase::Main);
    assert_eq!(r.game.turn_player(), 1);
    effect_bounce(&mut r, 1);
    pay(&mut r, false);
    assert_eq!(r.game.turn_player(), 1);
    assert!(r.game.players[0].battle_area[t.index as usize].is_suspended);
}
