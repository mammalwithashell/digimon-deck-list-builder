//! ST20-12 Sora Takenouchi & Kari Kamiya — Tamer, Red/Yellow, Cost 3, traits: [ADVENTURE].
//!
//! # Card text (official Bandai DB, `data/card_bundles/ST20-12.md`)
//!
//! **[Start of Your Main Phase]** If you have a Digimon with the [ADVENTURE]
//! trait, gain 1 memory.
//!
//! **[Your Turn]** When any Digimon cards with the [ADVENTURE] trait would be
//! played from your hand, by suspending this Tamer, reduce the play cost by 1.
//!
//! **Security Effect:** [Security] Play this card without paying the cost.
//!
//! Official Q&A: "Yes, the play cost will be reduced by a total of 2" when two
//! such Tamers both reduce the same play.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_12.cs`
//! - `OnStartMainPhase` ActivateClass (mandatory): on battle area + owner's turn;
//!   activation needs `CanAddMemory` and an owner Digimon whose TopCard
//!   `HasAdventureTraits`; `AddMemory(1)`.
//! - `BeforePayCost` ActivateClass (optional): see ST20-13 / ST21-13; suspends
//!   this Tamer, then a -1 `ChangeCostClass`.
//! - `SecuritySkill`: `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md section 4.3)
//! - Start-of-main-phase conditional memory gain (AD1-019 idiom): positive +
//!   negative per condition (no Digimon / non-ADVENTURE / Tamer-only / opponent's).
//! - [Your Turn] `kind: cost_reduction` with `pay_cost: [suspend self]` (sibling
//!   ST21-13): positive, negative per predicate, decline, already-suspended,
//!   event-log cost proof, and the official two-Tamer stacking ruling.
//! - `[Security] play self free` through the real security-check path.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::events::GameEvent;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST20-12";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, name: &str, level: u8, cost: u16, adventure: bool) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Yellow];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn tamer(id: &str, cost: u16, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Yellow];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-12 in embedded DSL pack")
        .add_card(filler("PAD"))
        .add_card(digimon("ADV-DIGI", "Adv Digimon", 4, 5, true))
        .add_card(digimon("PLAIN-DIGI", "Plain Digimon", 4, 5, false))
        .add_card(digimon("OPP-ADV", "Opp Adventure", 4, 4, true))
        .add_card(tamer("ADV-TAMER", 3, true))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

fn drive_accept(runner: &mut DebugRunner, player: u8) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(player, accept)
            .expect("execute pending selection");
    }
}

fn is_suspended(runner: &DebugRunner, player: u8, index: u8) -> bool {
    runner.game.players[player as usize].battle_area[index as usize].is_suspended
}

/// Run the start-of-main-phase window from memory 0 and return the memory.
fn start_main_memory(runner: &mut DebugRunner) -> i16 {
    runner.game.memory = 0;
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();
    runner.memory()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st20_12_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Sora Takenouchi & Kari Kamiya");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3), "ST20-12 prints Cost 3");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "ST20-12 carries the ADVENTURE trait, got {:?}",
        card.traits
    );
    assert!(card.color.contains(&CompiledColor::Red));
    assert!(card.color.contains(&CompiledColor::Yellow));
    assert_eq!(card.color.len(), 2, "Red/Yellow only");
}

#[test]
fn st20_12_has_start_main_cost_reduction_and_security_clauses() {
    let card = compiled(CARD_ID);
    let start_main = card
        .effects
        .iter()
        .filter(|c| {
            matches!(c, CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::StartOfYourMainPhase])
        })
        .count();
    let cost_reductions = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    let security = card
        .effects
        .iter()
        .filter(|c| {
            matches!(c, CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::OnSecurity])
        })
        .count();
    assert_eq!(start_main, 1, "one start_of_your_main_phase clause");
    assert_eq!(cost_reductions, 1, "one cost_reduction clause");
    assert_eq!(security, 1, "one on_security clause");
    assert_eq!(card.effects.len(), 3, "no clauses beyond the printed three");
}

#[test]
fn st20_12_start_main_clause_is_mandatory_and_not_once_per_turn() {
    let card = compiled(CARD_ID);
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::StartOfYourMainPhase] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("start-of-main clause");
    assert_eq!(t.scope, CompiledScope::FaceUp);
    assert!(
        !t.optional,
        "DCGO isOptional false: the memory gain is mandatory"
    );
    assert!(!t.once_per_turn, "no printed [Once Per Turn]");
    assert!(
        t.condition.is_some(),
        "'If you have a Digimon with [ADVENTURE]' gate"
    );
}

#[test]
fn st20_12_cost_reduction_is_optional_your_turn_amount_1_with_a_suspend_cost() {
    let card = compiled(CARD_ID);
    let (scope, active_when, optional, once_per_turn, amount, when_any, pay_cost) = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope,
                active_when,
                optional,
                once_per_turn,
                amount,
                when_any_ally_played,
                pay_cost,
                ..
            }) => Some((
                *scope,
                active_when.clone(),
                *optional,
                *once_per_turn,
                *amount,
                when_any_ally_played.clone(),
                pay_cost.clone(),
            )),
            _ => None,
        })
        .expect("cost_reduction clause");
    assert_eq!(scope, CompiledScope::FaceUp);
    assert!(
        optional,
        "'by suspending this Tamer' is an opt-in cost (DCGO isOptional)"
    );
    assert!(
        !once_per_turn,
        "no printed [Once Per Turn]; the suspend self-limits"
    );
    assert_eq!(amount, Some(1));
    assert!(
        when_any.is_some(),
        "ADVENTURE Digimon play predicate present"
    );
    assert!(active_when.is_some_and(|p| p.your_turn == Some(true)));
    assert!(!pay_cost.is_empty(), "suspend-self pay_cost present");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [Start of Your Main Phase] If you have an [ADVENTURE] Digimon, gain 1 memory
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st20_12_start_main_gains_1_memory_with_an_adventure_digimon() {
    let mut runner = builder().memory(0).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ADV-DIGI", Some(0));
    let cp = runner.event_checkpoint();

    assert_eq!(start_main_memory(&mut runner), 1, "+1 memory");

    let gained = runner.events_since(cp).iter().any(|e| {
        matches!(
            e,
            GameEvent::MemoryChange {
                player: 0,
                delta: 1,
                ..
            }
        )
    });
    assert!(
        gained,
        "the +1 memory shows up as a MemoryChange event in the log"
    );
}

#[test]
fn st20_12_start_main_no_gain_without_any_digimon() {
    let mut runner = builder().memory(0).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(start_main_memory(&mut runner), 0, "no Digimon: no gain");
}

#[test]
fn st20_12_start_main_no_gain_with_only_a_non_adventure_digimon() {
    let mut runner = builder().memory(0).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "PLAIN-DIGI", Some(0));
    assert_eq!(
        start_main_memory(&mut runner),
        0,
        "non-ADVENTURE Digimon: no gain"
    );
}

#[test]
fn st20_12_start_main_no_gain_with_only_an_adventure_tamer() {
    // The printed gate is "a Digimon with the [ADVENTURE] trait": a Tamer
    // (ST20-12 itself, or another ADVENTURE Tamer) does not satisfy it.
    let mut runner = builder().memory(0).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ADV-TAMER", Some(0));
    assert_eq!(start_main_memory(&mut runner), 0, "Tamers are not Digimon");
}

#[test]
fn st20_12_start_main_no_gain_when_only_the_opponent_has_an_adventure_digimon() {
    let mut runner = builder().memory(0).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-ADV", Some(0));
    assert_eq!(
        start_main_memory(&mut runner),
        0,
        "'you have' - own Digimon only"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn] by suspending this Tamer, reduce the play cost by 1
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st20_12_cost_reduction_fires_on_adventure_digimon_play() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !is_suspended(&runner, 0, tamer.index),
        "Tamer starts unsuspended"
    );

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "an optional cost-reduction prompt must install"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 4, "5 - 1 = 4");
    assert!(
        is_suspended(&runner, 0, tamer.index),
        "the 'by suspending this Tamer' cost was paid"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-DIGI"),
        "the Digimon was played"
    );
}

#[test]
fn st20_12_cost_reduction_event_log_records_reduced_cost() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));

    let cp = runner.event_checkpoint();
    let _ = runner.play(0, 0);
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let play = runner
        .events_since(cp)
        .iter()
        .find_map(|e| match e {
            GameEvent::Play {
                card_id,
                cost_paid,
                cost_printed,
                ..
            } if card_id == "ADV-DIGI" => Some((*cost_paid, *cost_printed)),
            _ => None,
        })
        .expect("a Play event for ADV-DIGI");
    assert_eq!(play, (4, 5), "Play event: paid 4 of printed 5");
}

#[test]
fn st20_12_cost_reduction_does_not_fire_on_non_adventure_digimon() {
    let mut runner = builder().hand(0, &["PLAIN-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "plain Digimon plays with no reduction prompt"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "full printed cost");
    assert!(
        !is_suspended(&runner, 0, tamer.index),
        "Tamer not suspended"
    );
}

#[test]
fn st20_12_cost_reduction_does_not_fire_on_adventure_tamer_card() {
    let mut runner = builder().hand(0, &["ADV-TAMER"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(played.is_some(), "Tamer plays with no reduction prompt");
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 3, "full printed cost 3");
    assert!(!is_suspended(&runner, 0, tamer.index));
}

#[test]
fn st20_12_cost_reduction_does_not_fire_on_opponents_turn() {
    let mut runner = builder().hand(1, &["ADV-DIGI"]).memory(0).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: player 1's turn");

    let mem_before = runner.memory();
    let played = runner.play(1, 0);
    assert!(played.is_some(), "player 1's play has no reduction prompt");
    drive_accept(&mut runner, 1);
    let _ = runner.auto_resolve();

    assert_eq!((runner.memory() - mem_before).abs(), 5, "full cost 5");
    assert!(!is_suspended(&runner, 0, tamer.index));
}

#[test]
fn st20_12_cost_reduction_unavailable_while_tamer_is_suspended() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "no reduction prompt when the cost is unpayable"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "full cost 5");
}

#[test]
fn st20_12_cost_reduction_can_be_declined() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    assert!(runner.game.pending_selection.is_some(), "prompt installs");
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        runner.execute_action(0, action).expect("decline");
    }
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "declined: full cost");
    assert!(!is_suspended(&runner, 0, tamer.index));
}

/// Official Q&A: with ST20-12 and ST20-13 both on the field, accepting both
/// reductions lowers a single play by a total of 2 (5 -> 3), suspending both.
#[test]
fn st20_12_and_st20_13_reductions_stack_for_a_total_of_2() {
    let mut runner = builder()
        .dsl_card("ST20-13")
        .expect("ST20-13 in embedded DSL pack")
        .hand(0, &["ADV-DIGI"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let sora = runner.place_on_field(0, CARD_ID, Some(0));
    let tai = runner.place_on_field(0, "ST20-13", Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 3, "5 - 1 - 1 = 3");
    assert!(is_suspended(&runner, 0, sora.index), "ST20-12 suspended");
    assert!(is_suspended(&runner, 0, tai.index), "ST20-13 suspended");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] Play this card without paying the cost
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st20_12_security_plays_self_free() {
    let mut attacker = filler("SEC-ATK");
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = builder()
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let atk = runner.place_on_field(0, "SEC-ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: Tamer in security"
    );
    let mem_before = runner.memory();

    let _ = runner.attack_player(atk, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "Tamer left the security stack");
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "ST20-12 is played onto the defender's field"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "played without paying the cost"
    );
}
