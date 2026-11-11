//! ST20-03 Birdramon — Digimon, Lv.4, Red, DP 4000, play cost 4.
//! Traits: Giant Bird, ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST20-03.md)
//!
//! ```text
//! [On Play] [When Digivolving] If your Tamers with the [ADVENTURE] trait have
//! 3 or more total colors, this Digimon may digivolve into a Digimon card with
//! the [ADVENTURE] trait in the hand without paying the cost.
//!
//! Inherited Effect
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! evo_costs:  Red Lv.3: Cost 2
//! xros_req:   [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_03.cs
//!   - Alternate Digivolution Requirement: `HasAdventureTraits && Level == 3`,
//!     cost 2, requirements not ignored.
//!   - [On Play] / [When Digivolving]: `isOptional: true`, gated on
//!     `Combinations.GetDifferenetColorCardCount(owner's ADVENTURE Tamers) >= 3`;
//!     body `DigivolveIntoHandOrTrashCard(payCost: false, requirements NOT
//!     ignored, isHand: true, cardCondition: IsDigimon && HasAdventureTraits)`.
//!   - Your Turn ESS: `ChangeSelfDPStaticEffect(+2000, inherited)`.
//!
//! # STATUS: PARTIAL — the [On Play]/[When Digivolving] clause is BLOCKED (DSL)
//!
//! The clause is gated on "the DISTINCT-colour count across your
//! [ADVENTURE]-trait Tamers is >= 3". The DSL has no boolean leaf for a
//! trait-filtered distinct-colour threshold (`distinct_tamer_colors_gte` is
//! unfiltered, `distinct_colors_count` is a formula `per:` aggregate only), so
//! the clause is not authored and the tests that exercise it are `#[ignore]`d
//! under the gap tag `G-DSL-DISTINCT-COLORS-FILTERED-GTE`. They assert the
//! printed behaviour and must pass unchanged once the vocabulary exists
//! (see the YAML header for the proposed shape).
//!
//! # Patterns (docs/RUST_DSL_TEST_API.md §4.3)
//! - D4 inherited [Your Turn] DP aura
//! - Alt-path digivolve (standard circle + xros_req box), cost deducted
//! - E  optional "may digivolve" follow-up gated by a board-state count (BLOCKED)
//!
//! The clause is optional but has no once-per-turn restriction, so no OPT
//! lockout test applies.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "ST20-03";

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st20_03_identity_matches_printed_card() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.name, "Birdramon");
    assert_eq!(c.level, Some(4));
    assert_eq!(c.dp, Some(4000));
    assert_eq!(c.cost, Some(4));
    let traits: Vec<String> = c.all_traits().iter().map(|t| t.to_lowercase()).collect();
    for want in ["giant bird", "adventure", "vaccine"] {
        assert!(
            traits.contains(&want.to_string()),
            "missing trait {want}: {traits:?}"
        );
    }
}

/// Two requirement-respecting cost-2 digivolve paths from Lv.3 (standard Red
/// circle + the xros_req Lv.3 [ADVENTURE] box).
#[test]
fn st20_03_declares_standard_and_xros_req_alt_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.alt_paths.len(), 2, "standard circle + xros_req box");
    for path in &c.alt_paths {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
        assert_eq!(path.cost, Some(CompiledCost::Literal(2)));
        assert!(
            !path.ignore_requirements,
            "DCGO: ignoreDigivolutionRequirement false"
        );
        assert_eq!(path.from.as_ref().expect("from").level_eq, Some(3));
    }
    assert!(
        c.alt_paths
            .iter()
            .any(|p| p.from.as_ref().unwrap().trait_has.as_deref() == Some("ADVENTURE")),
        "an [ADVENTURE] source path must exist"
    );
}

/// The inherited [Your Turn] +2000 DP aura is declared with inherited scope.
#[test]
fn st20_03_declares_inherited_plus_2000_dp_aura() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("compiles")
        .start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let aura = c
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                ..
            }) => Some((*scope, *dp_modifier)),
            _ => None,
        })
        .expect("inherited aura");
    assert_eq!(aura, (CompiledScope::Inherited, Some(2000)));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Inherited [Your Turn] +2000 DP
// ═══════════════════════════════════════════════════════════════════════════

fn carrier_runner() -> DebugRunner {
    let mut carrier = make_test_card_with_level("CARRIER", "Carrier", 5);
    carrier.dp = Some(5000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-03 compiles")
        .add_card(carrier)
        .memory(10)
        .start()
}

/// POSITIVE: Birdramon beneath a carrier gives it +2000 DP on your turn.
#[test]
fn st20_03_inherited_plus_2000_on_your_turn() {
    let mut runner = carrier_runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(runner.effective_dp(carrier), Some(7000));
}

/// NEGATIVE: [Your Turn] only — no bonus on the opponent's turn.
#[test]
fn st20_03_inherited_bonus_absent_on_opponents_turn() {
    let mut runner = carrier_runner();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(carrier), Some(5000));
}

/// NEGATIVE: an inherited effect does not apply to Birdramon itself while it is
/// the face-up top card.
#[test]
fn st20_03_face_up_birdramon_does_not_buff_itself() {
    let mut runner = carrier_runner();
    let birdramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(birdramon), Some(4000));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Alternate digivolution (evo_costs circle + xros_req box), cost 2
// ═══════════════════════════════════════════════════════════════════════════

fn base(id: &str, level: u8, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.colors = vec![color];
    c.dp = Some(3000);
    c.play_cost = 3;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// Try to digivolve Birdramon (in hand) onto `b`; returns (succeeded, memory spent).
fn try_digivolve(b: CardData) -> (bool, i16) {
    let base_id = b.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-03 in embedded DSL pack")
        .add_card(b)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();
    let handle = r.place_on_field(0, &base_id, Some(0));
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, 0, handle.index as usize, PlaySource::ByDigivolve);
    let _ = r.auto_resolve();
    (ok, before - r.memory())
}

/// Standard circle: Red Lv.3 -> cost 2.
#[test]
fn st20_03_digivolves_from_red_level_3_for_2() {
    assert_eq!(
        try_digivolve(base("RED-LV3", 3, CardColor::Red, false)),
        (true, 2)
    );
}

/// xros_req box: a non-red Lv.3 [ADVENTURE] Digimon -> cost 2.
#[test]
fn st20_03_digivolves_from_non_red_level_3_adventure_for_2() {
    assert_eq!(
        try_digivolve(base("BLUE-LV3-ADV", 3, CardColor::Blue, true)),
        (true, 2)
    );
}

/// NEGATIVE: a non-red Lv.3 without [ADVENTURE] is not a legal base.
#[test]
fn st20_03_rejects_non_red_level_3_without_adventure() {
    assert!(!try_digivolve(base("BLUE-LV3", 3, CardColor::Blue, false)).0);
}

/// NEGATIVE: the level still has to be 3 — a Lv.2 [ADVENTURE] base is rejected.
#[test]
fn st20_03_rejects_level_2_adventure_base() {
    assert!(!try_digivolve(base("RED-LV2-ADV", 2, CardColor::Red, true)).0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [On Play] / [When Digivolving] free digivolve (BLOCKED, ignored)
// ═══════════════════════════════════════════════════════════════════════════

const GAP: &str = "BLOCKED (DSL): G-DSL-DISTINCT-COLORS-FILTERED-GTE — no predicate for \
                   'distinct colors across [ADVENTURE] Tamers >= 3'; see ST20-03.yaml header";

/// A Tamer of one colour, optionally with the [ADVENTURE] trait.
fn tamer(id: &str, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// A Lv.5 red [ADVENTURE] Digimon that digivolves from Lv.4 red for cost 3, so a
/// "without paying the cost" digivolve is distinguishable from a paid one.
fn lv5_adventure(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 4,
        memory_cost: 3,
    }];
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

/// Birdramon in hand slot 0 and an [ADVENTURE] Lv.5 in hand slot 1, three
/// Tamers on the field with the given (colour, has-ADVENTURE) lines.
fn on_play_runner(tamers: &[(&str, CardColor, bool)], evo_adventure: bool) -> DebugRunner {
    on_play_runner_with(tamers, evo_adventure, Vec::new())
}

/// [`on_play_runner`] plus extra registered cards (e.g. a digivolve base).
fn on_play_runner_with(
    tamers: &[(&str, CardColor, bool)],
    evo_adventure: bool,
    extras: Vec<CardData>,
) -> DebugRunner {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-03 in embedded DSL pack")
        .add_card(lv5_adventure("EVO-LV5", evo_adventure))
        .add_card(make_test_card("FILLER", "Filler"));
    for extra in extras {
        builder = builder.add_card(extra);
    }
    for (id, color, adventure) in tamers {
        builder = builder.add_card(tamer(id, *color, *adventure));
    }
    let mut runner = builder
        .hand(0, &[CARD_ID, "EVO-LV5"])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();
    for (id, _, _) in tamers {
        runner.place_on_field(0, id, None);
    }
    runner
}

fn top_card_id(runner: &DebugRunner, handle: PermanentHandle) -> String {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn play_birdramon(runner: &mut DebugRunner) -> PermanentHandle {
    let index = runner.play(0, 0).expect("play Birdramon");
    PermanentHandle {
        player: 0,
        index: index as u8,
    }
}

/// POSITIVE: three [ADVENTURE] Tamers of three different colours -> the optional
/// free digivolve is offered on play; accepting digivolves Birdramon into the
/// [ADVENTURE] Lv.5 and only the 4 play cost is paid (the cost-3 digivolve is free).
#[test]
fn st20_03_on_play_offers_free_digivolve_with_three_adventure_tamer_colors() {
    let mut runner = on_play_runner(
        &[
            ("T-RED", CardColor::Red, true),
            ("T-BLUE", CardColor::Blue, true),
            ("T-GREEN", CardColor::Green, true),
        ],
        true,
    );
    let before = runner.memory();
    let birdramon = play_birdramon(&mut runner);
    assert!(runner.pending_selection().is_some(), "{}", GAP);
    assert!(
        runner.pending_is_optional(),
        "printed \"may digivolve\" must be declinable"
    );
    runner.auto_resolve().expect("accept and finish");
    assert_eq!(top_card_id(&runner, birdramon), "EVO-LV5");
    assert_eq!(
        before - runner.memory(),
        4,
        "only the play cost; the digivolve is free"
    );
}

/// NEGATIVE: only TWO [ADVENTURE] Tamer colours plus a third colour on a
/// non-[ADVENTURE] Tamer -> the gate is false and nothing is offered.
#[test]
fn st20_03_non_adventure_tamer_colour_does_not_open_the_gate() {
    let mut runner = on_play_runner(
        &[
            ("T-RED", CardColor::Red, true),
            ("T-BLUE", CardColor::Blue, true),
            ("T-GREEN-PLAIN", CardColor::Green, false),
        ],
        true,
    );
    let birdramon = play_birdramon(&mut runner);
    assert!(
        runner.pending_selection().is_none(),
        "3 colours across ALL Tamers but only 2 across [ADVENTURE] Tamers: no prompt"
    );
    assert_eq!(top_card_id(&runner, birdramon), CARD_ID);
}

/// NEGATIVE: two [ADVENTURE] Tamers of the SAME colour plus one other -> only 2
/// distinct colours, no prompt.
#[test]
fn st20_03_duplicate_colours_do_not_count_twice() {
    let mut runner = on_play_runner(
        &[
            ("T-RED-A", CardColor::Red, true),
            ("T-RED-B", CardColor::Red, true),
            ("T-BLUE", CardColor::Blue, true),
        ],
        true,
    );
    play_birdramon(&mut runner);
    assert!(runner.pending_selection().is_none());
}

/// Declining keeps Birdramon as it is and spends only the play cost.
#[test]
fn st20_03_declining_the_free_digivolve_changes_nothing() {
    let mut runner = on_play_runner(
        &[
            ("T-RED", CardColor::Red, true),
            ("T-BLUE", CardColor::Blue, true),
            ("T-GREEN", CardColor::Green, true),
        ],
        true,
    );
    let before = runner.memory();
    let birdramon = play_birdramon(&mut runner);
    while runner.pending_selection().is_some() {
        assert!(
            runner.pending_is_optional(),
            "every prompt of a \"may\" effect is declinable"
        );
        runner.execute_action(0, PASS).expect("decline");
    }
    assert_eq!(top_card_id(&runner, birdramon), CARD_ID);
    assert_eq!(before - runner.memory(), 4);
}

/// NEGATIVE: a Lv.5 WITHOUT the [ADVENTURE] trait is not a legal digivolve
/// target, so no digivolve happens even with the gate open.
#[test]
fn st20_03_non_adventure_hand_digimon_is_not_a_target() {
    let mut runner = on_play_runner(
        &[
            ("T-RED", CardColor::Red, true),
            ("T-BLUE", CardColor::Blue, true),
            ("T-GREEN", CardColor::Green, true),
        ],
        false,
    );
    let birdramon = play_birdramon(&mut runner);
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, birdramon), CARD_ID);
}

/// [When Digivolving] fires the same clause: Birdramon digivolved from a Lv.3
/// base with three [ADVENTURE] Tamer colours in play may digivolve again for free.
#[test]
fn st20_03_when_digivolving_offers_the_same_free_digivolve() {
    let mut runner = on_play_runner_with(
        &[
            ("T-RED", CardColor::Red, true),
            ("T-BLUE", CardColor::Blue, true),
            ("T-GREEN", CardColor::Green, true),
        ],
        true,
        vec![base("LV3-RED", 3, CardColor::Red, true)],
    );
    let handle = runner.place_on_field(0, "LV3-RED", Some(0));
    let before = runner.memory();
    assert!(runner
        .game
        .digivolve_from_hand(0, 0, handle.index as usize, PlaySource::ByDigivolve));
    assert!(runner.pending_selection().is_some(), "{}", GAP);
    runner.auto_resolve().expect("accept and finish");
    assert_eq!(top_card_id(&runner, handle), "EVO-LV5");
    assert_eq!(
        before - runner.memory(),
        2,
        "only Birdramon's own cost-2 digivolve"
    );
}
