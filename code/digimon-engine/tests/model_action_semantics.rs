use digimon_engine::{inference::contract::ModelContract,tensor_profiles};
#[test]
fn advertised_action_version_is_required_even_when_shapes_are_unchanged() {
 let profile=tensor_profiles::default_profile();
 let current=ModelContract::for_profile(&profile);
 let mut metadata=serde_json::to_value(&current).unwrap();
 assert_eq!(metadata["action_schema_version"],digimon_engine::action::space::SCHEMA_VERSION);
 metadata["action_schema_version"]=serde_json::json!(digimon_engine::action::space::SCHEMA_VERSION+1);
 assert!(ModelContract::parse_and_validate(&metadata.to_string(),&profile).is_err());
}
#[test]
fn missing_action_semantics_is_not_legacy_shape_compatibility() {
 let profile=tensor_profiles::default_profile();
 let mut metadata=serde_json::to_value(ModelContract::for_profile(&profile)).unwrap();
 metadata.as_object_mut().unwrap().remove("action_schema_version");
 assert!(ModelContract::parse_and_validate(&metadata.to_string(),&profile).is_err());
}

#[test]
fn malformed_action_versions_are_never_shape_compatible() {
    let profile = tensor_profiles::default_profile();
    for bad in [serde_json::Value::Null, serde_json::json!(true), serde_json::json!("1"), serde_json::json!(1.0), serde_json::json!(-1), serde_json::json!(0)] {
        let mut value = serde_json::to_value(ModelContract::for_profile(&profile)).unwrap();
        value["action_schema_version"] = bad.clone();
        assert!(ModelContract::parse_and_validate(&value.to_string(), &profile).is_err(), "accepted {bad}");
    }
}
