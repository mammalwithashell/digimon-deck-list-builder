//! EX13-025 Candlemon — Digimon, Lv.3, Yellow, DP 2000, Cost 3.
//! Traits: Flame (+ (Rule) Trait: Has [Witchelny]). Form: Rookie. Attribute: Data.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-025.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-025.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.2 / cost 0
//!
//! [Start of Your Main Phase] If you have 3 or more security cards, trash your
//! top or bottom security card, <Draw 1> (Draw 1 card from your deck.) and
//! gain 1 memory. Then, if you have 2 or fewer security cards, you may place 1
//! card with [Witchelny] in its text from your hand as the bottom security
//! card. (Rule) Trait: Has [Witchelny].
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon with [Dynasmon] or
//! [Witchelny] in its text would leave the battle area by your opponent's
//! effects, by trashing your top security card, it doesn't leave.
//! ```
//!
//! # DCGO C# reference
//! None for EX13_025.cs. The inherited clause is the BT18-030 Candlemon
//! inherited (`DCGO/Assets/Scripts/CardEffect/BT18/Yellow/BT18_030.cs`,
//! WhenRemoveField, OPT, optional, `IsOpponentEffect`, top-card gate,
//! `SecurityCards.Count >= 1`) with an [X]-in-text carrier gate.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Start-of-your-main-phase trigger, processing-condition gate.
//! - Branch choice (top / bottom security trash) via EffectChoice.
//! - Optional hand → bottom-security placement, [X]-in-text filter.
//! - F3 inherited leave-prevention replacement, OPT, security-trash cost,
//!   opponent-effect cause gate, carrier-text gate.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-025";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, traits: &[&str], text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-025 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("S1", "Sec One", 3, &[], ""))
        .add_card(digimon("S2", "Sec Two", 3, &[], ""))
        .add_card(digimon("S3", "Sec Three", 3, &[], ""))
        .add_card(digimon("S4", "Sec Four", 3, &[], ""))
        .add_card(digimon("S5", "Sec Five", 3, &[], ""))
        .add_card(digimon("FILL", "Filler", 3, &[], ""))
        .add_card(digimon("W-TRAIT", "Witch Trait", 4, &["Witchelny"], ""))
        .add_card(digimon(
            "W-TEXT",
            "Witch Text",
            4,
            &[],
            "[On Play] Reveal 3; add 1 card with [Witchelny] in its text.",
        ))
        .add_card(digimon("PLAIN", "Plainmon", 4, &[], ""))
        .add_card(digimon("CARRIER-W", "Witch Carrier", 4, &["Witchelny"], ""))
        .add_card(digimon("CARRIER-D", "Dynasmon", 6, &["Royal Knight"], ""))
        .add_card(digimon("CARRIER-PLAIN", "Plain Carrier", 4, &["Beast"], ""))
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game.enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn security_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .security
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn handle_of(r: &DebugRunner, id: &str) -> PermanentHandle {
    let i = field_ids(r)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on field"));
    PermanentHandle {
        player: 0,
        index: i as u8,
    }
}

/// Runner with Candlemon on player 0's field and the given security stack
/// (index 0 = bottom, last = top) and hand.
fn main_phase_setup(security: &[&str], hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .security(0, security)
        .hand(0, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(3)
        .start();
    r.skip_mulligan();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    (r, me)
}

/// Pick option `idx` of the pending EffectChoice (top = 0, bottom = 1).
fn choose(r: &mut DebugRunner, idx: usize) {
    let v = r.pending_selection_view().expect("effect choice pending");
    assert_eq!(v.kind, SelectionKind::EffectChoice, "{v:?}");
    let opts: Vec<u16> = v.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(opts.len(), 2, "exactly top / bottom offered: {v:?}");
    r.execute_action(v.selecting_player, opts[idx]).expect("choose");
}

fn offered_hand_count(r: &DebugRunner) -> usize {
    let v = r.pending_selection_view().expect("hand prompt pending");
    v.valid_action_ids.iter().filter(|&&a| a != PASS).count()
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let v = r.pending_selection_view().expect("hand prompt");
    assert_eq!(v.kind, SelectionKind::Hand, "{v:?}");
    let idx = hand_ids(r).iter().position(|c| c == id).expect("in hand");
    let a = [HAND_EFFECT_START + idx as u16, PLAY_HAND_START + idx as u16]
        .into_iter()
        .find(|a| v.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} selectable: {v:?}"));
    r.execute_action(v.selecting_player, a).expect("pick hand");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_025_printed_metadata_and_digivolve_path() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(2000), Some(3)));
    assert_eq!(c.color, vec![CompiledColor::Yellow]);
    assert!(c.traits.contains(&"Flame".to_string()));
    assert!(
        c.traits.contains(&"Witchelny".to_string()),
        "(Rule) Trait: Has [Witchelny]"
    );
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1, "Yellow Lv.2 circle");
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn ex13_025_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let t = triggered[0];
    assert_eq!(t.when, vec![CompiledTiming::StartOfYourMainPhase]);
    assert!(!t.optional, "the trash/draw/memory half is mandatory");
    assert!(!t.once_per_turn);
    assert_eq!(t.scope, CompiledScope::FaceUp);

    let replacements: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                scope,
                optional,
                once_per_turn,
                ..
            }) => Some((*scope, *optional, *once_per_turn)),
            _ => None,
        })
        .collect();
    assert_eq!(
        replacements,
        vec![(CompiledScope::Inherited, true, true)],
        "one inherited, optional (by trashing…), once-per-turn replacement"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [Start of Your Main Phase]
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_025_five_security_trash_top_draws_and_gains_memory() {
    let (mut r, me) = main_phase_setup(&["S1", "S2", "S3", "S4", "S5"], &["W-TRAIT"]);
    let mem = r.game.memory;
    let hand = r.game.players[0].hand.len();
    let cp = r.event_checkpoint();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, me);
    choose(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2", "S3", "S4"], "top (S5) trashed");
    assert!(trash_ids(&r).contains(&"S5".to_string()));
    assert_eq!(r.game.players[0].hand.len(), hand + 1, "<Draw 1>");
    assert_eq!(r.game.memory, mem + 1, "gain 1 memory");
    assert!(
        r.events_since(cp)
            .iter()
            .any(|e| matches!(e, GameEvent::MemoryChange { .. })),
        "memory gain is logged"
    );
    assert!(
        r.pending_selection_view().is_none(),
        "4 security left → no bottom-security placement offer"
    );
}

#[test]
fn ex13_025_five_security_trash_bottom() {
    let (mut r, me) = main_phase_setup(&["S1", "S2", "S3", "S4", "S5"], &[]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, me);
    choose(&mut r, 1);
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0), vec!["S2", "S3", "S4", "S5"], "bottom (S1) trashed");
    assert!(trash_ids(&r).contains(&"S1".to_string()));
}

#[test]
fn ex13_025_three_security_trash_then_offers_witchelny_bottom_placement() {
    let (mut r, me) = main_phase_setup(&["S1", "S2", "S3"], &["W-TRAIT", "W-TEXT", "PLAIN"]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, me);
    choose(&mut r, 0);
    let v = r.pending_selection_view().expect("2 security → placement offer");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional, "\"you may place\"");
    assert_eq!(
        offered_hand_count(&r),
        2,
        "only [Witchelny]-in-text cards (trait or effect text) are offered"
    );
    pick_hand(&mut r, "W-TEXT");
    let _ = r.auto_resolve();
    assert_eq!(
        security_ids(&r, 0),
        vec!["W-TEXT", "S1", "S2"],
        "placed as the BOTTOM security card"
    );
    assert!(!hand_ids(&r).contains(&"W-TEXT".to_string()));
}

#[test]
fn ex13_025_placement_may_be_declined() {
    let (mut r, me) = main_phase_setup(&["S1", "S2", "S3"], &["W-TRAIT"]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, me);
    choose(&mut r, 0);
    let v = r.pending_selection_view().expect("placement offer");
    r.execute_action(v.selecting_player, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2"]);
    assert!(hand_ids(&r).contains(&"W-TRAIT".to_string()));
}

#[test]
fn ex13_025_two_security_skips_trash_but_offers_placement() {
    let (mut r, me) = main_phase_setup(&["S1", "S2"], &["W-TRAIT"]);
    let mem = r.game.memory;
    let hand = r.game.players[0].hand.len();
    fire(&mut r, EffectTiming::StartOfYourMainPhase, me);
    let v = r.pending_selection_view().expect("placement offer");
    assert_eq!(v.kind, SelectionKind::Hand, "no top/bottom choice under 3 security");
    assert_eq!(r.game.memory, mem, "no memory gain");
    assert_eq!(r.game.players[0].hand.len(), hand, "no draw");
    pick_hand(&mut r, "W-TRAIT");
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0), vec!["W-TRAIT", "S1", "S2"]);
}

#[test]
fn ex13_025_four_security_no_placement_offer() {
    let (mut r, me) = main_phase_setup(&["S1", "S2", "S3", "S4"], &["W-TRAIT"]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, me);
    choose(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0).len(), 3, "3 left (> 2) → no offer");
    assert!(hand_ids(&r).contains(&"W-TRAIT".to_string()));
}

#[test]
fn ex13_025_no_witchelny_card_in_hand_no_prompt() {
    let (mut r, me) = main_phase_setup(&["S1", "S2"], &["PLAIN"]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, me);
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2"]);
    assert!(hand_ids(&r).contains(&"PLAIN".to_string()));
}

#[test]
fn ex13_025_fires_at_start_of_own_main_phase_integrated() {
    let (mut r, _me) = main_phase_setup(&["S1", "S2", "S3", "S4", "S5"], &[]);
    // P0 → P1: player 1's main phase must NOT fire Candlemon ("Your").
    r.end_turn();
    for _ in 0..4 {
        match r.pending_selection_view() {
            Some(v) => {
                assert_ne!(v.kind, SelectionKind::EffectChoice, "not on the opponent's turn");
                let a = v.valid_action_ids.first().copied().unwrap_or(PASS);
                r.execute_action(v.selecting_player, a).unwrap();
            }
            None => break,
        }
    }
    assert_eq!(security_ids(&r, 0).len(), 5);
    // P1 → P0: Candlemon's [Start of Your Main Phase] fires.
    r.end_turn();
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "the top/bottom choice surfaces at the start of P0's main phase"
    );
    choose(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2", "S3", "S4"]);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3/5 — Inherited [All Turns][OPT] leave prevention
// ════════════════════════════════════════════════════════════════════════════

fn inherited_setup(carrier: &str, security: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .security(0, security)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .start();
    r.skip_mulligan();
    let h = r.place_stack(0, &[CARD_ID, carrier]);
    (r, h)
}

fn accept_save(r: &mut DebugRunner) {
    let accept = r.pending_selection_view().expect("replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional, "\"by trashing…\" is a may-pay");
    r.accept_optional_trigger().expect("accept");
    let _ = r.auto_resolve();
}

#[test]
fn ex13_025_inherited_witchelny_carrier_trashes_top_security_and_stays() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", &["S1", "S2", "S3"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    accept_save(&mut r);
    assert!(field_ids(&r).contains(&"CARRIER-W".to_string()), "it doesn't leave");
    assert_eq!(security_ids(&r, 0), vec!["S1", "S2"], "top security trashed");
    assert!(
        trash_ids(&r).contains(&"S3".to_string()),
        "the cost moved the top security card (S3) to the trash"
    );
}

#[test]
fn ex13_025_inherited_dynasmon_named_carrier_is_protected() {
    let (mut r, carrier) = inherited_setup("CARRIER-D", &["S1", "S2"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    accept_save(&mut r);
    assert!(field_ids(&r).contains(&"CARRIER-D".to_string()));
    assert_eq!(security_ids(&r, 0), vec!["S1"]);
}

#[test]
fn ex13_025_inherited_not_offered_for_carrier_without_text() {
    let (mut r, carrier) = inherited_setup("CARRIER-PLAIN", &["S1", "S2"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r).contains(&"CARRIER-PLAIN".to_string()));
    assert_eq!(security_ids(&r, 0).len(), 2);
}

#[test]
fn ex13_025_inherited_not_offered_for_own_effect() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", &["S1", "S2"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    assert!(r.pending_selection_view().is_none(), "\"by your opponent's effects\" only");
    assert!(!field_ids(&r).contains(&"CARRIER-W".to_string()));
}

#[test]
fn ex13_025_inherited_not_offered_in_battle() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", &["S1", "S2"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::Battle);
    assert!(r.pending_selection_view().is_none(), "battle is not an effect");
    assert!(!field_ids(&r).contains(&"CARRIER-W".to_string()));
}

#[test]
fn ex13_025_inherited_not_offered_without_security() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", &[]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none(), "unpayable cost → no offer");
    assert!(!field_ids(&r).contains(&"CARRIER-W".to_string()));
}

#[test]
fn ex13_025_inherited_decline_lets_carrier_leave() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", &["S1", "S2"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(!field_ids(&r).contains(&"CARRIER-W".to_string()));
    assert_eq!(security_ids(&r, 0).len(), 2, "no cost paid");
}

#[test]
fn ex13_025_inherited_protects_only_this_digimon() {
    let (mut r, _carrier) = inherited_setup("CARRIER-W", &["S1", "S2"]);
    let other = r.place_on_field(0, "W-TRAIT", Some(0));
    r.game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r).contains(&"W-TRAIT".to_string()));
}

#[test]
fn ex13_025_inherited_once_per_turn() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", &["S1", "S2", "S3"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    accept_save(&mut r);
    let carrier = handle_of(&r, "CARRIER-W");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none(), "OPT spent this turn");
    assert!(!field_ids(&r).contains(&"CARRIER-W".to_string()));
}

#[test]
fn ex13_025_inherited_lockout_clears_next_turn() {
    let (mut r, carrier) = inherited_setup("CARRIER-W", &["S1", "S2", "S3"]);
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    accept_save(&mut r);
    r.end_turn();
    let _ = r.auto_resolve();
    let carrier = handle_of(&r, "CARRIER-W");
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "OPT resets on the next turn"
    );
}
