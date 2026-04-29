use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

#[test]
fn dp_budget_mask_exposes_only_candidates_within_remaining_budget() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("LOW", "Low"))
        .add_card(make_test_card("HIGH", "High"))
        .start();
    let p0 = 0;
    let p1 = 1;
    let source = r.place_on_field(p0, "SRC", Some(0));
    r.force_base_dp("LOW", 3000);
    r.force_base_dp("HIGH", 9000);
    let low = r.place_on_field(p1, "LOW", Some(0));
    let high = r.place_on_field(p1, "HIGH", Some(0));

    {
        let source_card = r.top_card(source);
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.select_opponent_permanents_by_dp_budget("budget", 5000, 0, |_, _| true, |_, _| {});
    }

    let mask = build_action_mask(&r.game, p0);
    assert_eq!(mask[encode_attack(0, low.index as u16) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, high.index as u16) as usize], 0.0);
    assert_eq!(mask[PASS as usize], 1.0);
}
