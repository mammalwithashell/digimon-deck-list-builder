//! EX7-051 Sparrowmon — Digimon, Lv.3, Purple, DP 1000, Cost 3.
//! Traits: Avian. Form: Rookie. Attribute: Data.
//!
//! # Card text (data/card_bundles/EX7-051.md — official Bandai DB, verbatim;
//! cross-checked against the card image EX7-051.webp)
//!
//! [Start of Your Main Phase] By placing 1 Option card with the [Three
//! Musketeers] trait from your hand or trash as 1 of your Digimon's bottom
//! digivolution card, ＜Draw 1＞.
//!
//! Inherited Effect: ＜Retaliation＞ (When only this Digimon is deleted in
//! battle, delete the Digimon it battled.)
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Purple Lv.2 / cost 0
//!   - xros_req: "[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Purple/EX7_051.cs
//!
//! # DCGO crosscheck
//! - [Start of Your Main Phase]: ActivateClass(isOptional: true); CanUse =
//!   IsExistOnBattleAreaDigimon && IsOwnerTurn. Body: hand/trash area choice
//!   (SetBoolSelection), then SelectHandEffect / SelectCardEffect(Root.Trash)
//!   over `IsOption && ContainsTraits("Three Musketeers")` (canNoSelect: true),
//!   then SelectPermanentEffect over own battle-area Digimon that are NOT
//!   tokens → AddDigivolutionCardsBottom → DrawClass(owner, 1).
//! - ESS: RetaliationSelfEffect(isInheritedEffect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - B1-adjacent: start-of-main trigger on a Digimon
//! - A6: union (hand ∪ trash) pick placed as another permanent's bottom source
//! - E2: declinable cost gating a Draw 1
//! - H: inherited <Retaliation> grant

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

const CARD_ID: &str = "EX7-051";
/// EX7-070 Der Blitz — a REAL [Three Musketeers]-trait Option.
const DER_BLITZ: &str = "EX7-070";

fn digimon(id: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn option(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Purple];
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn token(id: &str) -> CardData {
    let mut c = digimon(id, 3, CardColor::Purple, &[]);
    c.card_kind = CardKind::Token;
    c.play_cost = 0;
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(data).to_string()).collect()
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-051 YAML loads from the embedded pack")
        .dsl_card(DER_BLITZ)
        .expect("EX7-070 YAML loads from the embedded pack")
        .add_card(option("PLAIN-OPT", &[]))
        .add_card(digimon("TM-DIGI", 4, CardColor::Purple, &["Three Musketeers"]))
        .add_card(digimon("OTHER", 4, CardColor::Purple, &[]))
        .add_card(token("TOKEN"))
        .add_card(filler("FILL"))
        .add_card(digimon("CARRIER", 4, CardColor::Purple, &[]))
        .deck(0, &["FILL"; 6])
        .memory(5)
}

fn sources_of(runner: &DebugRunner, handle: digimon_engine::permanent::PermanentHandle) -> usize {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .len()
}

/// Enter Sparrowmon's controller's Main phase from a fresh runner where the
/// permanent is already staged on the field.
fn enter_main(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_051_has_optional_start_of_main_clause_and_inherited_retaliation() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::StartOfYourMainPhase]);
    assert!(triggered[0].optional, "'By placing ...' is a declinable cost");
    assert!(!triggered[0].once_per_turn);

    let retaliation = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, keyword, .. })
                if *scope == CompiledScope::Inherited && keyword == "Retaliation"
        )
    });
    assert!(retaliation, "inherited <Retaliation> grant_keyword clause must be present");

    let digivolve_paths = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(digivolve_paths, 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_051_start_of_main_prompts_union_pick_when_tm_option_in_hand() {
    let mut runner = base().hand(0, &[DER_BLITZ]).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    enter_main(&mut runner);
    let view = runner.pending_selection_view().expect("union prompt installs");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(runner.pending_is_optional(), "the cost is declinable");
}

#[test]
fn ex7_051_start_of_main_no_prompt_without_tm_option() {
    let mut runner = base().hand(0, &["PLAIN-OPT", "TM-DIGI"]).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.inject_trash(0, "PLAIN-OPT");
    enter_main(&mut runner);
    assert!(
        runner.pending_selection().is_none(),
        "a plain Option and a TM-trait DIGIMON are not 'Option cards with the [Three Musketeers] trait'"
    );
}

#[test]
fn ex7_051_union_pick_offers_only_tm_options_from_hand_and_trash() {
    let mut runner = base().hand(0, &["PLAIN-OPT", DER_BLITZ, "TM-DIGI"]).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.inject_trash(0, DER_BLITZ);
    runner.inject_trash(0, "PLAIN-OPT");
    enter_main(&mut runner);
    let view = runner.pending_selection_view().expect("union prompt");
    let hand_picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS && a < PLAY_HAND_START + 30)
        .collect();
    let trash_picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a >= TRASH_EFFECT_START)
        .collect();
    assert_eq!(hand_picks, vec![PLAY_HAND_START + 1], "only Der Blitz (hand idx 1)");
    assert_eq!(trash_picks.len(), 1, "only the trashed Der Blitz");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral outcome
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_051_places_hand_option_under_chosen_digimon_and_draws_one() {
    let mut runner = base().hand(0, &[DER_BLITZ]).start();
    let sparrow = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "OTHER", Some(0));
    enter_main(&mut runner);
    let deck_before = runner.deck_size(0);

    runner.execute_action(0, PLAY_HAND_START).expect("pick Der Blitz from hand");
    let view = runner.pending_selection_view().expect("own Digimon prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.valid_action_ids.contains(&PASS), "once the card is picked, the host pick is mandatory");
    assert_eq!(view.valid_action_ids.len(), 2, "Sparrowmon and OTHER are both legal hosts");
    // Pick the LAST legal host (OTHER — placed after Sparrowmon).
    let host_pick = *view.valid_action_ids.last().unwrap();
    runner.execute_action(0, host_pick).expect("choose OTHER");
    runner.auto_resolve().expect("finish");

    assert_eq!(sources_of(&runner, other), 2, "Der Blitz is now OTHER's bottom digivolution card");
    assert_eq!(sources_of(&runner, sparrow), 1, "Sparrowmon untouched");
    let bottom = runner.game.players[0].battle_area[other.index as usize].card_sources[0]
        .card_id(&runner.game.card_data);
    assert_eq!(bottom, DER_BLITZ, "placed as the BOTTOM source");
    assert_eq!(runner.deck_size(0), deck_before - 1, "Draw 1");
    assert_eq!(runner.hand_size(0), 1, "-1 placed, +1 drawn");
}

#[test]
fn ex7_051_places_trash_option_under_self_and_draws_one() {
    let mut runner = base().start();
    let sparrow = runner.place_on_field(0, CARD_ID, Some(0));
    runner.inject_trash(0, DER_BLITZ);
    enter_main(&mut runner);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    runner.execute_action(0, TRASH_EFFECT_START).expect("pick Der Blitz from trash");
    let view = runner.pending_selection_view().expect("own Digimon prompt");
    assert_eq!(view.valid_action_ids.len(), 1, "Sparrowmon is the only host");
    runner.execute_action(0, view.valid_action_ids[0]).expect("choose Sparrowmon");
    runner.auto_resolve().expect("finish");

    assert_eq!(sources_of(&runner, sparrow), 2, "Der Blitz placed under Sparrowmon");
    assert_eq!(runner.trash_size(0), trash_before - 1, "left the trash");
    assert_eq!(runner.deck_size(0), deck_before - 1, "Draw 1");
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn ex7_051_decline_draws_nothing_and_moves_nothing() {
    let mut runner = base().hand(0, &[DER_BLITZ]).start();
    let sparrow = runner.place_on_field(0, CARD_ID, Some(0));
    enter_main(&mut runner);
    let deck_before = runner.deck_size(0);
    runner.execute_action(0, PASS).expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.deck_size(0), deck_before, "no draw on decline");
    assert_eq!(runner.hand_size(0), 1, "Der Blitz stays in hand");
    assert_eq!(sources_of(&runner, sparrow), 1);
}

#[test]
fn ex7_051_token_is_not_a_legal_host() {
    let mut runner = base().hand(0, &[DER_BLITZ]).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "TOKEN", Some(0));
    enter_main(&mut runner);
    runner.execute_action(0, PLAY_HAND_START).expect("pick Der Blitz");
    let view = runner.pending_selection_view().expect("own Digimon prompt");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "DCGO `!permanent.IsToken`: a token cannot receive the digivolution card"
    );
}

#[test]
fn ex7_051_does_not_fire_on_opponents_main_phase() {
    let mut runner = base().hand(0, &[DER_BLITZ]).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn(); // P1's turn — P1's start-of-main must not fire P0's Sparrowmon
    runner.auto_resolve().ok();
    assert!(
        runner.pending_selection().is_none(),
        "[Start of YOUR Main Phase] — nothing fires on the opponent's turn"
    );
    assert_eq!(runner.hand_size(0), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited <Retaliation> + digivolution paths
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_051_inherited_retaliation_grants_keyword_to_carrier() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(runner.game.has_keyword(stack, Keyword::Retaliation));
    let alone = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(!runner.game.has_keyword(alone, Keyword::Retaliation), "inherited only");
}

#[test]
fn ex7_051_digivolves_for_zero_from_lv2_with_tm_in_text_regardless_of_color() {
    let mut red_tm_lv2 = digimon("RED-TM-LV2", 2, CardColor::Red, &[]);
    red_tm_lv2.effect_text = "... [Three Musketeers] ...".to_string();
    let mut runner = base().add_card(red_tm_lv2).hand(0, &[CARD_ID]).start();
    let base_perm = runner.place_on_field(0, "RED-TM-LV2", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before, "cost 0");
}

#[test]
fn ex7_051_cannot_digivolve_from_plain_red_lv2() {
    let mut runner = base()
        .add_card(digimon("RED-LV2", 2, CardColor::Red, &[]))
        .hand(0, &[CARD_ID])
        .start();
    let base_perm = runner.place_on_field(0, "RED-LV2", Some(0));
    assert!(!runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
}
