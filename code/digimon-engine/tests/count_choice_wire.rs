use digimon_engine::count_choice::{
    CountChoiceHost, CountChoiceMetadata, CountChoiceOperation, CountChoiceValue,
};
use digimon_engine::count_choice_wire::CountChoiceDto;
#[test]
fn count_transport_keeps_values_above_javascript_integer_precision_exact() {
    let dto = CountChoiceDto::from(CountChoiceMetadata {
        operation: CountChoiceOperation::Commit(CountChoiceValue {
            declared: 9_007_199_254_740_993,
            payable: 9_007_199_254_740_992,
        }),
        page_index: 10,
        page_count: 11,
        host: Some(CountChoiceHost {
            player: 1,
            field_index: 14,
            battle_identity: u64::MAX,
        }),
    });
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["declared_count"], "9007199254740993");
    assert_eq!(json["payable_count"], "9007199254740992");
    assert_eq!(json["host"]["battle_identity"], "18446744073709551615");
    assert_eq!(serde_json::from_value::<CountChoiceDto>(json).unwrap(), dto);
}
#[test]
fn navigation_is_typed_and_has_no_payable_value() {
    let dto = CountChoiceDto::from(CountChoiceMetadata {
        operation: CountChoiceOperation::NextPage,
        page_index: 0,
        page_count: 2,
        host: None,
    });
    let json = serde_json::to_value(dto).unwrap();
    assert_eq!(json["operation"], "next_page");
    assert!(json["declared_count"].is_null());
    assert!(json["payable_count"].is_null());
}
