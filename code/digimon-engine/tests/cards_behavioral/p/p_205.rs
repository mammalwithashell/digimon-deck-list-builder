//! P-205 Insane Synthetic Monster — Option, Purple, Cost 3, traits: [DM].
//!
//! # Card text (official Bandai DB / data/card_bundles/P-205.md — verbatim)
//!
//! While you have a Digimon or Tamer with the [DM] trait on the field, you
//! can ignore this card's color requirements.
//!
//! **[Main]** ＜Draw 2＞ (Draw 2 cards from your deck.) and trash 2 cards in
//! your hand. Then, place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn,
//! activate the effect below.)
//! ・By deleting 1 of your play cost 7 or lower Digimon, you may play 1
//! Digimon card with [Kimeramon] or [Millenniummon] in its name from your
//! trash with the play cost reduced by 3.
//!
//! **Inherited (Security):** [Security] ＜Draw 2＞ and trash 2 cards in your
//! hand. Then, place this card in the battle area.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/P/Purple/P_205.cs`
//!
//! # Patterns this test covers
//!
//! - **D3 Color ignore / bypass** — `kind: flood_gate` + `IgnoreColorRequirement`
//!   gated by an `any_of` over (own DM Digimon | own DM Tamer). Mirror of
//!   BT22-099 Clause 0 (CS trait swapped for DM).
//! - **Draw-N-then-trash-N mandatory pair** — `draw: {count: 2}` followed by
//!   two sequential mandatory `select_hand` + `trash_from_hand_by_index`
//!   steps (BT21-071 Clause 3 idiom), then `place_self_as_delay_option`.
//! - **Opt-cost→Mand `<Delay>`** — `kind: delay`, `trigger: delayed`
//!   (player-activated, standard printed `<Delay>`, matches P-035 / BT13-110 /
//!   BT22-099). Body: an optional "By deleting..." cost
//!   (`select_own_permanent` + `delete_permanent`, EX9-024 idiom) gating a
//!   SEPARATE optional "you may play..." payoff (`select_trash` +
//!   `play_from_trash` with `cost_delta: { reduce: 3 }`, BT15-096 idiom).
//! - **DCGO operator-precedence bug, printed-text-wins** — DCGO's
//!   `CanSelectCardCondition` (P_205.cs lines 101-106) is
//!   `cardSource.IsDigimon && cardSource.ContainsCardName("Kimeramon") ||
//!   cardSource.ContainsCardName("Millenniummon") && CanPlayAsNewPermanent(...)`.
//!   Because `&&` binds tighter than `||` in C#, this parses as
//!   `(IsDigimon && Kimeramon) || (Millenniummon && CanPlayAsNewPermanent)` —
//!   a "Millenniummon"-named card does not need `IsDigimon` to be true under
//!   DCGO's actual (buggy) precedence. Per source priority (printed text wins
//!   over a DCGO implementation quirk — see CLAUDE.md "Source priority"), we
//!   implement the OBVIOUSLY-INTENDED semantics: `IsDigimon AND (name has
//!   "Kimeramon" OR name has "Millenniummon")`, i.e. `kind: digimon` ANDed
//!   with an `any_of` over two `name_contains` leaves. This is also the only
//!   sane reading of the printed text "1 Digimon card with [Kimeramon] or
//!   [Millenniummon] in its name".
//!
//! # Faithfulness audit (per clause)
//!
//! 0. **Color bypass** — printed "While you have a Digimon or Tamer with the
//!    [DM] trait on the field, you can ignore this card's color
//!    requirements." DCGO `IgnoreColorConditionClass.CanUseCondition`:
//!    `HasMatchConditionPermanent((permanent) => permanent.TopCard.Owner ==
//!    card.Owner && (permanent.IsTamer || permanent.IsDigimon) &&
//!    permanent.TopCard.HasDMTraits)`. Both disjuncts (Digimon | Tamer) carry
//!    the DM-trait gate. `CardCondition: cardSource == card` — bypass is
//!    scoped to THIS card only (not e.g. a shared color-bypass umbrella).
//! 1. **[Main] Draw 2, trash 2, place self** — printed mandatory (no "you
//!    may"); DCGO `SetUpActivateClass(..., isOptional: -1 == not optional,
//!    ...)`. `CardEffectCommons.DrawAndDiscardCards(drawAmount: 2,
//!    trashAmount: 2)` then `PlaceDelayOptionCards`.
//! 2. **[Main] ＜Delay＞** — DCGO `EffectTiming.OnDeclaration`,
//!    `CanUseCondition: CanDeclareOptionDelayEffect(card)` (standard
//!    player-initiated Delay activation). Body cost: `SelectPermanentEffect`
//!    over own play-cost<=7 Digimon, `canNoSelect: true` (declinable — matches
//!    the Opt-cost→Mand rule: "By X-ing" costs are optional per
//!    `docs/digimon-rules/keyword-semantics.md`). Payoff:
//!    `SelectCardEffect` over trash Kimeramon/Millenniummon Digimon,
//!    `canNoSelect: true` (independently optional — printed "you may play").
//!    `int cost = selectedTrashCard.BasePlayCostFromEntity - 3;` then
//!    `PlayPermanentCards(payCost: true, fixedCost: cost)` — a REDUCE-3 delta,
//!    not a fixed/free play.
//! 3. **[Security] (inherited) mirror of Clause 1** — DCGO
//!    `EffectTiming.SecuritySkill`, same `DrawAndDiscardCards` +
//!    `PlaceDelayOptionCards` shared coroutine as Clause 1.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCostDelta, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-205";
const YAML: &str = include_str!("../../../cards/p/P-205.yaml");

// ═══════════════════════════════════════════════════════════════════════════
// Card-data factories
// ═══════════════════════════════════════════════════════════════════════════

/// A neutral filler card — no traits, default Digimon shape (level 3, cost 3).
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon-kind card carrying the [DM] trait. Enables the Clause 0
/// color-bypass gate and can also double as a generic own-field Digimon.
fn make_dm_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["DM".to_string()];
    c
}

/// A Tamer-kind card carrying the [DM] trait — alternative color-bypass gate
/// enabler per the printed "Digimon or Tamer" disjunction.
fn make_dm_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["DM".to_string()];
    c
}

/// A Digimon WITHOUT the DM trait — negative control for Clause 0.
fn make_non_dm_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Purple];
    c.traits = vec![];
    c
}

/// A Digimon with a controllable play cost — used as the Delay clause's
/// "play cost 7 or lower" deletion-cost target.
fn digimon_with_cost(card_id: &str, name: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c
}

/// A Digimon card carrying "Kimeramon" in its name, with a controllable play
/// cost — the trash-recursion payoff target.
fn kimeramon_named(card_id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(card_id, "Kimeramon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c
}

/// A Digimon card carrying "Millenniummon" in its name (substring match, like
/// "MoonMillenniummon" / "ZeedMillenniummon" in the real card pool).
fn millenniummon_named(card_id: &str, name: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(14000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c
}

/// A Tamer card whose NAME happens to contain "Kimeramon" — used to prove the
/// `kind: digimon` gate is enforced (a Tamer must NOT be a legal target even
/// if its name matches, closing the DCGO operator-precedence gap per printed
/// text "1 Digimon card with [Kimeramon]... in its name").
fn kimeramon_named_tamer(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Kimeramon Tactician");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c
}

/// A Digimon card with a name that matches NEITHER Kimeramon nor
/// Millenniummon — negative control for the trash-recursion payoff filter.
fn unrelated_digimon(card_id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(card_id, "Agumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

/// Drop a card directly into player 0's trash for the recursion fixtures.
fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card exists in registry");
    let src = CardSource::new(data_idx, 0, runner.game.next_card_index());
    runner.game.players[0].trash.push(src);
}

/// Force-seat P-205 as an already-mature `<Delay>` Option (past its placing
/// turn) directly on the field, skipping the [Main] draw/trash/place-self
/// sequence — mirrors the BT15-096 `place_bt15_096_as_mature_delay` idiom,
/// adapted for `DelayTrigger::MainPhaseActivated` (the player-declared
/// `<Delay>`, not the auto-firing `EndOfYourNextTurn` variant).
fn place_p205_as_mature_delay(runner: &mut DebugRunner) -> PermanentHandle {
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: 0,
        };
    // The engine gates activation on `turn_count > placed_on_turn`; bump the
    // turn counter forward so activation is legal without a full end_turn
    // cycle (keeps the fixture deterministic and independent of phase state).
    if runner.game.turn_count <= 0 {
        runner.game.turn_count = 1;
    }
    handle
}

fn runner_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-205 YAML must parse and compile")
        .add_card({
            let mut c = filler("P-205");
            c.card_kind = CardKind::Option;
            c.colors = vec![CardColor::Purple];
            c.play_cost = 3;
            c.traits = vec!["DM".to_string()];
            c
        })
        // Neutral deck-filler card, distinct from the real "P-205" card ID —
        // never reuse "P-205" as opponent deck padding (it would register a
        // second copy of the card under test as a draw candidate).
        .add_card(filler("FILL"))
}

fn runner() -> DebugRunner {
    runner_builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p_205_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-205 YAML must parse and compile without errors");
}

#[test]
fn p_205_is_option_cost_3_purple_with_dm_trait() {
    let runner = runner();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("P-205 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "P-205 must be an Option card"
    );
    assert_eq!(card.cost, Some(3), "P-205 prints Cost 3");
    assert!(
        card.traits.iter().any(|t| t.eq_ignore_ascii_case("DM")),
        "P-205 must carry the DM trait"
    );
}

/// Four clauses total:
///   [0] flood_gate (declarative, IgnoreColorRequirement, conditional on DM)
///   [1] main_from_hand (triggered, mandatory draw-2/trash-2/place-self)
///   [2] delay (declarative, player-activated Delay)
///   [3] inherited on_security (triggered, mandatory mirror of clause 1)
#[test]
fn p_205_has_four_clauses_in_expected_order() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("P-205 compiled");
    assert_eq!(
        card.effects.len(),
        4,
        "expected 4 clauses (flood_gate, main_from_hand, delay, inherited on_security); got {}",
        card.effects.len()
    );
}

#[test]
fn p_205_clause_0_is_flood_gate_with_ignore_color_modifier() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("P-205 compiled");

    let is_flood_gate_with_modifier = match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. }) => {
            modifier.eq_ignore_ascii_case("IgnoreColorRequirement")
        }
        _ => false,
    };
    assert!(
        is_flood_gate_with_modifier,
        "clause 0 must be a flood_gate carrying IgnoreColorRequirement; got {:?}",
        card.effects[0]
    );
}

#[test]
fn p_205_clause_1_main_from_hand_face_up_mandatory() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("P-205 compiled");

    match &card.effects[1] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 1 must fire at MainFromHand; got {:?}",
                t.when
            );
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "clause 1 must have FaceUp scope"
            );
            assert!(
                !t.optional,
                "clause 1 outer trigger is NOT optional — no 'you may' in printed text; \
                 DCGO SetUpActivateClass(..., false, ...)"
            );
        }
        other => panic!(
            "clause 1 must be Triggered(main_from_hand); got {:?}",
            other
        ),
    }
}

#[test]
fn p_205_clause_1_process_draws_2_then_trashes_2_then_places_self() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("P-205 compiled");

    match &card.effects[1] {
        CompiledClause::Triggered(t) => {
            assert!(
                matches!(t.process.first(), Some(CompiledStep::Draw { count: 2, .. })),
                "[Main] must start with Draw 2; got {:?}",
                t.process.first()
            );
            let trash_count = t
                .process
                .iter()
                .filter(|s| matches!(s, CompiledStep::TrashFromHandByIndex { .. }))
                .count();
            assert_eq!(
                trash_count, 2,
                "[Main] must trash exactly 2 cards from hand via 2 sequential steps; got {trash_count}"
            );
            assert!(
                t.process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)),
                "[Main] must place this card in the battle area as a Delay permanent"
            );
        }
        other => panic!("clause 1 must be Triggered; got {:?}", other),
    }
}

#[test]
fn p_205_clause_2_is_player_activated_delay() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("P-205 compiled");

    match &card.effects[2] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
            assert_eq!(
                *trigger,
                CompiledTiming::Delayed,
                "Delay trigger must be the player-activated standard <Delay> \
                 (DCGO EffectTiming.OnDeclaration / CanDeclareOptionDelayEffect); got {:?}",
                trigger
            );
        }
        other => panic!("clause 2 must be Declarative(Delay); got {:?}", other),
    }
}

#[test]
fn p_205_clause_2_process_deletes_then_may_play_from_trash_cost_minus_3() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("P-205 compiled");

    match &card.effects[2] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            assert!(
                process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::SelectOwnPermanent { .. })),
                "Delay body must select 1 of your Digimon to delete (the 'By deleting' cost)"
            );
            assert!(
                process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
                "Delay body must delete the selected Digimon"
            );
            // The "you may play from trash" payoff must be nested behind a
            // conditional gate (only fires if the deletion cost was paid) —
            // asserted behaviorally in Section 3; here we assert the
            // PlayFromTrash step with a Reduce(3) cost delta exists somewhere
            // in the (possibly nested) process via a recursive scan.
            fn contains_play_from_trash_reduce_3(steps: &[CompiledStep]) -> bool {
                steps.iter().any(|s| match s {
                    CompiledStep::PlayFromTrash {
                        cost_delta: Some(CompiledCostDelta::Reduce(3)),
                        ..
                    } => true,
                    CompiledStep::If { then, .. } => contains_play_from_trash_reduce_3(then),
                    _ => false,
                })
            }
            assert!(
                contains_play_from_trash_reduce_3(process),
                "Delay body must play the selected trash card with cost reduced by 3; got {:?}",
                process
            );
        }
        other => panic!("clause 2 must be Delay; got {:?}", other),
    }
}

#[test]
fn p_205_clause_3_inherited_security_mirrors_draw_trash_place_self() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("P-205 compiled");

    match &card.effects[3] {
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 3 must have Inherited scope"
            );
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 3 must fire at OnSecurity; got {:?}",
                t.when
            );
            assert!(
                !t.optional,
                "[Security] effects are not opt-out (RULES_CONTEXT.md §16); \
                 DCGO SecuritySkill shares the same non-optional DrawAndDiscardCards coroutine"
            );
            assert!(
                matches!(t.process.first(), Some(CompiledStep::Draw { count: 2, .. })),
                "[Security] must start with Draw 2"
            );
            let trash_count = t
                .process
                .iter()
                .filter(|s| matches!(s, CompiledStep::TrashFromHandByIndex { .. }))
                .count();
            assert_eq!(
                trash_count, 2,
                "[Security] must trash exactly 2 cards from hand"
            );
            assert!(
                t.process
                    .iter()
                    .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)),
                "[Security] must place this card in the battle area"
            );
        }
        other => panic!(
            "clause 3 must be Triggered(inherited on_security); got {:?}",
            other
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 0 color-bypass gate: positive/negative
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with ONLY an own DM-trait Digimon on the field (no purple
/// board presence at all), P-205 (Purple) is legal to play from hand — the
/// from-hand color-requirement bypass (`use_requirement:`) is genuinely
/// wired to the play-time gate, not just the battle-area flood_gate.
#[test]
fn p_205_dm_digimon_on_field_bypasses_color_requirement_to_play_from_hand() {
    let mut runner = runner_builder()
        .add_card(make_dm_digimon("DM-DIGI", "DM Digimon"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    // DM-DIGI here is colored Purple by `make_dm_digimon`'s default — swap to
    // a non-Purple color so this test isolates the DM-trait bypass path from
    // an incidental color match.
    let dm_digi_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DM-DIGI")
        .expect("DM-DIGI registered");
    runner.game.card_data[dm_digi_idx].colors = vec![CardColor::Yellow];

    runner.place_on_field(0, "DM-DIGI", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, digimon_engine::selection::OptionPlayResult::Pending),
        "P-205 (Purple) must be playable from hand with only a YELLOW DM-trait \
         Digimon on the field — the DM color-bypass must apply; got {result:?}"
    );
}

/// Positive (Tamer branch): an own DM-trait TAMER (not Digimon) on the field
/// also satisfies the printed "Digimon or Tamer" disjunction.
#[test]
fn p_205_dm_tamer_on_field_bypasses_color_requirement_to_play_from_hand() {
    let mut runner = runner_builder()
        .add_card(make_dm_tamer("DM-TAMER", "DM Tamer"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    let dm_tamer_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DM-TAMER")
        .expect("DM-TAMER registered");
    runner.game.card_data[dm_tamer_idx].colors = vec![CardColor::Yellow];

    runner.place_on_field(0, "DM-TAMER", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, digimon_engine::selection::OptionPlayResult::Pending),
        "P-205 (Purple) must be playable from hand with only a YELLOW DM-trait \
         Tamer on the field; got {result:?}"
    );
}

/// Negative: a non-DM-trait Digimon of a DIFFERENT color does not satisfy
/// either the color match or the DM-trait bypass — P-205 is illegal to play.
#[test]
fn p_205_non_dm_off_color_digimon_does_not_bypass_color_requirement() {
    let mut runner = runner_builder()
        .add_card(make_non_dm_digimon("OFFCOLOR", "Off-color Digimon"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    let offcolor_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OFFCOLOR")
        .expect("OFFCOLOR registered");
    runner.game.card_data[offcolor_idx].colors = vec![CardColor::Yellow];

    runner.place_on_field(0, "OFFCOLOR", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, digimon_engine::selection::OptionPlayResult::Invalid),
        "P-205 (Purple) must be ILLEGAL to play from hand with only a non-DM, \
         non-Purple Digimon on the field; got {result:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 1 [Main] Draw 2 / trash 2 / place self
// ═══════════════════════════════════════════════════════════════════════════

/// A plain purple filler Digimon — establishes color availability so P-205
/// (Purple) is legal to play from hand without depending on the DM-trait
/// color-bypass gate (that gate is exercised separately in Section 2).
fn purple_filler(id: &str) -> CardData {
    let mut c = filler(id);
    c.colors = vec![CardColor::Purple];
    c
}

#[test]
fn p_205_main_draws_2_then_prompts_two_mandatory_trashes_then_places_self() {
    let mut runner = runner_builder()
        .add_card(purple_filler("PURPLE-ENABLER"))
        .add_card(filler("HAND-A"))
        .add_card(filler("HAND-B"))
        .add_card(filler("DECK-DRAW-1"))
        .add_card(filler("DECK-DRAW-2"))
        .hand(0, &[CARD_ID, "HAND-A", "HAND-B"])
        .deck(0, &["DECK-DRAW-2", "DECK-DRAW-1"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "PURPLE-ENABLER", Some(0));

    // `play_option_from_hand` removes P-205 from hand BEFORE the [Main] body
    // runs (matches DCGO's play pipeline — the Option leaves hand as part of
    // being played), so the mandatory trash-2 never sees P-205 itself as a
    // candidate. `hand_before` is measured on the 2 OTHER cards only.
    let hand_before = runner.hand_size(0) - 1;
    let result = runner.game.play_option_from_hand(0, 0);
    assert!(
        matches!(result, digimon_engine::selection::OptionPlayResult::Pending),
        "playing P-205 must park on the mandatory trash selection; got {result:?}"
    );

    // Draw resolves immediately (not a selection); hand grew by 2 (draw) then
    // the mandatory trash prompt installs. P-205 itself already left the hand.
    assert_eq!(
        runner.hand_size(0),
        hand_before + 2,
        "hand must grow by 2 immediately after Draw 2 (trashes not yet applied)"
    );

    let view1 = runner
        .pending_selection_view()
        .expect("first mandatory trash prompt must install");
    assert_eq!(view1.kind, SelectionKind::Hand);
    assert!(
        !runner.pending_is_optional(),
        "the printed trash-2 has no 'you may' — mandatory when hand cards exist"
    );
    runner
        .execute_action(0, view1.valid_action_ids[0])
        .expect("trash first card");

    let view2 = runner
        .pending_selection_view()
        .expect("second mandatory trash prompt must install");
    assert_eq!(view2.kind, SelectionKind::Hand);
    runner
        .execute_action(0, view2.valid_action_ids[0])
        .expect("trash second card");

    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before, // +2 draw, -2 trash, and P-205 itself already left the hand via place_self
        "net hand size: +2 draw, -2 trash (P-205 leaves hand via place-self, not counted in trash)"
    );
    assert_eq!(
        runner.trash_size(0),
        2,
        "exactly 2 cards must land in the trash from the mandatory discard"
    );
    assert!(
        runner.game.players[0].battle_area.iter().any(|perm| {
            perm.top_card().card_id(&runner.game.card_data) == CARD_ID
                && matches!(perm.option_state, OptionState::Delayed { .. })
        }),
        "P-205 must be placed in the battle area as a Delay permanent"
    );
}

/// When the deck is empty (Draw 2 draws 0 cards) and only 1 other card sits
/// in hand at the time [Main] resolves, the "trash 2" degrades to trashing
/// however many remain — mirrors BT21-071's Math.Min(2, hand.Count) idiom.
/// The first trash prompt offers the sole ONLY-OTHER card; the second
/// degrades to zero eligible candidates and silently resolves.
#[test]
fn p_205_main_trash_degrades_when_hand_has_only_one_other_card() {
    let mut runner = runner_builder()
        .add_card(purple_filler("PURPLE-ENABLER"))
        .add_card(filler("ONLY-OTHER"))
        .hand(0, &[CARD_ID, "ONLY-OTHER"])
        .deck(0, &[]) // empty deck: Draw 2 draws 0 cards (no deck-out check mid-effect)
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "PURPLE-ENABLER", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert!(matches!(
        result,
        digimon_engine::selection::OptionPlayResult::Pending
    ));

    assert_eq!(
        runner.hand_size(0),
        1,
        "Draw 2 from an empty deck draws 0 cards; only ONLY-OTHER remains in hand"
    );

    let view1 = runner
        .pending_selection_view()
        .expect("first trash prompt installs");
    assert_eq!(
        view1.valid_action_ids.len(),
        1,
        "only ONLY-OTHER is trashable — P-205 already left the hand when played, deck was empty"
    );
    runner
        .execute_action(0, view1.valid_action_ids[0])
        .expect("trash the only other card");

    // Second trash prompt: hand is now empty. The second select_hand
    // degrades to zero eligible candidates and silently resolves without a
    // prompt — mirrors DCGO's Math.Min(2, hand.Count).
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area.iter().any(|perm| {
            perm.top_card().card_id(&runner.game.card_data) == CARD_ID
                && matches!(perm.option_state, OptionState::Delayed { .. })
        }),
        "P-205 must still be placed in the battle area even when the trash-2 degrades"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral: Clause 2 [Main] <Delay> — deletion cost + payoff
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: an eligible own play-cost<=7 Digimon exists. Paying the
/// "By deleting" cost installs the trash-recursion payoff prompt.
#[test]
fn p_205_delay_offers_deletion_cost_when_eligible_digimon_exists() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("CHEAP", "Cheap Digimon", 7))
        .hand(0, &["CHEAP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "CHEAP", Some(0));

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    assert!(runner.game.activate_delayed_option_main(handle));

    let view = runner
        .pending_selection_view()
        .expect("deletion-cost selection must install");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "'By deleting' cost is an Opt-cost→Mand pattern — the cost itself is declinable"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only CHEAP (play cost 7) is eligible"
    );
}

/// Negative: no own Digimon with play cost <= 7 exists (only an
/// above-threshold Digimon) — the deletion-cost prompt must NOT install, and
/// nothing is deleted or played.
#[test]
fn p_205_delay_no_eligible_digimon_above_cost_7_installs_no_prompt() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("EXPENSIVE", "Expensive Digimon", 8))
        .hand(0, &["EXPENSIVE"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "EXPENSIVE", Some(0));

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    let field_size_before = runner.battle_area_size(0);
    assert!(runner.game.activate_delayed_option_main(handle));

    assert!(
        runner.pending_selection().is_none(),
        "no play-cost<=7 Digimon exists — the deletion-cost prompt must not install"
    );
    // P-205 itself is trashed as the standard Delay activation cost, but
    // EXPENSIVE must survive untouched.
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EXPENSIVE"),
        "EXPENSIVE (cost 8, ineligible) must survive"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_size_before - 1,
        "only P-205 itself (the Delay activation cost) leaves the field"
    );
}

/// Negative (decline the deletion cost): the player may decline the "By
/// deleting" cost entirely — nothing is deleted, no trash-play payoff fires.
#[test]
fn p_205_delay_declining_deletion_cost_skips_the_entire_payoff() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("CHEAP", "Cheap Digimon", 7))
        .add_card(kimeramon_named("KIM-TRASH", 8))
        .hand(0, &["CHEAP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "CHEAP", Some(0));
    push_to_trash(&mut runner, "KIM-TRASH");

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    assert!(runner.game.activate_delayed_option_main(handle));
    runner
        .execute_action(0, PASS)
        .expect("decline the 'By deleting' cost");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CHEAP"),
        "declining the cost leaves CHEAP undeleted"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "KIM-TRASH"),
        "declining the cost means the trash-recursion payoff never fires — Kimeramon stays in trash"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"KIM-TRASH".to_string()),
        "KIM-TRASH must remain in trash when the cost was declined"
    );
}

/// Positive end-to-end: pay the deletion cost, then accept the "you may play"
/// payoff on a Kimeramon-named trash card. The played card's cost is reduced
/// by 3.
#[test]
fn p_205_delay_deletes_then_plays_kimeramon_named_card_with_cost_reduced_by_3() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("CHEAP", "Cheap Digimon", 7))
        .add_card(kimeramon_named("KIM-TRASH", 8))
        .hand(0, &["CHEAP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "CHEAP", Some(0));
    push_to_trash(&mut runner, "KIM-TRASH");
    runner.game.set_memory(10);

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    assert!(runner.game.activate_delayed_option_main(handle));

    // Pay the deletion cost — assert the trash-bound side effect fires via
    // the event log (the "By deleting" cost is a real state change, not a
    // side-effect-free formality).
    let del_view = runner
        .pending_selection_view()
        .expect("deletion-cost prompt");
    let cp = runner.event_checkpoint();
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete CHEAP");
    let deletion_events = runner.events_of_kind(cp, |e| {
        matches!(e, digimon_engine::events::GameEvent::Trash { card_id, .. } if card_id == "CHEAP")
    });
    assert_eq!(
        deletion_events.len(),
        1,
        "deleting CHEAP as the Delay cost must fire exactly one Trash event for CHEAP"
    );

    // Payoff: play prompt over trash Kimeramon/Millenniummon-named cards.
    let play_view = runner
        .pending_selection_view()
        .expect("trash-recursion play prompt must install after paying the deletion cost");
    assert_eq!(play_view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "printed 'you may play' — the payoff is independently declinable"
    );
    assert_eq!(
        play_view.valid_action_ids.len(),
        1,
        "only KIM-TRASH is an eligible Kimeramon/Millenniummon-named trash Digimon"
    );

    let memory_before = runner.memory();
    runner
        .execute_action(0, play_view.valid_action_ids[0])
        .expect("play KIM-TRASH from trash");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "KIM-TRASH"),
        "KIM-TRASH must be played onto the field"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CHEAP"),
        "CHEAP must have been deleted as the cost"
    );
    // KIM-TRASH costs 8; reduced by 3 = 5 memory spent (Free is NOT correct).
    assert_eq!(
        memory_before - runner.memory(),
        5,
        "playing KIM-TRASH (cost 8) must cost exactly 5 memory (8 - 3 reduction), not free"
    );
}

/// Positive: a Digimon whose name contains "Millenniummon" as a substring
/// (mirrors real cards like MoonMillenniummon / ZeedMillenniummon) is also a
/// legal payoff target.
#[test]
fn p_205_delay_plays_millenniummon_substring_named_card() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("CHEAP", "Cheap Digimon", 7))
        .add_card(millenniummon_named("MOON-MILLEN", "MoonMillenniummon", 6))
        .hand(0, &["CHEAP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "CHEAP", Some(0));
    push_to_trash(&mut runner, "MOON-MILLEN");

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    assert!(runner.game.activate_delayed_option_main(handle));
    let del_view = runner.pending_selection_view().expect("deletion prompt");
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete CHEAP");

    let play_view = runner
        .pending_selection_view()
        .expect("trash-recursion play prompt");
    assert_eq!(
        play_view.valid_action_ids.len(),
        1,
        "MoonMillenniummon must be eligible via the Millenniummon-substring name filter"
    );
    runner
        .execute_action(0, play_view.valid_action_ids[0])
        .expect("play MoonMillenniummon");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "MOON-MILLEN"),
        "MOON-MILLEN must be played onto the field"
    );
}

/// Negative (payoff filter): a trash Digimon whose name matches NEITHER
/// Kimeramon nor Millenniummon is not offered by the payoff prompt.
#[test]
fn p_205_delay_payoff_excludes_unrelated_named_digimon() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("CHEAP", "Cheap Digimon", 7))
        .add_card(unrelated_digimon("AGU-TRASH", 5))
        .hand(0, &["CHEAP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "CHEAP", Some(0));
    push_to_trash(&mut runner, "AGU-TRASH");

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    assert!(runner.game.activate_delayed_option_main(handle));
    let del_view = runner.pending_selection_view().expect("deletion prompt");
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete CHEAP");

    // No eligible Kimeramon/Millenniummon trash target — payoff prompt must
    // not install at all (empty candidate pool short-circuits silently).
    assert!(
        runner.pending_selection().is_none(),
        "AGU-TRASH (unrelated name) must not be offered by the payoff prompt"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"AGU-TRASH".to_string()),
        "AGU-TRASH must remain untouched in trash"
    );
}

/// Negative (kind gate closes DCGO's operator-precedence bug): a TAMER card
/// whose printed name contains "Kimeramon" must NOT be a legal payoff target.
/// Printed text is "1 Digimon card with [Kimeramon] or [Millenniummon] in
/// its name" — the [Digimon] kind gate applies to BOTH name branches. Under
/// DCGO's literal (buggy) C# operator precedence
/// (`IsDigimon && Kimeramon || Millenniummon && CanPlay`), a card matching
/// only the "Millenniummon" branch does not require IsDigimon — but a
/// "Kimeramon" match under DCGO's actual code DOES still require IsDigimon
/// (that branch is unaffected by the precedence bug). This test exercises
/// the *other* half of the precedent risk directly relevant to
/// faithfulness: our filter is `kind: digimon AND (name has Kimeramon OR
/// name has Millenniummon)` — i.e. `kind: digimon` gates BOTH names
/// (matching the obviously-intended printed semantics, per source priority
/// printed-text-wins over the DCGO precedence quirk).
#[test]
fn p_205_delay_payoff_excludes_non_digimon_kimeramon_named_card() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("CHEAP", "Cheap Digimon", 7))
        .add_card(kimeramon_named_tamer("KIM-TAMER"))
        .hand(0, &["CHEAP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "CHEAP", Some(0));
    push_to_trash(&mut runner, "KIM-TAMER");

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    assert!(runner.game.activate_delayed_option_main(handle));
    let del_view = runner.pending_selection_view().expect("deletion prompt");
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete CHEAP");

    assert!(
        runner.pending_selection().is_none(),
        "a Tamer named 'Kimeramon Tactician' must NOT be offered — the [Digimon] kind gate \
         applies to the Kimeramon name branch too, per printed text"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"KIM-TAMER".to_string()),
        "KIM-TAMER (a Tamer) must remain untouched in trash"
    );
}

/// Negative (decline the payoff after paying the cost): pay the deletion
/// cost, then decline the "you may play" payoff — the deleted Digimon stays
/// gone, but nothing is played from trash.
#[test]
fn p_205_delay_can_decline_the_payoff_after_paying_deletion_cost() {
    let mut runner = runner_builder()
        .add_card(digimon_with_cost("CHEAP", "Cheap Digimon", 7))
        .add_card(kimeramon_named("KIM-TRASH", 8))
        .hand(0, &["CHEAP"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_p205_as_mature_delay(&mut runner);
    runner.place_on_field(0, "CHEAP", Some(0));
    push_to_trash(&mut runner, "KIM-TRASH");

    let handle = {
        let idx = runner.game.players[0]
            .battle_area
            .iter()
            .position(|perm| perm.top_card().card_id(&runner.game.card_data) == CARD_ID)
            .expect("P-205 on field");
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };
    assert!(runner.game.activate_delayed_option_main(handle));
    let del_view = runner.pending_selection_view().expect("deletion prompt");
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete CHEAP");

    let _play_view = runner
        .pending_selection_view()
        .expect("payoff prompt installs");
    assert!(
        runner.pending_is_optional(),
        "the payoff selection must be declinable"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline the trash-recursion payoff");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CHEAP"),
        "CHEAP stays deleted — the deletion cost was already paid"
    );
    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "KIM-TRASH"),
        "declining the payoff means KIM-TRASH is NOT played"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"KIM-TRASH".to_string()),
        "KIM-TRASH remains in trash after declining"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Behavioral: Clause 3 [Security] (inherited) mirror
// ═══════════════════════════════════════════════════════════════════════════

/// Security check: P-205 hitting security fires the same Draw 2 / trash 2 /
/// place-self sequence as [Main].
#[test]
fn p_205_security_draws_2_trashes_2_and_places_self_in_battle_area() {
    use digimon_engine::combat::AttackResult;

    let mut runner = runner_builder()
        .add_card(filler("ATTACKER"))
        .add_card(filler("DEF-HAND-A"))
        .add_card(filler("DEF-HAND-B"))
        .add_card(filler("DEF-DECK-1"))
        .add_card(filler("DEF-DECK-2"))
        .security(1, &["P-205"])
        .hand(1, &["DEF-HAND-A", "DEF-HAND-B"])
        .deck(0, &["FILL"])
        .deck(1, &["DEF-DECK-2", "DEF-DECK-1"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let hand_before = runner.hand_size(1);
    let result = runner.attack_player(attacker, 1, false);
    // The security-triggered Draw 2 / trash 2 installs a pending selection
    // mid-resolution, so the attack pauses (`InProgress`) rather than
    // resolving to a terminal `AttackResult` immediately — the final result
    // is only observable after the trash prompts are driven to completion.
    assert_eq!(result, AttackResult::InProgress);

    // Draw 2 resolves immediately.
    assert_eq!(
        runner.hand_size(1),
        hand_before + 2,
        "security-triggered Draw 2 must land in the defender's hand"
    );

    // Two mandatory trash prompts for the defending player (player 1).
    let view1 = runner
        .pending_selection_view()
        .expect("first mandatory trash prompt (security) must install");
    assert_eq!(view1.selecting_player, 1);
    runner
        .execute_action(1, view1.valid_action_ids[0])
        .expect("trash first card");
    let view2 = runner
        .pending_selection_view()
        .expect("second mandatory trash prompt (security) must install");
    runner
        .execute_action(1, view2.valid_action_ids[0])
        .expect("trash second card");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        0,
        "P-205 left security via its own Security Effect (no card added to security stack)"
    );
    assert!(
        runner.game.players[1].battle_area.iter().any(|perm| {
            perm.top_card().card_id(&runner.game.card_data) == CARD_ID
                && matches!(perm.option_state, OptionState::Delayed { .. })
        }),
        "P-205 must be placed in the defender's battle area as a Delay permanent"
    );
    assert_eq!(
        runner.trash_size(1),
        2,
        "exactly 2 cards must be trashed by the security-triggered effect"
    );
}
