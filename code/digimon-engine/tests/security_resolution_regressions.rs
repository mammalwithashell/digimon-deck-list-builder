//! Broad-suite regressions: preserve replacement choices and Security conditions.
//! Printed EX12-063/P-193 behavior and rules 13-1-7, 15-8, 15-9; no oracle claim.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_attack, PASS, SECURITY_TARGET},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

fn fighter(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

fn main_turn(r: &mut DebugRunner, seat: u8) {
    r.game.turn_player_idx = seat as usize;
    r.game.memory_pair = (seat, 1 - seat);
    r.game.memory = 10;
    r.game.turn_count = 2;
    r.game.current_phase = GamePhase::Main;
}

fn choose(r: &mut DebugRunner, seat: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, seat)[action as usize], 1.0);
    r.game.decode_action(action, seat);
}

fn both_seats(mut scenario: impl FnMut(u8)) {
    let mut failed = Vec::new();
    for seat in 0..=1 {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scenario(seat))).is_err() {
            failed.push(seat);
        }
    }
    assert!(failed.is_empty(), "failed seats: {failed:?}");
}

fn barrier_then_inherited_choice(play: bool) {
    both_seats(|seat| {
        let other = 1 - seat;
        let mut barrier = fighter("BARRIER-TOP", 5000);
        barrier.keywords = vec![Keyword::Barrier];
        let mut target = fighter("TB-L4", 5000);
        target.traits = vec!["Puppet".into(), "TB".into()];
        let mut r = DebugRunner::builder()
            .dsl_card("EX12-063")
            .unwrap()
            .add_card(barrier)
            .add_card(target)
            .add_card(fighter("SEC-WALL", 12000))
            .add_card(fighter("FILLER", 1000))
            .deck(seat, &["TB-L4"])
            .security(seat, &["FILLER"; 2])
            .security(other, &["SEC-WALL"])
            .build();
        main_turn(&mut r, seat);
        let card = r.game.players[seat as usize].deck.pop().unwrap();
        r.game.players[seat as usize].trash.push(card);
        let carrier = r.place_stack(seat, &["EX12-063", "BARRIER-TOP"]);
        choose(
            &mut r,
            seat,
            encode_attack(carrier.index as u16, SECURITY_TARGET),
        );
        let view = r
            .pending_selection_view()
            .expect("one Barrier replacement gate");
        assert_eq!(view.kind, SelectionKind::Replacement);
        assert_eq!(view.selecting_player, seat);
        assert!(view.prompt.contains("Barrier"));
        choose(&mut r, seat, PASS);

        let view = r
            .pending_selection_view()
            .expect("inherited trash play after decline");
        assert_eq!(
            view.kind,
            SelectionKind::Trash,
            "a second Barrier gate is not inherited play"
        );
        assert_eq!(view.selecting_player, seat);
        assert!(view.is_optional);
        assert_eq!(r.game.players[seat as usize].security.len(), 2);
        assert!(
            r.game.players[seat as usize].battle_area.is_empty(),
            "carrier deleted, no automatic play"
        );
        let pick = *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        choose(&mut r, seat, if play { pick } else { PASS });
        r.auto_resolve().expect("finish selected inherited branch");
        let field = &r.game.players[seat as usize].battle_area;
        assert_eq!(field.len(), usize::from(play));
        if play {
            assert_eq!(field[0].top_card().card_id(&r.game.card_data), "TB-L4");
        }
        assert_eq!(r.game.players[seat as usize].security.len(), 2);
        assert!(r.game.pending_selection.is_none());
        assert!(r.game.pending_security.is_none());
        assert!(r.game.security_resolution.is_none());
        assert!(r.game.pending_attack.is_none());
    });
}

#[test]
fn declined_barrier_exposes_exact_inherited_trash_choice_and_plays_once() {
    barrier_then_inherited_choice(true);
}

#[test]
fn declined_barrier_allows_inherited_play_decline_without_replacement_reentry() {
    barrier_then_inherited_choice(false);
}

fn p193_security(seat: u8, fodder: bool) {
    let defender = 1 - seat;
    let mut hand = fighter("HAND", 1000);
    if fodder {
        hand.traits = vec!["Composite".into()];
    }
    let mut r = DebugRunner::builder()
        .dsl_card("P-193")
        .unwrap()
        .add_card(hand)
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DRAW", 1000))
        .hand(defender, &["HAND"])
        .deck(defender, &["DRAW"; 10])
        .security(defender, &["P-193"])
        .build();
    main_turn(&mut r, seat);
    let attacker = r.place_on_field(seat, "ATK", Some(0));
    choose(
        &mut r,
        seat,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    if fodder {
        let view = r.pending_selection_view().expect("fodder cost choice");
        assert_eq!(view.kind, SelectionKind::Hand);
        assert_eq!(view.selecting_player, defender);
        let pick = *view.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        choose(&mut r, defender, pick);
    }
    r.auto_resolve().expect("finish security check");
    let p = &r.game.players[defender as usize];
    assert_eq!(
        p.deck.len(),
        if fodder { 8 } else { 10 },
        "no draw unless cost paid"
    );
    assert_eq!(p.hand.len(), if fodder { 2 } else { 1 });
    assert_eq!(p.trash.len(), 1);
    assert_eq!(
        p.trash[0].card_id(&r.game.card_data),
        if fodder { "HAND" } else { "P-193" }
    );
    assert_eq!(
        p.battle_area.len(),
        usize::from(fodder),
        "no Delay placement unless cost paid"
    );
    if fodder {
        assert_eq!(
            p.battle_area[0].top_card().card_id(&r.game.card_data),
            "P-193"
        );
    }
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.pending_security.is_none());
    assert!(r.game.security_resolution.is_none());
}

#[test]
fn security_condition_without_fodder_skips_draw_and_delay_placement() {
    both_seats(|seat| p193_security(seat, false));
}

#[test]
fn security_condition_with_fodder_preserves_player_choice_draw_and_delay() {
    both_seats(|seat| p193_security(seat, true));
}
