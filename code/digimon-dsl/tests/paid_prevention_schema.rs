use digimon_dsl::{compile::compile, compiled::{CompiledClause,CompiledDeclarativeClause,CompiledStep}, pack::CardPack, spec::CardSpec};
fn spec(extra:&str,cost:&str,process:&str)->CardSpec {
    serde_yml::from_str(&format!("card: TEST-PAID\nname: Test\nkind: tamer\ncolor: [green]\ncost: 3\neffects:\n  - kind: replacement\n    trigger: when_would_be_deleted\n    optional: true\n{extra}{cost}    outcome: prevent\n{process}")).unwrap()
}
fn steps(card:&digimon_dsl::compiled::CompiledCard)->&[CompiledStep] {
    match &card.effects[0] {CompiledClause::Declarative(CompiledDeclarativeClause::Replacement{process,..})=>process,_=>panic!("replacement expected")}
}
#[test]
fn omitted_plurality_compiles_to_singular_payment() {
    let c=compile(&spec("","    cost: { delete_self: true }\n","")).unwrap();
    assert!(matches!(steps(&c),[CompiledStep::DeleteSelfAndCancelLeave]));
}
#[test]
fn explicit_plurality_survives_compilation_and_pack_roundtrip() {
    let c=compile(&spec("    prevent_all_matching: true\n","    cost: { delete_self: true }\n","")).unwrap();
    assert!(matches!(steps(&c),[CompiledStep::DeleteSelfAndCancelAllMatchingLeaves]));
    let p=CardPack::new("plural",vec![c]);
    let restored=CardPack::from_bytes(&p.to_bytes().unwrap()).unwrap();
    assert_eq!(p,restored);
}
#[test]
fn plural_flag_cannot_be_silently_ignored_without_carrier_payment() {
    let e=compile(&spec("    prevent_all_matching: true\n","","")).unwrap_err();
    assert!(e.iter().any(|e|e.path.ends_with("prevent_all_matching")));
}
#[test]
fn plural_flag_cannot_be_silently_ignored_by_explicit_process() {
    let e=compile(&spec("    prevent_all_matching: true\n","    cost: { delete_self: true }\n","    process:\n      - cancel_replacement: {}\n")).unwrap_err();
    assert!(e.iter().any(|e|e.path.ends_with("prevent_all_matching")));
}
