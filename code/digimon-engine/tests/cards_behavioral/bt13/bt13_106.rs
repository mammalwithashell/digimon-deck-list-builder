//! BT13-106 Odin's Breath
//!
//! Production YAML coverage for:
//! - effect-trash from security activating this card's `[Main]` body without
//!   using a hand copy.
//! - `[Security] Activate this card's [Main] effects.`

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCardKind, CompiledClause, CompiledColor, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::selection::OptionPlayResult;

fn digimon(card_id: &str, name: &str, color: CardColor, dp: i32) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(3);
    card.dp = Some(dp);
    card.play_cost = 3;
    card
}

fn resolve_first_pending(runner: &mut DebugRunner) {
    let (player, action) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection");
        (
            pending.selecting_player,
            pending
                .valid_action_ids
                .first()
                .copied()
                .expect("pending action"),
        )
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("resolve pending selection");
}

fn contains_security_attack_minus_one(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|step| match step {
        CompiledStep::AddModifier {
            modifier, value, ..
        } => modifier == "SecurityAttackChange" && value.literal_value() == Some(-1),
        CompiledStep::If {
            then, else_branch, ..
        } => {
            contains_security_attack_minus_one(then)
                || contains_security_attack_minus_one(else_branch)
        }
        CompiledStep::Optional(body) => contains_security_attack_minus_one(body),
        _ => false,
    })
}

fn runner_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT13-106")
        .expect("BT13-106 YAML loads")
        .add_card(digimon(
            "YELLOW-SRC",
            "Yellow Source",
            CardColor::Yellow,
            3000,
        ))
        .add_card(digimon("P0-A", "P0 A", CardColor::Yellow, 6000))
        .add_card(digimon("P0-B", "P0 B", CardColor::Yellow, 6000))
        .add_card(digimon("P1-A", "P1 A", CardColor::Purple, 6000))
        .add_card(digimon("P1-B", "P1 B", CardColor::Purple, 6000))
        .add_card(make_test_card("SEC-FILLER", "Security Filler"))
}

#[test]
fn bt13_106_has_printed_metadata_and_activation_clauses() {
    let runner = runner_builder().start();
    let card = runner.compiled_card("BT13-106").expect("compiled card");

    assert_eq!(card.name, "Odin's Breath");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] clause");
    assert!(contains_security_attack_minus_one(&main.process));

    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnDiscardSecurity) =>
        {
            matches!(
                triggered.process.as_slice(),
                [CompiledStep::ActivateMainEffects {
                    source: CompiledBindingRef::EventCard
                }]
            )
        }
        _ => false,
    }));

    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnSecurity) =>
        {
            triggered.scope == CompiledScope::Inherited
                && matches!(
                    triggered.process.as_slice(),
                    [CompiledStep::ActivateMainEffects {
                        source: CompiledBindingRef::SelfRef
                    }]
                )
        }
        _ => false,
    }));
}

#[test]
fn bt13_106_main_reduces_one_target_and_debuffs_all_opponent_digimon_when_total_security_lte_6() {
    let mut runner = runner_builder().hand(0, &["BT13-106"]).memory(10).start();
    runner.place_on_field(0, "YELLOW-SRC", None);
    let p1_a = runner.place_on_field(1, "P1-A", None);
    let p1_b = runner.place_on_field(1, "P1-B", None);

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, OptionPlayResult::Pending),
        "Odin's Breath should pause for the -3000 DP target"
    );
    resolve_first_pending(&mut runner);

    assert_eq!(runner.effective_dp(p1_a), Some(3000));
    assert_eq!(runner.effective_dp(p1_b), Some(6000));
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(p1_a, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(p1_b, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(runner.game.players[0].trash.len(), 1);
}

#[test]
fn bt13_106_effect_trash_from_security_activates_main_without_using_hand_copy() {
    let mut runner = runner_builder()
        .hand(1, &["BT13-106"])
        .security(
            0,
            &[
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
            ],
        )
        .security(1, &["BT13-106"])
        .memory(0)
        .start();
    let trasher = runner.place_on_field(0, "YELLOW-SRC", None);
    let p0_a = runner.place_on_field(0, "P0-A", None);
    let p0_b = runner.place_on_field(0, "P0-B", None);
    let source_card = runner.game.players[0].battle_area[trasher.index as usize]
        .top_card()
        .handle();

    runner.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(trasher), 0);
        assert!(ctx.trash_top_security(1));
    }
    runner.game.set_effect_source_player_for_test(None);

    resolve_first_pending(&mut runner);

    assert_eq!(runner.effective_dp(trasher), Some(0));
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(p0_a, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(p0_b, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(
        runner.game.players[1].hand.len(),
        1,
        "security-discard activation must not consume the hand copy"
    );
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.game.players[1].trash.len(), 1);
}

#[test]
fn bt13_106_security_check_activates_main_effects() {
    let mut runner = runner_builder()
        .security(
            0,
            &[
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
                "SEC-FILLER",
            ],
        )
        .security(1, &["BT13-106"])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "P0-A", None);
    let other = runner.place_on_field(0, "P0-B", None);

    let _ = runner.attack_player(attacker, 1, true);
    assert!(
        runner.game.pending_selection.is_some(),
        "[Security] should activate the [Main] selection"
    );
    resolve_first_pending(&mut runner);

    assert_eq!(runner.effective_dp(attacker), Some(3000));
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(attacker, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(other, ModifierType::SecurityAttackChange),
        -1
    );
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.game.players[1].trash.len(), 1);
}

trait ModifierValueExt {
    fn literal_value(&self) -> Option<i32>;
}

impl ModifierValueExt for digimon_dsl::compiled::CompiledModifierValue {
    fn literal_value(&self) -> Option<i32> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Formula(_) => None,
        }
    }
}
