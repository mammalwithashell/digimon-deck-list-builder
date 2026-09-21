//! EX7-011 Megadramon — Digimon, Lv.5, Red, DP 7000, Cost 7.
//! Traits: Cyborg. Form: Ultimate. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX7-011.md — official Bandai DB, verbatim)
//!
//! [On Play] [When Digivolving] By placing 1 Option card with the [Three
//! Musketeers] trait from your hand or trash as this Digimon's bottom
//! digivolution card, delete 1 of your opponent's Digimon with 6000 DP or
//! less.
//!
//! Inherited Effect: ＜Piercing＞ (When this Digimon attacks and deletes your
//! opponent's Digimon in battle, it checks security before the attack ends.)
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Red Lv.4 / cost 3
//!   - xros_req: "[Digivolve] Lv.4 w/[Three Musketeers] in text: Cost 3"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Red/EX7_011.cs
//!
//! # DCGO crosscheck
//! - Alt-digivolve: `HasText("Three Musketeers") && Level == 4`, cost 3.
//! - OP and WD: two ActivateClass entries (isOptional TRUE) with identical
//!   bodies. CanActivate = a TM-trait Option in hand OR trash. Body: hand /
//!   trash area choice, SelectHandEffect / SelectCardEffect(Root.Trash) over
//!   `IsOption && ContainsTraits("Three Musketeers")` (canNoSelect: true) →
//!   `card.PermanentOfThisCard().AddDigivolutionCardsBottom(selected)` →
//!   `if (HasMatchConditionPermanent(opp Digimon, DP <= MaxDP_DeleteEffect(6000)))`
//!   mandatory SelectPermanentEffect(Destroy).
//! - ESS: PierceSelfEffect(isInheritedEffect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A6/E2: declinable union (hand ∪ trash) placement-as-cost under self
//! - DP-capped opponent deletion (6000 or less)
//! - H3: inherited <Piercing>

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-011";
/// EX7-070 Der Blitz — a REAL [Three Musketeers]-trait Option.
const DER_BLITZ: &str = "EX7-070";

fn digimon(id: &str, level: u8, color: CardColor, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

fn option(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Red];
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

fn sources_of(runner: &DebugRunner, h: PermanentHandle) -> usize {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-011 YAML loads from the embedded pack")
        .dsl_card(DER_BLITZ)
        .expect("EX7-070 YAML loads from the embedded pack")
        .add_card(option("PLAIN-OPT", &[]))
        .add_card(digimon("TM-DIGI", 4, CardColor::Red, 4000))
        .add_card(digimon("RED-LV4", 4, CardColor::Red, 4000))
        .add_card(digimon("OPP-6000", 4, CardColor::Blue, 6000))
        .add_card(digimon("OPP-7000", 5, CardColor::Blue, 7000))
        .add_card(digimon("OPP-3000", 3, CardColor::Blue, 3000))
        .add_card(filler("FILL"))
        .add_card(digimon("CARRIER", 6, CardColor::Red, 11000))
        .deck(0, &["FILL"; 4])
        .memory(10)
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_011_structure_matches_printed_text() {
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
    assert_eq!(triggered[0].when, vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]);
    assert!(triggered[0].optional, "'By placing ...' is declinable");

    let piercing = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, keyword, .. })
                if *scope == CompiledScope::Inherited && keyword == "Piercing"
        )
    });
    assert!(piercing, "inherited <Piercing>");
    let paths = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(paths, 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_011_on_play_no_prompt_without_tm_option_in_hand_or_trash() {
    let mut runner = base().hand(0, &[CARD_ID, "PLAIN-OPT", "TM-DIGI"]).start();
    runner.inject_trash(0, "PLAIN-OPT");
    runner.place_on_field(1, "OPP-3000", Some(0));
    runner.play(0, 0).expect("play Megadramon");
    assert!(runner.pending_selection().is_none(), "the placement cost is unpayable");
    assert_eq!(runner.battle_area_size(1), 1, "no deletion");
}

#[test]
fn ex7_011_on_play_prompts_union_pick_with_tm_option_in_trash() {
    let mut runner = base().hand(0, &[CARD_ID]).start();
    runner.inject_trash(0, DER_BLITZ);
    runner.play(0, 0).expect("play Megadramon");
    let view = runner.pending_selection_view().expect("union prompt");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(runner.pending_is_optional(), "declinable");
    assert!(view.valid_action_ids.contains(&TRASH_EFFECT_START));
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral outcome
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_011_places_option_from_hand_under_self_and_deletes_low_dp_opponent() {
    let mut runner = base().hand(0, &[CARD_ID, DER_BLITZ]).start();
    runner.place_on_field(1, "OPP-6000", Some(0));
    runner.place_on_field(1, "OPP-7000", Some(0));
    let mega_idx = runner.play(0, 0).expect("play Megadramon");
    let mega = PermanentHandle { player: 0, index: mega_idx as u8 };

    runner.execute_action(0, PLAY_HAND_START).expect("pick Der Blitz from hand");
    let view = runner.pending_selection_view().expect("delete target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory once the cost is paid");
    assert_eq!(view.valid_action_ids.len(), 1, "only the 6000 DP Digimon qualifies (7000 excluded)");
    runner.execute_action(0, view.valid_action_ids[0]).expect("delete OPP-6000");
    runner.auto_resolve().expect("finish");

    assert_eq!(sources_of(&runner, mega), 2, "Der Blitz is Megadramon's bottom digivolution card");
    let bottom = runner.game.players[0].battle_area[mega.index as usize].card_sources[0]
        .card_id(&runner.game.card_data);
    assert_eq!(bottom, DER_BLITZ);
    assert_eq!(runner.hand_size(0), 0);
    assert_eq!(runner.battle_area_size(1), 1, "OPP-6000 deleted, OPP-7000 survives");
    let survivor = runner.game.players[1].battle_area[0]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(survivor, "OPP-7000");
}

#[test]
fn ex7_011_places_option_from_trash_under_self() {
    let mut runner = base().hand(0, &[CARD_ID]).start();
    runner.inject_trash(0, DER_BLITZ);
    runner.place_on_field(1, "OPP-3000", Some(0));
    let mega_idx = runner.play(0, 0).expect("play Megadramon");
    let mega = PermanentHandle { player: 0, index: mega_idx as u8 };
    runner.execute_action(0, TRASH_EFFECT_START).expect("pick Der Blitz from trash");
    let view = runner.pending_selection_view().expect("delete target prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("delete OPP-3000");
    runner.auto_resolve().expect("finish");
    assert_eq!(sources_of(&runner, mega), 2);
    assert_eq!(runner.trash_size(0), 0, "Der Blitz left the trash");
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn ex7_011_decline_places_nothing_and_deletes_nothing() {
    let mut runner = base().hand(0, &[CARD_ID, DER_BLITZ]).start();
    runner.place_on_field(1, "OPP-3000", Some(0));
    let mega_idx = runner.play(0, 0).expect("play Megadramon");
    let mega = PermanentHandle { player: 0, index: mega_idx as u8 };
    runner.execute_action(0, PASS).expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(sources_of(&runner, mega), 1);
    assert_eq!(runner.hand_size(0), 1, "Der Blitz stays in hand");
    assert_eq!(runner.battle_area_size(1), 1, "no deletion without paying the cost");
}

#[test]
fn ex7_011_cost_paid_but_no_legal_target_still_places_the_option() {
    let mut runner = base().hand(0, &[CARD_ID, DER_BLITZ]).start();
    runner.place_on_field(1, "OPP-7000", Some(0));
    let mega_idx = runner.play(0, 0).expect("play Megadramon");
    let mega = PermanentHandle { player: 0, index: mega_idx as u8 };
    runner.execute_action(0, PLAY_HAND_START).expect("pick Der Blitz");
    assert!(runner.pending_selection().is_none(), "7000 DP > 6000 → no legal target, no prompt");
    assert_eq!(sources_of(&runner, mega), 2, "the cost was paid regardless");
    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn ex7_011_when_digivolving_offers_the_same_effect() {
    let mut runner = base().hand(0, &[CARD_ID, DER_BLITZ]).start();
    let base_perm = runner.place_on_field(0, "RED-LV4", Some(0));
    runner.place_on_field(1, "OPP-6000", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before - 3, "Red Lv.4 / cost 3");
    runner.execute_action(0, PLAY_HAND_START).expect("pick Der Blitz");
    let view = runner.pending_selection_view().expect("delete target prompt");
    runner.execute_action(0, view.valid_action_ids[0]).expect("delete");
    runner.auto_resolve().expect("finish");
    assert_eq!(sources_of(&runner, base_perm), 3, "RED-LV4 + Der Blitz + Megadramon");
    assert_eq!(runner.battle_area_size(1), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited <Piercing> + digivolution paths
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_011_inherited_piercing_grants_keyword_to_carrier() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(runner.game.has_keyword(stack, Keyword::Piercing));
    let alone = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(!runner.game.has_keyword(alone, Keyword::Piercing), "inherited only");
}

#[test]
fn ex7_011_digivolves_from_black_lv4_with_tm_in_text_for_three() {
    let mut blk = digimon("BLK-TM-LV4", 4, CardColor::Black, 4000);
    blk.effect_text = "... [Three Musketeers] ...".to_string();
    let mut runner = base().add_card(blk).hand(0, &[CARD_ID]).start();
    let base_perm = runner.place_on_field(0, "BLK-TM-LV4", Some(0));
    let memory_before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
    assert_eq!(runner.memory(), memory_before - 3);
}

#[test]
fn ex7_011_cannot_digivolve_from_plain_black_lv4() {
    let mut runner = base()
        .add_card(digimon("BLK-LV4", 4, CardColor::Black, 4000))
        .hand(0, &[CARD_ID])
        .start();
    let base_perm = runner.place_on_field(0, "BLK-LV4", Some(0));
    assert!(!runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand));
}
