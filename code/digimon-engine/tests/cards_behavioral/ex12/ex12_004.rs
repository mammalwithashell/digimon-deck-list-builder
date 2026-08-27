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

// ── D1 exam triage: the granted-<Execute> ATTACK, end to end ────────────────
//
// `ex12_004_granted_execute_fires_at_end_of_your_turn` (above) stops at the
// park. Nothing downstream of it was pinned for the GRANTED path: whether the
// widened defender condition actually reaches the mask, and whether 16-37-1's
// "At the end of the attack, this Digimon is deleted" fires when the grant —
// not a printed keyword — is what put the Digimon in that window.
//
// The DCGO exam does not close that gap either. Its two accepted-gate lines
// (qa/dcgo-exams/EX12/EX12-004-{effect2,inherited0}.yaml) attack a 2000 DP
// Digimon with a 2000 DP carrier, so the attacker dies in the BATTLE and the
// self-delete is confounded — both engines end with the same trash either way.
// Here the carrier WINS its battle (5000 vs 2000), so surviving combat and
// still ending in the trash is attributable to <Execute> alone.
//
// Rules: 16-37-1 (may attack at end of your turn; also allows attacking an
// opponent's UNSUSPENDED Digimon; at the end of that attack this Digimon is
// deleted), 16-37-3 (optional). DCGO: `CardEffectCommons/KeyWordEffects/
// Execute.cs` — `ExecuteProcess` adds the `!defender.IsSuspended` defender
// condition, runs `SelectAttackEffect`, then queues `DeleteSelfEffect` on
// `EffectTiming.OnEndAttack`; `CardEffectFactory.ExecuteEffect` is what
// `GainExecute` installs for a GRANTED Execute, i.e. DCGO runs the identical
// process for granted and printed.

fn tb_carrier(id: &str, dp: i32) -> CardData {
    let mut c = purple_digimon(id, &["TB"]);
    c.dp = Some(dp);
    c
}

#[test]
fn ex12_004_granted_execute_attacks_unsuspended_then_self_deletes() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-004 YAML loads")
        .add_card(tb_carrier("TB-CARRIER", 5000))
        .add_card({
            // Lower DP than the carrier: the carrier WINS the battle, so any
            // deletion of it is <Execute>'s, not combat's.
            let mut c = purple_digimon("DEF", &[]);
            c.dp = Some(2000);
            c
        })
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "TB-CARRIER"]);
    let defender = runner.place_on_field(1, "DEF", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.players[1].battle_area[defender.index as usize].is_suspended,
        "precondition: the defender is UNSUSPENDED — only 16-37-1's widened \
         target set makes it attackable at all"
    );
    assert!(
        runner.game.has_keyword(carrier, Keyword::Execute),
        "precondition: the [TB] carrier holds the granted <Execute>"
    );

    // Negative control, so the mask assertion below cannot pass vacuously: in
    // the Main phase the very same (attacker, target) pair is ILLEGAL, because
    // an unsuspended Digimon is not an ordinary attack target.
    let bit = digimon_engine::action::space::encode_attack(
        carrier.index as u16,
        defender.index as u16,
    ) as usize;
    assert_eq!(runner.game.current_phase, GamePhase::Main);
    assert_eq!(
        digimon_engine::build_action_mask(&runner.game, 0)[bit],
        0.0,
        "control: without the end-of-turn grant this attack bit must be closed"
    );

    runner.game.end_turn();
    assert_eq!(
        runner.game.current_phase,
        GamePhase::EndOfTurnAction,
        "precondition: the granted trigger parks the optional gate"
    );

    // The widened defender condition must reach the ACTION SPACE, not just the
    // modifier table — an unsuspended defender is otherwise not a legal target.
    let mask = digimon_engine::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[bit], 1.0,
        "granted <Execute> must offer the attack bit against the UNSUSPENDED \
         defender (16-37-1 'also allows for attacking an opponent's \
         unsuspended Digimon')"
    );

    runner.attack_digimon(carrier, defender, false);

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "the defender loses the battle (5000 > 2000)"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "16-37-1: at the end of THAT attack the <Execute> Digimon is deleted — \
         even though it won its battle"
    );
    let trash: Vec<String> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        trash.iter().any(|id| id == "TB-CARRIER"),
        "the carrier's top card must land in the trash, got {trash:?}"
    );
    assert!(
        trash.iter().any(|id| id == CARD_ID),
        "the whole stack goes with it — EX12-004 was a digivolution source, \
         got {trash:?}"
    );
}

/// The other half of 16-37-3: this is the path 8 of the 10 exam scenarios
/// actually take (they DECLINE the granted gate to reach some other clause).
/// PASS at the park must leave the carrier untouched and expire the grant's
/// end-of-turn modifiers.
#[test]
fn ex12_004_granted_execute_declined_leaves_the_carrier_alive() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-004 YAML loads")
        .add_card(tb_carrier("TB-CARRIER", 5000))
        .add_card({
            let mut c = purple_digimon("DEF", &[]);
            c.dp = Some(2000);
            c
        })
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "TB-CARRIER"]);
    let _defender = runner.place_on_field(1, "DEF", Some(0));
    runner.game.tick_declarative_effects();

    runner.game.end_turn();
    assert_eq!(runner.game.current_phase, GamePhase::EndOfTurnAction);

    runner.game.pass_end_of_turn_action();

    assert_ne!(
        runner.game.current_phase,
        GamePhase::EndOfTurnAction,
        "PASS must rotate the turn out of the end-of-turn park"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "declining the granted <Execute> must not delete the carrier"
    );
    assert!(
        !runner.game.modifiers.has(carrier, ModifierType::MayAttack),
        "the grant's MayAttack must expire on rotation"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(carrier, ModifierType::CanAttackUnsuspended),
        "the grant's CanAttackUnsuspended must expire on rotation"
    );
    assert!(
        runner.game.players[0].trash.is_empty(),
        "nothing was deleted: no attack ever initiated"
    );
}
