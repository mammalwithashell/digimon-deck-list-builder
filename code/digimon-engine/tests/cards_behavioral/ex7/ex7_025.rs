//! EX7-025 ShoeShoemon.
//! Printed text:
//! - [When Digivolving] if you have 1 or fewer Tamers, you may play 1 Arisa
//!   Kinosaki from your hand without paying cost.
//! - Inherited: [Your Turn] All of your opponent's Security Digimon get -3000 DP.
//!
//! The inherited opponent security Digimon -3000 DP aura lowers via
//! `applies_to_opponent_security_dp: true` (G-OPPONENT-SECURITY-DP-AURA /
//! PUPPETS-G008, resolved Track I 2026-05-17) so the -3000 rides under the
//! attacker's stack and surfaces during the security battle.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn ex7_025_when_digivolving_may_play_arisa_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("ARISA", "Arisa Kinosaki"))
        .hand(0, &["ARISA"])
        .memory(10)
        .start();
    let shoe = runner.place_stack(0, &["BASE", "EX7-025"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(shoe),
    );
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("Arisa hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "play-Arisa selection is optional"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select Arisa");
    runner.auto_resolve().expect("finish play");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_name(&runner.game.card_data) == "Arisa Kinosaki"));
}

/// Structural: EX7-025 must compile an inherited opponent-security DP aura that
/// lowers through `applies_to_opponent_security_dp`.
#[test]
fn ex7_025_inherited_aura_clause_is_opponent_security_dp_minus_3000_inherited() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-025")
        .expect("EX7-025 must be compiled");

    let (scope, dp_modifier, applies_to_opponent_security_dp) = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                applies_to_opponent_security_dp,
                ..
            }) => Some((*scope, *dp_modifier, *applies_to_opponent_security_dp)),
            _ => None,
        })
        .expect("EX7-025 must compile an inherited opponent-security DP aura");

    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the opponent-security DP aura is inherited-only — it rides under the attacker's stack"
    );
    assert_eq!(
        dp_modifier,
        Some(-3000),
        "printed text grants -3000 DP to opponent Security Digimon"
    );
    assert!(
        applies_to_opponent_security_dp,
        "aura must lower via applies_to_opponent_security_dp so it surfaces in the security battle"
    );
}

/// Positive behavioral: EX7-025 carried as an inherited card under an 8000-DP
/// attacker drops a 9000-DP opponent Security Digimon to 6000, so the attacker
/// survives the security battle.
#[test]
fn ex7_025_inherited_security_dp_debuff_lets_8000_attacker_survive_9000_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "EX7-025")
            .expect("EX7-025 registered");
        let next = game.next_card_index();
        let perm = &mut game.players[atk.player as usize].battle_area[atk.index as usize];
        let mut src = CardSource::new(data_idx, atk.player, next);
        src.card_index = next;
        perm.card_sources.insert(0, src);
    }

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "8000 DP attacker survives after ShoeShoemon's -3000 drops 9000 security Digimon to 6000"
    );
    assert_eq!(runner.battle_area_size(0), 1);
}

/// Negative control: without EX7-025 in the attacker's stack the -3000 aura is
/// absent, so the 8000-DP attacker loses to the 9000-DP Security Digimon.
#[test]
fn ex7_025_inherited_security_dp_debuff_without_shoeshoemon_in_stack_loses_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without EX7-025 in stack, 8000 attacker loses to 9000 security Digimon"
    );
    assert_eq!(runner.battle_area_size(0), 0);
}

fn make_tamer(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card
}

fn attacker_lv4(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 5;
    card
}

fn digimon_security_card(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card
}
