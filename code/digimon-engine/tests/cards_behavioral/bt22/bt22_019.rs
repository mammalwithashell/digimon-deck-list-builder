//! Official BT22-019 observation 58a8b08fbf2d5f6ae25a80d1811b245c8cfa1260f683c9b6f2b3228319315576; image verified.
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};
use digimon_engine::replacement::ReplacementCause;
const ID: &str = "BT22-019";
fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT22-019 loaded");
    for (id, name, level) in [
        ("HOST", "AeroVeedramon", 5),
        ("OTHER", "Greymon", 4),
        ("EGG", "Egg", 2),
        ("CS-EGG", "CS Egg", 2),
        ("BAD-EGG", "Bad Egg", 2),
        ("FILL", "Fill", 3),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(level);
        c.colors = vec![CardColor::Blue];
        if id == "CS-EGG" {
            c.colors = vec![CardColor::Red];
            c.traits = vec!["CS".to_string()];
        }
        if id == "BAD-EGG" {
            c.colors = vec![CardColor::Red];
        }
        c.dp = Some(5000);
        c.evo_costs = vec![digimon_engine::card_data::EvoCost {
            level: 3,
            card_color: 1,
            memory_cost: 3,
        }];
        b = b.add_card(c);
    }
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
fn choose(r: &mut DebugRunner, accept: bool) {
    let v = r
        .pending_selection_view()
        .expect("optional protection choice");
    assert!(v.is_optional);
    let a = if accept {
        *v.valid_action_ids.iter().find(|&&a| a != PASS).unwrap()
    } else {
        PASS
    };
    assert!(
        digimon_engine::action::build_action_mask(&r.game, v.selecting_player)[a as usize] > 0.0
    );
    r.execute_action(v.selecting_player, a).unwrap();
}
#[test]
fn bt22_019_load_and_blue_egg_route() {
    let mut r = setup(&[ID]);
    let h = r.place_on_field(0, "EGG", None);
    let a = encode_digivolve(0, h.index as u16);
    assert!(digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0);
    let before = r.memory();
    r.game.decode_action(a, 0);
    assert_eq!(r.memory(), before);
}
#[test]
fn bt22_019_protection_accept_opt_and_no_other_host() {
    let mut r = setup(&[]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    let other = r.place_on_field(0, "HOST", None);
    r.game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    choose(&mut r, true);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    r.game.players[0].battle_area[0].is_suspended = false;
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
}
#[test]
fn bt22_019_protection_filters_and_decline() {
    for (top, cause, suspended, offered) in [
        ("HOST", ReplacementCause::OpponentEffect, false, true),
        ("HOST", ReplacementCause::OwnEffect, false, false),
        ("HOST", ReplacementCause::Battle, false, false),
        ("OTHER", ReplacementCause::OpponentEffect, false, false),
        ("HOST", ReplacementCause::OpponentEffect, true, false),
    ] {
        let mut r = setup(&[]);
        let h = r.place_stack(0, &[ID, top]);
        r.game.players[0].battle_area[0].is_suspended = suspended;
        r.game.delete_permanent_with_cause(h, cause);
        r.game.drain_effect_queue();
        assert_eq!(r.pending_selection().is_some(), offered, "{top} {cause:?}");
        if offered {
            choose(&mut r, false);
        }
        assert!(r.game.players[0].battle_area.is_empty());
    }
}

#[test]
fn bt22_019_reduction_only_self_veedramon_on_your_turn() {
    for (target, base, turn, cost) in [
        ("HOST", ID, 0, 2),
        ("OTHER", ID, 0, 3),
        ("HOST", "FILL", 0, 3),
    ] {
        let mut r = setup(&[target]);
        let h = r.place_on_field(0, base, None);
        if base != ID {
            r.place_on_field(0, ID, None);
        }
        r.game.turn_player_idx = turn;
        let before = r.memory();
        r.game.decode_action(encode_digivolve(0, h.index as u16), 0);
        assert_eq!(before - r.memory(), cost, "{target} {base} turn {turn}");
    }
}

#[test]
fn bt22_019_cannot_suspend_cannot_protect() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
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
fn bt22_019_cs_egg_route_and_wrong_color_excluded() {
    for (base, legal) in [("CS-EGG", true), ("BAD-EGG", false)] {
        let mut r = setup(&[ID]);
        let h = r.place_on_field(0, base, None);
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[a as usize] > 0.0,
            legal
        );
        if legal {
            let before = r.memory();
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), before);
        }
    }
}
