use digimon_dsl::{compile::compile, pack::CardPack, spec::CardSpec};
fn card(step: &str) -> CardSpec {
    serde_yml::from_str(&format!("card: TEST-GROUP\nname: Group\nkind: digimon\ncolor: [blue]\nlevel: 3\ncost: 3\ndp: 2000\neffects:\n - when: on_play\n   process:\n    - {step}\n")).unwrap()
}
#[test]
fn explicit_grouped_verbs_compile_and_pack_roundtrip() {
    for step in [
        "return_all_to_hand: { over: { of: opponent, kind: digimon }, selector: lowest_dp }",
        "return_all_to_deck: { over: { of: opponent, kind: digimon }, position: bottom }",
    ] {
        let c = compile(&card(step)).unwrap();
        let p = CardPack::new("grouped", vec![c]);
        assert_eq!(CardPack::from_bytes(&p.to_bytes().unwrap()).unwrap(), p);
    }
}
#[test]
fn grouped_return_checks_predicate_binding_scope() {
    let c = card("return_all_to_hand: {over: {binding_exists: missing}}");
    let e = digimon_dsl::validator::validate(
        &c,
        &digimon_dsl::validator::ValidationContext {
            raw_rust: &digimon_dsl::raw_rust_registry::StubRegistry::empty(),
        },
    );
    assert!(e.is_err());
}
#[test]
fn grouped_deck_random_and_unresolved_choice_are_rejected() {
    for position in ["random", "choice"] {
        let e = compile(&card(&format!(
            "return_all_to_deck: {{ over: {{of: opponent}}, position: {position} }}"
        )))
        .unwrap_err();
        assert!(e.iter().any(|e| e.path.ends_with("position")));
    }
}
