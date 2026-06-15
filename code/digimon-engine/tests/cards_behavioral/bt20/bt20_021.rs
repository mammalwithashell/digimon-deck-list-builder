//! BT20-021 Jesmon GX — Digimon (ACE), Lv.7, Red/Purple, DP 16000, Cost 9.
//! Traits: [Holy Warrior, X Antibody, Royal Knight]. Attribute: Data.
//!
//! # Printed card text (card image — authoritative; cross-checked cards.json)
//!
//! DNA Digivolve Lv.6 w/[Jesmon] in name + Lv.6 w/[Gankoomon] in name: Cost 0
//! [Hand] [Counter] ＜Blast Digivolve＞
//!   (Your Digimon may digivolve into this card without paying the cost.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn]
//!   By placing 1 [Royal Knight] trait card from your hand or trash as this
//!   Digimon's bottom digivolution card, delete 1 of your opponent's Digimon
//!   with as much or less DP as this Digimon.
//! [When Attacking] [Once Per Turn]
//!   This Digimon unsuspends. Then, for every 2 [Royal Knight] trait cards in
//!   this Digimon's digivolution cards, trash your opponent's top security card.
//!
//! Inherited Effect: Ace Overflow ＜-5＞.
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT20/Red/BT20_021.cs
//!
//! DCGO governs behavior:
//! - DNA Digivolution: Jogress of a Lv.6 with [Jesmon] in name + a Lv.6 with
//!   [Gankoomon] in name, cost 0.
//! - BlastDigivolve at OnCounterTiming.
//! - The place-source/delete clause is shared across [On Play]/[When
//!   Digivolving]/[When Attacking] (SetHashString "Delete_BT20_021"), each
//!   isOptional=true: the place is `canNoSelect: true` (you MAY place), and
//!   declining aborts the delete. Source card filter = HasRoyalKnightTraits.
//!   Delete target: opponent Digimon with `permanent.DP <= card
//!   .PermanentOfThisCard().DP` (live permanent DP = source_dp), delete select
//!   canNoSelect:false (mandatory once a card is placed and a target exists).
//! - The unsuspend/security-trash clause (OnAllyAttack, isOptional=FALSE →
//!   mandatory): unsuspend self, then trash `floor(RK-source-count / 2)`
//!   opponent top security (DigivolutionCards counts sources BELOW the top
//!   card — matches source_stack_count which skips the top).
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API)
//! - Blast Digivolve (hand counter marker grant_keyword)
//! - DNA digivolve alt-path (Jesmon-in-name + Gankoomon-in-name, cost 0)
//! - Three-timing OPT cost clause: place RK source from hand/trash (optional
//!   cost) → delete opp Digimon with DP <= source_dp; dp_lte filter exclusion;
//!   decline path; OPT lock
//! - When-Attacking mandatory clause: unsuspend self + per-2-RK-source security
//!   trash via source_stack_count/floor_div; count scaling; non-RK exclusion

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT20-021";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Royal-Knight-trait Digimon card (used as the placeable source / RK source).
fn rk_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["Royal Knight".to_string()];
    c
}

/// A plain (non-Royal-Knight) Digimon card.
fn plain_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["Beast".to_string()];
    c
}

/// An opponent Digimon with a given DP (to exercise the dp_lte delete filter).
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.traits = vec!["Beast".to_string()];
    c
}

/// A filler card for decks / a generic source carrier.
fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT20-021 YAML parses and compiles")
        .add_card(rk_card("RK-A"))
        .add_card(rk_card("RK-B"))
        .add_card(rk_card("RK-C"))
        .add_card(rk_card("RK-D"))
        .add_card(plain_card("PLAIN"))
        .add_card(opp_digimon("OPP-12K", 12000))
        .add_card(opp_digimon("OPP-16K", 16000))
        .add_card(opp_digimon("OPP-17K", 17000))
        .add_card(filler("DECK-PAD"))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
        .memory(10)
        .start()
}

// ─── Section 0 — load smoke ──────────────────────────────────────────────────

#[test]
fn bt20_021_loads_ace_stub() {
    DebugRunner::builder()
        .dsl_card("BT20-021")
        .expect("BT20-021 must load from embedded DSL pack")
        .start();
}

// ─── Section 1 — structural assertions ───────────────────────────────────────

#[test]
fn bt20_021_has_blast_digivolve_grant_keyword() {
    let runner = base();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT20-021 must be present in embedded DSL pack");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
        )),
        "BT20-021 must grant <Blast Digivolve> as a declarative keyword"
    );
}

#[test]
fn bt20_021_has_dna_digivolve_alt_path_cost_0() {
    let runner = base();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("compiled card present");

    let has_dna = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::DnaDigivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        has_dna,
        "BT20-021 must carry a DNA digivolve alt-path at cost 0 \
         (Lv.6 [Jesmon] + Lv.6 [Gankoomon])"
    );
}

#[test]
fn bt20_021_ace_overflow_is_minus_5() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled card present");
    assert_eq!(
        card.ace_overflow,
        Some(-5),
        "BT20-021 ACE Overflow must be -5"
    );
}

#[test]
fn bt20_021_cost_clause_carries_three_timings_and_opt() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled card present");

    // The place-source/delete clause must trigger on On Play, When Digivolving
    // AND When Attacking, and be Once Per Turn.
    let cost_clause = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t) => {
            let timings = &t.when;
            if timings.contains(&CompiledTiming::OnPlay)
                && timings.contains(&CompiledTiming::WhenDigivolving)
                && timings.contains(&CompiledTiming::WhenAttacking)
            {
                Some(t)
            } else {
                None
            }
        }
        _ => None,
    });
    let cost_clause = cost_clause.expect(
        "BT20-021 must carry a triggered clause on [On Play]+[When Digivolving]+[When Attacking]",
    );
    assert!(
        cost_clause.once_per_turn,
        "the place-source/delete clause is [Once Per Turn]"
    );
}

// ─── Section 2 — cost-delete clause (G-UNION-HAND-TRASH-SOURCE-COST) ──────────

/// On Play: place a [Royal Knight] card from hand as the bottom source, then
/// delete the eligible (<= source DP) opponent Digimon.
#[test]
fn bt20_021_on_play_places_rk_from_hand_then_deletes_eligible() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT20-021 compiles")
        .add_card(rk_card("RK-A"))
        .add_card(opp_digimon("OPP-12K", 12000))
        .add_card(filler("DECK-PAD"))
        .hand(0, &["RK-A"])
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    let weak = runner.place_on_field(1, "OPP-12K", None);

    let sources_before = runner.game.players[0].battle_area[jesmon.index as usize]
        .card_sources
        .len();
    let opp_before = runner.battle_area_size(1);
    let hand_before = runner.hand_size(0);

    runner.fire_on_play(0, jesmon.index as usize);

    assert!(
        runner.pending_selection().is_some(),
        "On Play must install a prompt to place a [Royal Knight] card as a bottom source"
    );

    // Place the RK card (optional cost), then delete the only eligible target.
    runner.auto_resolve().expect("place RK source + delete resolves");

    let sources_after = runner.game.players[0].battle_area[jesmon.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before + 1,
        "the [Royal Knight] card from hand must be added as a bottom digivolution source"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the placed RK card must leave the hand"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the eligible (12000 <= 16000 DP) opponent Digimon must be deleted"
    );
}

/// The delete filter is `DP <= this Digimon's DP` (16000). A 17000-DP opponent
/// must NOT be a legal delete target; a 16000-DP one (boundary, <=) must be.
#[test]
fn bt20_021_delete_filter_excludes_higher_dp_includes_equal() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT20-021 compiles")
        .add_card(rk_card("RK-A"))
        .add_card(opp_digimon("OPP-16K", 16000))
        .add_card(opp_digimon("OPP-17K", 17000))
        .add_card(filler("DECK-PAD"))
        .hand(0, &["RK-A"])
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    let equal = runner.place_on_field(1, "OPP-16K", None); // 16000 == 16000 → eligible
    let bigger = runner.place_on_field(1, "OPP-17K", None); // 17000 > 16000 → not

    runner.fire_on_play(0, jesmon.index as usize);

    // Accept/resolve the place-source cost first so the delete prompt installs.
    // Walk the selection chain manually so we can inspect the delete prompt.
    // 1) place-source prompt — accept it (pick the RK card, non-PASS).
    pick_first_non_pass(&mut runner);

    // 2) delete prompt — only the 16000-DP opponent may be offered.
    let del = runner
        .pending_selection()
        .expect("a delete-target selection must install after placing the source");
    let valid = del.valid_action_ids.clone();
    let eligible_action = encode_attack(0, equal.index as u16);
    let excluded_action = encode_attack(0, bigger.index as u16);
    assert!(
        valid.contains(&eligible_action),
        "the 16000-DP opponent (DP == this Digimon) must be a legal delete target"
    );
    assert!(
        !valid.contains(&excluded_action),
        "the 17000-DP opponent (DP > this Digimon) must NOT be a legal delete target"
    );

    // Resolve the rest; the 17000-DP Digimon must survive.
    runner.auto_resolve().expect("resolve delete");
    assert!(
        runner
            .game
            .players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-17K"),
        "the 17000-DP opponent Digimon must NOT be deleted"
    );
}

/// The place-source is an optional COST: declining it (PASS) skips the delete
/// entirely (no opponent Digimon is removed).
#[test]
fn bt20_021_declining_place_cost_skips_delete() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT20-021 compiles")
        .add_card(rk_card("RK-A"))
        .add_card(opp_digimon("OPP-12K", 12000))
        .add_card(filler("DECK-PAD"))
        .hand(0, &["RK-A"])
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-12K", None);
    let opp_before = runner.battle_area_size(1);

    runner.fire_on_play(0, jesmon.index as usize);

    // The place prompt must be optional (DCGO canNoSelect: true).
    let place = runner
        .pending_selection()
        .expect("place-source prompt installs");
    assert!(
        place.is_optional,
        "the place-source cost must be optional (DCGO canNoSelect: true)"
    );

    // Decline the cost.
    runner
        .decline_optional_trigger()
        .expect("decline the place-source cost");
    runner.auto_resolve().expect("nothing else to resolve");

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "declining the place-source cost must skip the delete (opponent Digimon survives)"
    );
}

/// No [Royal Knight] card in hand or trash → the cost is unpayable, so the
/// clause is a no-op (no delete prompt installs).
#[test]
fn bt20_021_no_rk_card_means_no_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT20-021 compiles")
        .add_card(plain_card("PLAIN"))
        .add_card(opp_digimon("OPP-12K", 12000))
        .add_card(filler("DECK-PAD"))
        .hand(0, &["PLAIN"])
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-12K", None);
    let opp_before = runner.battle_area_size(1);

    runner.fire_on_play(0, jesmon.index as usize);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "with no [Royal Knight] card in hand or trash, the cost is unpayable → no delete"
    );
}

/// The placeable card may come from TRASH as well as hand.
#[test]
fn bt20_021_places_rk_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT20-021 compiles")
        .add_card(rk_card("RK-A"))
        .add_card(opp_digimon("OPP-12K", 12000))
        .add_card(filler("DECK-PAD"))
        .deck(0, &["DECK-PAD"; 10])
        .deck(1, &["DECK-PAD"; 10])
        .memory(10)
        .start();

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-12K", None);
    runner.inject_trash(0, "RK-A");

    let sources_before = runner.game.players[0].battle_area[jesmon.index as usize]
        .card_sources
        .len();
    let opp_before = runner.battle_area_size(1);

    runner.fire_on_play(0, jesmon.index as usize);
    assert!(
        runner.pending_selection().is_some(),
        "a [Royal Knight] card in trash must let the place-source cost install"
    );
    runner.auto_resolve().expect("place from trash + delete resolves");

    let sources_after = runner.game.players[0].battle_area[jesmon.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before + 1,
        "the [Royal Knight] card from trash must be added as a bottom source"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the eligible opponent Digimon must be deleted"
    );
}

// ─── Section 3 — unsuspend + security trash (G-SOURCE-COUNT-SECURITY-TRASH) ───

/// When Attacking: unsuspend self, then trash floor(RK-source-count / 2)
/// opponent top security. With 4 RK sources → 2 security trashed.
#[test]
fn bt20_021_when_attacking_unsuspends_and_trashes_two_security_with_four_rk_sources() {
    let mut runner = base();
    runner.game.turn_count = 3;

    // BT20-021 on field with 4 [Royal Knight] digivolution sources beneath the
    // top card, plus an opponent Digimon to attack.
    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    for id in ["RK-A", "RK-B", "RK-C", "RK-D"] {
        runner.push_source(jesmon, id);
    }
    let defender = runner.place_on_field(1, "OPP-12K", None);

    // Give P1 a security stack to trash from.
    inject_security(&mut runner, 3);
    let sec_before = runner.security_count(1);

    // Suspend the attacker, then declare an attack (attack_digimon suspends it
    // again on declaration; the [When Attacking] clause must unsuspend it).
    runner.attack_digimon(jesmon, defender, false);

    // Resolve the When-Attacking clauses. The cost-delete clause may also fire,
    // but with no RK card in hand/trash it is a no-op; the unsuspend/trash
    // clause is mandatory.
    runner.auto_resolve().expect("when-attacking clauses resolve");

    assert!(
        !runner.game.players[0].battle_area[jesmon.index as usize].is_suspended,
        "BT20-021 must unsuspend itself via the [When Attacking] clause"
    );
    assert_eq!(
        runner.security_count(1),
        sec_before - 2,
        "4 [Royal Knight] sources → floor(4/2)=2 opponent top security trashed"
    );
}

/// 3 RK sources → floor(3/2) = 1 security trashed.
#[test]
fn bt20_021_three_rk_sources_trashes_one_security() {
    let mut runner = base();
    runner.game.turn_count = 3;

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    for id in ["RK-A", "RK-B", "RK-C"] {
        runner.push_source(jesmon, id);
    }
    let defender = runner.place_on_field(1, "OPP-12K", None);
    inject_security(&mut runner, 3);
    let sec_before = runner.security_count(1);

    runner.attack_digimon(jesmon, defender, false);
    runner.auto_resolve().expect("resolve");

    assert!(
        !runner.game.players[0].battle_area[jesmon.index as usize].is_suspended,
        "BT20-021 still unsuspends even with an odd RK-source count"
    );
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "3 [Royal Knight] sources → floor(3/2)=1 security trashed"
    );
}

/// 1 RK source → floor(1/2) = 0 security trashed (unsuspend still happens).
#[test]
fn bt20_021_one_rk_source_trashes_no_security_but_unsuspends() {
    let mut runner = base();
    runner.game.turn_count = 3;

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(jesmon, "RK-A");
    let defender = runner.place_on_field(1, "OPP-12K", None);
    inject_security(&mut runner, 3);
    let sec_before = runner.security_count(1);

    runner.attack_digimon(jesmon, defender, false);
    runner.auto_resolve().expect("resolve");

    assert!(
        !runner.game.players[0].battle_area[jesmon.index as usize].is_suspended,
        "BT20-021 unsuspends regardless of source count"
    );
    assert_eq!(
        runner.security_count(1),
        sec_before,
        "1 [Royal Knight] source → floor(1/2)=0 security trashed"
    );
}

/// Non-Royal-Knight sources are NOT counted: 2 RK + 2 plain sources → still
/// floor(2/2) = 1 security trashed (not floor(4/2)=2).
#[test]
fn bt20_021_non_rk_sources_not_counted() {
    let mut runner = base();
    runner.game.turn_count = 3;

    let jesmon = runner.place_on_field(0, CARD_ID, Some(0));
    // 2 Royal Knight + 2 plain sources.
    for id in ["RK-A", "PLAIN", "RK-B", "PLAIN"] {
        runner.push_source(jesmon, id);
    }
    let defender = runner.place_on_field(1, "OPP-12K", None);
    inject_security(&mut runner, 3);
    let sec_before = runner.security_count(1);

    runner.attack_digimon(jesmon, defender, false);
    runner.auto_resolve().expect("resolve");

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "only the 2 [Royal Knight] sources count → floor(2/2)=1 security trashed"
    );
}

// ─── Section 4 — helpers ─────────────────────────────────────────────────────

/// Inject `n` filler cards into player 1's security stack.
fn inject_security(runner: &mut DebugRunner, n: usize) {
    for _ in 0..n {
        runner
            .game
            .stage_inject_card(1, "DECK-PAD", "security_top")
            .expect("inject security card");
    }
}

/// Resolve the first non-PASS action of the pending selection (accepting an
/// optional cost prompt by picking a real candidate).
fn pick_first_non_pass(runner: &mut DebugRunner) {
    let (player, action) = {
        let p = runner
            .pending_selection()
            .expect("a selection must be pending");
        let action = *p
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("at least one non-PASS candidate");
        (p.selecting_player, action)
    };
    runner
        .execute_action(player, action)
        .expect("pick_first_non_pass resolves");
}
