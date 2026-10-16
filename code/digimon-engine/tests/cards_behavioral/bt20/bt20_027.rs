//! BT20-027 official observation d65ce267e6147e770d19a689be54b78549981c509f633db5dfd204a4ecc0b7b0 and image; Q4315-4319.
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::replacement::ReplacementCause;
const ID: &str = "BT20-027";
fn setup(hand: &[&str]) -> DebugRunner {
    let yaml = include_str!("../../../cards/bt20/BT20-027.yaml");
    let mut cards = digimon_engine::card_data::CardData::load_from_str(
        include_str!("../../../../../data/cards.json")
    ).unwrap();
    let printed = cards.remove(ID).unwrap();
    assert!(printed.effect_text.contains("Dracomon"));
    let mut b = DebugRunner::builder().from_dsl_yaml(&yaml).unwrap().add_card(printed);
    for (id, name, lv, color) in [
        ("BLUE", "Blue", 5, CardColor::Blue),
        ("RED", "Red", 5, CardColor::Red),
        ("WING", "Wingdramon", 4, CardColor::Yellow),
        ("GROUND", "Groundramon", 5, CardColor::Green),
        ("WRONG", "Wingdramon X", 5, CardColor::Green),
        ("QUAL", "Examon", 7, CardColor::Blue),
        ("QUAL2", "Dracomon", 3, CardColor::Green),
        ("PLAIN", "Plain", 5, CardColor::Yellow),
        ("FILL", "Fill", 3, CardColor::Red),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(lv);
        c.colors = vec![color];
        c.dp = Some(5000);
        b = b.add_card(c);
    }
    let mut c = make_test_card("TAMER", "Examon");
    c.card_kind = CardKind::Tamer;
    b = b.add_card(c);
    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .security(0, &["FILL"; 4])
        .security(1, &["FILL"; 4])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}
fn choose(r: &mut DebugRunner, a: u16) {
    let v = r.pending_selection_view().expect("choice");
    assert!(v.valid_action_ids.contains(&a) || (a == PASS && v.is_optional));
    assert!(
        digimon_engine::action::build_action_mask(&r.game, v.selecting_player)[a as usize] > 0.0
    );
    r.execute_action(v.selecting_player, a).unwrap();
}
fn accept(r: &mut DebugRunner) {
    let v = r.pending_selection_view().unwrap();
    let a = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    choose(r, a);
}
#[test]
fn bt20_027_routes_and_piercing() {
    for (base, cost) in [
        ("BLUE", 4),
        ("RED", 4),
        ("WING", 3),
        ("GROUND", 3),
        ("WRONG", -1),
    ] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, base, None);
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0,
            cost >= 0,
            "{base}"
        );
        if cost >= 0 {
            let before = r.memory();
            r.game.decode_action(a, 0);
            assert_eq!(before - r.memory(), cost);
            assert!(r.game.has_keyword(h, Keyword::Piercing));
        }
    }
}
#[test]
fn bt20_027_both_timings_strip_only_chosen_host_then_independent_delete() {
    for play in [true, false] {
        let mut r = setup(&[ID]);
        r.place_stack(1, &["FILL", "BLUE"]);
        r.place_stack(1, &["FILL", "FILL", "RED"]);
        r.place_on_field(1, "PLAIN", None);
        r.place_on_field(1, "TAMER", None);
        if play {
            r.play(0, 0).unwrap();
        } else {
            let h = r.place_on_field(0, "BLUE", None);
            let a = encode_digivolve(0, h.index as u16);
            assert!(digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0);
            r.game.decode_action(a, 0);
        }
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids,
            vec![ATTACK_START, ATTACK_START + 1]
        );
        choose(&mut r, ATTACK_START);
        assert!(!r.pending_selection_view().unwrap().is_optional);
        accept(&mut r);
        assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 1);
        assert_eq!(r.game.players[1].battle_area[1].card_sources.len(), 3);
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids,
            vec![ATTACK_START, ATTACK_START + 2]
        );
        choose(&mut r, ATTACK_START + 2);
        assert_eq!(r.game.players[1].battle_area.len(), 3);
    }
}
#[test]
fn bt20_027_deletion_without_any_sources() {
    let mut r = setup(&[ID]);
    r.place_on_field(1, "PLAIN", None);
    r.play(0, 0).unwrap();
    choose(&mut r, ATTACK_START);
    assert!(r.game.players[1].battle_area.is_empty());
}
#[test]
fn bt20_027_replacement_batch_clone_and_opt() {
    let mut r = setup(&[]);
    let carrier = r.place_stack(0, &[ID, "PLAIN"]);
    let a = r.place_on_field(0, "QUAL", None);
    let b = r.place_on_field(0, "QUAL2", None);
    r.game
        .delete_permanents_batch(vec![a, b], ReplacementCause::OwnEffect);
    assert!(r.pending_selection_view().unwrap().is_optional);
    let mut cloned = r.game.clone();
    accept(&mut r);
    let v = cloned.pending_selection.as_ref().unwrap();
    let action = *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
    cloned.decode_action(action, 0);
    assert_eq!(r.game.players[0].battle_area.len(), 3);
    assert_eq!(cloned.players[0].battle_area.len(), 3);
    assert!(r.game.players[0].battle_area[carrier.index as usize].is_suspended);
    r.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    r.game
        .delete_permanent_with_cause(a, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 2);
}
#[test]
fn bt20_027_replacement_filters_and_decline() {
    for (target, cause, suspended, offered) in [
        ("QUAL", ReplacementCause::Battle, false, false),
        ("PLAIN", ReplacementCause::OpponentEffect, false, false),
        ("TAMER", ReplacementCause::OpponentEffect, false, false),
        ("QUAL", ReplacementCause::OpponentEffect, true, false),
        ("QUAL", ReplacementCause::OpponentEffect, false, true),
    ] {
        let mut r = setup(&[]);
        let h = r.place_stack(0, &[ID, "PLAIN"]);
        r.game.players[0].battle_area[h.index as usize].is_suspended = suspended;
        let a = r.place_on_field(0, target, None);
        r.game.delete_permanent_with_cause(a, cause);
        r.game.drain_effect_queue();
        assert_eq!(r.pending_selection().is_some(), offered);
        if offered {
            choose(&mut r, PASS);
        }
        assert_eq!(r.game.players[0].battle_area.len(), 1);
    }
}

fn remove_security(r: &mut DebugRunner, owner: u8) {
    let card = r.game.players[owner as usize]
        .security
        .last()
        .unwrap()
        .handle();
    {
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut r.game, card, None, owner);
        assert!(ctx.trash_top_security(owner));
    }
    r.game.drain_effect_queue();
}
#[test]
fn bt20_027_security_removal_target_masks_both_turns_opt_and_decline() {
    for opponent_turn in [false, true] {
        let mut r = setup(&[]);
        r.place_on_field(0, ID, None);
        let a = r.place_on_field(0, "QUAL", None);
        let b = r.place_on_field(0, "QUAL2", None);
        let bad = r.place_on_field(0, "PLAIN", None);
        let t = r.place_on_field(0, "TAMER", None);
        r.place_on_field(1, "QUAL", None);
        for h in [a, b, bad, t] {
            r.game.players[0].battle_area[h.index as usize].is_suspended = true;
        }
        if opponent_turn {
            r.end_turn();
            r.auto_resolve().unwrap();
            r.game.memory = 3;
        }
        remove_security(&mut r, 0);
        assert!(r.pending_selection().is_none());
        remove_security(&mut r, 1);
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids,
            vec![ATTACK_START, ATTACK_START + 1, ATTACK_START + 2]
        );
        let mut clone = r.game.clone();
        choose(&mut r, ATTACK_START + 2);
        clone.decode_action(ATTACK_START + 2, 0);
        assert!(!r.game.players[0].battle_area[b.index as usize].is_suspended);
        assert!(!clone.players[0].battle_area[b.index as usize].is_suspended);
        assert!(r.game.players[0].battle_area[a.index as usize].is_suspended);
        remove_security(&mut r, 1);
        assert!(r.pending_selection().is_none());
    }
    let mut r = setup(&[]);
    r.place_on_field(0, ID, None);
    let a = r.place_on_field(0, "QUAL", None);
    r.game.players[0].battle_area[a.index as usize].is_suspended = true;
    remove_security(&mut r, 1);
    choose(&mut r, PASS);
    assert!(r.game.players[0].battle_area[a.index as usize].is_suspended);
    remove_security(&mut r, 1);
    assert!(r.pending_selection().is_some(), "decline must preserve OPT");
    choose(&mut r, ATTACK_START + 1);
    assert!(!r.game.players[0].battle_area[a.index as usize].is_suspended);
}
#[test]
fn bt20_027_any_three_sources_mask() {
    let mut r = setup(&[ID]);
    r.place_stack(1, &["FILL", "BLUE", "RED", "GROUND", "PLAIN"]);
    r.play(0, 0).unwrap();
    choose(&mut r, ATTACK_START);
    for i in 0..3 {
        let v = r.pending_selection_view().unwrap();
        assert_eq!(v.valid_action_ids.len(), 4 - i);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
            0.0
        );
        let action = *v.valid_action_ids.last().unwrap();
        choose(&mut r, action);
    }
    assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 2);
    assert_eq!(r.game.players[1].trash.len(), 3);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt20_027_cannot_suspend_cannot_pay_replacement() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    let mut r = setup(&[]);
    let carrier = r.place_stack(0, &[ID, "PLAIN"]);
    let ally = r.place_on_field(0, "QUAL", None);
    r.game.modifiers.add(
        carrier,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );
    r.game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "unpayable suspend must not be offered"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "unpaid suspension cannot protect"
    );
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn bt20_027_security_no_target_does_not_spend_opt() {
    let mut r = setup(&[]);
    let source = r.place_on_field(0, ID, None);
    let a = r.place_on_field(0, "QUAL", None);
    // Unsuspended qualifying Digimon are legal no-op targets. Make this a
    // genuine no-target case by prohibiting unsuspension on both candidates.
    for target in [source, a] {
        r.game.modifiers.add(
            target,
            digimon_engine::modifiers::ModifierEntry::simple(
                digimon_engine::enums::ModifierType::CannotUnsuspend,
                0,
                digimon_engine::enums::Expiry::Permanent,
                0,
            ),
        );
    }
    remove_security(&mut r, 1);
    assert!(r.pending_selection().is_none());
    r.game.modifiers.clear_permanent(a);
    r.game.players[0].battle_area[a.index as usize].is_suspended = true;
    remove_security(&mut r, 1);
    assert!(
        r.pending_selection().is_some(),
        "no eligible target must not consume OPT"
    );
}
