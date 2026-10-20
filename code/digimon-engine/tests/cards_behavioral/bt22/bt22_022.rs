//! BT22-022: official observation 7561b9df...ed70b2 and card image.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::replacement::ReplacementCause;
const ID: &str = "BT22-022";
fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT22-022 production YAML loads");
    for (id, name, lv, color, cs) in [
        ("BLUE", "Blue rookie", 3, CardColor::Blue, false),
        ("CS", "CS rookie", 3, CardColor::Red, true),
        ("BAD", "Wrong rookie", 3, CardColor::Red, false),
        ("HOST", "UlforceVeedramon", 6, CardColor::Blue, false),
        ("OTHER", "Unrelated", 6, CardColor::Blue, false),
        ("FILL", "Filler", 3, CardColor::Red, false),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(lv);
        c.colors = vec![color];
        c.dp = Some(6000);
        if cs {
            c.traits = vec!["CS".into()];
        }
        b = b.add_card(c);
    }
    for (id, text) in [("MATCH", "Your Veedramon gains power"), ("MISS", "Draw 1")] {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Tamer;
        c.level = None;
        c.dp = None;
        c.effect_text = text.into();
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn choose(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().expect("player choice");
    assert!(build_action_mask(&r.game, p.selecting_player)[a as usize] > 0.0);
    r.execute_action(p.selecting_player, a).unwrap();
}
fn accept(r: &mut DebugRunner) {
    let a = *r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    choose(r, a);
}
#[test]
fn bt22_022_routes() {
    for (base, legal) in [("BLUE", true), ("CS", true), ("BAD", false)] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, base, None);
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize] > 0.0,
            legal,
            "{base}"
        );
        if legal {
            let before = r.memory();
            r.game.decode_action(a, 0);
            assert_eq!(before - r.memory(), 2);
        }
    }
}
#[test]
fn bt22_022_free_tamer_text_filter_and_count_gate() {
    for count in 0..=2 {
        let mut r = setup(&[ID, "MATCH", "MISS"]);
        let h = r.place_on_field(0, "BLUE", None);
        for _ in 0..count {
            r.place_on_field(0, "MISS", None);
        }
        let action = encode_digivolve(0, h.index as u16);
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.0);
        r.game.decode_action(action, 0);
        if count == 2 {
            assert!(r.pending_selection().is_none());
            continue;
        }
        let before = r.memory();
        // Optional target picker exposes only the text-matching Tamer.
        let p = r.pending_selection_view().unwrap();
        assert!(p.is_optional);
        assert_eq!(p.valid_action_ids.iter().filter(|&&a| a != PASS).count(), 1);
        accept(&mut r);
        assert_eq!(r.memory(), before);
        assert_eq!(r.game.players[0].battle_area.len(), count + 2);
        assert!(r.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "MISS"));
    }
}
#[test]
fn bt22_022_free_tamer_decline_and_no_target() {
    for hand in [vec![ID, "MATCH"], vec![ID, "MISS"]] {
        let mut r = setup(&hand);
        let h = r.place_on_field(0, "BLUE", None);
        let action = encode_digivolve(0, h.index as u16);
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.0);
        r.game.decode_action(action, 0);
        if r.pending_selection().is_some() {
            choose(&mut r, PASS);
        }
        assert_eq!(r.game.players[0].battle_area.len(), 1);
    }
}
#[test]
fn bt22_022_inherited_protection_success_clone_and_opt() {
    let mut r = setup(&[]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().unwrap().is_optional);
    let mut clone = r.game.clone();
    let a = *r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    choose(&mut r, a);
    clone.decode_action(a, 0);
    for g in [&r.game, &clone] {
        assert_eq!(g.players[0].battle_area.len(), 1);
        assert!(g.players[0].battle_area[0].is_suspended);
    }
    r.game.players[0].battle_area[0].is_suspended = false;
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
}
#[test]
fn bt22_022_protection_filters_and_decline() {
    for (host, cause, suspended, offered) in [
        ("HOST", ReplacementCause::OwnEffect, false, false),
        ("HOST", ReplacementCause::Battle, false, false),
        ("OTHER", ReplacementCause::OpponentEffect, false, false),
        ("HOST", ReplacementCause::OpponentEffect, true, false),
        ("HOST", ReplacementCause::OpponentEffect, false, true),
    ] {
        let mut r = setup(&[]);
        let h = r.place_stack(0, &[ID, host]);
        r.game.players[0].battle_area[0].is_suspended = suspended;
        r.game.delete_permanent_with_cause(h, cause);
        r.game.drain_effect_queue();
        assert_eq!(r.pending_selection().is_some(), offered);
        if offered {
            choose(&mut r, PASS);
        }
        assert!(r.game.players[0].battle_area.is_empty());
    }
}
#[test]
fn bt22_022_cannot_suspend_does_not_protect() {
    let mut r = setup(&[]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
}
#[test]
fn bt22_022_payment_prevented_after_prompt_does_not_cancel() {
    let mut r = setup(&[]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_some());
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );
    accept(&mut r);
    assert!(r.game.players[0].battle_area.is_empty());
}
#[test]
fn bt22_022_only_protects_inherited_carrier() {
    let mut r = setup(&[]);
    let carrier = r.place_stack(0, &[ID, "HOST"]);
    let ally = r.place_on_field(0, "HOST", None);
    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(!r.game.players[0].battle_area[carrier.index as usize].is_suspended);
}
#[test]
fn bt22_022_inherited_prevents_opponent_hand_return() {
    let mut r = setup(&[]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.game.return_to_hand_from_effect(h, 1);
    r.game.drain_effect_queue();
    accept(&mut r);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert!(r.game.players[0].hand.is_empty());
}
