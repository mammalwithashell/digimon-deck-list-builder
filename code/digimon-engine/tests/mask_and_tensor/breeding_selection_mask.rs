use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::encode_breeding_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

#[test]
fn breeding_selection_mask_exposes_only_breeding_select_action() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("BREED", "Breed"))
        .start();
    let p0 = 0;
    let source = r.place_on_field(p0, "SRC", Some(0));
    let source_card = r.top_card(source);
    r.place_in_breeding(p0, "BREED");
    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.select_own_breeding_permanent("pick breeding", |_, _| true, |_, _| {});
    }

    let mask = build_action_mask(&r.game, p0);
    let legal: Vec<usize> = mask
        .iter()
        .enumerate()
        .filter_map(|(idx, value)| if *value > 0.5 { Some(idx) } else { None })
        .collect();
    assert_eq!(legal, vec![encode_breeding_select(p0).unwrap() as usize]);
}
