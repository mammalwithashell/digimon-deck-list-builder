//! EX7-025 ShoeShoemon — Digimon, Lv.4, Yellow.
//!
//! # Card text (cards.json)
//!
//! Main [When Digivolving]: If you have 1 or fewer Tamers, you may play 1
//! [Arisa Kinosaki] from your hand without paying the cost.
//!
//! Inherited Effect [Your Turn]: All of your opponent's Security Digimon get
//! -3000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Yellow/EX7_025.cs
//!
//! # Patterns this test covers
//! - B1-adjacent: When Digivolving conditional free-play of a named Tamer
//! - E2 optional decline ("you may")
//! - D-aura inherited opponent-security-Digimon DP debuff
//!   (G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008 — resolved 2026-05-17 Track I,
//!    DSL bridge `applies_to_opponent_security_dp: true`)

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Structural assertions ──────────────────────────────────────────

#[test]
fn ex7_025_has_when_digivolving_and_inherited_aura_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-025")
        .expect("EX7-025 in compiled cards");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    let declaratives: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        1,
        "EX7-025 has exactly one triggered clause (When Digivolving)"
    );
    assert_eq!(
        declaratives.len(),
        1,
        "EX7-025 has exactly one declarative clause (inherited security-DP aura)"
    );
}

#[test]
fn ex7_025_when_digivolving_clause_is_optional_main_scope() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-025")
        .expect("EX7-025 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("EX7-025 has a triggered clause");

    assert_eq!(triggered.scope, CompiledScope::FaceUp);
    assert_eq!(triggered.when, vec![CompiledTiming::WhenDigivolving]);
    assert!(
        triggered.optional,
        "the play-Arisa clause is optional (\"you may\")"
    );
    assert!(
        triggered.condition.is_some(),
        "clause is gated by the 1-or-fewer-Tamers condition"
    );
}

#[test]
fn ex7_025_inherited_aura_targets_opponent_security_dp_on_your_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-025")
        .expect("EX7-025 in compiled cards");

    let aura = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(d @ CompiledDeclarativeClause::Aura { .. }) => Some(d),
            _ => None,
        })
        .expect("EX7-025 has an Aura declarative clause");

    match aura {
        CompiledDeclarativeClause::Aura {
            scope,
            dp_modifier,
            applies_to_opponent_security_dp,
            active_when,
            ..
        } => {
            assert_eq!(
                *scope,
                CompiledScope::Inherited,
                "security-DP aura is an inherited effect"
            );
            assert_eq!(
                *dp_modifier,
                Some(-3000),
                "aura drops opponent security Digimon by 3000 DP"
            );
            assert!(
                *applies_to_opponent_security_dp,
                "aura must bridge to applies_to_opponent_security_dp"
            );
            assert!(
                active_when.is_some(),
                "aura is gated to your turn (active_when: your_turn)"
            );
        }
        _ => unreachable!("matched Aura above"),
    }
}

// ─── When Digivolving — behavioral ──────────────────────────────────

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

#[test]
fn ex7_025_when_digivolving_blocked_with_two_or_more_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("ARISA", "Arisa Kinosaki"))
        .add_card(make_tamer("TAMER-A", "Tamer A"))
        .add_card(make_tamer("TAMER-B", "Tamer B"))
        .hand(0, &["ARISA"])
        .memory(10)
        .start();
    runner.place_on_field(0, "TAMER-A", Some(0));
    runner.place_on_field(0, "TAMER-B", Some(0));
    let shoe = runner.place_stack(0, &["BASE", "EX7-025"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(shoe),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "with 2 Tamers on field the When Digivolving clause is gated off"
    );
}

// ─── Inherited [Your Turn] opponent-security-DP aura — behavioral ───
//
// The aura rides under the attacker's stack and surfaces as a -3000 swing
// against the opponent's Security Digimon during the security battle.
// Mirrors the ST19-03 sibling coverage in `st19/st19_03.rs`.

#[test]
fn ex7_025_inherited_security_dp_aura_lets_8000_attacker_survive_10000_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 10000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    insert_inherited_ex7_025(&mut runner, atk);

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "8000 DP attacker survives after ShoeShoemon's -3000 drops the 10000 Security Digimon to 7000"
    );
    assert_eq!(runner.battle_area_size(0), 1);
}

#[test]
fn ex7_025_inherited_security_dp_aura_without_carrier_in_stack_loses_battle() {
    // Negative: no EX7-025 inherited carrier on the field → no -3000 swing.
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-025")
        .expect("EX7-025 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 10000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without EX7-025 in the stack the 8000 attacker loses to the 10000 Security Digimon"
    );
    assert_eq!(runner.battle_area_size(0), 0);
}

// ─── Helpers ────────────────────────────────────────────────────────

fn make_tamer(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = digimon_engine::enums::CardKind::Tamer;
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

/// Slot an EX7-025 card source underneath the attacker stack so its inherited
/// `[Your Turn]` security-DP aura is in scope for the security battle. Mirrors
/// the ST19-03 sibling test's stack-insertion idiom.
fn insert_inherited_ex7_025(runner: &mut DebugRunner, atk: digimon_engine::permanent::PermanentHandle) {
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
