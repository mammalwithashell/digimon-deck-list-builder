//! BT15-003 Nyaromon - Digi-Egg, Lv.2, Yellow.
//!
//! # Card text (cards.json)
//!
//! Inherited Effect [When Attacking] [Once Per Turn]
//! By trashing the top or bottom card of your security stack, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT15/Yellow/BT15_003.cs
//!
//! # Patterns this test covers
//! - G4 inherited When Attacking on Digi-Egg.
//! - E2 OPT + optional cost payment, structurally exposed by the clause.
//! - Selection: top/bottom branch via SelectionKind::EffectChoice.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

fn nyaromon_runner(security: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT15-003")
        .expect("BT15-003 YAML must be present in the embedded DSL pack")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("DEFENDER", "Defender"))
        .add_card(make_test_card("SEC-BOTTOM", "Security Bottom"))
        .add_card(make_test_card("SEC-TOP", "Security Top"))
        .security(0, security)
        .memory(0)
        .start()
}

fn start_attack_with_nyaromon(mut runner: DebugRunner, expect_prompt: bool) -> DebugRunner {
    let attacker = runner.place_stack(0, &["BT15-003", "ATTACKER"]);

    let result = runner.attack_player(attacker, 1, false);
    if expect_prompt {
        assert!(
            runner.pending_selection().is_some(),
            "Nyaromon inherited When Attacking should park on its top/bottom choice; attack result: {result:?}"
        );
    }

    runner
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn bt15_003_yaml_loads_as_inherited_when_attacking_opt_clause() {
    let runner = nyaromon_runner(&["SEC-BOTTOM", "SEC-TOP"]);
    let compiled = runner
        .compiled_card("BT15-003")
        .expect("BT15-003 compiled card must be registered");

    assert_eq!(compiled.effects.len(), 1, "only one inherited clause");

    match &compiled.effects[0] {
        CompiledClause::Triggered(clause) => {
            assert_eq!(clause.scope, CompiledScope::Inherited);
            assert!(
                clause.when.contains(&CompiledTiming::WhenAttacking),
                "Nyaromon must trigger from inherited [When Attacking]"
            );
            assert!(clause.once_per_turn, "printed text is [Once Per Turn]");
            assert!(
                clause.optional,
                "'By trashing...' cost payment must be declinable"
            );
        }
        other => panic!("BT15-003 effect must be triggered; got {other:?}"),
    }
}

#[test]
fn bt15_003_when_attacking_offers_top_bottom_security_choice() {
    let runner = start_attack_with_nyaromon(nyaromon_runner(&["SEC-BOTTOM", "SEC-TOP"]), true);

    let view = runner
        .pending_selection_view()
        .expect("top/bottom choice must be pending");
    assert_eq!(view.kind, SelectionKind::EffectChoice);

    let labels: Vec<&str> = view
        .effect_choices
        .as_ref()
        .expect("effect-choice labels")
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert_eq!(
        labels,
        vec!["Trash top security", "Trash bottom security"],
        "the player-visible top-vs-bottom choice must be preserved"
    );
}

#[test]
fn bt15_003_top_branch_trashes_top_security_and_gains_memory() {
    let mut runner = start_attack_with_nyaromon(nyaromon_runner(&["SEC-BOTTOM", "SEC-TOP"]), true);
    let memory_before = runner.memory();
    let security_before = runner.security_count(0);

    runner
        .execute_branch(0)
        .expect("choose top security branch");
    runner.auto_resolve().expect("top branch resolves");

    assert_eq!(runner.security_count(0), security_before - 1);
    assert_eq!(runner.memory(), memory_before + 1);
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["SEC-TOP".to_string()],
        "the top security card is the last security entry"
    );
}

#[test]
fn bt15_003_bottom_branch_trashes_bottom_security_and_gains_memory() {
    let mut runner = start_attack_with_nyaromon(nyaromon_runner(&["SEC-BOTTOM", "SEC-TOP"]), true);
    let memory_before = runner.memory();
    let security_before = runner.security_count(0);

    runner
        .execute_branch(1)
        .expect("choose bottom security branch");
    runner.auto_resolve().expect("bottom branch resolves");

    assert_eq!(runner.security_count(0), security_before - 1);
    assert_eq!(runner.memory(), memory_before + 1);
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["SEC-BOTTOM".to_string()],
        "choosing bottom must trash the first security entry"
    );
}

#[test]
fn bt15_003_does_not_prompt_without_own_security() {
    let runner = start_attack_with_nyaromon(nyaromon_runner(&[]), false);

    assert!(
        runner.pending_selection().is_none(),
        "with no own security, the cost cannot be paid and no choice should be offered"
    );
    assert_eq!(runner.memory(), 0);
}
