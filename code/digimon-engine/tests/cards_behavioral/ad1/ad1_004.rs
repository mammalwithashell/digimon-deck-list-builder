//! AD1-004 WarGreymon

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ad1_004_has_keywords_and_end_turn_attack_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("AD1-004").expect("compiled card");

    let keyword_count = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .count();
    assert_eq!(keyword_count, 2);
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn)
    )));
}

#[ignore = "pending: G-FORMULA-SOURCE-DP — delete opponent Digimon with DP <= this Digimon's effective DP"]
#[test]
fn ad1_004_deletes_opponent_digimon_at_or_below_self_dp() {}

// ─── End-of-Turn attack: windowed grant (not synchronous inline) ─────────────
// 2026-06-02: AD1-004's "[End of Your Turn] 1 of your Digimon may attack" must
// GRANT a windowed MayAttack to the chosen Digimon (deferred to the EOT-action
// window) rather than declaring + resolving the attack inline. The windowed
// model lets sibling end-of-turn effects (e.g. an inherited DNA digivolve)
// resolve first and remove the attacker, so its attack fizzles — faithful to
// general_rule.pdf §15-4-2-3 (EOT triggers activate one at a time) and the
// "attack ends if the attacker leaves before it resolves" rule.
#[test]
fn ad1_004_eot_attack_is_windowed_grant_not_synchronous() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::{EffectTiming, ModifierType};
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(make_test_card("SEC", "SecCard"))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    let wg = runner.place_on_field(0, "AD1-004", None);
    let sec_before = runner.game.players[1].security.len();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(wg));
    runner.game.drain_effect_queue();
    // Drive the optional EOT clause: accept + pick WarGreymon as the attacker.
    let mut guard = 0;
    while let Some(view) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 8, "selection loop runaway");
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .unwrap_or(PASS);
        runner
            .execute_action(view.selecting_player, pick)
            .expect("drive EOT clause");
        if runner.game.modifiers.has(wg, ModifierType::MayAttack) {
            break;
        }
    }

    assert!(
        runner.game.modifiers.has(wg, ModifierType::MayAttack),
        "AD1-004 EOT attack must grant a windowed MayAttack to the chosen Digimon (deferred), not resolve inline"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before,
        "windowed grant must NOT check security synchronously during the EOT trigger"
    );
}

// Capstone fizzle: once AD1-004's EOT attack is a windowed grant, removing the
// chosen attacker (the user's DNA-digivolve-into-Omnimon line consuming
// WarGreymon) before it takes its deferred attack leaves the grant orphaned —
// no attack happens and the opponent's security is untouched. This is the
// system-level outcome the inline path could never produce.
#[test]
fn ad1_004_eot_attack_fizzles_when_attacker_is_removed_before_it_acts() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::{EffectTiming, ModifierType};
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(make_test_card("SEC", "SecCard"))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    let wg = runner.place_on_field(0, "AD1-004", None);
    let sec_before = runner.game.players[1].security.len();

    // EOT: grant the windowed attack to WarGreymon.
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(wg));
    runner.game.drain_effect_queue();
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 8, "selection loop runaway");
        let view = runner.pending_selection_view().unwrap();
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != digimon_engine::action::space::PASS)
            .unwrap_or(digimon_engine::action::space::PASS);
        runner.execute_action(view.selecting_player, pick).unwrap();
        if runner.game.modifiers.has(wg, ModifierType::MayAttack) {
            break;
        }
    }
    assert!(
        runner.game.modifiers.has(wg, ModifierType::MayAttack),
        "precondition: WarGreymon has the windowed MayAttack grant"
    );

    // The DNA digivolve consumes WarGreymon (simulated by removing it).
    runner.game.delete_permanent_with_effects(wg);

    // The granted attack is orphaned: no attack action for the gone attacker,
    // and security is untouched (the attack fizzled).
    let mask = digimon_engine::action::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_attack(wg.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "a removed attacker must expose no end-of-turn attack action"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before,
        "DNA'ing the attacker away before its deferred attack must leave opponent security untouched (fizzle)"
    );
}
