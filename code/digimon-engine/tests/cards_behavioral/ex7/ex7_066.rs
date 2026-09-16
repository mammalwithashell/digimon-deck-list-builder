//! EX7-066 Chaos Triangular — Option, Red, Use cost 6. Traits: Three Musketeers.
//!
//! # Card text (data/card_bundles/EX7-066.md — official Bandai DB)
//!
//! When effects trash this card from digivolution cards, 1 of your Digimon
//! gets +3000 DP until the end of your opponent's turn. While you have a
//! [Three Musketeers] trait Digimon, you may ignore this card's color
//! requirements. [Main] Delete 1 of your opponent's Digimon with 9000 DP or
//! less. For each of your [Three Musketeers] trait Digimon with different
//! names, add 3000 to this DP-deletion effect's maximum. Then, place this
//! card as the bottom digivolution card of 1 of your [Three Musketeers] trait
//! Digimon.
//!
//! Security Effect: [Security] Delete 1 of your opponent's Digimon with
//! 12000 DP or less.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Red/EX7_066.cs
//! - OnDigivolutionCardDiscarded (`-1, false` = mandatory; IsInheritedEffect +
//!   IsOptionEffect markers): CanTriggerOnTrashSelfDigivolutionCard(cardEffect
//!   != null) → SelectPermanentEffect over own battle-area Digimon
//!   (canNoSelect: false) → ChangeDigimonDP(+3000, UntilOpponentTurnEnd).
//! - IgnoreColorConditionClass gated on ≥1 own [Three Musketeers] Digimon,
//!   scoped to this card only (→ `use_requirement`, EX7-071 idiom).
//! - OptionSkill [Main]: maxDP = 9000 + 3000 × |distinct names among own
//!   [Three Musketeers] Digimon| (HasSameCardName de-dup); mandatory
//!   Destroy select over opp Digimon with DP ≤ maxDP; then mandatory select
//!   over own [Three Musketeers] Digimon → AddDigivolutionCardsBottom(this).
//! - SecuritySkill: mandatory Destroy select over opp Digimon DP ≤ 12000.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - Inherited on_digivolution_card_trashed (self-identity gate) → DP buff w/ expiry
//! - D3 color bypass via `use_requirement`
//! - Formula-scaled DP cap (G-DSL-FORMULA-DISTINCT-NAMES-COUNT) + place-self-under
//! - [Security] delete with a fixed DP cap

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardColor;
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "EX7-066";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 6;
    c.colors = vec![CardColor::Red];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-066 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "DECK-PAD"))
        // Own [Three Musketeers] Digimon: two copies of "Kidmon A", one "Kidmon B".
        .add_card(digimon("TM-A1", "Kidmon A", 4, 5000, &["Three Musketeers"]))
        .add_card(digimon("TM-A2", "Kidmon A", 4, 5000, &["Three Musketeers"]))
        .add_card(digimon("TM-B", "Kidmon B", 5, 7000, &["Three Musketeers"]))
        .add_card(digimon("PLAIN", "Plain", 4, 5000, &["Beast"]))
        // Opponent Digimon at the three cap thresholds.
        .add_card(digimon("OPP-9K", "Opp 9k", 5, 9000, &["Beast"]))
        .add_card(digimon("OPP-12K", "Opp 12k", 6, 12000, &["Beast"]))
        .add_card(digimon("OPP-15K", "Opp 15k", 6, 15000, &["Beast"]))
        .add_card(digimon("OPP-13K", "Opp 13k", 6, 13000, &["Beast"]))
        .deck(0, &["DECK-PAD"; 5])
        .deck(1, &["DECK-PAD"; 5])
}

fn opp_candidates(runner: &DebugRunner) -> usize {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.iter().filter(|&&a| a != PASS).count())
        .unwrap_or(0)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn ex7_066_yaml_has_printed_metadata_and_use_requirement() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX7-066 in embedded pack");
    assert_eq!(card.name, "Chaos Triangular");
    assert_eq!(card.cost, Some(6));
    assert!(card.traits.contains(&"Three Musketeers".to_string()));
    assert!(
        card.use_requirement.is_some(),
        "'while you have a [Three Musketeers] Digimon, ignore color requirements' → use_requirement"
    );
}

#[test]
fn ex7_066_clause_shapes() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3, "source-trash, [Main], [Security]");
    let src = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnDigivolutionCardTrashed])
        .expect("source-trash clause");
    assert_eq!(src.scope, CompiledScope::Inherited);
    assert!(
        !src.optional,
        "'1 of your Digimon gets +3000 DP' is mandatory (DCGO isOptional: false)"
    );
    assert!(triggered
        .iter()
        .any(|t| t.when == vec![CompiledTiming::MainFromHand] && !t.optional));
    assert!(triggered
        .iter()
        .any(|t| t.when == vec![CompiledTiming::OnSecurity]));
}

// ─── Section 2 — source trash → +3000 DP until end of opponent's turn ────────

#[test]
fn ex7_066_source_trash_by_effect_buffs_chosen_digimon_until_opponents_turn_ends() {
    let mut runner = base().build();
    let host = runner.place_on_field(0, "PLAIN", None);
    let other = runner.place_on_field(0, "TM-B", Some(0));
    let source = runner.push_source(host, CARD_ID);
    let dp_before = runner.dp_of(other).unwrap();

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(!runner.pending_is_optional(), "mandatory target pick");
    runner
        .execute_action(0, encode_attack(0, other.index as u16))
        .expect("pick TM-B");
    runner.auto_resolve().ok();
    assert_eq!(runner.dp_of(other).unwrap(), dp_before + 3000);

    // Expiry: survives the rest of P0's turn AND P1's turn; gone once P1's turn ends.
    runner.game.end_turn(); // P0 → P1
    runner.auto_resolve().ok();
    assert_eq!(
        runner.dp_of(other).unwrap(),
        dp_before + 3000,
        "still up during opponent's turn"
    );
    runner.game.end_turn(); // P1 → P0
    runner.auto_resolve().ok();
    assert_eq!(
        runner.dp_of(other).unwrap(),
        dp_before,
        "expired at end of opponent's turn"
    );
}

#[test]
fn ex7_066_source_trash_no_fire_for_a_different_card() {
    let mut runner = base().build();
    let host = runner.place_on_field(0, "PLAIN", None);
    runner.place_on_field(0, "TM-B", Some(0));
    runner.push_source(host, CARD_ID);
    let other_source = runner.push_source(host, "DECK-PAD");
    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, other_source);
    }
    assert!(
        runner.pending_selection().is_none(),
        "only THIS card's trash triggers it"
    );
}

#[test]
fn ex7_066_no_fire_when_stack_leaves_by_deletion_not_by_effect_trash() {
    // "When EFFECTS trash this card from digivolution cards" — a whole-stack
    // deletion routes the sources to trash WITHOUT the digivolution-card-
    // trashed observer (DCGO `cardEffect != null` gate), so no buff is offered.
    let mut runner = base().build();
    let host = runner.place_on_field(0, "PLAIN", None);
    runner.place_on_field(0, "TM-B", Some(0));
    runner.push_source(host, CARD_ID);
    runner.game.delete_permanent_with_effects(host);
    runner.game.drain_effect_queue();
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "the deleted stack's cards land in trash"
    );
    assert!(
        runner.pending_selection().is_none(),
        "a deletion is not an effect trashing this card from digivolution cards"
    );
}

// ─── Section 3 — [Main] scaled delete + place self under a TM Digimon ────────

/// Two distinct names among three TM Digimon (A, A, B) → cap 9000 + 2×3000
/// = 15000 → all of 9k / 12k / 15k are candidates; then the card is placed as
/// the bottom digivolution card of a chosen TM Digimon.
#[test]
fn ex7_066_main_cap_scales_by_distinct_tm_names_and_places_self() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let a1 = runner.place_on_field(0, "TM-A1", Some(0));
    runner.place_on_field(0, "TM-A2", Some(0));
    runner.place_on_field(0, "TM-B", Some(0));
    runner.place_on_field(1, "OPP-9K", Some(0));
    runner.place_on_field(1, "OPP-12K", Some(0));
    let big = runner.place_on_field(1, "OPP-15K", Some(0));

    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(!runner.pending_is_optional());
    assert_eq!(opp_candidates(&runner), 3, "cap 15000 → all three eligible");
    runner
        .execute_action(0, encode_attack(0, big.index as u16))
        .expect("delete the 15000 DP Digimon");
    assert_eq!(runner.battle_area_size(1), 2);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(
        !runner.pending_is_optional(),
        "placement is mandatory when a TM Digimon exists"
    );
    runner
        .execute_action(0, encode_attack(0, a1.index as u16))
        .expect("place under TM-A1");
    runner.auto_resolve().ok();

    let stack = &runner.game.players[0].battle_area[a1.index as usize].card_sources;
    assert_eq!(
        stack[0].card_id(&runner.game.card_data),
        CARD_ID,
        "bottom source"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "placed under a Digimon instead of being trashed"
    );
}

/// Duplicate names do not stack: A + A → one distinct name → cap 12000.
#[test]
fn ex7_066_main_duplicate_names_count_once() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "TM-A1", Some(0));
    runner.place_on_field(0, "TM-A2", Some(0));
    runner.place_on_field(1, "OPP-9K", Some(0));
    runner.place_on_field(1, "OPP-12K", Some(0));
    runner.place_on_field(1, "OPP-13K", Some(0));

    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(opp_candidates(&runner), 2, "cap 12000 → 9k and 12k only");
}

/// Base cap: no TM Digimon → 9000 only, and no placement step (the Option is
/// trashed normally).
#[test]
fn ex7_066_main_no_tm_digimon_uses_base_cap_and_trashes_self() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "PLAIN", Some(0)); // red permanent satisfies the color requirement
    runner.place_on_field(1, "OPP-9K", Some(0));
    runner.place_on_field(1, "OPP-12K", Some(0));

    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(
        opp_candidates(&runner),
        1,
        "cap 9000 → only the 9000 DP Digimon"
    );
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).expect("delete");
    runner.auto_resolve().ok();

    assert_eq!(runner.battle_area_size(1), 1);
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "no TM Digimon → no placement → the used Option is trashed"
    );
}

/// The [Three Musketeers] trait alone (no red permanent) satisfies the color
/// requirement (use_requirement), and a plain Digimon is never a placement
/// candidate.
#[test]
fn ex7_066_main_placement_offers_only_tm_digimon() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let tm = runner.place_on_field(0, "TM-B", Some(0));
    let plain = runner.place_on_field(0, "PLAIN", Some(0));

    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "usable via the [Three Musketeers] use_requirement"
    );
    // No opponent Digimon → the delete half self-skips; straight to placement.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let view = runner.pending_selection_view().unwrap();
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, tm.index as u16)));
    assert!(!view
        .valid_action_ids
        .contains(&encode_attack(0, plain.index as u16)));
}

// ─── Section 4 — [Security] delete DP ≤ 12000 ────────────────────────────────

#[test]
fn ex7_066_security_deletes_opp_digimon_with_dp_up_to_12000() {
    let mut runner = base().security(1, &[CARD_ID]).start();
    let attacker = runner.place_on_field(0, "OPP-12K", Some(0));
    runner.place_on_field(0, "OPP-13K", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    // The security effect (P1's perspective) offers only the 12000 one.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(opp_candidates(&runner), 1, "13000 DP exceeds the 12000 cap");
    runner.auto_resolve().ok();
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the 12000 DP attacker was deleted"
    );
}
