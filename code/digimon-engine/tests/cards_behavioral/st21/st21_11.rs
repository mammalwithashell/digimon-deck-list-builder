//! ST21-11 MetalGarurumon — Digimon, Lv.6, Purple/Blue, DP 12000, Cost 7,
//! Cyborg / ADVENTURE, Form Mega, Attribute Data.
//!
//! # Card text (official Bandai DB bundle / cards.json — verbatim)
//!
//! ```text
//! [Hand] [Counter] ＜Blast Digivolve＞ (Your Digimon may digivolve into this
//! card without paying the cost.)
//! [On Play] [When Digivolving] Return 1 of your opponent's level 4 or lower
//! Digimon to the bottom of the deck. For every 2 colors your Tamers have,
//! add 1 to this effect's level maximum.
//! [When Attacking] [Once Per Turn] You may play 1 level 4 or lower Digimon
//! card from your trash without paying the cost.
//!
//! Inherited: Ace Overflow ＜-4＞ (As this card moves from the field or under
//! a card to an area other than those, lose 4 memory.)
//!
//! Digivolve circles: Purple Lv.5 / 4, Blue Lv.5 / 4.
//! [Digivolve] Lv.5 w/[ADVENTURE] trait: Cost 3   (xros_req)
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_11.cs`
//!
//! - Alt Digivolution (`EffectTiming.None`): Lv.5 with ADVENTURE, cost 3,
//!   requirements respected.
//! - Blast Digivolve (`OnCounterTiming`).
//! - On Play / When Digivolving (`OnEnterFieldAnyone`): mandatory; opponent
//!   Digimon with `Level <= 4 + distinctTamerColours / 2`, put to the library
//!   bottom.
//! - When Attacking (`OnAllyAttack`): once per turn, optional (`canNoSelect`),
//!   Lv<=4 Digimon from trash, `payCost: false`.
//!
//! # Patterns this test covers
//!
//! - **Alt-path digivolve circles** (AD1-005 sibling): two printed colour
//!   circles at cost 4 (positive per colour, negative off-colour), the
//!   `[ADVENTURE]` cost-3 path (positive, negative on level).
//! - **Blast Digivolve** counter window (AD1-005 idiom) and **Ace Overflow -4**.
//! - **Formula-valued level cap** `4 + floor(distinct Tamer colours / 2)`:
//!   0 / 1 / 2 / 3 / 4 colours, duplicate colours counted once, the
//!   opponent's Tamers ignored, and the zero-eligible-target no-op.
//!   Both `[On Play]` (real play from hand) and `[When Digivolving]` (real
//!   digivolve from hand).
//! - **[When Attacking][Once Per Turn] optional play from trash**: accept,
//!   decline (the use is refunded — DCGO declines without spending it),
//!   lockout after use, no-candidate no-op, and next-turn reset.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-11";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Digimon of `level` (DP scaled so nothing here ever wins a fight by
/// accident) with explicit colour and optional ADVENTURE trait.
fn digimon(id: &str, level: u8, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, level);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(1000 * i32::from(level));
    c.play_cost = u16::from(level);
    c.colors = vec![color];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        vec![]
    };
    c
}

fn tamer(id: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = colors.to_vec();
    c
}

fn option_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn deck_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .deck
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st21_11_identity_matches_printed_card() {
    let c = compiled(CARD_ID);
    assert_eq!(c.card, CARD_ID);
    assert_eq!(c.name, "MetalGarurumon");
    assert_eq!(c.level, Some(6));
    assert_eq!(c.dp, Some(12000));
    assert_eq!(c.cost, Some(7));
    assert_eq!(c.color.len(), 2, "Purple + Blue; got {:?}", c.color);
    assert_eq!(c.form.as_deref(), Some("Mega"));
    assert!(
        c.attribute.iter().any(|a| a.eq_ignore_ascii_case("Data")),
        "attribute Data; got {:?}",
        c.attribute
    );
    for t in ["Cyborg", "ADVENTURE"] {
        assert!(
            c.traits.iter().any(|x| x.eq_ignore_ascii_case(t)),
            "must have the {t} trait; got {:?}",
            c.traits
        );
    }
}

#[test]
fn st21_11_ace_overflow_is_minus_4() {
    assert_eq!(compiled(CARD_ID).ace_overflow, Some(-4));
}

#[test]
fn st21_11_alt_paths_are_circles_adventure_and_blast_marker() {
    let c = compiled(CARD_ID);
    assert_eq!(c.alt_paths.len(), 3, "circles + xros_req + blast marker");

    let circles = &c.alt_paths[0];
    assert_eq!(circles.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(circles.cost, Some(CompiledCost::Literal(4)));
    assert_eq!(
        circles.from.as_ref().expect("circle from").level_eq,
        Some(5)
    );
    assert!(!circles.ignore_requirements);

    let adventure = &c.alt_paths[1];
    assert_eq!(adventure.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(adventure.cost, Some(CompiledCost::Literal(3)));
    let from = adventure.from.as_ref().expect("xros_req from");
    assert_eq!(from.level_eq, Some(5));
    assert!(
        from.trait_has
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "xros_req path must gate on the ADVENTURE trait; got {:?}",
        from.trait_has
    );
    assert!(
        !adventure.ignore_requirements,
        "DCGO passes ignoreDigivolutionRequirement: false"
    );

    let blast = &c.alt_paths[2];
    assert_eq!(blast.kind, CompiledAltPathKind::BurstDigivolve);
    assert_eq!(blast.cost, Some(CompiledCost::Literal(0)));
    assert!(blast.marker);
}

#[test]
fn st21_11_clause_layout() {
    let c = compiled(CARD_ID);
    assert_eq!(c.effects.len(), 3, "blast marker + OP/WD + WA");

    assert!(matches!(
        &c.effects[0],
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope: CompiledScope::FaceUp,
            ..
        }) if keyword == "BlastDigivolve"
    ));

    match &c.effects[1] {
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.when,
                vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]
            );
            assert!(!t.optional, "the bottom-deck is mandatory (canNoSelect: false)");
            assert!(!t.once_per_turn);
            assert!(t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::SelectOpponentPermanent { .. })));
            assert!(t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::ReturnToDeck { .. })));
        }
        other => panic!("clause 1 must be the OP/WD trigger; got {other:?}"),
    }

    match &c.effects[2] {
        CompiledClause::Triggered(t) => {
            assert_eq!(t.when, vec![CompiledTiming::WhenAttacking]);
            assert!(t.once_per_turn, "[Once Per Turn]");
            assert!(t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::SelectTrash { .. })));
            assert!(
                format!("{:?}", t.process).contains("PlayFromTrashFree"),
                "body must play from trash free; got {:?}",
                t.process
            );
        }
        other => panic!("clause 2 must be the When Attacking trigger; got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve circles / ADVENTURE alt path / Blast Digivolve
// ═══════════════════════════════════════════════════════════════════════════════

/// ST21-11 in hand above `base` on P0's battle area (Main phase).
fn digivolve_runner(base: CardData) -> (DebugRunner, usize) {
    let base_id = base.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-11 in embedded DSL pack")
        .add_card(base)
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, &base_id, Some(0)).index as usize;
    (r, slot)
}

fn offered(r: &DebugRunner, slot: usize) -> bool {
    let mask = build_action_mask(&r.game, 0);
    mask[encode_digivolve(0, slot as u16) as usize] == 1.0
}

#[test]
fn st21_11_purple_lv5_circle_digivolves_for_4() {
    let (mut r, slot) = digivolve_runner(digimon("P-LV5", 5, CardColor::Purple, false));
    assert!(offered(&r, slot), "purple Lv.5 circle must be offered");
    let before = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(before - r.game.memory, 4, "circle costs 4");
}

#[test]
fn st21_11_blue_lv5_circle_digivolves_for_4() {
    let (mut r, slot) = digivolve_runner(digimon("B-LV5", 5, CardColor::Blue, false));
    assert!(offered(&r, slot), "blue Lv.5 circle must be offered");
    let before = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(before - r.game.memory, 4, "circle costs 4");
}

/// Off-colour, non-ADVENTURE Lv.5 base matches no circle.
#[test]
fn st21_11_off_colour_non_adventure_lv5_is_rejected() {
    let (mut r, slot) = digivolve_runner(digimon("R-LV5", 5, CardColor::Red, false));
    assert!(!offered(&r, slot), "a red non-ADVENTURE Lv.5 matches no circle");
    let before = r.game.memory;
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(r.game.memory, before, "nothing is paid on rejection");
}

/// Wrong level: a purple Lv.4 base fails the Lv.5 circle.
#[test]
fn st21_11_purple_lv4_base_is_rejected() {
    let (mut r, slot) = digivolve_runner(digimon("P-LV4", 4, CardColor::Purple, false));
    assert!(!offered(&r, slot), "a Lv.4 base is below the Lv.5 circle");
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
}

/// xros_req: an off-colour Lv.5 [ADVENTURE] base digivolves for 3.
#[test]
fn st21_11_adventure_lv5_base_digivolves_for_3() {
    let (mut r, slot) = digivolve_runner(digimon("ADV-LV5", 5, CardColor::Red, true));
    assert!(offered(&r, slot), "Lv.5 [ADVENTURE] base must be offered");
    let before = r.game.memory;
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(
        before - r.game.memory,
        3,
        "the [ADVENTURE] path costs 3"
    );
}

/// xros_req level gate: a Lv.4 [ADVENTURE] base is not Lv.5.
#[test]
fn st21_11_adventure_lv4_base_is_rejected() {
    let (mut r, slot) = digivolve_runner(digimon("ADV-LV4", 4, CardColor::Red, true));
    assert!(!offered(&r, slot), "xros_req needs Lv.5 exactly");
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
}

/// [Hand][Counter] <Blast Digivolve>: ST21-11 in the defender's hand is
/// offered in the counter window over a legal Lv.5 base.
#[test]
fn st21_11_blast_digivolve_is_offered_in_the_counter_window() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-11 in embedded DSL pack")
        .add_card(digimon("ATK", 5, CardColor::Red, false))
        .add_card(digimon("TARGET", 5, CardColor::Purple, false))
        .hand(1, &[CARD_ID])
        .start();

    let attacking = runner.place_on_field(0, "ATK", Some(0));
    let defending = runner.place_on_field(1, "TARGET", Some(0));

    let result = runner.attack_digimon(attacking, defending, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(
        runner.current_phase(),
        GamePhase::CounterTiming,
        "the attack enters the counter window"
    );

    let prompt = runner
        .pending_selection()
        .expect("counter-window selection must be present");
    assert_eq!(prompt.selecting_player, 1);
    let blast = encode_digivolve(0, 0);
    assert!(
        prompt.valid_action_ids.contains(&blast),
        "ST21-11 must be offered as a Blast Digivolve; expected {blast}, got {:?}",
        prompt.valid_action_ids
    );
}

/// Ace Overflow <-4>: leaving the field costs 4 memory.
#[test]
fn st21_11_ace_overflow_loses_4_memory_on_leaving_the_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-11 in embedded DSL pack")
        .memory(10)
        .start();
    let mg = runner.place_on_field(0, CARD_ID, Some(0));
    let before = runner.game.memory;

    runner.game.delete_permanent_with_cause(
        PermanentHandle {
            player: 0,
            index: mg.index,
        },
        ReplacementCause::OwnEffect,
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.memory,
        before - 4,
        "Ace Overflow <-4> must subtract 4 memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play] / [When Digivolving]: bottom-deck an opponent Digimon
//             with level <= 4 + floor(distinct Tamer colours / 2)
// ═══════════════════════════════════════════════════════════════════════════════

use CardColor::{Blue, Green, Purple, Red, Yellow};

/// Build a runner with ST21-11 in P0's hand, the given opponent Digimon
/// `(id, level)` on P1's field (slot order preserved) and the given P0 / P1
/// Tamers `(id, colours)` on the respective battle areas.
fn op_runner(
    opp: &[(&str, u8)],
    own_tamers: &[(&str, Vec<CardColor>)],
    opp_tamers: &[(&str, Vec<CardColor>)],
) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-11 in embedded DSL pack")
        .add_card(filler("FILL"));
    for (id, lv) in opp {
        b = b.add_card(digimon(id, *lv, Red, false));
    }
    for (id, colors) in own_tamers.iter().chain(opp_tamers.iter()) {
        b = b.add_card(tamer(id, colors));
    }
    let mut runner = b
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    for (id, _) in opp {
        runner.place_on_field(1, id, Some(0));
    }
    for (id, _) in own_tamers {
        runner.place_on_field(0, id, Some(0));
    }
    for (id, _) in opp_tamers {
        runner.place_on_field(1, id, Some(0));
    }
    runner
}

/// Index of an opponent battle-area slot by card id.
fn p1_slot(runner: &DebugRunner, id: &str) -> u16 {
    runner.game.players[1]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not on P1's field")) as u16
}

/// Play ST21-11 from hand and return the [On Play] OppField prompt.
fn play_and_view(runner: &mut DebugRunner) -> Option<digimon_engine::selection::PendingSelectionView> {
    let _ = runner.play(0, 0);
    runner.pending_selection_view()
}

/// 0 Tamer colours: only Lv.4 or lower is a legal pick; the picked Digimon
/// goes to the BOTTOM of the opponent's deck.
#[test]
fn st21_11_on_play_with_no_tamers_offers_only_level_4_or_lower() {
    let mut runner = op_runner(&[("OPP-LV4", 4), ("OPP-LV5", 5)], &[], &[]);
    let lv4 = p1_slot(&runner, "OPP-LV4");
    let lv5 = p1_slot(&runner, "OPP-LV5");
    let mem_before = runner.memory();
    let deck_before = runner.deck_size(1);

    let view = play_and_view(&mut runner).expect("[On Play] prompt must install");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional, "the bottom-deck is mandatory");
    assert!(view.valid_action_ids.contains(&encode_attack(0, lv4)));
    assert!(
        !view.valid_action_ids.contains(&encode_attack(0, lv5)),
        "a Lv.5 Digimon is above the base cap of 4"
    );

    runner
        .execute_action(view.selecting_player, encode_attack(0, lv4))
        .expect("pick OPP-LV4");
    let _ = runner.auto_resolve();

    let field = field_ids(&runner, 1);
    assert!(!field.contains(&"OPP-LV4".to_string()), "field: {field:?}");
    assert!(field.contains(&"OPP-LV5".to_string()), "field: {field:?}");
    assert_eq!(runner.deck_size(1), deck_before + 1);
    assert_eq!(
        deck_ids(&runner, 1)[0],
        "OPP-LV4",
        "the returned Digimon is the BOTTOM card of the deck"
    );
    assert_eq!(mem_before - runner.memory(), 7, "ST21-11 costs 7 to play");
}

/// 2 Tamer colours (one Red/Blue Tamer): cap 5.
#[test]
fn st21_11_two_tamer_colours_raise_the_cap_to_level_5() {
    let mut runner = op_runner(
        &[("OPP-LV5", 5), ("OPP-LV6", 6)],
        &[("T1", vec![Red, Blue])],
        &[],
    );
    let lv5 = p1_slot(&runner, "OPP-LV5");
    let lv6 = p1_slot(&runner, "OPP-LV6");

    let view = play_and_view(&mut runner).expect("prompt");
    assert!(view.valid_action_ids.contains(&encode_attack(0, lv5)));
    assert!(
        !view.valid_action_ids.contains(&encode_attack(0, lv6)),
        "Lv.6 exceeds the cap of 5 at two colours"
    );
}

/// Duplicate colours across Tamers are counted ONCE: two Red Tamers = 1
/// colour = +0, so a Lv.5 Digimon is NOT eligible and nothing is prompted.
#[test]
fn st21_11_duplicate_tamer_colours_count_once() {
    let mut runner = op_runner(
        &[("OPP-LV5", 5)],
        &[("T1", vec![Red]), ("T2", vec![Red])],
        &[],
    );
    let view = play_and_view(&mut runner);
    assert!(
        view.is_none(),
        "1 distinct colour → cap 4: no legal target, so no prompt; got {view:?}"
    );
    assert!(field_ids(&runner, 1).contains(&"OPP-LV5".to_string()));
}

/// Two Red/Blue Tamers still total only 2 distinct colours → cap 5, not 6.
#[test]
fn st21_11_two_identical_two_colour_tamers_total_two_colours() {
    let mut runner = op_runner(
        &[("OPP-LV5", 5), ("OPP-LV6", 6)],
        &[("T1", vec![Red, Blue]), ("T2", vec![Red, Blue])],
        &[],
    );
    let lv5 = p1_slot(&runner, "OPP-LV5");
    let lv6 = p1_slot(&runner, "OPP-LV6");

    let view = play_and_view(&mut runner).expect("prompt");
    assert!(view.valid_action_ids.contains(&encode_attack(0, lv5)));
    assert!(!view.valid_action_ids.contains(&encode_attack(0, lv6)));
}

/// 3 distinct colours: integer halving → +1 → cap 5 (not 5.5 / 6).
#[test]
fn st21_11_three_tamer_colours_still_cap_at_level_5() {
    let mut runner = op_runner(
        &[("OPP-LV5", 5), ("OPP-LV6", 6)],
        &[("T1", vec![Red, Blue]), ("T2", vec![Blue, Yellow])],
        &[],
    );
    let lv5 = p1_slot(&runner, "OPP-LV5");
    let lv6 = p1_slot(&runner, "OPP-LV6");

    let view = play_and_view(&mut runner).expect("prompt");
    assert!(view.valid_action_ids.contains(&encode_attack(0, lv5)));
    assert!(
        !view.valid_action_ids.contains(&encode_attack(0, lv6)),
        "3 colours halve to +1: Lv.6 is still out of reach"
    );
}

/// 4 distinct colours: +2 → cap 6; Lv.7 stays out of reach.
#[test]
fn st21_11_four_tamer_colours_raise_the_cap_to_level_6() {
    let mut runner = op_runner(
        &[("OPP-LV6", 6), ("OPP-LV7", 7)],
        &[("T1", vec![Red, Blue]), ("T2", vec![Yellow, Green])],
        &[],
    );
    let lv6 = p1_slot(&runner, "OPP-LV6");
    let lv7 = p1_slot(&runner, "OPP-LV7");

    let view = play_and_view(&mut runner).expect("prompt");
    assert!(view.valid_action_ids.contains(&encode_attack(0, lv6)));
    assert!(!view.valid_action_ids.contains(&encode_attack(0, lv7)));
}

/// "your Tamers": the OPPONENT's Tamers' colours do not raise the cap.
#[test]
fn st21_11_opponents_tamer_colours_do_not_count() {
    let mut runner = op_runner(
        &[("OPP-LV5", 5)],
        &[],
        &[("OPP-T", vec![Red, Blue, Yellow, Green])],
    );
    let view = play_and_view(&mut runner);
    assert!(
        view.is_none(),
        "the opponent's four colours must not lift my cap above 4; got {view:?}"
    );
}

/// Zero legal targets: nothing is prompted, nothing leaves the field.
#[test]
fn st21_11_on_play_with_no_eligible_target_is_a_no_op() {
    let mut runner = op_runner(&[("OPP-LV6", 6)], &[], &[]);
    let deck_before = runner.deck_size(1);
    let view = play_and_view(&mut runner);
    assert!(view.is_none(), "no Lv<=4 target → no prompt; got {view:?}");
    assert_eq!(field_ids(&runner, 1), vec!["OPP-LV6".to_string()]);
    assert_eq!(runner.deck_size(1), deck_before);
}

/// [When Digivolving] runs the same body: a real digivolve from hand over a
/// Lv.5 purple base, with a 2-colour Tamer lifting the cap to Lv.5.
#[test]
fn st21_11_when_digivolving_runs_the_same_bottom_deck_body() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-11 in embedded DSL pack")
        .add_card(digimon("P-LV5", 5, Purple, false))
        .add_card(digimon("OPP-LV5", 5, Red, false))
        .add_card(digimon("OPP-LV6", 6, Red, false))
        .add_card(tamer("T1", &[Red, Blue]))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    let base = runner.place_on_field(0, "P-LV5", Some(0));
    runner.place_on_field(0, "T1", Some(0));
    runner.place_on_field(1, "OPP-LV5", Some(0));
    runner.place_on_field(1, "OPP-LV6", Some(0));
    let lv5 = p1_slot(&runner, "OPP-LV5");
    let lv6 = p1_slot(&runner, "OPP-LV6");

    assert!(runner
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand));
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("[When Digivolving] prompt must install");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional);
    assert!(view.valid_action_ids.contains(&encode_attack(0, lv5)));
    assert!(!view.valid_action_ids.contains(&encode_attack(0, lv6)));

    runner
        .execute_action(view.selecting_player, encode_attack(0, lv5))
        .expect("pick OPP-LV5");
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 1).contains(&"OPP-LV5".to_string()));
    assert_eq!(deck_ids(&runner, 1)[0], "OPP-LV5");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [When Attacking][Once Per Turn]: may play Lv<=4 Digimon from trash
// ═══════════════════════════════════════════════════════════════════════════════

/// ST21-11 on P0's field (can attack), `trash` cards in P0's trash, a fat
/// opposing security stack so repeated attacks never end the game.
fn wa_runner(trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-11 in embedded DSL pack")
        .add_card(filler("FILL"))
        .add_card(digimon("T-LV4", 4, Red, false))
        .add_card(digimon("T-LV4B", 4, Red, false))
        .add_card(digimon("T-LV5", 5, Red, false))
        .add_card(option_card("T-OPT"))
        .security(1, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    for id in trash {
        runner.inject_trash(0, id);
    }
    let mg = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, mg)
}

fn unsuspend(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.game.players[0].battle_area[h.index as usize].is_suspended = false;
}

/// Accept: only the Lv<=4 Digimon is offered (not the Lv.5 Digimon or the
/// Option); it enters play free.
#[test]
fn st21_11_when_attacking_plays_a_level_4_digimon_from_trash_free() {
    let (mut runner, mg) = wa_runner(&["T-LV5", "T-OPT", "T-LV4"]);
    let mem_before = runner.memory();

    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("[When Attacking] trash prompt");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert_eq!(view.selecting_player, 0);
    assert!(view.is_optional, "printed 'You may play'");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        1,
        "only the Lv.4 Digimon is legal (not the Lv.5 Digimon / Option); got {picks:?}"
    );

    runner
        .execute_action(0, picks[0])
        .expect("play the Lv.4 Digimon");
    let _ = runner.auto_resolve();

    let field = field_ids(&runner, 0);
    assert!(field.contains(&"T-LV4".to_string()), "field: {field:?}");
    assert!(!field.contains(&"T-LV5".to_string()), "field: {field:?}");
    assert_eq!(runner.memory(), mem_before, "the play is free");
}

/// Decline: PASS plays nothing AND refunds the once-per-turn use (DCGO
/// declines the optional prompt without spending it) — a second attack in the
/// same turn re-offers the prompt.
#[test]
fn st21_11_declining_does_not_spend_the_once_per_turn_use() {
    let (mut runner, mg) = wa_runner(&["T-LV4"]);

    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("first prompt");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline");
    let _ = runner.auto_resolve();
    assert!(
        !field_ids(&runner, 0).contains(&"T-LV4".to_string()),
        "declining plays nothing"
    );

    unsuspend(&mut runner, mg);
    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection_view().is_some(),
        "the declined use was refunded, so the second attack re-offers the play"
    );
}

/// OPT lockout: after the effect is used once, a second attack in the same
/// turn does not re-trigger it even though another Lv<=4 Digimon is in trash.
#[test]
fn st21_11_once_per_turn_blocks_a_second_activation() {
    let (mut runner, mg) = wa_runner(&["T-LV4", "T-LV4B"]);

    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("first prompt");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a legal pick");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("play");
    let _ = runner.auto_resolve();

    unsuspend(&mut runner, mg);
    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection_view().is_none(),
        "[Once Per Turn]: the second attack must not offer the play again"
    );
}

/// No Lv<=4 Digimon in trash (only a Lv.5 Digimon and an Option): the clause
/// is a silent no-op — no prompt, nothing played.
#[test]
fn st21_11_when_attacking_is_a_no_op_without_a_legal_trash_card() {
    let (mut runner, mg) = wa_runner(&["T-LV5", "T-OPT"]);

    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection_view().is_none(),
        "no legal trash card → no prompt"
    );
    let _ = runner.auto_resolve();
    let field = field_ids(&runner, 0);
    assert!(
        !field.contains(&"T-LV5".to_string()),
        "the Lv.5 Digimon must not be played; field: {field:?}"
    );
}

/// The once-per-turn use resets on the next own turn.
#[test]
fn st21_11_once_per_turn_resets_on_the_next_turn() {
    let (mut runner, mg) = wa_runner(&["T-LV4", "T-LV4B"]);

    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("first prompt");
    let pick = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a legal pick");
    runner
        .execute_action(view.selecting_player, pick)
        .expect("play");
    let _ = runner.auto_resolve();

    runner.end_turn(); // P1's turn
    runner.end_turn(); // back to P0
    unsuspend(&mut runner, mg);
    runner.attack_player(mg, 1, false);
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection_view().is_some(),
        "the use resets each turn: the next turn's attack re-offers the play"
    );
}

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = tamer("X", &[Red]);
    let _ = option_card("X");
}
