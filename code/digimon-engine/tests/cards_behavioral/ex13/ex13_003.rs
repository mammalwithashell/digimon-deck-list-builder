//! EX13-003 Kyaromon — Digi-Egg, Lv.2, Yellow. Traits: Lesser / DATA SQUAD.
//! Form: In-Training.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-003.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-003.md` agrees)
//!
//! ```text
//! Inherited Effect:
//! [Your Turn] [Once Per Turn] When your security stack is removed from, this
//! Digimon may digivolve into a Digimon card with [Kentaurosmon] in its name
//! or the [Holy Beast] trait in the hand with the cost reduced by 1.
//! ```
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_003.cs` at `b9a0638cd` (see
//! `.claude/plans/author-set-ex13.md` open item 1). Printed text + the
//! ST24-01 Koromon / BT21-001 Gigimon inherited-digivolve idiom are the refs.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - G4 inherited trigger on a Digi-Egg (`on_own_security_removed`).
//! - E2 [Your Turn] gate + [Once Per Turn] + optional ("may") hand pick.
//! - Effect-initiated digivolve of the carrier, printed cost reduced by 1.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-003";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16 + 2;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Lv.4 yellow digivolve target with a printed Lv.3-yellow cost of 3.
fn target(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = digimon(id, name, 4, traits);
    c.evo_costs = vec![EvoCost {
        card_color: 2, // Yellow
        level: 3,
        memory_cost: 3,
    }];
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-003 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("CARRIER", "Carrier", 3, &["DATA SQUAD"]))
        .add_card(target("HB-TARGET", "Holy Beast Target", &["Holy Beast"]))
        .add_card(target("KENT-TARGET", "Kentaurosmon Junior", &["Royal Knight"]))
        .add_card(target("PLAIN-TARGET", "Plain Target", &["Beast"]))
        .add_card(digimon("FILL", "Fill", 3, &[]))
        .add_card(digimon("SEC", "Security", 3, &[]))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .security(0, &["SEC", "SEC", "SEC"])
        .memory(6)
        .start()
}

/// Carrier (Lv.3) on top of Kyaromon — the inherited effect's host.
fn carrier_over_egg(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "CARRIER"])
}

/// Trash player 0's top security card by an effect controlled by `actor`,
/// then drain the queue (fires the `on_own_security_removed` observer).
fn trash_p0_security(r: &mut DebugRunner, actor: u8) {
    let by: CardHandle = r.game.players[0].security[0].handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, by, None, actor);
        assert!(ctx.trash_top_security(0), "security card trashed");
    }
    r.game.drain_effect_queue();
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_003_is_yellow_data_squad_digi_egg() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    assert!(card.traits.iter().any(|t| t == "Lesser"));
}

#[test]
fn ex13_003_has_one_inherited_opt_security_removed_clause() {
    let r = runner();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "exactly one triggered clause");
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::Inherited);
    assert_eq!(t.when, vec![CompiledTiming::OnOwnSecurityRemoved]);
    assert!(t.once_per_turn, "[Once Per Turn]");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating ([Your Turn] / eligible hand card)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_003_your_turn_security_removal_offers_the_digivolve() {
    let mut r = runner();
    carrier_over_egg(&mut r);
    r.add_to_hand(0, "HB-TARGET");
    trash_p0_security(&mut r, 0);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Hand),
        "the optional hand pick installs on your turn"
    );
    assert!(r.pending_is_optional(), "'may digivolve' is declinable");
}

#[test]
fn ex13_003_opponents_turn_security_removal_does_not_fire() {
    let mut r = runner();
    carrier_over_egg(&mut r);
    r.add_to_hand(0, "HB-TARGET");
    r.end_turn();
    assert_eq!(r.turn_player(), 1, "now the opponent's turn");
    trash_p0_security(&mut r, 1);
    assert!(
        r.pending_selection().is_none(),
        "[Your Turn] gate: nothing fires on the opponent's turn"
    );
}

#[test]
fn ex13_003_no_eligible_hand_card_does_not_prompt() {
    let mut r = runner();
    carrier_over_egg(&mut r);
    r.add_to_hand(0, "PLAIN-TARGET");
    trash_p0_security(&mut r, 0);
    assert!(
        r.pending_selection().is_none(),
        "no [Kentaurosmon]-name / [Holy Beast] Digimon in hand → no prompt"
    );
}

#[test]
fn ex13_003_not_inherited_when_egg_is_not_under_a_digimon() {
    let mut r = runner();
    // Only the carrier (no egg) — the inherited effect must not exist.
    r.place_on_field(0, "CARRIER", Some(0));
    r.add_to_hand(0, "HB-TARGET");
    trash_p0_security(&mut r, 0);
    assert!(r.pending_selection().is_none());
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcomes
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_003_digivolves_into_holy_beast_with_cost_reduced_by_one() {
    let mut r = runner();
    let carrier = carrier_over_egg(&mut r);
    r.add_to_hand(0, "HB-TARGET");
    r.add_to_hand(0, "PLAIN-TARGET");
    let memory_before = r.memory();
    trash_p0_security(&mut r, 0);

    let view = r.pending_selection_view().expect("hand pick");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the [Holy Beast] card is offered (PLAIN-TARGET is not): {:?}",
        view.valid_action_ids
    );
    r.execute_action(0, view.valid_action_ids[0])
        .expect("pick Holy Beast");
    r.auto_resolve().expect("drain");

    assert_eq!(top_id(&r, carrier), "HB-TARGET", "carrier digivolved");
    assert_eq!(
        r.memory(),
        memory_before - 2,
        "printed digivolve cost 3 reduced by 1 → 2 paid"
    );
}

#[test]
fn ex13_003_digivolves_into_kentaurosmon_named_card() {
    let mut r = runner();
    let carrier = carrier_over_egg(&mut r);
    r.add_to_hand(0, "KENT-TARGET");
    trash_p0_security(&mut r, 0);
    let view = r.pending_selection_view().expect("hand pick");
    assert_eq!(view.valid_action_ids.len(), 1);
    r.execute_action(0, view.valid_action_ids[0])
        .expect("pick [Kentaurosmon]-named card");
    r.auto_resolve().expect("drain");
    assert_eq!(top_id(&r, carrier), "KENT-TARGET");
}

#[test]
fn ex13_003_declining_leaves_the_carrier_unchanged() {
    let mut r = runner();
    let carrier = carrier_over_egg(&mut r);
    r.add_to_hand(0, "HB-TARGET");
    let memory_before = r.memory();
    trash_p0_security(&mut r, 0);
    r.execute_action(0, PASS).expect("decline");
    r.auto_resolve().expect("drain");
    assert_eq!(top_id(&r, carrier), "CARRIER");
    assert_eq!(r.memory(), memory_before);
    assert_eq!(r.hand_size(0), 1, "HB-TARGET stays in hand");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_003_once_per_turn_blocks_second_trigger_and_clears_next_turn() {
    let mut r = runner();
    carrier_over_egg(&mut r);
    r.add_to_hand(0, "HB-TARGET");
    trash_p0_security(&mut r, 0);
    assert!(r.pending_selection().is_some(), "first removal fires");
    r.execute_action(0, PASS).expect("decline first");
    r.auto_resolve().expect("drain");

    trash_p0_security(&mut r, 0);
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: second removal the same turn does not fire"
    );

    r.end_turn();
    r.auto_resolve().ok();
    r.end_turn();
    r.auto_resolve().ok();
    assert_eq!(r.turn_player(), 0, "back to player 0's turn");
    trash_p0_security(&mut r, 0);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Hand),
        "OPT lockout clears on the next turn"
    );
}
