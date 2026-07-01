//! BT4-098 Atomic Inferno — Option, Red, Cost 1.
//!
//! Printed text (card face, authoritative):
//!   [Main] 1 of your Digimon with [Hybrid] in its form gets +3000 DP,
//!     <Security Attack +1> (This Digimon checks 1 additional security card),
//!     and "[Your Turn] When this Digimon is blocked, gain +3 memory" for the
//!     turn.
//!   [Security] All of your Digimon gain <Security Attack +1> (This Digimon
//!     checks 1 additional security card) until the end of your next turn.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT4/Red/BT4_098.cs
//!   - OptionSkill: SelectPermanent over the OWNER's battle-area Digimon whose
//!     TopCard.CardTraits contains "Hybrid" (maxCount min(1,..), canNoSelect:
//!     false → MANDATORY when ≥1 valid target). On the pick: ChangeDigimonDP
//!     +3000 (UntilEachTurnEnd), ChangeDigimonSAttack +1 (UntilEachTurnEnd),
//!     then AddEffectToPermanent an OnBlockAnyone ActivateClass whose
//!     CanUseCondition checks IsOwnerTurn + CanTriggerOnAttack(selectedPermanent)
//!     — i.e. the granted "gain +3 memory" fires only when the SELECTED Digimon
//!     is the one blocked, on the owner's turn. Body: Owner.AddMemory(3).
//!   - SecuritySkill: ChangeDigimonSAttackPlayerEffect +1 over all of owner's
//!     battle-area Digimon, UntilOwnerTurnEnd (= end of your next turn).

use digimon_dsl::compiled::{CompiledClause, CompiledModifierValue, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const ATOMIC_INFERNO_YAML: &str = include_str!("../../../cards/bt4/BT4-098.yaml");
const CARD_ID: &str = "BT4-098";

/// A synthetic own Digimon whose FORM trait set includes "Hybrid" (legal target).
fn hybrid_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Hybrid".to_string()];
    c
}

/// A synthetic own Digimon WITHOUT the Hybrid trait (illegal target).
fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Warrior".to_string()];
    c
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(ATOMIC_INFERNO_YAML)
        .expect("BT4-098 YAML parses and compiles")
        .add_card(hybrid_digimon("HYBRID-A"))
        .add_card(hybrid_digimon("HYBRID-B"))
        .add_card(plain_digimon("PLAIN"))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start()
}

// ============================================================================
// Section 1 — Structural assertions
// ============================================================================

#[test]
fn bt4_098_has_main_and_security_clauses() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("BT4-098 compiles");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::MainFromHand)),
        "BT4-098 must carry a [Main] (main_from_hand) clause"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "BT4-098 must carry a [Security] (on_security) clause"
    );
}

#[test]
fn bt4_098_main_clause_grants_dp_sattack_and_block_trigger() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("BT4-098 compiles");

    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("main_from_hand clause present");

    // The [Main] clause must add a ChangeDp modifier (+3000) and a
    // SecurityAttackChange modifier (+1), and grant an OnBlock triggered effect.
    let has_change_dp = main.process.iter().any(|s| {
        matches!(s, CompiledStep::AddModifier { modifier, value, .. }
            if modifier == "ChangeDp" && *value == CompiledModifierValue::Literal(3000))
    });
    let has_satk = main.process.iter().any(|s| {
        matches!(s, CompiledStep::AddModifier { modifier, value, .. }
            if modifier == "SecurityAttackChange" && *value == CompiledModifierValue::Literal(1))
    });
    let has_block_grant = main.process.iter().any(
        |s| matches!(s, CompiledStep::GrantTriggeredEffect { timing, .. } if timing == "on_block"),
    );

    assert!(has_change_dp, "[Main] must add ChangeDp +3000");
    assert!(has_satk, "[Main] must add SecurityAttackChange +1");
    assert!(
        has_block_grant,
        "[Main] must grant an on_block triggered effect ('gain +3 memory')"
    );

    // It must also pick exactly 1 of your own Digimon (no auto-select).
    let has_select = main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectOwnPermanent { .. }));
    assert!(
        has_select,
        "[Main] must surface a select_own_permanent (no auto-select)"
    );
}

// ============================================================================
// Section 2 — Behavioral: [Main] target selection + grants
// ============================================================================

#[test]
fn bt4_098_main_requires_selection_among_only_hybrid_digimon() {
    let mut runner = base_runner();
    // One Hybrid (legal) and one non-Hybrid (illegal) own Digimon.
    runner.place_on_field(0, "HYBRID-A", Some(0));
    runner.place_on_field(0, "PLAIN", Some(0));

    runner.play(0, 0); // play Atomic Inferno from hand (index 0)

    let pending = runner
        .pending_selection()
        .expect("[Main] must surface a mandatory own-Digimon selection");
    assert_eq!(pending.kind, SelectionKind::OwnField);
    assert!(
        !pending.is_optional,
        "selection is MANDATORY when a Hybrid Digimon exists (canNoSelect: false)"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only the Hybrid Digimon is a legal target; the non-Hybrid is excluded"
    );
}

#[test]
fn bt4_098_main_grants_dp_and_security_attack_to_selected_hybrid() {
    let mut runner = base_runner();
    let target = runner.place_on_field(0, "HYBRID-A", Some(0));

    let dp_before = runner.effective_dp(target).expect("target has DP");

    runner.play(0, 0);
    let action = runner
        .pending_selection()
        .expect("selection installed")
        .valid_action_ids
        .first()
        .copied()
        .expect("a legal Hybrid target");
    runner.execute_action(0, action).expect("select the Hybrid");
    runner.auto_resolve().expect("resolve grants");

    // +3000 DP modifier present and reflected in effective DP.
    assert_eq!(
        runner.game.modifiers.sum(target, ModifierType::ChangeDp),
        3000,
        "selected Hybrid Digimon must gain +3000 DP"
    );
    assert_eq!(
        runner.effective_dp(target).expect("target DP"),
        dp_before + 3000,
        "effective DP reflects the +3000 buff"
    );
    // <Security Attack +1>.
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(target, ModifierType::SecurityAttackChange),
        1,
        "selected Hybrid Digimon must gain Security Attack +1"
    );
    assert!(
        runner.game.has_security_attack_change(target),
        "the temporary SecurityAttackChange satisfies the Security A. predicate"
    );
}

#[test]
fn bt4_098_main_no_hybrid_target_installs_nothing() {
    let mut runner = base_runner();
    // Only a non-Hybrid own Digimon on the field.
    runner.place_on_field(0, "PLAIN", Some(0));

    runner.play(0, 0);

    assert!(
        runner.pending_selection().is_none(),
        "no selection should install when there is no Hybrid Digimon to target"
    );
}

// ============================================================================
// Section 3 — Behavioral: granted "[Your Turn] when blocked, gain +3 memory"
// ============================================================================

/// Positive case: after the grant is installed, when the SELECTED Hybrid
/// Digimon is the one blocked, the controller gains 3 memory. We drive the
/// OnBlock event directly via the BlockDeclared trigger source (mirroring how
/// sibling tests enqueue combat timings) with the granted Digimon as the
/// blocked attacker.
#[test]
fn bt4_098_blocked_selected_hybrid_gains_three_memory() {
    // Start at low memory so the +3 gain is not clamped at the gauge ceiling.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ATOMIC_INFERNO_YAML)
        .expect("BT4-098 YAML parses and compiles")
        .add_card(hybrid_digimon("HYBRID-A"))
        .add_card(hybrid_digimon("HYBRID-B"))
        .add_card(plain_digimon("PLAIN"))
        .hand(0, &[CARD_ID])
        .memory(1)
        .start();
    let target = runner.place_on_field(0, "HYBRID-A", Some(0));
    // A blocker on the opponent side (the host permanent of the block).
    let blocker = runner.place_on_field(1, "HYBRID-B", Some(0));

    // Install the [Main] grant on the Hybrid Digimon.
    runner.play(0, 0);
    let action = runner
        .pending_selection()
        .expect("selection installed")
        .valid_action_ids
        .first()
        .copied()
        .expect("a legal Hybrid target");
    runner.execute_action(0, action).expect("select the Hybrid");
    runner.auto_resolve().expect("resolve grants");

    let mem_before = runner.memory();

    // The selected Hybrid (P0) is the attacker that gets blocked by P1's Digimon.
    let top_card = runner.top_card(target);
    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnBlock,
        TriggerSource::BlockDeclared {
            attacker: target,
            blocker,
            card: top_card,
        },
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    // P0 (turn player) gaining 3 memory moves the shared gauge 3 steps toward P0
    // (i.e. game.memory increases by 3 for the turn player).
    assert_eq!(
        runner.memory() - mem_before,
        3,
        "blocking the granted Hybrid Digimon must gain its controller +3 memory"
    );
}

// ============================================================================
// Section 4 — Behavioral: [Security] all your Digimon gain Security Attack +1
// ============================================================================

/// [Security] structural shape: the on_security clause grants SecurityAttackChange
/// +1 to your battle-area Digimon, expiring at end_of_your_next_turn ("until the
/// end of your next turn"). Driving an Option's [Security] effect end-to-end needs
/// the full security-check flow (the option is not placed as a field source), so
/// like EX1-068 the [Security] clause is asserted structurally here.
#[test]
fn bt4_098_security_clause_grants_security_attack_until_your_next_turn() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("BT4-098 compiles");

    let sec = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("on_security clause present");

    assert!(
        !sec.optional,
        "[Security] effects are mandatory (not optional)"
    );

    let satk = sec
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::AddModifier {
                modifier,
                value,
                expiry,
                ..
            } if modifier == "SecurityAttackChange" => Some((value.clone(), expiry.clone())),
            _ => None,
        })
        .expect("[Security] must add a SecurityAttackChange modifier");

    assert_eq!(
        satk.0,
        CompiledModifierValue::Literal(1),
        "[Security] grants Security Attack +1"
    );
    assert_eq!(
        satk.1, "end_of_your_next_turn",
        "[Security] grant lasts until the end of your next turn"
    );
}
