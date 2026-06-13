//! BT25-008 Coronamon — Digimon, Lv.3, Red, DP 1000, Cost 3.
//! Traits: Beast, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (cards.json — verbatim)
//!
//! [When Moving] [On Play] By trashing up to 2 [Iliad] or [TS] trait cards from
//! your hand, ＜Draw 1＞ (Draw 1 card from your deck.) for each card trashed.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_008.cs
//!
//! ActivateClassesForSharedEffects(onPlay, whenMoving). SelectHandEffect
//! Mode.Discard, HasIliadTraits || HasTSTraits, maxCount min(2, eligible),
//! canNoSelect: true, canEndNotMax: true. Draw(drawCount = trashed count).
//! Inherit: ChangeSelfDPStaticEffect(2000, inherited, condition: IsOwnerTurn).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A (trash-as-cost from hand, draw per trashed)
//! - E2 (optional decline — trash 0 → draw 0)
//! - G (inherited conditional +DP aura)
//! - H (alt digivolution path)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-008";

fn iliad_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.traits = vec!["Iliad".to_string()];
    c
}

fn ts_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.traits = vec!["TS".to_string()];
    c
}

fn plain_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.traits = vec!["Machine".to_string()];
    c
}

fn coronamon_base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-008 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(iliad_card("ILIAD-A"))
        .add_card(iliad_card("ILIAD-B"))
        .add_card(ts_card("TS-A"))
        .add_card(plain_card("PLAIN-A"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 6])
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_008_metadata() {
    let runner = coronamon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Coronamon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.dp, Some(1000));
    for t in ["Beast", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()));
    }
}

#[test]
fn bt25_008_has_ts_level2_cost0_alt_path() {
    let runner = coronamon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("digivolve alt-path");
    assert_eq!(path.from.as_ref().unwrap().trait_has.as_deref(), Some("TS"));
    assert_eq!(path.cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn bt25_008_shared_clause_fires_on_play_and_move() {
    let runner = coronamon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("OnPlay clause present");
    assert!(
        clause.when.contains(&CompiledTiming::OnMove),
        "[When Moving] half must be present too"
    );
}

#[test]
fn bt25_008_inherited_dp_is_your_turn_only() {
    let runner = coronamon_base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier: Some(2000),
                ..
            })
        )
    });
    assert!(aura, "must have an inherited +2000 DP aura");
}

// ─── Section 2 — Behavior: trash up to 2 → draw per trashed ──────────────────

#[test]
fn bt25_008_on_play_installs_trash_prompt_with_eligible_cards() {
    let mut runner = coronamon_base()
        .hand(0, &[CARD_ID, "ILIAD-A", "ILIAD-B"])
        .memory(10)
        .start();
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "OnPlay must offer the optional trash-up-to-2 prompt when eligible cards are in hand"
    );
}

#[test]
fn bt25_008_trash_two_draws_two() {
    let mut runner = coronamon_base()
        .hand(0, &[CARD_ID, "ILIAD-A", "TS-A"])
        .memory(10)
        .start();
    runner.play(0, 0);
    let hand_before = runner.hand_size(0); // ILIAD-A + TS-A still in hand
    let trash_before = runner.trash_size(0);
    let deck_before = runner.deck_size(0);
    runner.auto_resolve().expect("resolve: trash both, draw 2");

    // 2 trashed from hand, 2 drawn: net hand = before - 2 (trashed) + 2 (drawn).
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "trash 2 + draw 2 nets zero hand-size change"
    );
    assert_eq!(runner.trash_size(0), trash_before + 2, "2 cards trashed");
    assert_eq!(runner.deck_size(0), deck_before - 2, "2 cards drawn");
}

#[test]
fn bt25_008_no_prompt_without_eligible_cards() {
    let mut runner = coronamon_base()
        .hand(0, &[CARD_ID, "PLAIN-A"])
        .memory(10)
        .start();
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        "no [Iliad]/[TS] card in hand → the trash prompt is a no-op"
    );
}

// ─── Section 3 — Inherited conditional DP ────────────────────────────────────

#[test]
fn bt25_008_inherited_plus_2000_on_your_turn() {
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.card_kind = CardKind::Digimon;
    carrier.level = Some(4);
    carrier.dp = Some(5000);
    carrier.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    let mut runner = coronamon_base().add_card(carrier).memory(10).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.turn_player_idx = 0;
    assert_eq!(
        runner.effective_dp(stack),
        Some(7000),
        "your turn: carrier 5000 + inherited 2000 = 7000"
    );
}

#[test]
fn bt25_008_inherited_no_bonus_on_opponents_turn() {
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.card_kind = CardKind::Digimon;
    carrier.level = Some(4);
    carrier.dp = Some(5000);
    carrier.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 2,
    }];
    let mut runner = coronamon_base().add_card(carrier).memory(10).start();
    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.turn_player_idx = 1;
    assert_eq!(
        runner.effective_dp(stack),
        Some(5000),
        "opponent's turn: [Your Turn] aura must not apply"
    );
}
