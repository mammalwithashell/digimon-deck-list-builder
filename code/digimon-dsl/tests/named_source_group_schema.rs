//! Authoring guards for named-role source selections.
fn compiles(extra:&str)->bool{
 let yaml=format!(r#"
card: ROLE-SCHEMA
name: Role Schema
kind: digimon
level: 5
color: [black]
cost: 7
dp: 7000
effects:
  - when: on_play
    process:
      - select_materials:
          of_permanent: source
          max: 2
          prompt: Choose named roles
{extra}
"#);
 let spec:digimon_dsl::CardSpec=serde_yml::from_str(&yaml).unwrap();digimon_dsl::validator::validate(&spec, &digimon_dsl::validator::ValidationContext { raw_rust: &digimon_dsl::raw_rust_registry::StubRegistry::empty() }).is_ok()
}
#[test]
fn named_group_authoring_rejects_ambiguous_or_incompatible_constraints(){
 for extra in [
  "          require_max: true",
  "          named_groups: [SkullKnightmon, SkullKnightmon]",
  "          named_groups: ['']",
  "          named_groups: [SkullKnightmon]\n          uniqueness: name",
  "          named_groups: [SkullKnightmon]\n          same_by: level",
 ] {assert!(!compiles(extra),"accepted invalid author input: {extra}");}
}
#[test]
fn named_groups_accept_distinct_names_with_optional_zero_and_require_max(){
 assert!(compiles("          named_groups: [SkullKnightmon, DeadlyAxemon]\n          require_max: true\n          optional_zero: true"));
}
