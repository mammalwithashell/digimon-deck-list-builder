//! ST23-03 Cougarmon — Digimon, Lv.4, Yellow, DP 4000, Cost 4.
//! Traits: Mammal, Glowing Dawn, BEATBREAK.
//!
//! # Card text (CARD IMAGE — authoritative)
//! Digivolve box (printed corner): Lv.3 / cost 2.
//! [Digivolve] Lv.3 w/[Glowing Dawn] trait: Cost 2.   (DCGO alt-digivolve box)
//! [On Play] [When Digivolving] Add your top security card to the hand. Then,
//!   <Recovery +1> (Place the top card of your deck as your top security card.)
//! [Your Turn] When this Digimon would digivolve into a [Glowing Dawn] trait
//!   Digimon card, by trashing the bottom face-down card from under any of your
//!   Tamers, reduce the cost by 2.
//! Inherited: <Barrier> (When this Digimon would be deleted in battle, by
//!   trashing your top security card, it isn't deleted.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Yellow/ST23_03.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - D2 — BeforePayCost digivolve-into-trait cost reduction with an INTERACTIVE
//!   `trash_bottom_face_down_source_under_tamer` pay_cost
//!   (G-COST-REDUCTION-INTERACTIVE-PAY-COST, digivolve half).
//! - On Play / When Digivolving add-top-security + <Recovery +1>.
//! - inherited <Barrier> keyword grant.
//! - alt-digivolve from Lv.3 [Glowing Dawn] cost 2.
//!
//! # Verdict — IMPLEMENTED (2026-06-15)
//! The Glowing-Dawn digivolve cost reducer (clause 2) is now IMPLEMENTED: engine
//! gap G-COST-REDUCTION-INTERACTIVE-PAY-COST is closed for the digivolve path
//! (`game_actions/digivolve.rs::try_prompt_interactive_digivolve_cost_reducer`),
//! so the interactive Tamer-pick pay_cost parks, resolves, the bottom face-down
//! stash is trashed, AND the -2 reduction is credited.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST23-03";

// Evo-color bytes (match `card_data::parse_card_color`): Yellow == 2.
const EVO_YELLOW: u8 = 2;

// ─── Card-data factories ─────────────────────────────────────────────────────

/// ST23-03 itself, the BASE that hosts the interactive digivolve reducer.
/// The DSL pack gives it empty `evo_costs`; as the *source* of a digivolve that
/// does not matter (only the TARGET's evo cost is paid).
fn cougarmon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A Lv.5 Yellow [Glowing Dawn] Digimon — the digivolve TARGET (the card
/// ST23-03 digivolves INTO). Printed evo cost from a Lv.4 Yellow base is 4.
fn glowing_dawn_lv5() -> CardData {
    let mut c = make_test_card("GD-LV5", "GlowingDawnLv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.traits = vec!["Glowing Dawn".to_string()];
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        level: 4,
        card_color: EVO_YELLOW,
        memory_cost: 4,
    }];
    c
}

/// A Lv.5 Yellow Digimon WITHOUT the [Glowing Dawn] trait (same evo cost 4) —
/// the negative digivolve target (reducer must NOT fire).
fn plain_lv5() -> CardData {
    let mut c = make_test_card("PLAIN-LV5", "PlainLv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.traits = vec!["Beast".to_string()];
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        level: 4,
        card_color: EVO_YELLOW,
        memory_cost: 4,
    }];
    c
}

/// A vanilla Tamer that can host a face-down stash (the FD-trash cost target).
fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: usize, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in card_data"));
    let card_index = runner.game.next_card_index();
    runner.game.players[p]
        .hand
        .push(CardSource::new(data_idx, p as u8, card_index));
    runner.game.players[p].hand.len() - 1
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_03_compiles_as_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Cougarmon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    for t in ["Mammal", "Glowing Dawn", "BEATBREAK"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn st23_03_has_onplay_whendigivolving_and_inherited_barrier() {
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
        "OnPlay/WhenDigivolving add-security + recovery clause present"
    );

    let has_barrier = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Barrier" && *scope == CompiledScope::Inherited
        )
    });
    assert!(has_barrier, "inherited <Barrier> keyword grant present");
}

#[test]
fn st23_03_has_glowing_dawn_alt_digivolve() {
    let card = compiled(CARD_ID);
    let has_alt = card
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve));
    assert!(has_alt, "alt-digivolve path (Lv.3 Glowing Dawn) present");
}

/// Clause 2 (the interactive-pay_cost digivolve cost reducer) now compiles as a
/// `CostReduction` declarative clause keyed on `when_any_ally_digivolves_into`.
#[test]
fn st23_03_cost_reduction_clause_present_with_pay_cost() {
    let card = compiled(CARD_ID);
    let cr = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(cr @ CompiledDeclarativeClause::CostReduction { .. }) => {
            Some(cr)
        }
        _ => None,
    });
    let CompiledDeclarativeClause::CostReduction {
        when_any_ally_digivolves_into,
        pay_cost,
        ..
    } = cr.expect("interactive digivolve cost-reduction clause must compile")
    else {
        unreachable!()
    };
    assert!(
        when_any_ally_digivolves_into.is_some(),
        "the reducer must be digivolve-target-keyed"
    );
    assert!(
        !pay_cost.is_empty(),
        "the reducer must carry an interactive pay_cost (trash FD under Tamer)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [Your Turn] digivolve-into-[Glowing Dawn] cost -2 via trashing a
// bottom face-down card under a Tamer (G-COST-REDUCTION-INTERACTIVE-PAY-COST)
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a runner: ST23-03 on field (hosting the reducer) with one face-down
/// stash beneath a Tamer, and a Lv.5 [Glowing Dawn] target in hand.
/// Returns (runner, base field index, tamer field index, target hand index).
fn digivolve_runner(target_glowing_dawn: bool) -> (DebugRunner, usize, usize, usize) {
    let target = if target_glowing_dawn {
        glowing_dawn_lv5()
    } else {
        plain_lv5()
    };
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-03 in embedded DSL pack")
        .add_card(target)
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"; 5])
        .deck(1, &["FILLER-DECK"; 5])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // ST23-03 base on field (the digivolve source / reducer host).
    let base = runner.place_on_field(0, CARD_ID, Some(0));
    // Tamer (top of [STASH, TAMER]) with one face-down stash source beneath it.
    let tamer_perm = runner.place_stack(0, &["STASH", "TAMER"]);
    runner.game.players[0].battle_area[tamer_perm.index as usize].card_sources[0].face_down = true;

    let target_id = if target_glowing_dawn { "GD-LV5" } else { "PLAIN-LV5" };
    let hand_idx = push_to_hand(&mut runner, 0, target_id);

    (
        runner,
        base.index as usize,
        tamer_perm.index as usize,
        hand_idx,
    )
}

/// POSITIVE: digivolving ST23-03 INTO a Lv.5 [Glowing Dawn] card with one
/// eligible Tamer (one face-down stash) installs the optional accept gate;
/// accepting parks on the Tamer pick, resolving it trashes the face-down source
/// AND credits the -2 reduction (printed digivolve cost 4 → effective 2).
#[test]
fn st23_03_digivolve_reducer_credits_minus_two_on_paid_park() {
    let (mut runner, base, _tamer, hand_idx) = digivolve_runner(true);

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    // Begin the digivolve into the GD target → installs the cost-reduction
    // accept/decline gate (DCGO's canNoSelect → an exposed decline choice).
    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, base, PlaySource::ByHand);
    assert!(
        !ok,
        "digivolve must park while the interactive reducer prompt is pending"
    );
    assert!(
        runner.pending_selection().is_some(),
        "the -2 reducer installs an accept/decline gate"
    );
    assert!(runner.pending_is_optional(), "the reducer is optional (decline allowed)");
    assert_eq!(runner.memory(), mem_before, "no cost paid before the gate resolves");

    runner
        .accept_optional_trigger()
        .expect("accept the -2 cost reduction");
    // The pay_cost parks on the single-eligible-Tamer pick — resolve it.
    let _ = runner.auto_resolve();
    assert!(runner.game.pending_selection.is_none(), "digivolution completes");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the bottom face-down stash source was trashed as the cost"
    );
    assert_eq!(
        mem_before - runner.memory(),
        2,
        "digivolve cost 4 reduced by 2 → paid 2 memory (reduction credited behind the park)"
    );
}

/// CONTROL: with NO face-down stash under any Tamer the pay_cost is unpayable,
/// so the digivolve proceeds at the FULL printed cost (4) and nothing is
/// trashed. Guards against crediting a reduction that was never paid for.
#[test]
fn st23_03_digivolve_reducer_no_reduction_when_unpayable() {
    let (mut runner, base, tamer_idx, hand_idx) = digivolve_runner(true);
    // Make the Tamer's only source face-UP (no FD stash → unpayable).
    runner.game.players[0].battle_area[tamer_idx].card_sources[0].face_down = false;

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let _ = runner
        .game
        .digivolve_from_hand(0, hand_idx, base, PlaySource::ByHand);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "no face-down stash → nothing trashed"
    );
    assert_eq!(
        mem_before - runner.memory(),
        4,
        "unpayable reducer → full digivolve cost 4 paid, no reduction credited"
    );
}

/// DECLINE: with an eligible Tamer present, DECLINING the optional reducer must
/// let the digivolve complete at FULL cost (4) — FD stash untouched.
#[test]
fn st23_03_digivolve_reducer_decline_pays_full_cost() {
    let (mut runner, base, _tamer, hand_idx) = digivolve_runner(true);

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let _ = runner
        .game
        .digivolve_from_hand(0, hand_idx, base, PlaySource::ByHand);
    assert!(runner.pending_is_optional(), "an accept/decline gate installs");
    runner
        .decline_optional_trigger()
        .expect("decline the optional reducer");
    let _ = runner.auto_resolve();

    assert!(runner.game.pending_selection.is_none(), "digivolution completes");
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declined → FD stash untouched (nothing trashed)"
    );
    assert_eq!(
        mem_before - runner.memory(),
        4,
        "declined reducer → full digivolve cost 4 paid, no reduction credited"
    );
}

/// NEGATIVE: digivolving into a NON-[Glowing Dawn] target never fires the
/// reducer (condition fails) — full cost 4, FD stash untouched, no prompt.
#[test]
fn st23_03_digivolve_reducer_inactive_on_non_glowing_dawn_target() {
    let (mut runner, base, _tamer, hand_idx) = digivolve_runner(false);

    let mem_before = runner.memory();
    let trash_before = runner.trash_size(0);

    let ok = runner
        .game
        .digivolve_from_hand(0, hand_idx, base, PlaySource::ByHand);
    assert!(ok, "non-Glowing-Dawn digivolve completes synchronously");
    assert!(
        runner.game.pending_selection.is_none(),
        "no reducer prompt for a non-[Glowing Dawn] target"
    );
    let _ = runner.auto_resolve();

    assert_eq!(runner.trash_size(0), trash_before, "FD stash untouched");
    assert_eq!(
        mem_before - runner.memory(),
        4,
        "non-[Glowing Dawn] target pays the full digivolve cost 4 (no reduction)"
    );
}
