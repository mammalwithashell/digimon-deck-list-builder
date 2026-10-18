//! Hina's pending cost observes captured evolution facts; payout requires a live target.
//! Official EX3-065 Q3430 + general_rule.pdf 15-7-5 and 15-4-4-3.
//! Base EX3_065.cs gates activation on Hina's suspend cost, then attempts On Play.
use digimon_engine::action::{mask::build_action_mask, space::PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, GamePhase};
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn fixture(hina_count: usize) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card("EX3-065").unwrap()
        .dsl_card("EX13-044").unwrap()
        .dsl_card("ST1-03").unwrap()
        .memory(4).start();
    r.set_first_player(0);
    r.game.current_phase = GamePhase::Main;
    for _ in 0..hina_count { r.place_on_field(0, "EX3-065", Some(0)); }
    let subject = r.place_on_field(0, "EX13-044", Some(0));
    r.place_on_field(1, "ST1-03", Some(0));
    (r, subject)
}
fn enqueue(r: &mut DebugRunner, subject: PermanentHandle) {
    let card = r.game.players[0].battle_area[subject.index as usize].top_card().handle();
    // Isolate the real global observer fanout after a completed evolution.
    // No effect body or predicate is mocked; departure happens before activation.
    r.game.enqueue_triggered(EffectTiming::OnDigivolve, TriggerSource::Digivolved {
        player: 0, permanent: subject, card, effect_initiated: false, dna_origin: false,
    });
}
fn choose(game: &mut Game, action: u16) {
    let player = game.pending_selection.as_ref().expect("pending choice").selecting_player;
    let mask = build_action_mask(game, player);
    assert!(mask[action as usize] > 0.0, "choice must be legally masked");
    game.resolve_selection(player, action).unwrap();
}
fn optional_action(game: &Game) -> u16 {
    let p = game.pending_selection.as_ref().expect("Hina must retain optional cost choice");
    assert_eq!(p.kind, SelectionKind::Replacement);
    assert!(p.is_optional);
    assert!(build_action_mask(game, p.selecting_player)[PASS as usize] > 0.0);
    p.valid_action_ids[0]
}
fn assert_no_payout(game: &Game) {
    assert!(game.pending_selection.is_none(), "departed target cannot refire its On Play");
    assert_eq!(game.players[1].battle_area.len(), 1);
    assert!(!game.players[1].battle_area[0].is_suspended);
}

#[test]
fn lone_hina_may_pay_after_evolved_target_returns_to_hand_and_slot_is_reused() {
    let (mut r, target) = fixture(1);
    enqueue(&mut r, target);
    assert!(r.game.return_to_hand(target).is_some());
    let replacement = r.place_on_field(0, "EX13-044", Some(0));
    assert_eq!(replacement.index, target.index);
    r.game.drain_effect_queue();
    let cloned = r.game.clone();
    for mut game in [r.game, cloned] {
        let accept = optional_action(&game);
        choose(&mut game, accept);
        assert!(game.players[0].battle_area[0].is_suspended);
        assert!(!game.players[0].battle_area[replacement.index as usize].is_suspended);
        assert_no_payout(&game);
    }
}
#[test]
fn lone_hina_can_decline_after_evolved_target_leaves() {
    let (mut r, target) = fixture(1);
    enqueue(&mut r, target);
    assert!(r.game.return_to_hand(target).is_some());
    r.game.drain_effect_queue();
    optional_action(&r.game);
    choose(&mut r.game, PASS);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert_no_payout(&r.game);
}
#[test]
fn two_pending_hina_can_each_pay_after_evolved_target_leaves() {
    let (mut r, target) = fixture(2);
    enqueue(&mut r, target);
    assert!(r.game.return_to_hand(target).is_some());
    r.game.drain_effect_queue();
    let cloned = r.game.clone();
    for mut game in [r.game, cloned] {
        let p = game.pending_selection.as_ref().expect("two Hina choices");
        assert_eq!(p.kind, SelectionKind::TriggerOrder);
        assert_eq!(p.valid_action_ids.len(), 2);
        let first = p.valid_action_ids[0];
        // Choosing which pending effect activates (§15-4-3-5-1) is not choosing
        // to execute its optional processing condition "by suspending this
        // Tamer" (§15-7-1, §15-7-5): the picked Hina keeps its own accept/decline
        // gate before anything is suspended (da93979e8, run_picked_trigger).
        choose(&mut game, first);
        assert_eq!(game.players[0].battle_area.iter().filter(|p| p.is_suspended).count(), 0);
        let accept = optional_action(&game);
        choose(&mut game, accept);
        assert_eq!(game.players[0].battle_area.iter().filter(|p| p.is_suspended).count(), 1);
        let accept = optional_action(&game);
        choose(&mut game, accept);
        assert_eq!(game.players[0].battle_area.iter().filter(|p| p.is_suspended).count(), 2);
        assert_no_payout(&game);
    }
}
#[test]
fn departed_hina_source_cannot_pay_even_when_evolution_event_remains() {
    let (mut r, target) = fixture(1);
    enqueue(&mut r, target);
    assert!(r.game.return_to_hand(PermanentHandle {player:0,index:0}).is_some());
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_none());
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn already_suspended_hina_has_no_legal_cost_choice_after_target_leaves() {
    let (mut r, target) = fixture(1);
    r.game.players[0].battle_area[0].is_suspended = true;
    enqueue(&mut r, target);
    assert!(r.game.return_to_hand(target).is_some());
    r.game.drain_effect_queue();
    assert_no_payout(&r.game);
    assert!(r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn hina_does_not_offer_cost_on_opponents_turn() {
    let (mut r, target) = fixture(1);
    r.set_first_player(1);
    enqueue(&mut r, target);
    assert!(r.game.return_to_hand(target).is_some());
    r.game.drain_effect_queue();
    assert_no_payout(&r.game);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
