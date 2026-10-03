//! EX13-068 Takato Matsuki — Tamer, Red, Cost 4.
//!
//! # Card text (`data/cards.json` — the corpus files the [Security] line under
//! `inherited_effect_description_eng`; official Bandai DB bundle
//! `data/card_bundles/EX13-068.md` is authoritative)
//!
//! ```text
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [Start of Your Main Phase] By returning this Tamer to the bottom of the
//! deck, you may play 1 [Takato Matsuki] from your hand without paying the
//! cost. After, if you don't have a Digimon, you may play 1 [Guilmon] from
//! your trash without paying the cost.
//!
//! Security Effect: [Security] Play this card without paying the cost.
//! ```
//! Official Q&A: if you don't return this card to the bottom of the deck you
//! can't process the part after "after".
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_068.cs` at `b9a0638cd`. Authored from the printed
//! text + Q&A with the EX12-066 (memory set-to-3), BT24-088 (optional clause +
//! `activation_cost: return_self_to_deck_bottom`) and BT17-093 (Tamer replay
//! from hand) idioms.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Start-of-turn memory set (conditional).
//! - Optional self-return activation cost → optional free play from hand →
//!   conditional ("if you don't have a Digimon") optional free play from trash.
//! - Security: play self.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    HAND_EFFECT_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-068";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Red];
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c
}

fn digimon(id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = cost;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-068 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("TAKATO-ALT", "Takato Matsuki"))
        .add_card(tamer("TAKATO-AND", "Takato Matsuki & Guilmon Fans"))
        .add_card(tamer("OTHER-TAMER", "Rika Nonaka"))
        .add_card(digimon("GUILMON", "Guilmon", 3))
        .add_card(digimon("GUILMON-X", "Guilmon (X Antibody)", 3))
        .add_card(digimon("GROWL", "Growlmon", 5))
        .add_card(digimon("FILL", "Filler", 3))
}

fn ids(cards: &[CardSource], r: &DebugRunner) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn push_to_trash(r: &mut DebugRunner, id: &str) {
    let data_index = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == id)
        .expect("card registered");
    let card_index = r.game.next_card_index();
    r.game.players[0]
        .trash
        .push(CardSource::new(data_index, 0, card_index));
}

/// Takato on the field, `hand` in hand, `trash` in trash.
fn setup(hand: &[&str], trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(3)
        .start();
    r.skip_mulligan();
    let takato = r.place_on_field(0, CARD_ID, Some(0));
    for id in trash {
        push_to_trash(&mut r, id);
    }
    (r, takato)
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, takato: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(takato));
    r.game.drain_effect_queue();
}

fn offered_hand_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    let mut v: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            let slot = if a >= HAND_EFFECT_START {
                a - HAND_EFFECT_START
            } else {
                a - PLAY_HAND_START
            };
            r.game.players[0].hand.get(slot as usize)
        })
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    let slot = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("in hand") as u16;
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} not selectable: {view:?}"));
    r.execute_action(0, a).expect("pick hand card");
}

fn offered_trash_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("trash prompt");
    assert_eq!(view.kind, SelectionKind::Trash);
    let mut v: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(TRASH_EFFECT_START))
        .filter_map(|i| r.game.players[0].trash.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    v.sort();
    v
}

fn pick_trash(r: &mut DebugRunner, id: &str) {
    let idx = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("in trash") as u16;
    r.execute_action(0, TRASH_EFFECT_START + idx)
        .expect("pick trash card");
}

fn triggered(c: &digimon_dsl::compiled::CompiledCard, t: CompiledTiming) -> Vec<&digimon_dsl::compiled::CompiledTriggeredClause> {
    c.effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(tr) if tr.when.contains(&t) => Some(tr),
            _ => None,
        })
        .collect()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_068_printed_metadata() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.kind, CompiledCardKind::Tamer);
    assert_eq!(c.color, vec![CompiledColor::Red]);
    assert_eq!(c.cost, Some(4));
    assert!(c.traits.is_empty());
}

#[test]
fn ex13_068_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.effects.len(), 3);

    let sot = triggered(c, CompiledTiming::StartOfYourTurn);
    assert_eq!(sot.len(), 1);
    assert!(!sot[0].optional, "memory set is mandatory");
    assert!(sot[0].condition.is_some(), "gated on 2 or less memory");

    let som = triggered(c, CompiledTiming::StartOfYourMainPhase);
    assert_eq!(som.len(), 1);
    assert!(som[0].optional, "\"By returning…, you may\" — declinable cost");
    assert!(!som[0].once_per_turn);
    assert!(matches!(
        som[0].process.first(),
        Some(CompiledStep::ActivationCost { .. })
    ));

    let sec = triggered(c, CompiledTiming::OnSecurity);
    assert_eq!(sec.len(), 1);
    assert_eq!(sec[0].process, vec![CompiledStep::PlayFromSecurity]);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [Start of Your Turn] memory set
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_068_start_of_turn_sets_memory_to_three_when_two_or_less() {
    let (mut r, takato) = setup(&[], &[]);
    r.game.set_memory(2);
    fire(&mut r, EffectTiming::StartOfYourTurn, takato);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3);
}

#[test]
fn ex13_068_start_of_turn_sets_memory_from_zero() {
    let (mut r, takato) = setup(&[], &[]);
    r.game.set_memory(0);
    fire(&mut r, EffectTiming::StartOfYourTurn, takato);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 3);
}

#[test]
fn ex13_068_start_of_turn_leaves_memory_above_two_alone() {
    let (mut r, takato) = setup(&[], &[]);
    r.game.set_memory(4);
    fire(&mut r, EffectTiming::StartOfYourTurn, takato);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 4, "3+ memory is not changed (never lowered)");
}

/// Drive end_turn → opponent's turn → end_turn back to us, declining any
/// optional offer, and return our memory at the start of our main phase.
fn memory_after_real_turn_cycle(with_takato: bool) -> i16 {
    let mut r = builder()
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(3)
        .start();
    r.skip_mulligan();
    if with_takato {
        r.place_on_field(0, CARD_ID, Some(0));
    }
    r.end_turn();
    let _ = r.auto_resolve();
    // Opponent passes with 1 memory on their side, so we start our turn
    // with 1 memory (≤ 2) before any start-of-turn effect.
    r.game.memory = 1;
    r.end_turn();
    for _ in 0..4 {
        match r.pending_selection_view() {
            Some(v) if v.is_optional => {
                r.execute_action(v.selecting_player, PASS).expect("decline");
            }
            _ => break,
        }
    }
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player(), 0, "back on our turn");
    r.memory()
}

#[test]
fn ex13_068_start_of_turn_fires_through_real_turn_flow() {
    let without = memory_after_real_turn_cycle(false);
    assert!(without <= 2, "control: we start the turn at ≤2 memory ({without})");
    assert_eq!(
        memory_after_real_turn_cycle(true),
        3,
        "[Start of Your Turn] sets ≤2 memory to 3"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2/3 — [Start of Your Main Phase] return → Takato → (no Digimon) Guilmon
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_068_start_of_main_is_declinable_and_decline_keeps_tamer() {
    let (mut r, takato) = setup(&["TAKATO-ALT"], &["GUILMON"]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, takato);
    assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder));
    assert!(r.pending_is_optional());
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()], "Tamer stays");
    assert_eq!(ids(&r.game.players[0].hand, &r), vec!["TAKATO-ALT".to_string()]);
    assert_eq!(ids(&r.game.players[0].trash, &r), vec!["GUILMON".to_string()]);
}

#[test]
fn ex13_068_start_of_main_returns_self_and_offers_only_takato_named_cards() {
    let (mut r, takato) = setup(&["TAKATO-ALT", "TAKATO-AND", "OTHER-TAMER"], &[]);
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, takato);
    r.accept_optional_trigger().expect("accept");
    assert!(field_ids(&r, 0).is_empty(), "cost: Tamer left the field");
    assert_eq!(r.deck_size(0), deck_before + 1);
    assert_eq!(
        r.game.players[0].deck[0].card_id(&r.game.card_data),
        CARD_ID,
        "returned to the BOTTOM of the deck"
    );
    assert_eq!(
        offered_hand_ids(&r),
        vec!["TAKATO-ALT".to_string()],
        "exactly [Takato Matsuki] — not other Tamers or compound names"
    );
}

#[test]
fn ex13_068_plays_takato_free_then_guilmon_free_when_no_digimon() {
    let (mut r, takato) = setup(&["TAKATO-ALT"], &["GUILMON", "GUILMON-X", "GROWL"]);
    r.game.set_memory(3);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, takato);
    r.accept_optional_trigger().expect("accept");
    pick_hand(&mut r, "TAKATO-ALT");
    assert_eq!(
        offered_trash_ids(&r),
        vec!["GUILMON".to_string()],
        "exactly [Guilmon] from trash"
    );
    pick_trash(&mut r, "GUILMON");
    let _ = r.auto_resolve();
    let mut field = field_ids(&r, 0);
    field.sort();
    assert_eq!(field, vec!["GUILMON".to_string(), "TAKATO-ALT".to_string()]);
    assert_eq!(r.memory(), 3, "both plays are free");
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex13_068_no_guilmon_offer_when_you_have_a_digimon() {
    let (mut r, takato) = setup(&["TAKATO-ALT"], &["GUILMON"]);
    r.place_on_field(0, "GROWL", Some(0));
    fire(&mut r, EffectTiming::StartOfYourMainPhase, takato);
    r.accept_optional_trigger().expect("accept");
    pick_hand(&mut r, "TAKATO-ALT");
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none(), "you have a Digimon → no Guilmon");
    assert_eq!(ids(&r.game.players[0].trash, &r), vec!["GUILMON".to_string()]);
    let mut field = field_ids(&r, 0);
    field.sort();
    assert_eq!(field, vec!["GROWL".to_string(), "TAKATO-ALT".to_string()]);
}

#[test]
fn ex13_068_guilmon_offered_even_without_a_takato_in_hand() {
    let (mut r, takato) = setup(&["OTHER-TAMER"], &["GUILMON"]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, takato);
    r.accept_optional_trigger().expect("accept");
    // No [Takato Matsuki] in hand: the hand play is skipped, the cost is paid.
    if let Some(v) = r.pending_selection_view() {
        if v.kind == SelectionKind::Hand {
            r.execute_action(0, PASS).expect("nothing to play");
        }
    }
    assert_eq!(offered_trash_ids(&r), vec!["GUILMON".to_string()]);
    pick_trash(&mut r, "GUILMON");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["GUILMON".to_string()]);
    assert_eq!(ids(&r.game.players[0].hand, &r), vec!["OTHER-TAMER".to_string()]);
}

#[test]
fn ex13_068_takato_play_is_optional() {
    let (mut r, takato) = setup(&["TAKATO-ALT"], &["GUILMON"]);
    fire(&mut r, EffectTiming::StartOfYourMainPhase, takato);
    r.accept_optional_trigger().expect("accept");
    let v = r.pending_selection_view().expect("hand prompt");
    assert!(v.is_optional, "\"you may play\" {v:?}");
    r.execute_action(0, PASS).expect("decline Takato");
    // Still no Digimon → Guilmon offered.
    assert_eq!(offered_trash_ids(&r), vec!["GUILMON".to_string()]);
    let v = r.pending_selection_view().expect("trash prompt");
    assert!(v.is_optional, "\"you may play\" Guilmon");
    r.execute_action(0, PASS).expect("decline Guilmon");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
    assert_eq!(ids(&r.game.players[0].hand, &r), vec!["TAKATO-ALT".to_string()]);
    assert_eq!(ids(&r.game.players[0].trash, &r), vec!["GUILMON".to_string()]);
}

// ════════════════════════════════════════════════════════════════════════════
// [Security]
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_068_security_plays_itself_for_free() {
    let mut r = builder()
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    r.skip_mulligan();
    let attacker = r.place_on_field(0, "GROWL", Some(0));
    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec![CARD_ID.to_string()]);
    assert_eq!(r.security_count(1), 0);
}
