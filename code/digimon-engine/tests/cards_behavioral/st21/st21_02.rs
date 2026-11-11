//! ST21-02 Gomamon — Digimon, Lv.3, Blue, DP 2000, Cost 3.
//! Traits: Sea Beast / ADVENTURE. Form: Rookie. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/ST21-02.md — official Bandai DB, verbatim)
//!
//! ```text
//! Digivolve: Blue Lv.2 / cost 0
//! [Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0
//! [All Turns] Your opponent can't gain memory other than by Tamer effects.
//!
//! Official Q&A: "With this effect, your opponent's memory will no longer
//! increase by any effects except Tamer effects. This effect affects the
//! opponent player."
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST21/Blue/ST21_02.cs
//!
//! Two `EffectTiming.None` statics:
//!   1. AddSelfDigivolutionRequirementStaticEffect — `Level == 2 &&
//!      TopCard.HasAdventureTraits`, cost 0, requirement-respecting (the
//!      printed xros_req box; the standard blue Lv.2 circle is the first
//!      alt_path).
//!   2. CannotAddMemoryClass — `IsExistOnBattleArea(card)`, player ==
//!      card.Owner.Enemy, effect has a source card && !IsTamerEffect
//!      (twin of BT3-077 Gazimon / EX8-030 Tapirmon).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - D6 flood gate / opponent restriction (`CannotGainMemoryExceptFromTamers`)
//! - All-turns static effect (installs at tick, no timing trigger, no OPT)
//! - "[Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0" alt-path (ST20-10 exemplar)
//!   positive AND negative per condition, plus an event-log cost test
//!
//! No triggered clause and no once-per-turn clause exist, so there is no OPT
//! lockout test.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledPlayerRef,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlaySource};
use digimon_engine::events::GameEvent;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-02";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn tamer_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

/// A Lv.2 Digimon of `color`; `adventure` toggles the [ADVENTURE] trait.
fn lv2(id: &str, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c.play_cost = 0;
    c.colors = vec![color];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

/// A Lv.3 Digimon (wrong level for either of Gomamon's circles).
fn lv3_adventure(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn digivolve_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-02 in embedded DSL pack")
        .add_card(lv2("BLUE2", CardColor::Blue, false))
        .add_card(lv2("RED2-ADV", CardColor::Red, true))
        .add_card(lv2("RED2-PLAIN", CardColor::Red, false))
        .add_card(lv3_adventure("BLUE3-ADV"))
        .memory(5)
        .start()
}

/// Digivolve Gomamon from hand onto `base` (placed on P0's field). Returns
/// (digivolve succeeded, memory paid).
fn digivolve_from(base: &str) -> (bool, i16) {
    let mut r = digivolve_runner();
    let h = r.place_on_field(0, base, Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand);
    (ok, before - r.memory())
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_02_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Gomamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(2000));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert!(
        card.traits.iter().any(|t| t == "Sea Beast"),
        "{:?}",
        card.traits
    );
    assert!(
        card.traits.iter().any(|t| t == "ADVENTURE"),
        "{:?}",
        card.traits
    );
}

#[test]
fn st21_02_has_two_cost_zero_requirement_respecting_digivolve_paths() {
    let card = compiled(CARD_ID);
    let paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        paths.len(),
        2,
        "printed blue Lv.2 circle + [Digivolve] Lv.2 w/[ADVENTURE] box"
    );
    for p in &paths {
        assert_eq!(p.cost, Some(CompiledCost::Literal(0)), "both cost 0");
        assert!(
            !p.ignore_requirements,
            "neither path ignores digivolution requirements (DCGO ignoreDigivolutionRequirement: false)"
        );
    }
    let std_from = paths[0].from.as_ref().expect("blue Lv.2 `from:`");
    assert_eq!(std_from.level_eq, Some(2));
    assert_eq!(std_from.color_is, Some(CompiledColor::Blue));
    let adv_from = paths[1].from.as_ref().expect("ADVENTURE Lv.2 `from:`");
    assert_eq!(adv_from.level_eq, Some(2));
    let pretty = format!("{adv_from:?}");
    assert!(
        pretty.contains("ADVENTURE"),
        "xros_req `from:` must gate on the ADVENTURE trait; got {pretty}"
    );
}

#[test]
fn st21_02_memory_floodgate_is_the_only_effect_clause() {
    let card = compiled(CARD_ID);
    let floodgate = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            target_player,
            ..
        }) => Some((modifier.as_str(), *target_player)),
        _ => None,
    });
    assert_eq!(
        floodgate,
        Some((
            "CannotGainMemoryExceptFromTamers",
            Some(CompiledPlayerRef::Opponent),
        )),
        "the [All Turns] memory lock is an opponent-scoped player floodgate"
    );
    assert_eq!(card.effects.len(), 1, "no other printed effect");
    assert!(
        !card
            .effects
            .iter()
            .any(|c| matches!(c, CompiledClause::Triggered(_))),
        "no triggered clause (and therefore no OPT) is printed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve conditions (positive + negative per condition)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_02_digivolves_from_blue_lv2_for_zero() {
    let (ok, paid) = digivolve_from("BLUE2");
    assert!(ok, "a blue Lv.2 satisfies the printed circle");
    assert_eq!(paid, 0, "cost 0");
}

#[test]
fn st21_02_digivolves_from_a_non_blue_lv2_with_adventure_trait_for_zero() {
    let (ok, paid) = digivolve_from("RED2-ADV");
    assert!(
        ok,
        "[Digivolve] Lv.2 w/[ADVENTURE] trait lifts the colour gate"
    );
    assert_eq!(paid, 0, "cost 0");
}

#[test]
fn st21_02_cannot_digivolve_from_a_non_blue_lv2_without_adventure() {
    let (ok, _) = digivolve_from("RED2-PLAIN");
    assert!(
        !ok,
        "red Lv.2 without [ADVENTURE] matches neither the circle nor the xros_req box"
    );
}

#[test]
fn st21_02_cannot_digivolve_from_a_lv3_adventure_digimon() {
    let (ok, _) = digivolve_from("BLUE3-ADV");
    assert!(
        !ok,
        "both circles require Lv.2 — a Lv.3 [ADVENTURE] Digimon is the wrong level"
    );
}

#[test]
fn st21_02_digivolve_cost_zero_emits_no_memory_payment_event() {
    let mut r = digivolve_runner();
    let h = r.place_on_field(0, "RED2-ADV", Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let cp = r.event_checkpoint();
    assert!(r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand));
    let paid = r
        .events_since(cp)
        .iter()
        .any(|e| matches!(e, GameEvent::MemoryChange { delta, .. } if *delta < 0));
    assert!(
        !paid,
        "a cost-0 digivolution must not spend memory (no negative MemoryChange)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns] opponent memory floodgate (behavioral)
// ═══════════════════════════════════════════════════════════════════════════

fn floodgate_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-02 in embedded DSL pack")
        .add_card(make_test_card(
            "OPP-DIGIMON-SOURCE",
            "Opponent Digimon Source",
        ))
        .add_card(make_test_card("OWN-DIGIMON-SOURCE", "Own Digimon Source"))
        .add_card(tamer_card("OPP-TAMER-SOURCE"))
        .start()
}

#[test]
fn st21_02_blocks_opponent_non_tamer_memory_gain_but_allows_tamer_source_gain() {
    let mut runner = floodgate_runner();

    let _gomamon = runner.place_on_field(0, CARD_ID, None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "Gomamon must install CannotGainMemoryExceptFromTamers on its opponent"
    );

    let digimon_source = runner.game.player(1).battle_area[opp_digimon.index as usize]
        .top_card()
        .handle();
    let tamer_source = runner.game.player(1).battle_area[opp_tamer.index as usize]
        .top_card()
        .handle();

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, 0,
        "opponent Digimon-sourced memory gain must be blocked by Gomamon"
    );

    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, tamer_source, Some(opp_tamer), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "opponent Tamer-sourced memory gain stays legal (\"other than by Tamer effects\")"
    );
}

#[test]
fn st21_02_floodgate_is_scoped_to_opponent_and_does_not_block_controllers_own_gain() {
    let mut runner = floodgate_runner();

    let _gomamon = runner.place_on_field(0, CARD_ID, None);
    let own_digimon = runner.place_on_field(0, "OWN-DIGIMON-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotGainMemoryExceptFromTamers),
        "Gomamon's floodgate must target the opponent, not its own controller"
    );

    let own_source = runner.game.player(0).battle_area[own_digimon.index as usize]
        .top_card()
        .handle();
    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, own_source, Some(own_digimon), 0);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, 2,
        "the controller's own non-Tamer memory gain must remain legal"
    );
}

#[test]
fn st21_02_floodgate_is_inactive_while_gomamon_is_not_on_the_battle_area() {
    let mut runner = floodgate_runner();
    let _in_hand = runner.add_to_hand(0, CARD_ID);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "DCGO CanUseCondition = IsExistOnBattleArea — a Gomamon in hand locks nothing"
    );

    let digimon_source = runner.game.player(1).battle_area[opp_digimon.index as usize]
        .top_card()
        .handle();
    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(runner.game.memory, -2, "memory gain is unrestricted");
}

#[test]
fn st21_02_floodgate_persists_across_turns_all_turns_scope() {
    let mut runner = floodgate_runner();

    runner.place_on_field(0, CARD_ID, None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "floodgate must be active on player 0's turn"
    );

    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "[All Turns] must keep the floodgate active through the opponent's own turn"
    );

    let digimon_source = runner.game.player(1).battle_area[opp_digimon.index as usize]
        .top_card()
        .handle();
    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, 0,
        "opponent Digimon-sourced memory gain must still be blocked during their own turn"
    );

    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "floodgate must remain active back on player 0's turn"
    );
}

#[test]
fn st21_02_floodgate_clears_when_gomamon_leaves_the_battle_area() {
    let mut runner = floodgate_runner();

    let gomamon = runner.place_on_field(0, CARD_ID, None);
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-SOURCE", None);
    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers));

    runner.game.players[0]
        .battle_area
        .remove(gomamon.index as usize);
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "the static floodgate must clear after Gomamon leaves the battle area"
    );

    let digimon_source = runner.game.player(1).battle_area[opp_digimon.index as usize]
        .top_card()
        .handle();
    runner.game.set_memory(0);
    {
        let mut ctx = EffectContext::new(&mut runner.game, digimon_source, Some(opp_digimon), 1);
        ctx.gain_memory(2);
    }
    assert_eq!(
        runner.game.memory, -2,
        "Digimon-sourced memory gain works again once Gomamon is gone"
    );
}
