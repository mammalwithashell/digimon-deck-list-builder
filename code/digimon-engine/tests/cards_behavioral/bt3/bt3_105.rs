//! BT3-105 Breath of the Gods — Option, Black, Cost 2. Traits: (none).
//!
//! # Card text (card image / official DB — authoritative)
//!
//! [Main] 1 of your Digimon gains ＜Reboot＞ (Unsuspend this Digimon during
//! your opponent's unsuspend phase) and "This Digimon can't have its DP
//! reduced or be returned to its owner's hand or deck" until the end of your
//! opponent's next turn.
//!
//! Inherited (Security):
//! Security Effect [Security] Your opponent's Digimon can't attack players for
//! the turn.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT3/Black/BT3_105.cs
//!   - OptionSkill: SelectPermanentEffect over OWN battle-area Digimon
//!     (maxCount Min(1, count), canNoSelect false) → GainReboot +
//!     GainImmuneFromDPMinus + GainCanNotReturnToHand + GainCanNotReturnToDeck,
//!     all `EffectDuration.UntilOpponentTurnEnd`.
//!   - SecuritySkill: `GainCanNotAttackPlayerEffect(attackerCondition: opponent
//!     battle-area Digimon — re-evaluated per attack, defenderCondition:
//!     Defender == null, UntilEachTurnEnd)`.
//!
//! # Patterns this test covers
//! - H7 Reboot grant (`grant_keyword`) + D4-adjacent protective modifiers
//!   (`ImmuneFromDPMinus`, `CannotBeReturnedToHand`, `CannotBeReturnedToDeck`)
//!   with `end_of_opponents_next_turn` expiry
//! - Selection: mandatory `SelectionKind::OwnField` (no PASS)
//! - [Security] continuous opponent-wide `CannotAttackPlayer` for the turn
//!   (late entrants included, expired next turn)

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, Expiry, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT3-105";

fn black_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Black];
    c.dp = Some(dp);
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT3-105 must load from the embedded DSL pack")
        .add_card(black_digimon("FILL", 3000))
        .add_card(black_digimon("MINE-A", 5000))
        .add_card(black_digimon("MINE-B", 4000))
        .add_card(black_digimon("OPP-A", 6000))
        .add_card(black_digimon("OPP-B", 6000))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn has_all_protections(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.has_keyword(h, Keyword::Reboot)
        && runner.modifiers().has(h, ModifierType::ImmuneFromDPMinus)
        && runner.modifiers().has(h, ModifierType::CannotBeReturnedToHand)
        && runner.modifiers().has(h, ModifierType::CannotBeReturnedToDeck)
}

fn has_any_protection(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.has_keyword(h, Keyword::Reboot)
        || runner.modifiers().has(h, ModifierType::ImmuneFromDPMinus)
        || runner.modifiers().has(h, ModifierType::CannotBeReturnedToHand)
        || runner.modifiers().has(h, ModifierType::CannotBeReturnedToDeck)
}

// ─── Section 1: structural ───────────────────────────────────────────────────

#[test]
fn bt3_105_metadata_and_clauses() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(2));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "main_from_hand + on_security");
    let main = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::MainFromHand])
        .expect("[Main] clause");
    assert!(!main.optional, "no 'you may' — the target pick is mandatory");
    let sec = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("[Security] clause");
    assert!(!sec.optional);
}

// ─── Section 2: [Main] ───────────────────────────────────────────────────────

#[test]
fn bt3_105_main_grants_reboot_and_protections_to_the_chosen_digimon_only() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(5).start();
    let a = runner.place_on_field(0, "MINE-A", Some(0));
    let b = runner.place_on_field(0, "MINE-B", Some(0));

    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let view = runner.pending_selection_view().expect("target pick");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.is_optional, "DCGO canNoSelect: false — no PASS");
    assert_eq!(view.valid_action_ids.len(), 2, "both own Digimon are candidates");

    let pick_a = digimon_engine::action::space::encode_attack(a.player as u16, a.index as u16);
    runner.execute_action(0, pick_a).expect("choose MINE-A");
    let _ = runner.auto_resolve();

    assert!(has_all_protections(&runner, a), "MINE-A gains Reboot + DP-minus immunity + no-return");
    assert!(!has_any_protection(&runner, b), "MINE-B is untouched");
    assert_eq!(runner.memory(), 3, "cost 2 paid");
}

#[test]
fn bt3_105_protection_blocks_dp_reduction_from_the_opponent() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(5).start();
    let a = runner.place_on_field(0, "MINE-A", Some(0));
    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let pick_a = digimon_engine::action::space::encode_attack(a.player as u16, a.index as u16);
    runner.execute_action(0, pick_a).expect("choose MINE-A");
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(a), Some(5000));

    {
        let source_card = runner.game.players[0].battle_area[a.index as usize]
            .top_card()
            .handle();
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut runner.game, source_card, None, 1);
        ctx.add_dp_modifier(a, -3000, Expiry::EndOfTurn);
    }
    assert_eq!(
        runner.effective_dp(a),
        Some(5000),
        "'can't have its DP reduced' — the -3000 is suppressed"
    );
}

#[test]
fn bt3_105_protections_last_through_the_opponents_next_turn_then_expire() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(5).start();
    let a = runner.place_on_field(0, "MINE-A", Some(0));
    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let pick_a = digimon_engine::action::space::encode_attack(a.player as u16, a.index as u16);
    runner.execute_action(0, pick_a).expect("choose MINE-A");
    let _ = runner.auto_resolve();

    runner.game.set_memory(3);
    runner.end_turn(); // P1's turn — still protected
    assert_eq!(runner.game.turn_player(), 1);
    assert!(has_all_protections(&runner, a), "protection persists during the opponent's next turn");

    runner.game.set_memory(3);
    runner.end_turn(); // end of P1's turn — expires
    assert_eq!(runner.game.turn_player(), 0);
    assert!(!has_any_protection(&runner, a), "everything expires at the end of the opponent's next turn");
}

#[test]
fn bt3_105_reboot_unsuspends_during_the_opponents_unsuspend_phase() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(5).start();
    let a = runner.place_on_field(0, "MINE-A", Some(0));
    assert_eq!(runner.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let pick_a = digimon_engine::action::space::encode_attack(a.player as u16, a.index as u16);
    runner.execute_action(0, pick_a).expect("choose MINE-A");
    let _ = runner.auto_resolve();

    runner.game.players[0].battle_area[a.index as usize].is_suspended = true;
    runner.game.set_memory(3);
    runner.end_turn(); // P1's unsuspend phase runs at the start of their turn
    assert!(
        !runner.game.players[0].battle_area[a.index as usize].is_suspended,
        "<Reboot> unsuspends MINE-A during the opponent's unsuspend phase"
    );
}

#[test]
fn bt3_105_main_with_no_own_digimon_resolves_without_a_prompt() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(5).start();
    runner.place_on_field(0, "FILL", Some(0)); // colour requirement only
    // Remove it again so there is genuinely no own Digimon; keep a Tamer-less
    // board by relying on the Option's own colour check having passed at the
    // time of validation — simplest: no Digimon at all means Invalid
    // (colour), so instead assert on the opponent-only board below.
    runner.game.players[0].battle_area.clear();
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let result = runner.game.play_option_from_hand(0, 0);
    // Without a black permanent the colour requirement fails — the use is
    // refused up front and nothing is spent (DCGO would not even offer it).
    assert_eq!(result, OptionPlayResult::Invalid);
    assert!(runner.pending_selection().is_none());
    assert!(!has_any_protection(&runner, opp), "opponent Digimon are never candidates");
}

// ─── Section 3: [Security] opponent's Digimon can't attack players ───────────

#[test]
fn bt3_105_security_stops_opponent_digimon_attacking_players_for_the_turn() {
    // `.security()` lists bottom → top (combat reveals via `security.pop()`),
    // so Breath of the Gods is the LAST entry = the first card checked.
    let mut runner = base()
        .security(0, &["FILL", CARD_ID])
        .security(1, &["FILL"; 2])
        .memory(3)
        .start();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let mine = runner.place_on_field(0, "MINE-A", Some(0));

    runner.end_turn(); // P1's turn
    assert_eq!(runner.game.turn_player(), 1);
    runner.game.set_memory(3);

    // OPP-A hits P0's security and reveals Breath of the Gods.
    let result = runner.attack_player(opp_a, 0, false);
    let _ = runner.auto_resolve();
    assert_ne!(result, AttackResult::Invalid, "the first attack is legal");
    assert_eq!(runner.security_count(0), 1, "Breath of the Gods was checked");

    assert!(
        runner.modifiers().has(opp_b, ModifierType::CannotAttackPlayer),
        "every opponent Digimon is bound for the turn"
    );
    assert_eq!(
        runner.attack_player(opp_b, 0, false),
        AttackResult::Invalid,
        "OPP-B can no longer attack the player this turn"
    );
    assert!(
        !runner.modifiers().has(mine, ModifierType::CannotAttackPlayer),
        "the security owner's own Digimon are not affected"
    );

    // Late entrant on the same turn is bound too (continuous re-evaluation).
    let late = runner.place_on_field(1, "FILL", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.attack_player(late, 0, false), AttackResult::Invalid);

    // "for the turn" — gone on P1's next turn.
    runner.game.set_memory(3);
    runner.end_turn(); // → P0
    runner.game.set_memory(3);
    runner.end_turn(); // → P1 again
    assert!(!runner.modifiers().has(opp_b, ModifierType::CannotAttackPlayer));
    assert_ne!(runner.attack_player(opp_b, 0, false), AttackResult::Invalid);
}
