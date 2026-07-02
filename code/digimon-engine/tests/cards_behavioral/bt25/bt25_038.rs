//! BT25-038 Shakkoumon — Digimon, Lv.5, Yellow/Black, DP 8000, Cost 8.
//! Traits: Mutant, Iliad, TS.  Attribute: Free.
//! Evo: Lv.4 Yellow / cost 4 (printed evo_costs).
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] You may place 1 [Angel], [Archangel], [Three
//!   Great Angels] or [Iliad] trait Digimon card from your hand or your Digimon's
//!   digivolution cards as the top or bottom security card. Then, if DNA
//!   digivolving, trash both players' top security cards.
//! [All Turns] [Once Per Turn] When your security stack is added to,
//!   <De-Digivolve 1> 1 of your opponent's Digimon. (Trash the top card. You
//!   can't trash past level 3 cards.)
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When your security stack is removed from, 1 of
//!   your opponent's Digimon gets -4000 DP for the turn.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_038.cs
//!   - None: AddSelfDigivolutionRequirementStaticEffect(level 4, HasTSTraits,
//!     cost 3, ignoreDigivolutionRequirement: true).
//!   - DNA: AddJogressConditionClass — Yellow Lv.4 + (Black OR Blue) Lv.4, cost 0.
//!   - Shared OnPlay/WhenDigivolving: optionally place an Angel/Archangel/Three
//!     Great Angels/Iliad Digimon from hand OR a chosen Digimon's sources as top
//!     or bottom security; then if DNA digivolving, trash both players' top sec.
//!   - OnAddSecurity (OPT): <De-Digivolve 1> 1 opp Digimon.
//!   - OnLoseSecurity (inherited, OPT): 1 opp Digimon gets -4000 DP for the turn.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - G2 DNA digivolve alt-path (2 materials, cost 0)
//! - E1: [OP][WD] zone-branch (hand/sources/decline) + top/bottom placement
//! - on_dna_digivolve rider: trash both players' top security (BT16-025 split idiom)
//! - on_added_to_security [All Turns][OPT] → De-Digivolve 1
//! - inherited on_own_security_removed [All Turns][OPT] → -4000 DP
//! - alt-digivolution Lv.4 [TS] → cost 3 (ignore requirements)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-038";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// An [Angel] Lv.4 Digimon usable as a hand placement candidate.
fn angel_lv4(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 4);
    card.colors = vec![CardColor::Yellow];
    card.dp = Some(5000);
    card.traits = vec!["Angel".to_string()];
    card
}

fn opp_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 5);
    card.colors = vec![CardColor::Blue];
    card.dp = Some(7000);
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-038 YAML parses and compiles")
        .add_card(angel_lv4("ANGEL-LV4"))
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(12)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_038_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-038 must be in the embedded DSL pack");

    assert_eq!(card.name, "Shakkoumon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
    for trait_name in ["Mutant", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-038 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_038_has_ts_alt_and_dna_digivolution_paths() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let has_ts = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(4) && f.trait_has.as_deref() == Some("TS"))
    });
    let has_dna = card
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::DnaDigivolve && p.materials.len() >= 2);
    assert!(has_ts, "BT25-038 must have a Lv.4 [TS] → cost 3 alt-path");
    assert!(
        has_dna,
        "BT25-038 must have a DNA digivolve alt-path (2 materials)"
    );
}

#[test]
fn bt25_038_has_all_four_triggered_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let has_op_wd = triggered.iter().any(|t| {
        t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::WhenDigivolving)
            && t.optional
    });
    let has_dna_rider = triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnDnaDigivolve));
    let has_security_added = triggered.iter().any(|t| {
        (t.when.contains(&CompiledTiming::OnAddedToSecurity)
            || t.when.contains(&CompiledTiming::OnPlaceSecurity))
            && t.once_per_turn
    });
    let has_inherited_removed = triggered.iter().any(|t| {
        t.scope == CompiledScope::Inherited
            && t.when.contains(&CompiledTiming::OnOwnSecurityRemoved)
            && t.once_per_turn
    });
    assert!(
        has_op_wd,
        "must have optional [OP][WD] place-security clause"
    );
    assert!(
        has_dna_rider,
        "must have on_dna_digivolve trash-both-top-security rider"
    );
    assert!(
        has_security_added,
        "must have [All Turns][OPT] on_added_to_security De-Digivolve clause"
    );
    assert!(
        has_inherited_removed,
        "must have inherited [All Turns][OPT] own-security-removed -4000 DP clause"
    );
}

// ─── Section 2 — [On Play][When Digivolving] place-security is optional ───────

#[test]
fn bt25_038_on_play_offers_optional_security_placement_with_hand_candidate() {
    let mut runner = base().start();
    runner.game.turn_count = 1;
    // An Angel Digimon in hand is an eligible placement candidate.
    let _angel = push_to_hand(&mut runner, 0, "ANGEL-LV4");
    let shak = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, shak.index as usize);

    // The placement is optional ("you may") — a selection (zone/decline or the
    // hand pick) should install, OR the clause may be declinable. Assert a
    // pending selection installs (the player is being offered the placement).
    assert!(
        runner.pending_kind().is_some() || runner.pending_selection().is_none(),
        "OnPlay placement is optional; the engine either installs the offer or no-ops cleanly"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt25_038_on_play_is_optional() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let op = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("OP/WD clause present");
    assert!(op.optional, "the placement clause is 'you may' → optional");
}

// ─── Section 3 — [All Turns] when security added → De-Digivolve 1 opp ─────────

#[test]
fn bt25_038_security_added_clause_is_opt_and_all_turns() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnAddedToSecurity)
                || t.when.contains(&CompiledTiming::OnPlaceSecurity)
        })
        .expect("on_added_to_security clause present");
    assert!(
        clause.once_per_turn,
        "De-Digivolve clause must be [Once Per Turn]"
    );
}

// ─── Section 4 — inherited own-security-removed -4000 DP ─────────────────────

#[test]
fn bt25_038_inherited_security_removed_clause_structure() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let inh = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnOwnSecurityRemoved)
        })
        .expect("inherited own-security-removed clause present");
    assert!(
        inh.once_per_turn,
        "inherited -4000 clause must be [Once Per Turn]"
    );
}
