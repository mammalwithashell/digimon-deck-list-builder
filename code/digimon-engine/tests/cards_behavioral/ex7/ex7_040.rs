//! EX7-040 ToyAgumon — Digimon, Lv.3, Black, DP 1000, Cost 3.
//! Traits: Puppet. Form: Rookie. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX7-040.md — official Bandai DB, verbatim;
//! cross-checked against the card image EX7-040.webp)
//!
//! [On Play] By trashing 1 card with the [Three Musketeers] trait in your
//! hand, ＜Draw 2＞.
//!
//! Inherited Effect: ＜Reboot＞ (This Digimon also unsuspends in your
//! opponent's unsuspend phase.)
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Black Lv.2 / cost 0
//!   - xros_req: "[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Black/EX7_040.cs
//!
//! # DCGO crosscheck
//! - Alt-digivolve: AddSelfDigivolutionRequirementStaticEffect gated on
//!   `TopCard.IsLevel2 && TopCard.ContainsTraits("Three Musketeers")`, cost 0,
//!   ignoreDigivolutionRequirement: false. The PRINTED condition is "w/[Three
//!   Musketeers] IN TEXT" (official DB), which is broader than DCGO's trait
//!   check — printed text governs, so `in_text_contains` is authored (EX7-048
//!   sibling idiom). A trait carrier still matches via the trait scan.
//! - [On Play]: ActivateClass(isOptional: true); CanActivateCondition =
//!   HasMatchConditionOwnersHand(ContainsTraits("Three Musketeers")).
//!   SelectHandEffect(canNoSelect: true, maxCount 1, Mode.Discard) →
//!   AfterSelect: `if (cardSources.Count > 0) DrawClass(owner, 2)`.
//! - ESS: RebootSelfStaticEffect(isInheritedEffect: true).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - E2: optional trash-as-cost from hand gating a Draw 2
//! - H7: inherited <Reboot> grant
//! - G-DSL-IN-TEXT-CONTAINS alt-path (standard + special digivolve routes)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX7-040";

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
        .expect("EX7-040 YAML loads from the embedded pack")
        .add_card(digimon("TM-HAND", 4, CardColor::Black, &["Three Musketeers"]))
        .add_card(digimon("TM-HAND-2", 4, CardColor::Black, &["Three Musketeers"]))
        .add_card(digimon("PLAIN-HAND", 4, CardColor::Black, &["Puppet"]))
        .add_card(filler("FILL"))
        .add_card(digimon("CARRIER", 4, CardColor::Black, &[]))
}

fn hand_action(runner: &DebugRunner, player: u8, card_id: &str) -> u16 {
    let idx = runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    PLAY_HAND_START + idx as u16
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_040_has_optional_on_play_clause_and_inherited_reboot() {
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
    assert_eq!(triggered.len(), 1, "exactly one triggered clause ([On Play])");
    let op = triggered[0];
    assert_eq!(op.when, vec![CompiledTiming::OnPlay]);
    assert!(op.optional, "'By trashing ...' is a declinable cost");
    assert!(!op.once_per_turn);
    assert_eq!(op.scope, CompiledScope::FaceUp);

    let reboot = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, keyword, .. })
                if *scope == CompiledScope::Inherited && keyword == "Reboot"
        )
    });
    assert!(reboot, "inherited <Reboot> grant_keyword clause must be present");
}

#[test]
fn ex7_040_has_two_cost_zero_lv2_digivolve_paths() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        2,
        "standard Black Lv.2 / 0 + special 'Lv.2 w/[Three Musketeers] in text' / 0"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_040_on_play_prompts_when_tm_card_in_hand() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "TM-HAND"])
        .deck(0, &["FILL"; 6])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play ToyAgumon");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "a TM-trait card in hand → the trash-as-cost prompt installs"
    );
    assert!(runner.pending_is_optional(), "the cost is declinable (PASS legal)");
}

#[test]
fn ex7_040_on_play_no_prompt_without_tm_card_in_hand() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "PLAIN-HAND"])
        .deck(0, &["FILL"; 6])
        .memory(10)
        .start();
    let hand_before = runner.hand_size(0);
    runner.play(0, 0).expect("play ToyAgumon");
    assert!(
        runner.pending_selection().is_none(),
        "no [Three Musketeers] card in hand → the cost is unpayable, nothing installs"
    );
    assert_eq!(runner.hand_size(0), hand_before - 1, "only the play left the hand");
}

#[test]
fn ex7_040_on_play_only_tm_trait_cards_are_selectable() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "TM-HAND", "PLAIN-HAND"])
        .deck(0, &["FILL"; 6])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play ToyAgumon");
    let view = runner.pending_selection_view().expect("hand prompt");
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(picks, vec![hand_action(&runner, 0, "TM-HAND")], "PLAIN-HAND is filtered out");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral outcome
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_040_on_play_trash_tm_card_draws_two() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "TM-HAND"])
        .deck(0, &["FILL"; 6])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play ToyAgumon");
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);
    let pick = hand_action(&runner, 0, "TM-HAND");
    runner.execute_action(0, pick).expect("trash TM-HAND");
    runner.auto_resolve().expect("resolve");

    assert_eq!(runner.deck_size(0), deck_before - 2, "Draw 2");
    assert_eq!(runner.trash_size(0), trash_before + 1, "the TM card was trashed");
    assert!(
        zone_ids(&runner.game.players[0].trash, &runner.game.card_data).contains(&"TM-HAND".to_string())
    );
    assert_eq!(runner.hand_size(0), 2, "hand: -1 trashed, +2 drawn (ToyAgumon already played)");
}

#[test]
fn ex7_040_on_play_decline_keeps_hand_and_deck_unchanged() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "TM-HAND"])
        .deck(0, &["FILL"; 6])
        .memory(10)
        .start();
    runner.play(0, 0).expect("play ToyAgumon");
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);
    runner.execute_action(0, PASS).expect("decline");
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.deck_size(0), deck_before, "no draw on decline");
    assert_eq!(runner.trash_size(0), trash_before, "no trash on decline");
    assert_eq!(runner.hand_size(0), 1, "TM-HAND stays in hand");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited <Reboot>
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_040_inherited_reboot_grants_keyword_to_carrier() {
    let mut runner = base().start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        runner.game.has_keyword(stack, Keyword::Reboot),
        "a Digimon with ToyAgumon in its digivolution cards has <Reboot>"
    );
}

#[test]
fn ex7_040_face_up_toyagumon_has_no_reboot_itself() {
    let mut runner = base().start();
    let alone = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !runner.game.has_keyword(alone, Keyword::Reboot),
        "<Reboot> is an INHERITED effect — the face-up card itself does not carry it"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Digivolution paths
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ex7_040_digivolves_for_zero_from_lv2_with_tm_in_text_regardless_of_color() {
    let mut red_tm_lv2 = digimon("RED-TM-LV2", 2, CardColor::Red, &[]);
    red_tm_lv2.effect_text = "[Your Turn] ... [Three Musketeers] ...".to_string();
    let mut runner = base()
        .add_card(red_tm_lv2)
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();
    let base_perm = runner.place_on_field(0, "RED-TM-LV2", Some(0));
    let memory_before = runner.memory();
    assert!(
        runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand),
        "special path: Lv.2 with [Three Musketeers] in text, cost 0 (colour irrelevant)"
    );
    assert_eq!(runner.memory(), memory_before, "cost 0");
}

#[test]
fn ex7_040_digivolves_for_zero_from_plain_black_lv2() {
    let mut runner = base()
        .add_card(digimon("BLK-LV2", 2, CardColor::Black, &[]))
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();
    let base_perm = runner.place_on_field(0, "BLK-LV2", Some(0));
    assert!(
        runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand),
        "standard printed circle: Black Lv.2 / cost 0"
    );
}

#[test]
fn ex7_040_cannot_digivolve_from_plain_red_lv2() {
    let mut runner = base()
        .add_card(digimon("RED-LV2", 2, CardColor::Red, &[]))
        .hand(0, &[CARD_ID])
        .memory(5)
        .start();
    let base_perm = runner.place_on_field(0, "RED-LV2", Some(0));
    assert!(
        !runner.game.digivolve_from_hand(0, 0, base_perm.index as usize, PlaySource::ByHand),
        "a red Lv.2 without [Three Musketeers] in text satisfies neither route"
    );
}
