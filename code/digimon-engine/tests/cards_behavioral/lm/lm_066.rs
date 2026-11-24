//! LM-066 Zephagamon (dual) — card-text tests.
//!
//! Digimon face: <Piercing> <Vortex> <Blocker>. [When Digivolving] [When Attacking]
//! [Once Per Turn] You may unsuspend 1 Digimon. Then, this Digimon may battle 1 of
//! your opponent's Digimon. [All Turns] [Once Per Turn] When this Digimon would
//! leave the battle area other than by your effects, by suspending 1 Digimon, it
//! doesn't leave. (Rule) Trait: Has [Bird Dragon] Type.
//! Option face (use 6): When this card would be used, by suspending 2 Digimon,
//! reduce the cost by 2. [Main] 1 of your opponent's Digimon or Tamers can't
//! unsuspend until their turn ends. Then, for every 2 suspended Digimon, return 1
//! of their Digimon to the bottom of the deck. Arts Digivolve.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "LM-066";

fn security_option_filler() -> CardData {
    let mut c = make_test_card("SEC-OPT", "Security Option Filler");
    c.card_kind = CardKind::Option;
    c
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("LM-066 YAML parses and compiles")
        .add_card(security_option_filler())
        .deck(0, &["SEC-OPT"; 10])
        .deck(1, &["SEC-OPT"; 10])
        .security(0, &["SEC-OPT"; 3])
        .security(1, &["SEC-OPT"; 3])
        .memory(8)
        .start()
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize]
        .battle_area
        .get(h.index as usize)
        .map(|p| p.is_suspended)
        .unwrap_or(false)
}

#[test]
fn lm_066_metadata() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Zephagamon");
    assert_eq!(card.kind, CompiledCardKind::Dual);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(6));
    assert_eq!(card.dp, Some(13000));
    for t in ["Magic Knight", "Vortex Warriors", "LIBERATOR", "Bird Dragon"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
    let dual = card.dual.as_ref().expect("dual block");
    assert_eq!(dual.digimon.level, 6);
    assert_eq!(dual.option.use_cost, 6);
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "Option face has Arts Digivolve"
    );
}

#[test]
fn lm_066_alt_path_vortex_warriors_play_cost_11() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(card.alt_paths.iter().any(|p| {
        p.cost == Some(CompiledCost::Literal(2))
            && p.from
                .as_ref()
                .and_then(|f| f.trait_has.as_deref())
                .map(|t| t == "Vortex Warriors")
                .unwrap_or(false)
    }));
    assert!(card.alt_paths.iter().any(|p| p.cost == Some(CompiledCost::Literal(4))));
}

#[test]
fn lm_066_wd_wa_shared_opt_clause_unsuspends_then_battles() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    let clause = dual
        .digimon
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("WD/WA clause");
    assert!(clause.once_per_turn);
    let has_battle = format!("{:?}", clause.process).contains("Battle");
    assert!(has_battle, "battle step present");
}

#[test]
fn lm_066_has_leave_replacement_and_option_main() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    assert!(dual.digimon.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
    )));
    assert!(
        dual.option
            .effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Triggered(_))),
        "Option face [Main]"
    );
}

#[test]
fn lm_066_attack_offers_optional_unsuspend_of_any_suspended_digimon() {
    let mut runner = base();
    let zeph = runner.place_stack(0, &["LM-066"]);
    let opp = runner.place_stack(1, &["LM-066"]);
    runner.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    runner.attack_player(zeph, 1, false);
    assert!(is_suspended(&runner, zeph), "attacker suspends");
    let view = runner
        .pending_selection_view()
        .expect("unsuspend prompt installs");
    // Both the suspended attacker and the opponent's suspended Digimon are
    // candidates, and declining is legal ("You may").
    assert_eq!(view.valid_action_ids.len(), 2, "both suspended Digimon");
    assert!(view.is_optional, "unsuspend is optional");
}

#[test]
fn lm_066_unsuspend_offers_unsuspended_digimon_but_not_ones_that_cannot_unsuspend() {
    let mut runner = base();
    let zeph = runner.place_stack(0, &["LM-066"]);
    let locked = runner.place_stack(1, &["LM-066"]);
    runner.game.players[1].battle_area[locked.index as usize].is_suspended = true;
    runner.game.modifiers.add(
        locked,
        ModifierEntry::simple(ModifierType::CannotUnsuspend, 0, Expiry::Permanent, 1),
    );
    runner.place_stack(1, &["LM-066"]);
    runner.attack_player(zeph, 1, false);
    let view = runner.pending_selection_view().expect("unsuspend prompt installs");
    // The suspended attacker and the opponent's unsuspended Digimon (a legal
    // no-op target); the suspended one under CannotUnsuspend is excluded.
    assert_eq!(view.valid_action_ids.len(), 2);
    assert!(view.is_optional);
}

#[test]
fn lm_066_declining_unsuspend_still_offers_the_battle() {
    let mut runner = base();
    let zeph = runner.place_stack(0, &["LM-066"]);
    runner.place_stack(1, &["LM-066"]);
    runner.attack_player(zeph, 1, false);
    runner.pending_selection_view().expect("unsuspend prompt installs");
    runner.decline_prompt().expect("unsuspend is optional");
    let view = runner
        .pending_selection_view()
        .expect("\"Then, this Digimon may battle\" is offered after declining the unsuspend");
    assert!(view.prompt.contains("battle"), "battle prompt, got {:?}", view.prompt);
    assert_eq!(view.valid_action_ids.len(), 1, "the opponent's Digimon");
    assert!(view.is_optional, "the battle is optional");
}

#[test]
fn lm_066_suspension_costs_require_a_digimon_that_can_suspend() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    let digimon = format!("{:?}", dual.digimon.effects);
    let option = format!("{:?}", dual.option.effects);
    assert!(digimon.contains("can_change_orientation: Some(true)"), "leave replacement payment");
    assert_eq!(option.matches("can_change_orientation: Some(true)").count(), 3, "cost gate + both payments");
}
