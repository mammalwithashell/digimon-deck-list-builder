use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn self_aura_dp_formula_recomputes_after_stack_depth_changes() {
    let yaml = r#"
card: TEST-DP-FORMULA
name: Formula Body
kind: digimon
color: [black]
level: 4
cost: 4
dp: 4000
effects:
  - kind: aura
    target: {}
    dp_modifier_fn: { base: 0, per: material_count, delta: 1000 }
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(make_test_card("BT5-008", "Material One"))
        .add_card(make_test_card("BT5-009", "Material Two"))
        .start();
    r.place_stack(0, &["BT5-008", "BT5-009", "TEST-DP-FORMULA"]);
    let h = PermanentHandle {
        player: 0,
        index: 0,
    };

    assert_eq!(r.game.effective_dp(h), Some(6000));
    r.game.players[0].battle_area[0].card_sources.remove(0);
    assert_eq!(
        r.game.effective_dp(h),
        Some(5000),
        "formula must be live, not snapshotted"
    );
}

#[test]
fn security_attack_formula_recomputes_at_attack_resolution() {
    let yaml = r#"
card: TEST-SA-FORMULA
name: Security Formula Body
kind: digimon
color: [red]
level: 4
cost: 4
dp: 4000
effects:
  - kind: aura
    target: {}
    security_attack_fn: { base: 1, per: material_count, delta: 1 }
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(make_test_card("BT5-008", "Material One"))
        .add_card(make_test_card("BT1-010", "Security One"))
        .add_card(make_test_card("BT1-011", "Security Two"))
        .add_card(make_test_card("BT1-012", "Security Three"))
        .security(1, &["BT1-010", "BT1-011", "BT1-012"])
        .start();
    let h = r.place_stack(0, &["BT5-008", "TEST-SA-FORMULA"]);

    r.attack_player(h, 1, false);
    assert_eq!(
        r.game.player(1).security.len(),
        1,
        "base 1 plus one material performs two checks"
    );
}

#[test]
fn nonmatching_security_attack_formula_aura_preserves_default_check() {
    let yaml = r#"
card: TEST-SA-FORMULA-AURA
name: Security Formula Aura
kind: digimon
color: [red]
level: 4
cost: 4
dp: 4000
effects:
  - kind: aura
    target: { owner: you, trait: Boosted }
    security_attack_fn: { base: 2, per: material_count, delta: 0 }
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register DSL card")
        .add_card(make_test_card("BT5-008", "Normal Attacker"))
        .add_card(make_test_card("BT1-010", "Security One"))
        .add_card(make_test_card("BT1-011", "Security Two"))
        .security(1, &["BT1-010", "BT1-011"])
        .start();
    r.place_on_field(0, "TEST-SA-FORMULA-AURA", None);
    let attacker = r.place_on_field(0, "BT5-008", None);

    let result = r.attack_player(attacker, 1, true);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        r.game.player(1).security.len(),
        1,
        "nonmatching formula aura must not suppress the default security check"
    );
}

#[test]
fn multiple_security_attack_formula_auras_use_max_not_sum() {
    let first = r#"
card: TEST-SA-FORMULA-FIRST
name: Security Formula First
kind: digimon
color: [red]
level: 4
cost: 4
dp: 4000
effects:
  - kind: aura
    target: { owner: you, trait: Boosted }
    security_attack_fn: { base: 1, per: material_count, delta: 0 }
"#;
    let second = r#"
card: TEST-SA-FORMULA-SECOND
name: Security Formula Second
kind: digimon
color: [red]
level: 4
cost: 4
dp: 4000
effects:
  - kind: aura
    target: { owner: you, trait: Boosted }
    security_attack_fn: { base: 1, per: material_count, delta: 0 }
"#;
    let mut attacker_card = make_test_card("BT5-008", "Boosted Attacker");
    attacker_card.traits.push("Boosted".to_string());
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(first)
        .expect("register first DSL card")
        .from_dsl_yaml(second)
        .expect("register second DSL card")
        .add_card(attacker_card)
        .add_card(make_test_card("BT1-010", "Security One"))
        .add_card(make_test_card("BT1-011", "Security Two"))
        .add_card(make_test_card("BT1-012", "Security Three"))
        .security(1, &["BT1-010", "BT1-011", "BT1-012"])
        .start();
    r.place_on_field(0, "TEST-SA-FORMULA-FIRST", None);
    r.place_on_field(0, "TEST-SA-FORMULA-SECOND", None);
    let h = r.place_on_field(0, "BT5-008", None);

    let result = r.attack_player(h, 1, true);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        r.game.player(1).security.len(),
        2,
        "two base-inclusive formulas with the same value should perform one check, not two"
    );
}
