use digimon_engine::action::space::{
    decode_source_select, encode_source_select, ACTION_SPACE_SIZE, SOURCES_PER_FIELD,
    SOURCE_SELECT_END, SOURCE_SELECT_START,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::{action::mask::build_action_mask, action::space::PASS};

#[test]
fn source_select_encoder_round_trips_existing_range() {
    let action = encode_source_select(3, 5).expect("field 3 source 5 fits");
    assert_eq!(action, SOURCE_SELECT_START + 3 * SOURCES_PER_FIELD + 5);
    assert_eq!(decode_source_select(action), (3, 5));
}

#[test]
fn source_select_encoder_rejects_values_outside_existing_range() {
    assert_eq!(encode_source_select(14, 0), None);
    assert_eq!(encode_source_select(0, SOURCES_PER_FIELD), None);
    assert_eq!(SOURCE_SELECT_END as usize, ACTION_SPACE_SIZE);
}

#[test]
fn source_multi_mask_only_exposes_selecting_players_pending_actions() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC-A", "Source A"))
        .add_card(make_test_card("TOP-A", "Top A"))
        .start();
    let p0 = 0;
    let p1 = 1;
    let stack = r.place_stack(p0, &["SRC-A", "TOP-A"]);
    let stack_top = r.top_card(stack);
    {
        let mut ctx = EffectContext::new(&mut r.game, stack_top, Some(stack), p0);
        ctx.select_own_sources(
            "pick one source",
            1,
            1,
            move |_, source| source.card != stack_top,
            |_, _| {},
        );
    }

    let p0_mask = build_action_mask(&r.game, p0);
    let p1_mask = build_action_mask(&r.game, p1);
    assert!(
        p0_mask.iter().any(|v| *v > 0.5),
        "selecting player sees source action"
    );
    assert!(
        p1_mask.iter().all(|v| *v == 0.0),
        "non-selecting player sees empty mask"
    );
    assert_eq!(
        p0_mask[PASS as usize], 0.0,
        "exact one source cannot PASS before picking"
    );
}
