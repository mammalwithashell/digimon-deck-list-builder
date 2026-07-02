//! BT25-044 Junomon — Digimon, Lv.6, Yellow/White, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (card image BT25-044 — authoritative for printed text)
//!
//! When this card would be played, if there are 6 or fewer total cards in both
//! players' security stacks, reduce the cost by 5.
//! [On Play] [When Digivolving] By placing 1 other Digimon as the top security
//! card, trash both players' top security cards.
//! [All Turns] [Once Per Turn] When your security stack is removed from, you may
//! play 1 play cost 8 or lower [Angel], [Archangel] or [Iliad] trait card from
//! your hand or trash without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_044.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D2 conditional cost reduction (security-sum gate)
//! - place-permanent-on-security as a cost + trash both top securities
//! - on_lose_security observer + C1 union-zone play-free (E2 OPT)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-044";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-044 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OWN-OTHER", 4, 4000, &["Beast"]))
        .add_card(make_digimon("ANGEL-LOW", 4, 4000, &["Angel"]))
        .add_card(make_digimon("IRRELEVANT", 4, 9000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

// ─── Section 1 — structural ─────────────────────────────────────────────────

#[test]
fn bt25_044_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Junomon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_044_has_cost_reduction() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(has_cr, "cost-reduction clause present");
}

#[test]
fn bt25_044_has_op_wd_place_security_clause() {
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
        .expect("OP/WD place-security clause present");
    assert!(clause.optional, "printed 'By placing' -> optional");
}

#[test]
fn bt25_044_has_on_lose_security_play_free_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnLoseSecurity) => {
                Some(t)
            }
            _ => None,
        })
        .expect("on_lose_security clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(clause.optional, "printed 'you may' -> optional");
}

// ─── Section 2 — behavior: place other Digimon as security + trash both ──────

#[test]
fn bt25_044_on_play_places_other_digimon_and_trashes_both_top_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 2])
        .memory(12)
        .start();
    // An "other" own Digimon to place into security.
    let other = runner.place_on_field(0, "OWN-OTHER", Some(0));

    let own_sec_before = runner.security_count(0);
    let opp_sec_before = runner.security_count(1);

    let _j = runner.play(0, 0).expect("play Junomon");

    // OnPlay installs the place-1-other selection (optional).
    let kind = runner
        .pending_kind()
        .expect("place-security selection installs");
    // Drive the placement of the other Digimon, then auto-resolve the trashes.
    let view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick other Digimon to place");
    runner.auto_resolve().expect("resolve placement + trashes");

    // Placement (+1 own sec from the placed Digimon) then trash own top (-1) and
    // opp top (-1). Net own security == before; opp security == before - 1.
    assert_eq!(
        runner.security_count(1),
        opp_sec_before - 1,
        "opponent's top security card is trashed"
    );
    assert_eq!(
        runner.security_count(0),
        own_sec_before,
        "own: +1 placed Digimon, -1 trashed top = unchanged"
    );
}

#[test]
fn bt25_044_on_play_no_prompt_without_other_digimon() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["PAD"; 2])
        .security(1, &["PAD"; 2])
        .memory(12)
        .start();

    // No other Digimon on field -> the clause condition fails, no prompt.
    let _j = runner.play(0, 0).expect("play Junomon");
    assert!(
        runner.pending_selection().is_none(),
        "no other Digimon -> place-security clause is a no-op"
    );
}
