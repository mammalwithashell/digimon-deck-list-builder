//! BT25-066 Guardromon — Digimon, Lv.4, Black, DP 5000, Cost 5.
//! Trait line: Machine / Iliad / TS. Attribute: Virus.
//!
//! # Card text (cards.json + DCGO BT25_066.cs — authoritative)
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [All Turns] When this Digimon would leave the battle area, by trashing 1 of
//!   its link cards, it doesn't leave.
//! Inherited [All Turns]: this Digimon gets +1000 DP.
//! Alt-digivolution requirement (DCGO AddSelfDigivolutionRequirementStaticEffect,
//!   PermanentCondition = TopCard.HasTSTraits, level 3, cost 2): may digivolve
//!   from a level-3 [TS] Digimon for cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_066.cs
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3)
//! - grant_keyword Blocker (declarative, self)
//! - kind: replacement when_would_leave_battle_area + cost: trash_own_link_card
//!   (optional / SetIsSkippable, gated on having >=1 link card; surfaces the
//!   WHICH-link-card-to-trash choice even for a single card) — G-DSL-LINK-TRASH-
//!   AS-REPLACEMENT-COST.
//! - inherited aura +1000 DP
//! - alt_paths digivolve from { level_eq: 3, trait_has: TS } cost 2

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-066";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-066 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("LINKEE", 3, 2000, 3, &["TS"]))
        .add_card(make_digimon("TS-LV3", 3, 4000, 3, &["TS"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_066_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Guardromon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.cost, Some(5));
    assert!(
        card.traits.iter().any(|t| t == "TS"),
        "Guardromon carries the TS trait"
    );
}

#[test]
fn bt25_066_grants_blocker() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_blocker = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Blocker" && *scope == CompiledScope::FaceUp
        )
    });
    assert!(has_blocker, "BT25-066 grants <Blocker> (self, non-inherited)");
}

#[test]
fn bt25_066_has_inherited_dp_buff() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_inherited_dp = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, dp_modifier, .. })
                if *scope == CompiledScope::Inherited && *dp_modifier == Some(1000)
        )
    });
    assert!(
        has_inherited_dp,
        "BT25-066 inherited [All Turns] grants +1000 DP"
    );
}

#[test]
fn bt25_066_has_ts_lv3_cost2_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("must have a digivolve alt-path");
    assert_eq!(path.cost, Some(CompiledCost::Literal(2)));
}

// ─── Section 2 — Blocker behavior (live keyword) ─────────────────────────────

#[test]
fn bt25_066_blocker_keyword_is_live_on_field() {
    let mut r = base().memory(5).start();
    let guard = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(guard, Keyword::Blocker),
        "BT25-066 has <Blocker> on the field"
    );
}

// ─── Section 3 — Leave-replacement: trash 1 link card → doesn't leave ─────────

#[test]
fn bt25_066_leave_replacement_trashes_link_card_and_stays() {
    let mut r = base().memory(0).start();
    let guard = r.place_on_field(0, CARD_ID, Some(0));
    r.push_linked_owned(guard, "LINKEE", 0);
    advance_to_main(&mut r);

    let trash_before = r.trash_size(0);
    r.game.delete_permanent_with_effects(guard);

    // The "by trashing... it doesn't leave" is a *may* (SetIsSkippable) — the
    // controller is offered the optional accept prompt.
    assert!(
        r.game.pending_selection.is_some(),
        "leave-replacement offers the optional accept prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the replacement");

    // The WHICH-link-card-to-trash choice is surfaced even with a single card
    // (no auto-selection).
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("link-card-trash selection installed")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick link card");

    assert_eq!(r.battle_area_size(0), 1, "the Digimon did NOT leave");
    assert_eq!(
        r.game.player(0).battle_area[guard.index as usize]
            .linked_cards
            .len(),
        0,
        "the chosen link card was trashed as the cost"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "exactly the link card went to trash"
    );
}

#[test]
fn bt25_066_leave_replacement_declined_lets_it_leave() {
    let mut r = base().memory(0).start();
    let guard = r.place_on_field(0, CARD_ID, Some(0));
    r.push_linked_owned(guard, "LINKEE", 0);
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(guard);
    assert!(
        r.game.pending_selection.is_some(),
        "the optional accept prompt is offered"
    );
    // Decline (PASS): the Digimon leaves and the link card is NOT trashed.
    r.game
        .resolve_selection(0, PASS)
        .expect("decline the replacement");

    assert_eq!(r.battle_area_size(0), 0, "declined → the Digimon leaves");
}

#[test]
fn bt25_066_leave_replacement_not_offered_without_link_cards() {
    let mut r = base().memory(0).start();
    let guard = r.place_on_field(0, CARD_ID, Some(0));
    // No link cards attached.
    advance_to_main(&mut r);

    r.game.delete_permanent_with_effects(guard);
    assert!(
        r.game.pending_selection.is_none(),
        "0 link cards → the replacement is gated off (not offered)"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "with no link card to trash, the Digimon leaves normally"
    );
}
