//! BT25-090 Tomoro Tenma — Tamer, Green, Cost 4. Traits: Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json, confirmed vs DCGO)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When any Digimon suspend, by suspending this Tamer, you may place
//!   the top 2 cards of your deck face down under this Tamer.
//! [Your Turn] [Once Per Turn] When you would use [Glowing Dawn] trait Option
//!   cards, by trashing the bottom face-down card under any of your Tamers,
//!   reduce the cost by 1.
//! Inherited: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_090.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - B1 start-of-turn tamer (memory swing — set to 3)
//! - on-suspend (any Digimon) self-suspend cost → place top 2 face down (stash substrate)
//! - Tamer [Security] play-self
//! - D2 — BeforePayCost Option-USE cost reduction with an INTERACTIVE
//!   `trash_bottom_face_down_source_under_tamer` pay_cost
//!   (G-COST-REDUCTION-INTERACTIVE-PAY-COST, Option-use half).
//!
//! # Verdict — IMPLEMENTED (2026-06-15)
//! Clause 3 (Glowing Dawn Option-USE cost reduction by trashing a face-down card
//! under a Tamer, -1) is now IMPLEMENTED: engine gap
//! G-COST-REDUCTION-INTERACTIVE-PAY-COST is closed for the Option-use path, so
//! the interactive Tamer-pick pay_cost parks, resolves, the bottom face-down
//! stash is trashed, AND the -1 reduction is credited. Clauses 1, 2, 4 already
//! IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-090";

fn tomoro() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_090_compiles_as_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
}

#[test]
fn bt25_090_has_start_of_turn_suspend_and_security_clauses() {
    let card = compiled(CARD_ID);
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
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "start-of-your-turn clause present"
    );
    let on_suspend = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSuspend])
        .expect("on-suspend clause present");
    assert!(on_suspend.optional, "'you may place' → optional clause");
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "security play-self clause present"
    );
}

/// Clause 3 (the interactive Option-use cost reducer) now compiles as a
/// `CostReduction` declarative clause keyed on `when_any_ally_played` (Option +
/// Glowing Dawn). G-COST-REDUCTION-INTERACTIVE-PAY-COST is closed for the
/// Option-use path.
#[test]
fn bt25_090_cost_reduction_clause_present_with_pay_cost() {
    let card = compiled(CARD_ID);
    let cr = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(cr @ CompiledDeclarativeClause::CostReduction { .. }) => {
            Some(cr)
        }
        _ => None,
    });
    let CompiledDeclarativeClause::CostReduction {
        when_any_ally_played,
        when_any_ally_digivolves_into,
        pay_cost,
        once_per_turn,
        ..
    } = cr.expect("interactive Option-use cost-reduction clause must compile")
    else {
        unreachable!()
    };
    assert!(
        when_any_ally_played.is_some(),
        "the reducer must be keyed on using an Option (when_any_ally_played)"
    );
    assert!(
        when_any_ally_digivolves_into.is_none(),
        "the Option-use reducer must NOT be a digivolve-into reducer"
    );
    assert!(
        !pay_cost.is_empty(),
        "the reducer must carry an interactive pay_cost (trash FD under Tamer)"
    );
    assert!(*once_per_turn, "[Once Per Turn] reducer");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [Start of Your Turn] set memory to 3
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_090_start_of_turn_sets_low_memory_to_three() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(1)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 1;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 3, "memory <=2 is set to 3");
}

#[test]
fn bt25_090_start_of_turn_does_not_change_high_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 5;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 5, "memory >2 is unchanged");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns] on ANY Digimon suspend → may suspend self + place 2
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_090_on_any_digimon_suspend_suspends_self_and_places_top_two() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(8)
        .start();

    let tomoro_perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0));

    let deck_before = runner.deck_size(0);
    // Any Digimon suspending fires the trigger — suspend the opponent's Digimon.
    runner.game.suspend(opp_digi);

    assert!(
        runner.game.pending_selection.is_some(),
        "an optional accept/decline prompt installs when a Digimon suspends"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the suspend+place");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[tomoro_perm.index as usize].is_suspended,
        "Tomoro is suspended (the activation cost was paid)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "top 2 deck cards placed under Tomoro"
    );
    let perm = &runner.game.players[0].battle_area[tomoro_perm.index as usize];
    assert_eq!(perm.card_sources.len(), 3, "own card + 2 face-down stash");
    assert!(
        perm.card_sources[0].face_down && perm.card_sources[1].face_down,
        "the two placed sources are face-down"
    );
}

#[test]
fn bt25_090_on_suspend_does_not_fire_for_tamer_suspend() {
    // A Tamer suspending (not a Digimon) must NOT trigger (event_target_kind: digimon).
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(8)
        .start();

    let tomoro_perm = runner.place_on_field(0, CARD_ID, Some(0));
    // Suspend Tomoro itself (a Tamer). The clause gates on a Digimon suspending.
    runner.game.suspend(tomoro_perm);
    let _ = runner.auto_resolve();

    // No place-2 prompt fired (Tamer suspend is not a Digimon suspend).
    let perm = &runner.game.players[0].battle_area[tomoro_perm.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "a Tamer suspend must not trigger the place-2 effect (event_target_kind: digimon gate)"
    );
}

#[test]
fn bt25_090_on_suspend_decline_does_nothing() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(8)
        .start();

    let tomoro_perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let deck_before = runner.deck_size(0);

    runner.game.suspend(opp_digi);
    assert!(runner.game.pending_selection.is_some());
    assert!(runner.pending_is_optional());
    runner
        .decline_optional_trigger()
        .expect("decline the optional clause");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[tomoro_perm.index as usize].is_suspended,
        "declining leaves Tomoro unsuspended"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no cards placed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] play-self structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_090_clause_on_security_present() {
    let card = compiled(CARD_ID);
    let on_sec = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnSecurity])
    });
    assert!(on_sec, "[Security] play-self clause must compile");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — [Your Turn][Once Per Turn] Glowing-Dawn Option-USE cost -1 via
// trashing a bottom face-down card under a Tamer
// (G-COST-REDUCTION-INTERACTIVE-PAY-COST, Option-use path)
// ═══════════════════════════════════════════════════════════════════════════════

const AMOUNT: i16 = 1;

/// A cost-3 [Glowing Dawn] Option with a benign [Main] body (gains 0 memory).
fn make_glowing_dawn_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A cost-3 plain (non-Glowing-Dawn) Option with a benign [Main] body.
fn make_plain_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

/// A vanilla Tamer that can host a face-down stash.
fn make_plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

/// Benign [Main] body so a synthetic Option is playable (gains 0 memory).
struct OptionMainNoop;
impl CardEffect for OptionMainNoop {
    fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .option_main()
            .name("noop option main")
            .process(|_ctx| {})
            .build()]
    }
}

/// Build a runner: BT25-090 (Tomoro) on field (reducer host), a separate Tamer
/// with one face-down stash, and an Option (cost 3) in hand at index 0.
fn option_runner(option_id: &str, glowing_dawn: bool) -> DebugRunner {
    let option = if glowing_dawn {
        make_glowing_dawn_option(option_id)
    } else {
        make_plain_option(option_id)
    };
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(option)
        .add_card(make_plain_tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .add_card(make_filler("FILLER-DECK"))
        .hand(0, &[option_id])
        .deck(0, &["FILLER-DECK"; 5])
        .deck(1, &["FILLER-DECK"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.register_effect(option_id, std::sync::Arc::new(OptionMainNoop));

    runner.place_on_field(0, CARD_ID, Some(0));
    let tamer_perm = runner.place_stack(0, &["STASH", "TAMER"]);
    runner.game.players[0].battle_area[tamer_perm.index as usize].card_sources[0].face_down = true;
    runner
}

/// POSITIVE: using the [Glowing Dawn] Option (cost 3) with one eligible Tamer
/// installs the optional accept gate; accepting parks on the Tamer pick,
/// resolving it trashes the face-down source AND credits the -1 reduction
/// (cost 3 → effective 2; +1 trash for the FD stash, +1 for the resolved Option).
#[test]
fn bt25_090_option_use_reducer_credits_minus_one_on_paid_park() {
    let mut runner = option_runner("GD-OPT", true);

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, digimon_engine::selection::OptionPlayResult::Pending),
        "using the Glowing Dawn Option installs the cost-reduction prompt"
    );
    assert!(runner.pending_is_optional(), "the -1 reducer is optional (decline allowed)");
    assert_eq!(runner.memory(), mem_before, "no cost paid before the gate resolves");

    runner
        .accept_optional_trigger()
        .expect("accept the -1 cost reduction");
    let _ = runner.auto_resolve();
    assert!(runner.game.pending_selection.is_none(), "the Option resolves");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the FD stash source was trashed as the cost (+1) and the resolved Option \
         went to trash (+1)"
    );
    assert_eq!(
        mem_before - runner.memory(),
        3 - AMOUNT,
        "Option use cost 3 reduced by 1 → paid 2 memory (reduction credited behind the park)"
    );
}

/// CONTROL: with NO face-down stash the pay_cost is unpayable, so the Option is
/// used at FULL cost (3) and only the Option itself is trashed.
#[test]
fn bt25_090_option_use_reducer_no_reduction_when_unpayable() {
    let mut runner = option_runner("GD-OPT", true);
    let tamer_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "TAMER")
        .expect("Tamer on field");
    runner.game.players[0].battle_area[tamer_idx].card_sources[0].face_down = false;

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "no FD stash → only the resolved Option is trashed (no FD trash)"
    );
    assert_eq!(
        mem_before - runner.memory(),
        3,
        "unpayable reducer → full Option use cost 3 paid, no reduction credited"
    );
}

/// DECLINE: with an eligible Tamer present, DECLINING the optional reducer must
/// let the Option play complete at FULL cost (3) — FD stash untouched.
#[test]
fn bt25_090_option_use_reducer_decline_pays_full_cost() {
    let mut runner = option_runner("GD-OPT", true);

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(matches!(
        result,
        digimon_engine::selection::OptionPlayResult::Pending
    ));
    assert!(runner.pending_is_optional(), "an accept/decline gate installs");
    runner
        .decline_optional_trigger()
        .expect("decline the optional reducer");
    let _ = runner.auto_resolve();

    assert!(runner.game.pending_selection.is_none(), "the Option play completes");
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "declined → FD stash untouched; only the resolved Option is trashed"
    );
    assert_eq!(
        mem_before - runner.memory(),
        3,
        "declined reducer → full Option use cost 3 paid, no reduction credited"
    );
}

/// NEGATIVE: using a NON-[Glowing Dawn] Option never fires the reducer
/// (condition fails) — full cost 3, FD stash untouched, no reducer prompt.
#[test]
fn bt25_090_option_use_reducer_inactive_on_non_glowing_dawn_option() {
    let mut runner = option_runner("PLAIN-OPT", false);

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "non-Glowing-Dawn Option → FD stash untouched; only the Option is trashed"
    );
    assert_eq!(
        mem_before - runner.memory(),
        3,
        "non-[Glowing Dawn] Option pays the full use cost 3 (no reduction)"
    );
}
