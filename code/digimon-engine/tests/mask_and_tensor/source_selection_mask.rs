use digimon_engine::action::space::{
    decode_source_select, encode_source_select, ACTION_SPACE_SIZE, SOURCES_PER_FIELD,
    SOURCE_SELECT_END, SOURCE_SELECT_START,
};

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
