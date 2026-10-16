use digimon_engine::inference::contract::ModelContract;
use digimon_engine::tensor_profiles::{
    default_profile,
    standard::{v2_full, v2_lite},
};

#[test]
fn exact_runtime_contract_is_accepted() {
    for profile in [default_profile(), v2_lite::PROFILE, v2_full::PROFILE] {
        let expected = ModelContract::for_profile(&profile);
        let raw = serde_json::to_string(&expected).unwrap();
        assert_eq!(
            ModelContract::parse_and_validate(&raw, &profile).unwrap(),
            expected
        );
    }
}
#[test]
fn same_shape_old_schema_or_layout_cannot_be_used() {
    let p = default_profile();
    for field in [
        "observation_profile",
        "tensor_version",
        "feature_schema_version",
        "tensor_layout_hash",
        "tensor_size",
        "action_space_size",
    ] {
        let mut value = serde_json::to_value(ModelContract::for_profile(&p)).unwrap();
        value[field] = match field {
            "tensor_version" => serde_json::json!(1),
            "tensor_size" => serde_json::json!(p.tensor_size + 1),
            "action_space_size" => serde_json::json!(2191),
            _ => serde_json::json!("old-or-unknown"),
        };
        assert!(
            ModelContract::parse_and_validate(&value.to_string(), &p).is_err(),
            "{field}"
        );
    }
}
#[test]
fn missing_or_malformed_fields_are_not_inferred_from_shape() {
    let p = default_profile();
    for field in [
        "observation_profile",
        "tensor_version",
        "feature_schema_version",
        "tensor_size",
        "tensor_layout_hash",
        "action_space_size",
    ] {
        let mut value = serde_json::to_value(ModelContract::for_profile(&p)).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(
            ModelContract::parse_and_validate(&value.to_string(), &p).is_err(),
            "{field}"
        );
        value[field] = serde_json::Value::Null;
        assert!(
            ModelContract::parse_and_validate(&value.to_string(), &p).is_err(),
            "null {field}"
        );
    }
    assert!(ModelContract::parse_and_validate("{}", &p).is_err());
}
#[test]
fn alternate_profile_is_rejected_by_default_only_runtime() {
    let p = default_profile();
    let alternate = ModelContract::for_profile(&v2_lite::PROFILE);
    assert!(alternate.validate_for_profile(&p).is_err());
}
