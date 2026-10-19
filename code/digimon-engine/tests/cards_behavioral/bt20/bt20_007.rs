//! BT20-007: image and official observation verified; DCGO BT20_007.cs cross-check.
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::{
    make_test_card, make_test_egg, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::SelectionKind;
use digimon_engine::selection::TriggerSource;
const ID: &str = "BT20-007";
fn base() -> DebugRunnerBuilder {
    let mut option = make_test_card("TEXT", "Unrelated Option");
    option.card_kind = CardKind::Option;
    option.effect_text = "Choose [Examon].".into();
    let mut inherited = make_test_card("INHERITED", "Unrelated Name");
    inherited.inherited_text = "[Dracomon] gets +1000 DP.".into();
    let mut beby = make_test_egg("BEBY", "Bebydomon");
    beby.colors = vec![CardColor::Blue];
    let mut wrong = beby.clone();
    wrong.card_id = "WRONG".into();
    wrong.card_name = "Bebydomon Variant".into();
    DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-007.yaml"))
        .expect("BT20-007 must be implemented")
        .add_card(option)
        .add_card(inherited)
        .add_card(beby)
        .add_card(wrong)
        .add_card(make_test_egg("RED", "Red Egg"))
        .add_card(make_test_card("PLAIN", "Plain"))
        .deck(0, &["PLAIN"; 12])
        .deck(1, &["PLAIN"; 12])
        .memory(5)
}
fn setup(hand: &[&str]) -> DebugRunner {
    let mut r = base().hand(0, hand).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(h),
    );
    r.game.drain_effect_queue();
    r
}
fn reach_hand(r: &mut DebugRunner) {
    if r.pending_kind() != Some(SelectionKind::Hand) {
        r.accept_optional_trigger().unwrap();
    }
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
}
#[test]
fn paid_cost_draws_one_gains_one_and_exposes_only_text_matches() {
    for index in [0, 1] {
        let mut r = setup(&["TEXT", "INHERITED", "PLAIN"]);
        reach_hand(&mut r);
        let mask = digimon_engine::action::build_action_mask(&r.game, 0);
        assert_eq!(mask[PLAY_HAND_START as usize], 1.0);
        assert_eq!(mask[PLAY_HAND_START as usize + 1], 1.0);
        assert_eq!(mask[PLAY_HAND_START as usize + 2], 0.0);
        assert_eq!(mask[PASS as usize], 1.0);
        r.execute_action(0, PLAY_HAND_START + index).unwrap();
        assert_eq!(r.memory(), 6);
        assert_eq!(r.hand_size(0), 3);
        assert_eq!(r.trash_size(0), 1);
        assert_eq!(r.game.players[0].deck.len(), 11);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn decline_cost_does_not_draw_or_gain_memory() {
    let mut r = setup(&["TEXT"]);
    reach_hand(&mut r);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.memory(), 5);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(r.game.players[0].deck.len(), 12);
}
#[test]
fn no_valid_cost_does_not_prompt_or_reward() {
    let r = setup(&["PLAIN"]);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 5);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.game.players[0].deck.len(), 12);
}
#[test]
fn inherited_dp_only_on_owners_turn_and_only_as_source() {
    let mut r = base().start();
    r.skip_mulligan();
    let top = r.place_on_field(0, ID, Some(0));
    let carrier = r.place_stack(0, &[ID, "PLAIN"]);
    assert_eq!(r.effective_dp(top), Some(1000));
    assert_eq!(r.effective_dp(carrier), Some(4000));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(carrier), Some(2000));
}
#[test]
fn normal_and_exact_named_digivolution_paths_cost_zero() {
    for (egg, allowed) in [("RED", true), ("BEBY", true), ("WRONG", false)] {
        let mut r = base().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, egg, Some(0));
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[action as usize] > 0.0,
            allowed,
            "{egg}"
        );
        if allowed {
            r.game.decode_action(action, 0);
            assert_eq!(r.memory(), 5);
            assert_eq!(r.hand_size(0), 1);
            assert_eq!(
                r.game.players[0].battle_area[h.index as usize]
                    .top_card()
                    .card_id(&r.game.card_data),
                ID
            );
        }
    }
}
