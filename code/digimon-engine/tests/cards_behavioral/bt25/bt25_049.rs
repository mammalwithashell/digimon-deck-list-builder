//! BT25-049 Armalizamon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Reptile, Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json, confirmed vs DCGO)
//! [On Play] [When Digivolving] You may suspend 1 of your opponent's Digimon.
//! [Your Turn] [Once Per Turn] When you would use an Option card with the
//!   [Glowing Dawn] trait, by trashing the bottom face-down card under any of
//!   your Tamers, reduce the cost by 3.
//! Inherited: <Piercing>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_049.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - OnPlay/WhenDigivolving optional suspend opponent Digimon
//! - alt-digivolve from Glowing Dawn Lv.3
//! - H3 inherited Piercing
//! - D2 — BeforePayCost Option-USE cost reduction with an INTERACTIVE
//!   `trash_bottom_face_down_source_under_tamer` pay_cost
//!   (G-COST-REDUCTION-INTERACTIVE-PAY-COST, Option-use half).
//!
//! # Verdict — IMPLEMENTED (2026-06-15)
//! Clause 2 (Glowing Dawn Option-USE cost reduction by trashing a face-down card
//! under a Tamer, -3) is now IMPLEMENTED: engine gap
//! G-COST-REDUCTION-INTERACTIVE-PAY-COST is closed for the Option-use path
//! (`game_actions/options.rs::try_prompt_interactive_option_use_cost_reducer`),
//! so the interactive Tamer-pick pay_cost parks, resolves, the bottom face-down
//! stash is trashed, AND the -3 reduction is credited. Clauses 1, inherited
//! Piercing, and the alt-digivolve were already IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-049";

fn armalizamon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
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
fn bt25_049_compiles_as_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
}

#[test]
fn bt25_049_has_onplay_whendigivolving_clause_and_inherited_piercing() {
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
            .any(|t| t.when == vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]),
        "OnPlay/WhenDigivolving suspend clause present"
    );

    let has_inherited_keyword = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, .. })
                if *scope == CompiledScope::Inherited
        )
    });
    assert!(has_inherited_keyword, "inherited grant_keyword (Piercing) present");
}

#[test]
fn bt25_049_has_glowing_dawn_alt_digivolve() {
    let card = compiled(CARD_ID);
    let has_alt = card
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve));
    assert!(has_alt, "alt-digivolve path (Lv.3 Glowing Dawn) present");
}

/// Clause 2 (the interactive Option-use cost reducer) now compiles as a
/// `CostReduction` declarative clause keyed on `when_any_ally_played` (Option +
/// Glowing Dawn). G-COST-REDUCTION-INTERACTIVE-PAY-COST is closed for the
/// Option-use path.
#[test]
fn bt25_049_cost_reduction_clause_present_with_pay_cost() {
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
// Section 2 — [On Play][When Digivolving] you may suspend 1 opponent Digimon
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_049_on_play_suspends_chosen_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(armalizamon())
        .add_card(make_opp_digimon("OPP"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP", Some(0));
    let arm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, arm.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("optional suspend prompt installs (opponent Digimon present)");
    assert!(view.is_optional, "the suspend is 'you may' → optional");
    // Pick the opponent Digimon (first non-PASS action).
    let target = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("an opponent Digimon target exists");
    runner
        .execute_action(view.selecting_player, target)
        .expect("suspend the opponent Digimon");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "the chosen opponent Digimon is suspended"
    );
}

#[test]
fn bt25_049_on_play_can_decline_suspend() {
    let mut runner = DebugRunner::builder()
        .add_card(armalizamon())
        .add_card(make_opp_digimon("OPP"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP", Some(0));
    let arm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, arm.index as usize);

    assert!(runner.pending_is_optional(), "the suspend is optional");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional suspend");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "declining leaves the opponent Digimon unsuspended"
    );
}

#[test]
fn bt25_049_on_play_no_prompt_when_no_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(armalizamon())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let arm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, arm.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → no suspend prompt"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn][Once Per Turn] Glowing-Dawn Option-USE cost -3 via
// trashing a bottom face-down card under a Tamer
// (G-COST-REDUCTION-INTERACTIVE-PAY-COST, Option-use path)
// ═══════════════════════════════════════════════════════════════════════════════

const AMOUNT: i16 = 3;

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

fn make_tamer(id: &str) -> CardData {
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

/// Build a runner: BT25-049 on field (reducer host), a Tamer with one face-down
/// stash, and an Option (cost 3) in hand at index 0.
fn option_runner(option_id: &str, glowing_dawn: bool) -> DebugRunner {
    let option = if glowing_dawn {
        make_glowing_dawn_option(option_id)
    } else {
        make_plain_option(option_id)
    };
    let mut runner = DebugRunner::builder()
        .add_card(armalizamon())
        .add_card(option)
        .add_card(make_tamer("TAMER"))
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
/// resolving it trashes the face-down source AND credits the -3 reduction
/// (cost 3 → effective 0; +1 trash for the FD stash, +1 for the resolved Option).
#[test]
fn bt25_049_option_use_reducer_credits_minus_three_on_paid_park() {
    let mut runner = option_runner("GD-OPT", true);

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, digimon_engine::selection::OptionPlayResult::Pending),
        "using the Glowing Dawn Option installs the cost-reduction prompt"
    );
    assert!(runner.pending_is_optional(), "the -3 reducer is optional (decline allowed)");
    assert_eq!(runner.memory(), mem_before, "no cost paid before the gate resolves");

    runner
        .accept_optional_trigger()
        .expect("accept the -3 cost reduction");
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
        "Option use cost 3 reduced by 3 → paid 0 memory (reduction credited behind the park)"
    );
}

/// CONTROL: with NO face-down stash the pay_cost is unpayable, so the Option is
/// used at FULL cost (3) and only the Option itself is trashed.
#[test]
fn bt25_049_option_use_reducer_no_reduction_when_unpayable() {
    let mut runner = option_runner("GD-OPT", true);
    // Find the Tamer and make its only source face-UP (no FD stash).
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
fn bt25_049_option_use_reducer_decline_pays_full_cost() {
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
fn bt25_049_option_use_reducer_inactive_on_non_glowing_dawn_option() {
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
