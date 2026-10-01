//! EX13-032 Chirinmon — Digimon, Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Holy Beast / DATA SQUAD. Form: Ultimate. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-032.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-032.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.4 / cost 3;  [Digivolve] Lv.4 w/[DATA SQUAD] trait: Cost 3
//!
//! [When Digivolving] [When Attacking] [Once Per Turn] By trashing your top
//! security card or the bottom face-down card from under any of your Tamers,
//! this Digimon unsuspends. After, 1 of your opponent's Digimon can't activate
//! [When Digivolving] effects until their turn ends.
//! [All Turns] When this Digimon would leave the battle area, by placing its
//! top stacked card as the top security card, it doesn't leave.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon with [Kentaurosmon] in its
//! name would leave the battle area, by placing its top stacked card as the top
//! security card, it doesn't leave.
//! ```
//! Official Q&A: the part after "After" resolves ONLY if one of the two costs
//! was actually paid.
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_032.cs` at `b9a0638cd`. "Top stacked card" is
//! DCGO's `Permanent.TopCard` (BT20_084.cs `AddSecurityCard(card, toTop)`,
//! BT23_008.cs `AddDigivolutionCardsBottom({ TopCard })`, BT26_058.cs).
//!
//! # Verdict — BLOCKED (engine), partial YAML
//! Clause 1 is authored and tested live. Clauses 2 and 3 (would-leave →
//! place the TOP card as the top security card, the rest of the stack stays)
//! are BLOCKED on `G-TOP-STACKED-CARD-TO-SECURITY` (qa/archetype-qa/
//! engine-gaps.md): the only verb, `security_place_top_stacked_card`, extracts
//! `card_sources[len-2]` (the card UNDER the top) — not DCGO's `TopCard` — and
//! no primitive removes a permanent's top card to security while the permanent
//! stays in play topped by the next card. Their tests are `#[ignore]`d with the
//! gap id; the clauses are NOT authored (no approximation).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - E2 optional processing condition with a two-way cost choice (top security
//!   OR bottom face-down card under a Tamer), [Once Per Turn], WD + WA.
//! - Unsuspend self; opponent Digimon gets CannotActivateWhenDigivolvingEffects
//!   until the end of their turn.
//! - F replacement would-leave (BLOCKED).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-032";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-032 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("FILL", 3, 1000))
        .add_card(digimon("SEC", 3, 1000))
        .add_card(digimon("BASE-L4", 4, 4000))
        .add_card(digimon("OPP", 4, 4000))
        .add_card({
            let mut k = digimon("KENT", 6, 12000);
            k.card_name = "Kentaurosmon".to_string();
            k
        })
        .add_card(tamer("TAMER"))
}

fn runner(own_security: usize) -> DebugRunner {
    let sec = vec!["SEC"; own_security];
    builder()
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &sec)
        .security(1, &["SEC"; 5])
        .memory(5)
        .start()
}

fn tamer_with_face_down(r: &mut DebugRunner) -> PermanentHandle {
    let t = r.place_stack(0, &["FILL", "TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    t
}

fn face_down_count(r: &DebugRunner, t: PermanentHandle) -> usize {
    r.game.players[0].battle_area[t.index as usize]
        .card_sources
        .iter()
        .filter(|c| c.face_down)
        .count()
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Chirinmon on field, opponent Digimon present; attack the player so the
/// [When Attacking] fires (attacking suspends Chirinmon).
fn attack(r: &mut DebugRunner) -> (PermanentHandle, PermanentHandle) {
    let chirin = r.place_stack(0, &["BASE-L4", CARD_ID]);
    let opp = r.place_on_field(1, "OPP", Some(0));
    r.attack_player(chirin, 1, false);
    (chirin, opp)
}

fn drain(r: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        if view.valid_action_ids.is_empty() {
            r.execute_action(view.selecting_player, PASS).expect("pass");
        } else {
            r.execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("first");
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_032_is_yellow_lv5_holy_beast_data_squad() {
    let r = runner(3);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert!(card.traits.iter().any(|t| t == "Holy Beast"));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    let digi = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(digi, 2, "yellow Lv.4 circle + Lv.4 w/[DATA SQUAD] condition");
}

#[test]
fn ex13_032_clause1_is_optional_opt_wd_wa() {
    let r = runner(3);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenAttacking) => {
                Some(t)
            }
            _ => None,
        })
        .expect("WD/WA clause");
    assert_eq!(t.scope, CompiledScope::FaceUp);
    assert!(t.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(t.once_per_turn);
    assert!(t.optional, "'By trashing…' optional processing condition");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (a payable cost must exist)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_032_no_payable_cost_does_not_offer_the_clause() {
    let mut r = runner(0);
    let (chirin, _opp) = attack(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "no security and no face-down Tamer card → nothing offered"
    );
    assert!(suspended(&r, chirin));
}

#[test]
fn ex13_032_security_only_offers_the_clause() {
    let mut r = runner(2);
    attack(&mut r);
    assert!(r.pending_is_optional(), "outer accept/decline installs");
}

#[test]
fn ex13_032_face_down_only_offers_the_clause() {
    let mut r = runner(0);
    tamer_with_face_down(&mut r);
    attack(&mut r);
    assert!(r.pending_is_optional(), "outer accept/decline installs");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcomes per cost branch
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_032_both_costs_available_offers_a_two_way_choice() {
    let mut r = runner(2);
    tamer_with_face_down(&mut r);
    attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.effect_choices.as_ref().unwrap().len(), 2);
}

#[test]
fn ex13_032_security_branch_unsuspends_and_locks_opponent_digimon() {
    let mut r = runner(2);
    let t = tamer_with_face_down(&mut r);
    let (chirin, opp) = attack(&mut r);
    assert!(suspended(&r, chirin));
    r.accept_optional_trigger().expect("accept");
    r.execute_branch(0).expect("trash top security");
    drain(&mut r);
    assert_eq!(r.security_count(0), 1, "1 security card trashed");
    assert_eq!(face_down_count(&r, t), 1, "Tamer stash untouched");
    assert!(!suspended(&r, chirin), "this Digimon unsuspends");
    assert!(
        r.modifiers()
            .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects),
        "opponent Digimon can't activate [When Digivolving]"
    );
}

#[test]
fn ex13_032_face_down_branch_unsuspends_and_locks_opponent_digimon() {
    let mut r = runner(2);
    let t = tamer_with_face_down(&mut r);
    let (chirin, opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    r.execute_branch(1).expect("trash face-down card");
    drain(&mut r);
    assert_eq!(r.security_count(0), 2, "security untouched");
    assert_eq!(face_down_count(&r, t), 0, "bottom face-down card trashed");
    assert!(!suspended(&r, chirin));
    assert!(r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
}

#[test]
fn ex13_032_face_down_only_pays_without_a_choice() {
    let mut r = runner(0);
    let t = tamer_with_face_down(&mut r);
    let (chirin, opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    assert_ne!(
        r.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "only one payable cost → no cost menu"
    );
    drain(&mut r);
    assert_eq!(face_down_count(&r, t), 0);
    assert!(!suspended(&r, chirin));
    assert!(r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
}

#[test]
fn ex13_032_declining_pays_nothing_and_does_nothing() {
    let mut r = runner(2);
    let (chirin, opp) = attack(&mut r);
    r.decline_optional_trigger().expect("decline");
    drain(&mut r);
    assert_eq!(r.security_count(0), 2);
    assert!(suspended(&r, chirin), "stays suspended");
    assert!(!r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
}

#[test]
fn ex13_032_lock_expires_when_opponents_turn_ends() {
    let mut r = runner(2);
    let (_chirin, opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    drain(&mut r);
    r.end_turn();
    drain(&mut r);
    assert!(r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
    r.end_turn();
    drain(&mut r);
    assert!(!r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
}

#[test]
fn ex13_032_when_digivolving_offers_the_clause() {
    let mut r = runner(2);
    let base = r.place_on_field(0, "BASE-L4", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    let idx = r.add_to_hand(0, CARD_ID);
    let ok = {
        let hand_card = r.game.players[0].hand[idx].handle();
        let mut ctx =
            digimon_engine::effect_context::EffectContext::new(&mut r.game, hand_card, None, 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            digimon_engine::enums::CardSourceRef::Hand(0, idx),
            base,
            digimon_engine::enums::CostDelta::Fixed(0),
            false,
        )
    };
    assert!(ok, "Chirinmon digivolves onto a yellow Lv.4");
    assert!(r.pending_is_optional(), "[When Digivolving] offers the clause");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_032_once_per_turn_blocks_a_second_attack_trigger() {
    let mut r = runner(3);
    let (chirin, _opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    drain(&mut r);
    assert!(!suspended(&r, chirin), "unsuspended by the first use");
    r.attack_player(chirin, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: the second attack does not offer it again"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// BLOCKED — would-leave clauses (G-TOP-STACKED-CARD-TO-SECURITY)
// ════════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "pending: G-TOP-STACKED-CARD-TO-SECURITY from qa/archetype-qa/engine-gaps.md"]
fn ex13_032_would_leave_places_top_card_as_top_security_and_stays() {
    let mut r = runner(1);
    let chirin = r.place_stack(0, &["BASE-L4", CARD_ID]);
    let opp = r.place_on_field(1, "KENT", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    r.attack_digimon(chirin, opp, false);
    r.decline_optional_trigger().ok(); // [When Attacking] cost
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.accept_optional_trigger().expect("accept would-leave save");
    drain(&mut r);
    let top = r.game.players[0].security.last().unwrap();
    assert_eq!(top.card_id(&r.game.card_data), CARD_ID, "Chirinmon → top security");
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),
        "BASE-L4",
        "the Digimon stays, topped by the next card"
    );
}

#[test]
#[ignore = "pending: G-TOP-STACKED-CARD-TO-SECURITY from qa/archetype-qa/engine-gaps.md"]
fn ex13_032_inherited_save_only_for_kentaurosmon_named_carrier() {
    // Inherited [All Turns][OPT]: a [Kentaurosmon]-named carrier over Chirinmon
    // that would leave places its top card (the Kentaurosmon) as the top
    // security card and stays as Chirinmon. Authored once the gap closes.
    let mut r = runner(1);
    let carrier = r.place_stack(0, &[CARD_ID, "KENT"]);
    let _ = carrier;
    panic!("authored when G-TOP-STACKED-CARD-TO-SECURITY closes");
}
