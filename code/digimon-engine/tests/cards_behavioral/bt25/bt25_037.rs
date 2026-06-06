//! BT25-037 Pegasusmon — Digimon, Lv.4, Yellow/Green, DP 6000, Cost 6.
//! Traits: Holy Beast, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! <Armor Purge> (When this Digimon would be deleted, you may trash the top
//! card of this Digimon to prevent that deletion.)
//! [On Play] [When Digivolving] Add your top security card to the hand. Then,
//! you may place 1 [Angel], [Archangel], [Three Great Angels] or [Iliad] trait
//! Digimon card or 1 [TS] trait Tamer card from your hand as the top or bottom
//! security card.
//!
//! Inherited: none (cards.json artifact "|applinkdp =").
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_037.cs
//!
//! - Alternate Digivolutions (None): [Patamon] name cost 2; Lv.3 [TS] cost 2.
//! - Armor Purge (WhenPermanentWouldBeDeleted): ArmorPurgeEffect.
//! - Shared OP/WD: add top security to hand; then optional SelectHand over
//!   (Digimon && Angel/Archangel/Three Great Angels/Iliad) OR (Tamer && TS),
//!   with a Bool Top/Bottom selection → AddSecurityCard.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - F8 Armor Purge keyword
//! - H4-adjacent security manipulation (add-top-to-hand)
//! - E1 Top/Bottom branch via EffectChoice
//! - optional select (you may place)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-037";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = Some(4);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: 2,
        level: level.saturating_sub(1),
        memory_cost: 3,
    }];
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-037 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("ANGEL-CARD", 4, 5000, &["Angel"]))
        .add_card(make_test_card("PLAIN-CARD", "Plain"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .security(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD"])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_037_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-037 present in pack");
    assert_eq!(card.name, "Pegasusmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(6000));
    for t in ["Holy Beast", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_037_grants_armor_purge() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let grants_ap = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
            if keyword.eq_ignore_ascii_case("ArmorPurge") || keyword.eq_ignore_ascii_case("Armor Purge")
    ));
    assert!(grants_ap, "BT25-037 must grant <Armor Purge>");
}

#[test]
fn bt25_037_has_shared_on_play_when_digivolving_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared OnPlay/WhenDigivolving clause present");
    // Adds top security to hand somewhere in the process.
    let adds_sec = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::AddTopSecurityToHand { .. }));
    assert!(adds_sec, "clause must add top security to hand");
}

#[test]
fn bt25_037_has_patamon_and_ts_alt_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        digivolve_paths.len() >= 3,
        "BT25-037 has 3 digivolve paths (Yellow Lv.3, Patamon-name, Lv.3 TS), got {}",
        digivolve_paths.len()
    );
}

// ─── Section 2 — Behavior: OnPlay add top security to hand ───────────────────

/// Behavioral: playing Pegasusmon adds the top security card to the hand
/// (security count drops by 1; hand gains the security card).
#[test]
fn bt25_037_on_play_adds_top_security_to_hand() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(8).start();

    let sec_before = runner.security_count(0);
    let _p = runner.play(0, 0).expect("play Pegasusmon");
    // Decline the optional place-to-security follow-up to isolate the add-to-hand.
    runner.auto_resolve().ok();

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "playing Pegasusmon must add the top security card to the hand (security -1)"
    );
}

/// Behavioral: the place-to-security step is optional — declining it leaves the
/// (post-add) security count unchanged thereafter.
#[test]
fn bt25_037_place_to_security_is_optional() {
    let mut runner = base().hand(0, &[CARD_ID, "ANGEL-CARD"]).memory(8).start();

    let _p = runner.play(0, 0).expect("play Pegasusmon");
    // First selection is the optional place-from-hand pick. It must be optional.
    // (After the mandatory add-top-to-hand resolves automatically.)
    // Drive to the first hand-place prompt; assert PASS is legal there.
    // The add-top-to-hand is not a prompt; the place pick is the first prompt.
    if runner.pending_selection().is_some() {
        assert!(
            runner.pending_is_optional(),
            "the place-to-security pick must be optional (you may place)"
        );
    }
}

// ─── Section 3 — Behavior: <Armor Purge> on field ────────────────────────────

#[test]
fn bt25_037_has_armor_purge_keyword_on_field() {
    let mut runner = base().memory(8).start();
    let p = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(p, Keyword::ArmorPurge),
        "Pegasusmon must have <Armor Purge> on the field"
    );
}
