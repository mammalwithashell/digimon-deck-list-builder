use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword, ModifierType};

const CARD_ID: &str = "EX12-004";

fn purple_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.play_cost = 3;
    card.dp = Some(3000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

#[test]
fn ex12_004_inherited_grants_execute_to_tb_carrier_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-004 YAML loads")
        .add_card(purple_digimon("TB-CARRIER", &["TB"]))
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "TB-CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(carrier, Keyword::Execute),
        "TB carrier should inherit Execute from EX12-004"
    );
}

/// TDD RED — task_69f10a66 ruling (a): a DECLARATIVELY granted `<Execute>`
/// must actually fire its end-of-your-turn trigger and surface the printed
/// "may attack" choice, exactly like printed Execute does
/// (`keyword_phase_f/execute.rs`): `end_turn` parks `GamePhase::EndOfTurnAction`
/// with `MayAttack` + `CanAttackUnsuspended` granted on the carrier, so the
/// §4.6 mask offers the attack and PASS declines.
///
/// Rules: 16-37-2 (<Execute> is a trigger-type effect that triggers at the
/// end of your turn) + 16-37-3 ("The processing from <Execute> is optional")
/// + 15-9-2-2 (the player CAN choose to execute optional processing — which
/// requires the choice to be surfaced at all). DCGO: `EX12_004.cs` registers
/// `ExecuteSelfEffect` at `OnEndTurn` gated on owner-turn + top-card TB;
/// `CardEffectFactory.ExecuteEffect` passes `isOptional: true`, so DCGO asks.
///
/// Today this FAILS: the grant is only a keyword-registry marker.
/// `build_effects_for_card` skips auto-effect synthesis for declarative
/// grants with a condition (`grant.condition.is_some()` → skip, game/mod.rs),
/// and `grant_declarative_keyword` (unlike runtime `grant_keyword`) never
/// routes through `grant_keyword_triggered_auto_effects` — so NO
/// `EndOfYourTurn` body exists anywhere, no `MayAttack` is granted, and
/// `end_turn` rotates without parking. The choice never reaches the action
/// space (rule-17 violation). Exam witness: qa/dcgo-exams/EX12/
/// EX12-061-inherited0.yaml desyncs at exactly this boundary (DCGO asks the
/// OptionalSkill gate after the T3 main pass; our line surfaces nothing).
#[test]
fn ex12_004_granted_execute_fires_at_end_of_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-004 YAML loads")
        .add_card({
            let mut c = purple_digimon("TB-CARRIER", &["TB"]);
            c.dp = Some(5000);
            c
        })
        .memory(5)
        .start();

    // EX12-004 under a [TB] top card — the aura grants <Execute> on your turn.
    let carrier = runner.place_stack(0, &[CARD_ID, "TB-CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(carrier, Keyword::Execute),
        "precondition: the declarative grant is visible to has_keyword"
    );
    assert_eq!(runner.game.turn_player(), 0, "carrier controller's turn");
    assert!(
        runner.game.can_attack(carrier, false),
        "precondition: carrier is attack-legal (not summoning-sick)"
    );

    // End the controller's turn: 16-37-2 says the trigger fires NOW, and the
    // optional processing (16-37-3) must be the player's choice — surfaced
    // here as the EndOfTurnAction park (attack via the mask, or PASS).
    runner.game.end_turn();

    assert!(
        runner.game.modifiers.has(carrier, ModifierType::MayAttack),
        "granted <Execute> must grant MayAttack at end of your turn \
         (the EndOfYourTurn body never fired — declarative grant is inert)"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(carrier, ModifierType::CanAttackUnsuspended),
        "granted <Execute> must also widen targeting to unsuspended Digimon"
    );
    assert_eq!(
        runner.game.current_phase,
        GamePhase::EndOfTurnAction,
        "end_turn must park so the player can choose the Execute attack or PASS \
         (rule 16-37-3 optional processing; no-approximations rule 17)"
    );
}

#[test]
fn ex12_004_inherited_does_not_grant_execute_to_non_tb_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-004 YAML loads")
        .add_card(purple_digimon("PLAIN-CARRIER", &[]))
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "PLAIN-CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(carrier, Keyword::Execute),
        "non-TB carrier should not inherit Execute from EX12-004"
    );
}
