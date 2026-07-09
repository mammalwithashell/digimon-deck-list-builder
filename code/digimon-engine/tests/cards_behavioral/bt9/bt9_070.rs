//! BT9-070 Gazimon (X Antibody) — Digimon, Lv.3, Purple, Cost 3, DP 3000.
//! Traits: Mammal / X Antibody. Form: Rookie. Attribute: Virus.
//!
//! # Card text (card image + official Bandai DB bundle — verbatim)
//!
//! ```text
//! Digivolve: 0 from [Gazimon]
//! [When Digivolving] Trash the top 3 cards of your deck.
//! ```
//!
//! No inherited/security effect, no [Once Per Turn], no [On Play].
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT9/Purple/BT9_070.cs —
//! `AddSelfDigivolutionRequirementStaticEffect` with
//! `PermanentCondition = TopCard.CardNames.Contains("Gazimon")` (EXACT
//! element-membership over the base's effective name list, NOT a substring
//! scan) at digivolutionCost 0. `OnEnterFieldAnyone`: mandatory
//! `IAddTrashCardsFromLibraryTop(3, card.Owner, ...)` gated only on
//! `IsExistOnBattleArea` + `LibraryCards.Count >= 1`.
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt9/BT9-070.yaml
//!
//! # Faithfulness note — exact-name alt-path gate
//! The cost-0 "[Digivolve] 0 from [Gazimon]" alt-path is authored as
//! `from: { name_is: "Gazimon" }` (exact match), NOT `name_contains`. DCGO's
//! `List<string>.Contains("Gazimon")` is exact element-membership, so a base
//! whose name merely CONTAINS "Gazimon" (e.g. this very card, "Gazimon (X
//! Antibody)") must NOT qualify for the cost-0 route. Section 3 below pins
//! both the positive (exact "Gazimon" base) and negative (substring-only
//! base) cases.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::CompiledAltPathKind;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT9-070";

// ─── helpers ──────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Lv.2 Purple Digimon named exactly "Gazimon" — the real base card
/// (BT3-077 in the actual pool), qualifying for BOTH the cost-0 alt-path
/// (exact-name gate) AND the standard Lv.2 Purple / cost 0 circle.
fn make_exact_gazimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Gazimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(2000);
    c.colors = vec![digimon_engine::enums::CardColor::Purple];
    c
}

/// A Lv.3 Red Digimon whose name CONTAINS "Gazimon" as a substring but is
/// not exactly "Gazimon" — must be REJECTED by the exact-name alt-path gate.
/// Deliberately off-level/off-color (Lv.3 Red, not Lv.2 Purple) so it does
/// NOT separately qualify for the standard Lv.2 Purple / cost 0 circle —
/// isolating the exact-name alt-path as the only route under test.
fn make_substring_gazimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Gazimon (X Antibody)");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![digimon_engine::enums::CardColor::Red];
    c
}

/// A Lv.2 unrelated Digimon (not Purple, not named Gazimon) — must not
/// qualify for either alt-path.
fn make_unrelated_lvl2(id: &str) -> CardData {
    let mut c = make_test_card(id, "Palmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(1000);
    c.colors = vec![digimon_engine::enums::CardColor::Green];
    c
}

fn find_hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> Option<usize> {
    let data = &runner.game.card_data;
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(data) == card_id)
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 in embedded DSL pack")
        .add_card(make_exact_gazimon("EXACT-GAZIMON"))
        .add_card(make_substring_gazimon("SUB-GAZIMON"))
        .add_card(make_unrelated_lvl2("UNRELATED"))
        .add_card(make_filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt9_070_compiles_from_dsl_pack() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 in embedded pack")
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(3000));
    assert!(card.traits.iter().any(|t| t == "Mammal"));
    assert!(card.traits.iter().any(|t| t == "X Antibody"));
}

/// The cost-0 "[Digivolve] 0 from [Gazimon]" alt-path must use an EXACT
/// name match (`name_is == Some("Gazimon")`), not a substring match
/// (`name_contains`). This is the reviewer-mandated fix: DCGO's
/// `CardNames.Contains("Gazimon")` is exact element-membership, and a
/// substring gate would illegally let a "Gazimon (X Antibody)" base (this
/// very card) satisfy its own cost-0 alt-path.
#[test]
fn bt9_070_has_gazimon_name_gated_cost_0_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 in embedded pack")
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    let exact_name_cost0 = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(digimon_dsl::compiled::CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .and_then(|f| f.name_is.as_deref())
                .map(|n| n == "Gazimon")
                .unwrap_or(false)
    });
    assert!(
        exact_name_cost0,
        "must have a cost-0 digivolve alt-path gated on EXACT name \"Gazimon\" (name_is), not name_contains"
    );

    // Guard against regression to the substring form.
    let uses_substring_gate = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(digimon_dsl::compiled::CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .and_then(|f| f.name_contains.as_deref())
                .map(|n| n.eq_ignore_ascii_case("Gazimon"))
                .unwrap_or(false)
    });
    assert!(
        !uses_substring_gate,
        "the cost-0 alt-path must NOT use name_contains (substring match) — \
         DCGO's CardNames.Contains(\"Gazimon\") is exact-name matching"
    );
}

/// Standard printed route (cards.json evo_costs: Lv.2 Purple / 0) must also
/// exist as an explicit alt_path (G-DSL-FIXTURE-EVO-COSTS: compiled cards
/// carry no printed evo_costs in fixture tests).
#[test]
fn bt9_070_has_standard_lvl2_purple_cost_0_alt_path() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 in embedded pack")
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    let standard_route = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(digimon_dsl::compiled::CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .and_then(|f| f.level_eq)
                .map(|l| l == 2)
                .unwrap_or(false)
            && p.from.as_ref().and_then(|f| f.color_is).is_some()
    });
    assert!(
        standard_route,
        "must have the standard Lv.2 Purple / cost 0 digivolve alt-path"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving] mandatory mill 3
// ════════════════════════════════════════════════════════════════════════════

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

/// Digivolving into BT9-070 mills the top 3 cards of the controller's deck.
/// No player choice is exposed (mandatory, unconditional). BT9-070 itself
/// must be the TOP card of the permanent (the carrier of the effect) — place
/// it stacked on top of a base, mirroring the EX3-057 precedent.
#[test]
fn bt9_070_when_digivolving_mills_top_3() {
    let mut r = base_runner();
    let stack = r.place_stack(0, &["EXACT-GAZIMON", CARD_ID]);
    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);

    fire(&mut r, EffectTiming::WhenDigivolving, stack);

    assert!(
        r.game.pending_selection.is_none(),
        "mandatory unconditional mill exposes no player choice"
    );
    assert_eq!(
        r.deck_size(0),
        deck_before - 3,
        "deck must shrink by 3 from the mandatory mill"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 3,
        "3 cards must land in the trash"
    );
}

/// Short deck: with fewer than 3 cards remaining, the mill naturally trashes
/// only what's available and does not panic or block resolution.
#[test]
fn bt9_070_when_digivolving_mills_short_deck_without_panic() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 in embedded pack")
        .add_card(make_exact_gazimon("EXACT-GAZIMON"))
        .add_card(make_filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"]) // only 1 card in deck
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let stack = r.place_stack(0, &["EXACT-GAZIMON", CARD_ID]);
    let trash_before = r.trash_size(0);

    fire(&mut r, EffectTiming::WhenDigivolving, stack);

    assert_eq!(r.deck_size(0), 0, "deck is exhausted, not underflowed");
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "only the 1 available card is milled"
    );
}

/// Empty deck: the mill naturally no-ops without panicking.
#[test]
fn bt9_070_when_digivolving_mills_empty_deck_without_panic() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT9-070 in embedded pack")
        .add_card(make_exact_gazimon("EXACT-GAZIMON"))
        .add_card(make_filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &[]) // empty deck
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let stack = r.place_stack(0, &["EXACT-GAZIMON", CARD_ID]);
    let trash_before = r.trash_size(0);

    fire(&mut r, EffectTiming::WhenDigivolving, stack);

    assert_eq!(r.deck_size(0), 0);
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "empty deck ⇒ no cards trashed, no panic"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — exact-name digivolve gate: positive / negative
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: a base named exactly "Gazimon" qualifies for the cost-0
/// alt-path via `Game::digivolve_from_hand`.
#[test]
fn bt9_070_alt_path_accepts_exact_gazimon_base_at_cost_0() {
    let mut r = base_runner();
    let base = r.place_on_field(0, "EXACT-GAZIMON", Some(0));
    let mem_before = r.memory();

    let hand_idx = find_hand_index(&r, 0, CARD_ID).expect("BT9-070 in hand");
    let ok = r
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        ok,
        "digivolve_from_hand must succeed from an exact-name \"Gazimon\" base"
    );
    r.game.drain_effect_queue();

    assert_eq!(
        r.memory(),
        mem_before,
        "cost-0 alt-path must not spend memory"
    );
    assert_eq!(
        r.battle_area_size(0),
        1,
        "the base is now the BT9-070 permanent (stacked, not a new permanent)"
    );
}

/// NEGATIVE: a base whose name merely CONTAINS "Gazimon" as a substring
/// (e.g. "Gazimon (X Antibody)") must NOT qualify for the cost-0 alt-path.
/// This is the reviewer-mandated regression guard for the
/// name_contains → name_is faithfulness fix. SUB-GAZIMON is deliberately
/// Lv.3 Red (not Lv.2 Purple), so it also fails to qualify for the standard
/// circle — isolating the exact-name alt-path as the only possible route,
/// and confirming `digivolve_from_hand` is rejected entirely.
#[test]
fn bt9_070_alt_path_rejects_substring_gazimon_base_at_cost_0() {
    let mut r = base_runner();
    let base = r.place_on_field(0, "SUB-GAZIMON", Some(0));

    let hand_idx = find_hand_index(&r, 0, CARD_ID).expect("BT9-070 in hand");
    let ok = r
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);

    assert!(
        !ok,
        "a substring-only \"Gazimon (X Antibody)\" base (Lv.3 Red, no \
         qualifying standard circle) must NOT unlock the cost-0 alt-path — \
         name_is requires an EXACT name match, not name_contains"
    );
}

/// NEGATIVE (structural control): an unrelated Lv.2 non-Purple, non-Gazimon
/// base has no qualifying route at all — confirms the test fixtures
/// themselves are discriminating correctly.
#[test]
fn bt9_070_rejects_unrelated_base_entirely() {
    let mut r = base_runner();
    let base = r.place_on_field(0, "UNRELATED", Some(0));

    let hand_idx = find_hand_index(&r, 0, CARD_ID).expect("BT9-070 in hand");
    let ok = r
        .game
        .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByDigivolve);
    assert!(
        !ok,
        "an unrelated Lv.2 non-Purple, non-Gazimon base must not permit digivolving BT9-070 onto it"
    );
}
