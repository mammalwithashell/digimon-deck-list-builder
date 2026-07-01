//! Ice-Snow (Suzune Kazuki) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/ice-snow-model.md`. The deck's whole plan is to strip
//! the opponent's **digivolution cards** (the sources under their Digimon), then
//! win with &lt;Iceclad&gt; (compare stack counts, not DP) and the "while your
//! opponent has no Digimon with digivolution cards" payoffs.
//!
//! This file pins the **multi-card** interactions the per-card
//! `cards_behavioral/<set>/` tests can't see — specifically the two things the
//! deck lives or dies on, and the two the user asked to lock down:
//!   1. **Digivolution COSTS** — the reduced-cost ramp (EX8-019 Penguinmon's
//!      `[Your Turn]` cost reduction) applied to a *real climb* whose payoff
//!      then fires, vs the control climb with no reducer.
//!   2. **`[When Digivolving]` effects ACTUALLY FIRE** — the digivolve removes
//!      the opponent's sources, and the conditional "opp sourceless" riders gate
//!      correctly; the Suzune Kazuki engine fires on an Ice-Snow digivolve.
//!
//! Each test maps 1:1 to a combo in the model. Real implemented DSL cards fill
//! every archetype-piece role (EX8-019, EX7-020, EX8-022, EX8-066); synthetic
//! `make_test_card` is used ONLY for the neutral stack-base / opponent-source /
//! vanilla-target roles no real Ice-Snow card can fill without dragging in its
//! own effects (commented at each use), per the harness's evo_costs caveat
//! (`reference-debugrunner-empty-evo-costs`: DSL cards' printed evo_costs are
//! backfilled from YAML `alt_paths`; a synthetic base lets us pin which path the
//! engine charges).
//!
//! Sources: card images (printed text) + `cards.json`; DCGO C# crosschecks in
//! each card's YAML header; `general_rule.pdf` §16 (Iceclad / Jamming / Blocker).

#![allow(dead_code)]

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

/// `evo_costs[*].card_color` integer for blue (cards.json convention; matches
/// the `EVO_BLUE` used by the other archetype suites).
const EVO_BLUE: u8 = 1;

// ─── Synthetic-role fixtures (neutral / vanilla only — see header) ───────────

/// A blue Lv.3 Digimon WITHOUT the [Ice-Snow] trait, carrying an explicit
/// blue-Lv.3 evo box. Used as a digivolve BASE when we must force a real
/// target's *blue* evo path (not its alt Ice-Snow path) and ensure NO cost
/// reducer is present — no real non-Ice-Snow blue Lv.3 belongs to this deck.
fn plain_blue_lv3_base(id: &str) -> CardData {
    let mut c = make_test_card(id, "Plain Blue Lv3");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(4000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Beast".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: EVO_BLUE,
        level: 3,
        memory_cost: 0,
    }];
    c
}

/// A blue Lv.4 Digimon WITHOUT [Ice-Snow] — forces a real target's *blue* evo
/// path (the standard, non-reduced box) for the alt-cost contrast test.
fn plain_blue_lv4_base(id: &str) -> CardData {
    let mut c = make_test_card(id, "Plain Blue Lv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Beast".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: EVO_BLUE,
        level: 4,
        memory_cost: 0,
    }];
    c
}

/// A RED Lv.4 [Ice-Snow] Digimon — matches ONLY a target's "Lv.4 w/[Ice-Snow]"
/// alt evo box (not its blue box), so the reduced printed cost is unambiguous.
fn red_ice_snow_lv4_base(id: &str) -> CardData {
    let mut c = make_test_card(id, "Red IceSnow Lv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A vanilla (effectless) Lv.4 [Ice-Snow] Digimon that digivolves from a blue
/// Lv.3 — the neutral digivolve TARGET for the Suzune engine test, so ONLY the
/// Tamer's observer fires (a real Ice-Snow Lv.4 would drag in its own [WD]).
fn ice_snow_lv4_vanilla(id: &str) -> CardData {
    let mut c = make_test_card(id, "IceSnow Lv4 vanilla");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: EVO_BLUE,
        level: 3,
        memory_cost: 2,
    }];
    c
}

/// A generic digivolution-source filler (stacked under an opponent top card).
fn source_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Source Filler");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![CardColor::Red];
    c
}

/// A generic opponent TOP Digimon (carries the stacked sources).
fn opp_top(id: &str) -> CardData {
    let mut c = make_test_card(id, "Opp Top");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.colors = vec![CardColor::Red];
    c
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Put a registered card into player 0's hand; return its hand index.
fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    use digimon_engine::card_source::CardSource;
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("put_in_hand: card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

/// Number of digivolution-source cards under a battle-area permanent (its
/// `card_sources` minus the top card).
fn source_count(runner: &DebugRunner, handle: PermanentHandle) -> usize {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .len()
        .saturating_sub(1)
}

// ─────────────────────────────────────────────────────────────────────────────
// Combo C + A — cost-reduction ramp that ALSO fires a [When Digivolving] strip
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-019 Penguinmon's `[Your Turn]` "when this Digimon would digivolve into a
/// Digimon card with the [Ice-Snow] trait, reduce the digivolution cost by 1"
/// applies to a REAL climb into EX7-020 Paledramon (printed blue-Lv.3 evo box,
/// cost 2; Paledramon is [Ice-Snow], so the reducer fires → cost **1**). Then
/// Paledramon's own `[When Digivolving]` strips the opponent's bottom 2 sources.
///
/// This is a genuine two-card interaction: the cost reducer (on the BASE) and
/// the removal payoff (on the TARGET) compound in a single digivolve — the
/// per-card tests, which digivolve into synthetic targets, never see the two
/// together. EX7-020 is deliberately chosen because its only evo path is
/// blue-Lv.3 (no alt Ice-Snow box to shadow the cost), so the charged cost is
/// unambiguous.
///
/// - Card text: cards.json EX8-019 / EX7-020.
/// - DCGO: `EX8_019.cs` (BeforePayCost reduction), `EX7_020.cs` (trash bottom 2).
#[test]
fn ice_snow_ex8019_ramp_into_paledramon_reduces_cost_then_strips_two_sources() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-019")
        .expect("EX8-019 in DSL pack")
        .dsl_card("EX7-020")
        .expect("EX7-020 in DSL pack")
        .add_card(source_filler("OPP-S1"))
        .add_card(source_filler("OPP-S2"))
        .add_card(source_filler("OPP-S3"))
        .add_card(opp_top("OPP-TOP"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Opponent Digimon with 3 sources (bottom → top: S1, S2, S3, then TOP).
    let opp = runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-S3", "OPP-TOP"]);
    assert_eq!(
        source_count(&runner, opp),
        3,
        "opponent starts with 3 sources"
    );

    // EX8-019 Penguinmon on field; EX7-020 Paledramon in hand to digivolve over it.
    let base = runner.place_on_field(0, "EX8-019", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "EX7-020");

    let memory_before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "EX8-019 (blue Lv.3 [Ice-Snow]) must digivolve into EX7-020 via its blue-Lv.3 evo box"
    );

    // COST: blue-Lv.3 box is 2; EX8-019 reduces an [Ice-Snow] digivolution by 1.
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "the printed cost 2 is reduced to 1 by EX8-019's [Your Turn] cost reduction"
    );

    // [WHEN DIGIVOLVING] FIRES: Paledramon trashes the opponent's bottom 2 sources.
    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("Paledramon's [When Digivolving] must offer the opponent Digimon to strip");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon to trash its bottom 2 sources");
    let _ = runner.auto_resolve();

    let opp = PermanentHandle {
        player: 1,
        index: 0,
    };
    assert_eq!(
        source_count(&runner, opp),
        1,
        "the bottom 2 of 3 sources are trashed; the top card + 1 source remain"
    );
}

/// CONTROL: the same climb into EX7-020 from a NON-reducer, non-[Ice-Snow] blue
/// Lv.3 base charges the FULL printed cost 2 — proving the −1 above is EX8-019's
/// reduction, not a quirk of the path.
#[test]
fn ice_snow_paledramon_charges_full_cost_without_the_ex8019_reducer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-020")
        .expect("EX7-020 in DSL pack")
        // Synthetic neutral base: no Ice-Snow trait (so no reducer) + a blue-Lv.3
        // evo box so the climb is legal — no real non-Ice-Snow blue Lv.3 fits.
        .add_card(plain_blue_lv3_base("BASE"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let base = runner.place_on_field(0, "BASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "EX7-020");

    let memory_before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "the plain blue Lv.3 base must digivolve into EX7-020 via its blue-Lv.3 evo box"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 2,
        "with no EX8-019 reducer present, the full printed cost 2 is charged"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Combo A — EX8-022 Frigimon [When Digivolving] strip + sourceless rider gating
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-022 Frigimon's `[On Play][When Digivolving]` "trash the bottom 2
/// digivolution cards from 1 of your opponent's Digimon. Then, if your opponent
/// has no Digimon with digivolution cards, gain 1 memory." Climbing into it over
/// a real Ice-Snow base fires the strip; because the opponent KEEPS a source
/// (3 → 1), the conditional memory rider must NOT fire.
///
/// - Card text: cards.json EX8-022. DCGO `EX8_022.cs`.
#[test]
fn ice_snow_frigimon_ex8022_strips_two_and_withholds_memory_while_opponent_stays_sourced() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-022")
        .expect("EX8-022 in DSL pack")
        // Neutral blue Lv.3 base (forces EX8-022's blue evo box; no own effects).
        .add_card(plain_blue_lv3_base("BASE"))
        .add_card(source_filler("OPP-S1"))
        .add_card(source_filler("OPP-S2"))
        .add_card(source_filler("OPP-S3"))
        .add_card(opp_top("OPP-TOP"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let opp = runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-S3", "OPP-TOP"]);
    assert_eq!(source_count(&runner, opp), 3);

    let base = runner.place_on_field(0, "BASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "EX8-022");

    let memory_before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "the blue Lv.3 base must digivolve into EX8-022 (blue-Lv.3 evo box)"
    );
    let memory_after_cost = runner.game.memory;
    assert_eq!(
        memory_after_cost,
        memory_before - 3,
        "EX8-022's printed blue-Lv.3 evo box charges cost 3 (no reducer present)"
    );

    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("EX8-022's [When Digivolving] must offer the opponent Digimon to strip");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("strip the opponent's bottom 2 sources");
    let _ = runner.auto_resolve();

    let opp = PermanentHandle {
        player: 1,
        index: 0,
    };
    assert_eq!(
        source_count(&runner, opp),
        1,
        "bottom 2 of 3 sources trashed"
    );
    assert_eq!(
        runner.game.memory, memory_after_cost,
        "the opponent still has a sourced Digimon, so the +1-memory rider must NOT fire"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Combo B — Suzune Kazuki (EX8-066) trash engine fires on an Ice-Snow digivolve
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-066 Suzune Kazuki, `[All Turns]` "When your Digimon are played or
/// digivolve, if any of them have the [Ice-Snow] trait, by suspending this
/// Tamer, trash any 1 digivolution card from your opponent's Digimon." With
/// Suzune on the field, digivolving into a (vanilla) [Ice-Snow] Digimon fires the
/// observer: accepting the suspend cost trashes 1 opponent source.
///
/// The digivolve TARGET is a synthetic vanilla Ice-Snow Lv.4 so that ONLY the
/// Tamer's observer fires — a real Ice-Snow payoff would also fire its own [WD]
/// and confound the single-source assertion.
///
/// - Card text: cards.json EX8-066. DCGO `EX8_066.cs` (TriggerOrder decline gate
///   → suspend cost → SelectTrashDigivolutionCards over opponent permanents).
#[test]
fn ice_snow_suzune_ex8066_suspends_to_trash_a_source_on_an_ice_snow_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-066")
        .expect("EX8-066 in DSL pack")
        .add_card(plain_blue_lv3_base("BASE"))
        .add_card(ice_snow_lv4_vanilla("ICE-4"))
        .add_card(source_filler("OPP-S1"))
        .add_card(source_filler("OPP-S2"))
        .add_card(opp_top("OPP-TOP"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Suzune Kazuki on the field; an opponent Digimon with 2 sources to strip.
    let tamer = runner.place_on_field(0, "EX8-066", Some(0));
    let opp = runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-TOP"]);
    assert_eq!(source_count(&runner, opp), 2);

    let base = runner.place_on_field(0, "BASE", Some(1));
    let hand_idx = put_in_hand(&mut runner, 0, "ICE-4");

    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "the blue Lv.3 base digivolves into the vanilla [Ice-Snow] Lv.4"
    );
    runner.game.drain_effect_queue();

    // Suzune's observer surfaces as a TriggerOrder decline gate: PASS declines,
    // picking the entry pays the suspend cost and runs the trash body.
    let view = runner
        .pending_selection_view()
        .expect("Suzune's [All Turns] observer must offer on an Ice-Snow digivolve");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(
        view.is_optional,
        "\"by suspending this Tamer\" is a declinable cost"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept: pay the suspend cost");

    // The body installs the opponent-source trash pick.
    let pick = runner
        .pending_selection_view()
        .expect("accepting must install the opponent-source trash selection");
    runner
        .execute_action(0, pick.valid_action_ids[0])
        .expect("trash 1 opponent digivolution card");
    let _ = runner.auto_resolve();

    // Suzune (a Tamer at index 0) does not move, so its original handle holds.
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "paying the cost suspends Suzune Kazuki"
    );
    let opp = PermanentHandle {
        player: 1,
        index: 0,
    };
    assert_eq!(
        source_count(&runner, opp),
        1,
        "exactly 1 of the opponent's 2 sources is trashed by the engine"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Combo C — printed "alt [Ice-Snow] evo box" charges less than the blue box
// ─────────────────────────────────────────────────────────────────────────────

/// EX8-023 PolarBearmon prints two evo boxes: the standard "Lv.4 from blue: Cost
/// 4" and the reduced "Digivolve Lv.4 w/[Ice-Snow] trait: Cost 3". Digivolving
/// from an [Ice-Snow] Lv.4 must charge **3**; from a plain blue Lv.4, **4**.
/// This is the deck's other tempo lever (a printed reduced-cost path, distinct
/// from EX8-019's modifier) — the same card, two costs, chosen by the base's
/// trait.
///
/// The bases are synthetic-and-controlled so exactly ONE box matches each run
/// (a blue [Ice-Snow] base would match BOTH and make the charged cost
/// ambiguous). Cost is read before draining the [On Play]/[When Digivolving]
/// strip, so the payoff's own effects can't perturb the cost assertion.
///
/// - Card text: cards.json EX8-023. DCGO `EX8_023.cs` (alt digivolution req,
///   EqualsTraits("Ice-Snow") && Level == 4 → cost 3).
#[test]
fn ice_snow_ex8023_alt_ice_snow_evo_box_costs_one_less_than_the_blue_box() {
    // Alt [Ice-Snow] box: red Ice-Snow Lv.4 base → cost 3.
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-023")
        .expect("EX8-023 in DSL pack")
        .add_card(red_ice_snow_lv4_base("ICEBASE"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "ICEBASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "EX8-023");
    let before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "a Lv.4 [Ice-Snow] base must digivolve into EX8-023 via the alt evo box"
    );
    assert_eq!(
        runner.game.memory,
        before - 3,
        "the printed alt [Ice-Snow] evo box charges cost 3"
    );

    // Standard blue box: plain blue Lv.4 base → cost 4.
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-023")
        .expect("EX8-023 in DSL pack")
        .add_card(plain_blue_lv4_base("BLUEBASE"))
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "BLUEBASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "EX8-023");
    let before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "a plain blue Lv.4 base must digivolve into EX8-023 via the standard blue box"
    );
    assert_eq!(
        runner.game.memory,
        before - 4,
        "the standard blue evo box charges cost 4 (the alt [Ice-Snow] box does not match a non-Ice-Snow base)"
    );
}
