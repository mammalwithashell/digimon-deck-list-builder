//! EX11-066 Xeno — Tamer, Black, Cost 4, traits: [LIBERATOR].
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX11-066.md`)
//!
//! [Start of Your Main Phase] [On Play] By trashing 1 card with [Vemmon] in its
//! text from your hand, <Draw 1> and gain 1 memory.
//! [All Turns] When your Digimon are played or digivolve, if any of them have
//! [Vemmon] in their texts, by suspending this Tamer, reveal the top 2 cards of
//! your deck. Place all [Vemmon] among them as any of those Digimon's bottom
//! digivolution cards. Trash the rest.
//! (Rule) Name: Also treated as [Zenith].
//!
//! Security Effect: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX11/Black/EX11_066.cs`
//!
//! # Patterns
//! - `also_treated_as: [Zenith]` (Rule name grant — dropped by cards.json).
//! - Shared [SOYMP]/[On Play] optional hand-trash cost (`select_hand
//!   { optional, cost }`) → draw 1 + gain 1 memory.
//! - [All Turns] play/digivolve observer (`on_enter_field_anyone` /
//!   `on_digivolve`) gated by `event_target_in_text_contains: Vemmon`, suspend
//!   cost via `activation_cost: { suspend_self }` (EX12-066 shape).
//! - Reveal 2 → every [Vemmon] (exact name) placed as bottom source of the
//!   triggering Digimon (`choose_from_reveal` → `bottom_source_of`, mandatory,
//!   so the player orders them) → trash the rest.
//! - Tamer [Security] `play_from_security`.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX11-066";

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(level as i32 * 1000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Black as u8,
        level: level.saturating_sub(1),
        memory_cost: 1,
    }];
    c
}

/// A card named exactly [Vemmon].
fn vemmon(id: &str) -> CardData {
    digimon(id, "Vemmon", 3, 3)
}

/// A Digimon with [Vemmon] in its printed text (not named Vemmon).
fn vemmon_text(id: &str, level: u8) -> CardData {
    let mut c = digimon(id, "Snatchmon", level, 3);
    c.effect_text = "[On Play] Add 1 card with [Vemmon] in its text to the hand.".into();
    c
}

fn plain(id: &str, level: u8) -> CardData {
    digimon(id, "Plainmon", level, 3)
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-066 YAML parses and compiles")
        .add_card(vemmon("VEM"))
        .add_card(vemmon_text("VTXT", 4))
        .add_card(vemmon_text("VTXT5", 5))
        .add_card(plain("PLAIN", 4))
        .add_card(plain("PLAIN3", 3))
        .add_card(filler("FILL"))
        .add_card(filler("OTHER"))
}

fn hand_has(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn trash_count(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .trash
        .iter()
        .filter(|c| c.card_id(&r.game.card_data) == id)
        .count()
}

fn find_perm(r: &DebugRunner, p: u8, id: &str) -> Option<PermanentHandle> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == id)
        .map(|i| PermanentHandle {
            player: p,
            index: i as u8,
        })
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("card in hand")
}

// ═══ Section 1 — structure ═══════════════════════════════════════════════════

#[test]
fn ex11_066_structure() {
    let r = builder().build();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "LIBERATOR"));
    let data = r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .unwrap();
    assert_eq!(data.colors, vec![CardColor::Black]);
    assert!(
        data.also_treated_as.iter().any(|n| n == "Zenith"),
        "(Rule) Name: Also treated as [Zenith]"
    );
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::StartOfYourMainPhase)));
    assert!(triggered.iter().any(|t| t.optional
        && t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
        && t.when.contains(&CompiledTiming::OnDigivolve)));
    assert!(triggered
        .iter()
        .any(|t| t.when.contains(&CompiledTiming::OnSecurity)));
}

// ═══ [On Play] / [Start of Your Main Phase] ═══════════════════════════════════

#[test]
fn ex11_066_on_play_trash_vemmon_text_card_draws_and_gains_memory() {
    let mut r = builder()
        .hand(0, &[CARD_ID, "VTXT", "PLAIN"])
        .deck(0, &["FILL", "FILL", "OTHER"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    r.game.enter_main_phase();
    let mem_before = r.memory();
    r.play(0, 0).expect("play Xeno");
    // Memory after paying 4: mem_before - 4.
    let view = r.pending_selection_view().expect("hand cost prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(r.pending_is_optional(), "'By trashing' cost is declinable");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the [Vemmon]-text card"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash VTXT");
    r.auto_resolve().expect("resolve");
    assert_eq!(trash_count(&r, 0, "VTXT"), 1, "cost card trashed");
    assert!(hand_has(&r, 0, "OTHER"), "drew the top card");
    assert_eq!(r.memory(), mem_before - 4 + 1, "gained 1 memory");
}

#[test]
fn ex11_066_on_play_decline_does_nothing() {
    let mut r = builder()
        .hand(0, &[CARD_ID, "VEM"])
        .deck(0, &["FILL", "FILL", "OTHER"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    r.game.enter_main_phase();
    let mem_before = r.memory();
    r.play(0, 0).expect("play Xeno");
    let view = r.pending_selection_view().expect("hand cost prompt");
    r.execute_action(view.selecting_player, PASS)
        .expect("decline");
    r.auto_resolve().expect("resolve");
    assert!(hand_has(&r, 0, "VEM"), "card kept");
    assert!(!hand_has(&r, 0, "OTHER"), "no draw");
    assert_eq!(r.memory(), mem_before - 4, "no memory gain");
}

#[test]
fn ex11_066_on_play_without_vemmon_text_card_has_no_prompt() {
    let mut r = builder()
        .hand(0, &[CARD_ID, "PLAIN"])
        .deck(0, &["FILL", "FILL", "OTHER"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    r.game.enter_main_phase();
    r.play(0, 0).expect("play Xeno");
    assert!(r.pending_selection().is_none(), "no eligible cost card");
    assert!(!hand_has(&r, 0, "OTHER"));
}

#[test]
fn ex11_066_start_of_main_phase_trash_draw_memory() {
    let mut r = builder()
        .hand(0, &["VEM"])
        .deck(0, &["FILL", "FILL", "OTHER"])
        .deck(1, &["FILL", "FILL"])
        .memory(3)
        .start();
    r.place_on_field(0, CARD_ID, Some(0));
    let mem_before = r.memory();
    r.game.enter_main_phase();
    let view = r.pending_selection_view().expect("SOYMP hand cost prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash Vemmon");
    r.auto_resolve().expect("resolve");
    assert_eq!(trash_count(&r, 0, "VEM"), 1);
    assert!(hand_has(&r, 0, "OTHER"));
    assert_eq!(r.memory(), mem_before + 1);
}

// ═══ [All Turns] play / digivolve observer ════════════════════════════════════

#[test]
fn ex11_066_played_vemmon_text_digimon_suspend_reveal_places_vemmon_trashes_rest() {
    let mut r = builder()
        .hand(0, &["VTXT"])
        // last = top: top two are VEM (top) and OTHER.
        .deck(0, &["FILL", "FILL", "OTHER", "VEM"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    // Enter main phase first so Xeno's own [Start of Your Main Phase] does
    // not fire in these observer tests.
    r.game.enter_main_phase();
    let xeno = r.place_on_field(0, CARD_ID, Some(0));
    let _ = r.play(0, 0);

    let view = r
        .pending_selection_view()
        .expect("optional activation prompt");
    assert!(
        r.pending_is_optional(),
        "'by suspending this Tamer' is optional"
    );
    r.accept_optional_trigger().expect("accept");
    r.auto_resolve().expect("resolve reveal");

    assert!(
        r.game.players[0].battle_area[xeno.index as usize].is_suspended,
        "Xeno suspended as the cost"
    );
    let host = find_perm(&r, 0, "VTXT").expect("played Digimon");
    let src = source_ids(&r, host);
    assert_eq!(
        src.first().map(String::as_str),
        Some("VEM"),
        "Vemmon placed at bottom"
    );
    assert_eq!(trash_count(&r, 0, "OTHER"), 1, "the rest trashed");
    assert_eq!(r.deck_size(0), 2, "exactly 2 revealed");
}

#[test]
fn ex11_066_two_vemmon_revealed_both_placed() {
    let mut r = builder()
        .hand(0, &["VTXT"])
        .deck(0, &["FILL", "FILL", "VEM", "VEM"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r.place_on_field(0, CARD_ID, Some(0));
    let _ = r.play(0, 0);
    r.accept_optional_trigger().expect("accept");
    r.auto_resolve().expect("resolve reveal");
    let host = find_perm(&r, 0, "VTXT").expect("played Digimon");
    let src = source_ids(&r, host);
    assert_eq!(
        src.iter().filter(|s| *s == "VEM").count(),
        2,
        "'Place ALL [Vemmon]' — both placed"
    );
    assert_eq!(trash_count(&r, 0, "VEM"), 0);
}

#[test]
fn ex11_066_no_vemmon_revealed_trashes_both() {
    let mut r = builder()
        .hand(0, &["VTXT"])
        .deck(0, &["FILL", "FILL", "OTHER", "OTHER"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r.place_on_field(0, CARD_ID, Some(0));
    let _ = r.play(0, 0);
    r.accept_optional_trigger().expect("accept");
    r.auto_resolve().expect("resolve reveal");
    assert_eq!(trash_count(&r, 0, "OTHER"), 2);
    let host = find_perm(&r, 0, "VTXT").expect("played Digimon");
    assert_eq!(source_ids(&r, host).len(), 1, "no sources added");
}

#[test]
fn ex11_066_vemmon_text_card_in_reveal_is_not_placed() {
    // "Place all [Vemmon]" means cards NAMED Vemmon (DCGO EqualsCardName), not
    // cards that merely mention [Vemmon] in their text.
    let mut r = builder()
        .hand(0, &["VTXT"])
        .deck(0, &["FILL", "FILL", "VTXT5", "OTHER"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r.place_on_field(0, CARD_ID, Some(0));
    let _ = r.play(0, 0);
    r.accept_optional_trigger().expect("accept");
    r.auto_resolve().expect("resolve reveal");
    assert_eq!(trash_count(&r, 0, "VTXT5"), 1);
    assert_eq!(trash_count(&r, 0, "OTHER"), 1);
}

#[test]
fn ex11_066_decline_keeps_tamer_unsuspended_and_deck_intact() {
    let mut r = builder()
        .hand(0, &["VTXT"])
        .deck(0, &["FILL", "FILL", "OTHER", "VEM"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    // Enter main phase first so Xeno's own [Start of Your Main Phase] does
    // not fire in these observer tests.
    r.game.enter_main_phase();
    let xeno = r.place_on_field(0, CARD_ID, Some(0));
    let _ = r.play(0, 0);
    r.decline_optional_trigger().expect("decline");
    r.auto_resolve().expect("settle");
    assert!(!r.game.players[0].battle_area[xeno.index as usize].is_suspended);
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex11_066_non_vemmon_digimon_played_does_not_trigger() {
    let mut r = builder()
        .hand(0, &["PLAIN"])
        .deck(0, &["FILL", "FILL", "OTHER", "VEM"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r.place_on_field(0, CARD_ID, Some(0));
    let _ = r.play(0, 0);
    assert!(
        r.pending_selection().is_none(),
        "no [Vemmon] in text → no trigger"
    );
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex11_066_suspended_tamer_cannot_pay_cost_no_trigger() {
    let mut r = builder()
        .hand(0, &["VTXT"])
        .deck(0, &["FILL", "FILL", "OTHER", "VEM"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    // Enter main phase first so Xeno's own [Start of Your Main Phase] does
    // not fire in these observer tests.
    r.game.enter_main_phase();
    let xeno = r.place_on_field(0, CARD_ID, Some(0));
    r.game.players[0].battle_area[xeno.index as usize].is_suspended = true;
    let _ = r.play(0, 0);
    assert!(
        r.pending_selection().is_none(),
        "already suspended → cost unpayable"
    );
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex11_066_opponent_vemmon_digimon_does_not_trigger() {
    let mut r = builder()
        .hand(1, &["VTXT"])
        .deck(0, &["FILL", "FILL", "OTHER", "VEM"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.place_on_field(0, CARD_ID, Some(0));
    r.end_turn();
    r.game.enter_main_phase();
    assert_eq!(r.game.turn_player(), 1);
    let idx = hand_index(&r, 1, "VTXT");
    r.play(1, idx);
    assert!(
        r.pending_selection().is_none()
            || r.pending_selection_view().unwrap().selecting_player != 0,
        "only YOUR Digimon trigger Xeno"
    );
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex11_066_digivolve_into_vemmon_text_digimon_triggers() {
    let mut r = builder()
        .hand(0, &["VTXT5"])
        // Digivolving draws 1 (the top FILL); the next two are VEM, OTHER.
        .deck(0, &["FILL", "FILL", "OTHER", "VEM", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "PLAIN", Some(0));
    let ok = r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve);
    assert!(ok, "digivolve PLAIN -> VTXT5");
    assert!(r.pending_is_optional(), "Xeno's activation is declinable");
    r.accept_optional_trigger().expect("accept Xeno");
    r.auto_resolve().expect("resolve");
    let host = find_perm(&r, 0, "VTXT5").expect("digivolved Digimon");
    let src = source_ids(&r, host);
    assert_eq!(
        src.first().map(String::as_str),
        Some("VEM"),
        "Vemmon at bottom"
    );
    assert_eq!(trash_count(&r, 0, "OTHER"), 1);
}

// ═══ [Security] ═══════════════════════════════════════════════════════════════

#[test]
fn ex11_066_security_plays_self() {
    let mut atk = make_test_card("ATK", "Attacker");
    atk.card_kind = CardKind::Digimon;
    atk.level = Some(5);
    atk.dp = Some(9000);
    let mut r = builder()
        .add_card(atk)
        .security(1, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    let a = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(a, 1, false);
    r.auto_resolve().expect("resolve");
    assert!(
        find_perm(&r, 1, CARD_ID).is_some(),
        "Xeno played from security"
    );
}
