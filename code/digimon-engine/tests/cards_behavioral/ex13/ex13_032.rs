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
//! # Verdict — IMPLEMENTED (all clauses)
//! Clauses 2 and 3 (would-leave → place the TOP card as the top security
//! card; the rest of the stack stays) were unblocked by
//! `G-TOP-STACKED-CARD-TO-SECURITY` (RESOLVED 2026-10-01):
//! `security_place_top_stacked_card` now moves the visible top card
//! (`card_sources[len-1]`, DCGO `TopCard`), not the card under it.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - E2 optional processing condition with a two-way cost choice (top security
//!   OR bottom face-down card under a Tamer), [Once Per Turn], WD + WA.
//! - Unsuspend self; opponent Digimon gets CannotActivateWhenDigivolvingEffects
//!   until the end of their turn.
//! - F replacement would-leave (optional processing cost, cancel_replacement).

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

/// The labels of the pending effect choice, in order.
fn choice_labels(r: &DebugRunner) -> Vec<String> {
    r.pending_selection_view()
        .and_then(|v| v.effect_choices)
        .unwrap_or_default()
        .iter()
        .map(|c| c.label.clone())
        .collect()
}

/// Pick the effect-choice row whose label contains `needle`.
fn choose(r: &mut DebugRunner, needle: &str) {
    let view = r.pending_selection_view().expect("effect choice");
    let row = view
        .effect_choices
        .as_ref()
        .and_then(|cs| cs.iter().find(|c| c.label.contains(needle)))
        .unwrap_or_else(|| panic!("no `{needle}` row in {:?}", choice_labels(r)))
        .action_id;
    r.execute_action(view.selecting_player, row).expect("choose");
}

/// DCGO EX13_032.cs offers each payable cost plus "Don't pay the cost" as one
/// `SetIntSelection` (owner ruling 2026-11-23: parity with DCGO).
#[test]
fn ex13_032_both_costs_available_offers_both_costs_and_dont_pay() {
    let mut r = runner(2);
    tamer_with_face_down(&mut r);
    attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    let labels = choice_labels(&r);
    assert_eq!(labels.len(), 3, "{labels:?}");
    assert!(labels[0].contains("security"), "{labels:?}");
    assert!(labels[1].contains("Tamers"), "{labels:?}");
    assert!(labels[2].contains("Don't pay"), "{labels:?}");
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

/// Only the Tamer cost is payable: DCGO lists that cost and "Don't pay the
/// cost" (no dead security row).
#[test]
fn ex13_032_face_down_only_offers_that_cost_and_dont_pay() {
    let mut r = runner(0);
    let t = tamer_with_face_down(&mut r);
    let (chirin, opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    let labels = choice_labels(&r);
    assert_eq!(labels.len(), 2, "{labels:?}");
    assert!(labels[0].contains("Tamers"), "{labels:?}");
    assert!(labels[1].contains("Don't pay"), "{labels:?}");
    choose(&mut r, "Tamers");
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

// ─── [OPT] declined cost keeps the use (owner ruling 2026-11-23) ─────────────
//
// DCGO EX13_032.cs: offered only while a cost is payable
// (`AdditionalActivateCondition`); skippable; then one `SetIntSelection` of
// each payable cost plus "Don't pay the cost". "Don't pay" -> `RemoveUse()`.
// The Tamer cost opens a Tamer `SelectPermanentEffect` with canNoSelect true;
// no Tamer chosen -> nothing paid -> `RemoveUse()`. A paid cost spends the use
// and runs the unsuspend + "After" lock (Q7301).
// qa/rules/mechanics-gaps-2026-10.md W14 row (f).

fn set_unsuspended(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = false;
}

/// Resolve the rest of the attack without touching Chirinmon's clause.
fn drain_without_paying(r: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        let a = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        r.execute_action(view.selecting_player, a).expect("resolve");
    }
}

/// After the use was kept, a second attack the same turn (staged by
/// unsuspending Chirinmon) offers the clause again.
fn second_attack_offers_it(r: &mut DebugRunner, chirin: PermanentHandle) {
    drain_without_paying(r);
    set_unsuspended(r, chirin);
    r.attack_player(chirin, 1, false);
    assert!(
        r.pending_is_optional(),
        "the kept [Once Per Turn] is offered on the next [When Attacking] this turn"
    );
}

#[test]
fn ex13_032_outer_decline_keeps_opt() {
    let mut r = runner(2);
    let (chirin, _opp) = attack(&mut r);
    r.decline_optional_trigger().expect("decline");
    assert_eq!(r.security_count(0), 2);
    second_attack_offers_it(&mut r, chirin);
}

#[test]
fn ex13_032_dont_pay_keeps_opt_and_does_nothing() {
    let mut r = runner(2);
    let t = tamer_with_face_down(&mut r);
    let (chirin, opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    choose(&mut r, "Don't pay");
    assert!(
        r.pending_selection_view()
            .is_none_or(|v| v.kind != SelectionKind::EffectChoice),
        "the clause asks nothing more"
    );
    assert_eq!(r.security_count(0), 2, "no security trashed");
    assert_eq!(face_down_count(&r, t), 1, "no face-down card trashed");
    assert!(suspended(&r, chirin), "not paid: no unsuspend");
    assert!(!r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
    second_attack_offers_it(&mut r, chirin);
}

#[test]
fn ex13_032_security_only_offers_that_cost_and_dont_pay() {
    let mut r = runner(2);
    let (chirin, _opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    let labels = choice_labels(&r);
    assert_eq!(labels.len(), 2, "{labels:?}");
    assert!(labels[0].contains("security"), "{labels:?}");
    assert!(labels[1].contains("Don't pay"), "{labels:?}");
    choose(&mut r, "Don't pay");
    assert_eq!(r.security_count(0), 2);
    second_attack_offers_it(&mut r, chirin);
}

/// The Tamer cost chosen, then no Tamer picked: nothing paid, the use kept.
#[test]
fn ex13_032_tamer_cost_declined_at_the_tamer_pick_keeps_opt() {
    let mut r = runner(0);
    let t = tamer_with_face_down(&mut r);
    let (chirin, opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    choose(&mut r, "Tamers");
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField), "Tamer pick");
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0,
        "DCGO EX13_032.cs Tamer SelectPermanentEffect canNoSelect: the pick may be declined"
    );
    r.execute_action(0, PASS).expect("decline the Tamer pick");
    assert_eq!(face_down_count(&r, t), 1, "nothing trashed");
    assert!(suspended(&r, chirin), "not paid: no unsuspend");
    assert!(!r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
    second_attack_offers_it(&mut r, chirin);
}

/// Paying the security cost spends the use, with the Tamer cost still payable
/// on the next attack.
#[test]
fn ex13_032_paid_cost_spends_opt() {
    let mut r = runner(2);
    tamer_with_face_down(&mut r);
    let (chirin, _opp) = attack(&mut r);
    r.accept_optional_trigger().expect("accept");
    choose(&mut r, "security");
    drain(&mut r);
    assert!(!suspended(&r, chirin), "paid: unsuspended");
    r.attack_player(chirin, 1, false);
    assert!(
        r.pending_selection_view().is_none_or(|v| !v.is_optional),
        "spent: the second attack does not offer it"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 6 — would-leave saves (G-TOP-STACKED-CARD-TO-SECURITY, RESOLVED)
// ════════════════════════════════════════════════════════════════════════════

use super::orphan_support::{assert_replacement_prompt, delete_with};
use digimon_engine::replacement::ReplacementCause;

fn top_of(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn top_security_id(r: &DebugRunner) -> String {
    r.game.players[0]
        .security
        .last()
        .expect("security top")
        .card_id(&r.game.card_data)
        .to_string()
}

#[test]
fn ex13_032_would_leave_places_top_card_as_top_security_and_stays() {
    let mut r = runner(1);
    let chirin = r.place_stack(0, &["BASE-L4", CARD_ID]);
    let sec_before = r.game.players[0].security.len();
    delete_with(&mut r, chirin, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    r.accept_optional_trigger().expect("accept would-leave save");
    drain(&mut r);
    assert_eq!(r.game.players[0].security.len(), sec_before + 1);
    assert_eq!(top_security_id(&r), CARD_ID, "Chirinmon itself → top security");
    let placed = r.game.players[0].security.last().unwrap().card_index;
    assert!(
        !r.game.players[0].face_up_security.contains(&placed),
        "placed face-down (no face-up instruction)"
    );
    assert_eq!(r.game.players[0].battle_area.len(), 1, "the Digimon doesn't leave");
    assert_eq!(top_of(&r, chirin), "BASE-L4", "now topped by the card that was under it");
    assert!(
        !r.game.players[0].trash.iter().any(|c| c.card_id(&r.game.card_data) == "BASE-L4"),
        "nothing was trashed"
    );
}

#[test]
fn ex13_032_would_leave_decline_lets_it_leave() {
    let mut r = runner(1);
    let chirin = r.place_stack(0, &["BASE-L4", CARD_ID]);
    let sec_before = r.game.players[0].security.len();
    delete_with(&mut r, chirin, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    r.decline_optional_trigger().expect("decline");
    drain(&mut r);
    assert!(r.game.players[0].battle_area.is_empty(), "declined → deleted");
    assert_eq!(r.game.players[0].security.len(), sec_before);
}

#[test]
fn ex13_032_would_leave_not_offered_without_a_card_under_it() {
    // Moving the top card would empty the Digimon: no stacked card under the
    // top → the save is unpayable and never offered.
    let mut r = runner(1);
    let chirin = r.place_on_field(0, CARD_ID, Some(0));
    delete_with(&mut r, chirin, ReplacementCause::OpponentEffect);
    drain(&mut r);
    assert!(r.game.players[0].battle_area.is_empty(), "deleted, no save offered");
    assert_eq!(r.game.players[0].security.len(), 1);
}

#[test]
fn ex13_032_would_leave_save_is_not_once_per_turn() {
    let mut r = runner(1);
    let chirin = r.place_stack(0, &["FILL", "BASE-L4", CARD_ID]);
    delete_with(&mut r, chirin, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().expect("first save");
    drain(&mut r);
    assert_eq!(top_of(&r, chirin), "BASE-L4");
    // Chirinmon is now in security; BASE-L4 (no Chirinmon text) is the top, so
    // a second removal is not saved by Chirinmon's face-up clause.
    delete_with(&mut r, chirin, ReplacementCause::OpponentEffect);
    drain(&mut r);
    assert!(r.game.players[0].battle_area.is_empty());
}

#[test]
fn ex13_032_inherited_save_only_for_kentaurosmon_named_carrier() {
    // [Kentaurosmon]-named carrier over Chirinmon: its top card (Kentaurosmon)
    // goes to the top of security and the Digimon stays, now as Chirinmon.
    let mut r = runner(1);
    let carrier = r.place_stack(0, &["BASE-L4", CARD_ID, "KENT"]);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    assert_replacement_prompt(&r);
    r.accept_optional_trigger().expect("accept inherited save");
    drain(&mut r);
    assert_eq!(top_security_id(&r), "KENT", "the Kentaurosmon top card → top security");
    assert_eq!(top_of(&r, carrier), CARD_ID, "stays, topped by Chirinmon");

    // A non-[Kentaurosmon] carrier over Chirinmon gets no inherited save.
    let mut r = runner(1);
    let carrier = r.place_stack(0, &["BASE-L4", CARD_ID, "OPP"]);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    drain(&mut r);
    assert!(r.game.players[0].battle_area.is_empty(), "no save for a non-Kentaurosmon carrier");
}

#[test]
fn ex13_032_inherited_save_is_once_per_turn() {
    let mut r = runner(1);
    let carrier = r.place_stack(0, &["FILL", CARD_ID, "KENT", "KENT"]);
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    r.accept_optional_trigger().expect("first inherited save");
    drain(&mut r);
    assert_eq!(top_of(&r, carrier), "KENT", "still a Kentaurosmon over Chirinmon");
    delete_with(&mut r, carrier, ReplacementCause::OpponentEffect);
    drain(&mut r);
    assert!(
        r.game.players[0].battle_area.is_empty(),
        "[Once Per Turn]: the second removal is not saved"
    );
}
