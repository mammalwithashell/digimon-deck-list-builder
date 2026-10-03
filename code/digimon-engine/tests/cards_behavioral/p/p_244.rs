//! P-244 Unique Emblem: Ragnarok Attainer — Option, Black, Cost 3, traits: [LIBERATOR].
//!
//! # Card text (card image + DCGO; bundle / cards.json are empty for this card)
//!
//! [Main] You may play 1 [Vemmon] or [Zenith] from your hand or trash without
//! paying the cost. Then, place this card in the battle area.
//! [Your Turn] When effects place [Vemmon] as any of your Digimon's
//! digivolution cards, <Delay>.
//! ・1 of your Digimon with [Vemmon] in its text may digivolve into a Digimon
//! card with [Vemmon] in its text in the hand or trash with the cost reduced
//! by 3.
//! [Security] Activate this card's [Main] effect.
//!
//! (Fandom lists the second clause as "[All Turns]"; the card image —
//! 自分のターン — and DCGO `IsOwnerTurn` both say [Your Turn].)
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/P/Black/P_244.cs`
//!
//! # Patterns
//! - Unique Emblem [Main]: optional union-zone free play (`name_is`, alias-aware
//!   so EX11-066 Xeno "also treated as [Zenith]" qualifies) +
//!   `place_self_as_delay_option` (BT22-098 shape).
//! - Event-gated `<Delay>` on `on_add_digivolution_cards` with
//!   `event_added_card_any: { name_is: Vemmon }`; body = BT25-092 hand-or-trash
//!   digivolve (`has_digivolve_candidate` / `can_digivolve_onto`) cost -3.
//! - Option [Security] re-runs [Main].
//! - Integration: EX11-066 Xeno's reveal effect is the "effect places [Vemmon]"
//!   trigger source.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "P-244";
const XENO: &str = "EX11-066";

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, name: &str, level: u8, cost: u16, evo_cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(level as i32 * 1000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Black as u8,
        level: level.saturating_sub(1),
        memory_cost: evo_cost,
    }];
    c
}

fn vemmon_text(id: &str, level: u8, evo_cost: u16) -> CardData {
    let mut c = digimon(id, "Snatchmon", level, 3, evo_cost);
    c.effect_text = "[On Play] Add 1 card with [Vemmon] in its text to the hand.".into();
    c
}

fn zenith_tamer() -> CardData {
    let mut c = make_test_card("ZENITH", "Zenith");
    c.card_kind = CardKind::Tamer;
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-244 YAML parses and compiles")
        .add_card(digimon("VEM", "Vemmon", 3, 3, 0))
        .add_card(zenith_tamer())
        .add_card(vemmon_text("VTXT", 4, 2))
        .add_card(vemmon_text("EVO", 5, 4))
        .add_card(digimon("ANCHOR", "Anchormon", 3, 3, 0))
        .add_card(digimon("PLAIN5", "Plainmon", 5, 5, 4))
        .add_card(filler("FILL"))
        .add_card(filler("OTHER"))
}

fn on_field(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .any(|perm| !perm.card_sources.is_empty() && perm.top_card().card_id(&r.game.card_data) == id)
}

fn in_trash(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn in_hand(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn attainer_state(r: &DebugRunner, p: u8) -> Option<OptionState> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|perm| perm.option_state.clone())
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("in hand")
}

// ═══ Section 1 — structure ═══════════════════════════════════════════════════

#[test]
fn p_244_structure() {
    let r = builder().build();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let data = r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .unwrap();
    assert_eq!(data.card_kind, CardKind::Option);
    assert_eq!(data.play_cost, 3);
    assert_eq!(data.colors, vec![CardColor::Black]);
    assert_eq!(data.traits, vec!["LIBERATOR".to_string()]);
    assert_eq!(card.effects.len(), 3);
    let delay = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                active_when,
                ..
            }) => Some((trigger.clone(), format!("{active_when:?}"))),
            _ => None,
        })
        .expect("<Delay> clause");
    assert_eq!(delay.0, CompiledTiming::OnAddDigivolutionCards);
    assert!(delay.1.contains("your_turn: Some(true)"), "[Your Turn] gate: {}", delay.1);
    assert!(delay.1.contains("Vemmon"), "added-card [Vemmon] gate");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity)
                && t.process.iter().any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption))
    )));
}

// ═══ [Main] ══════════════════════════════════════════════════════════════════

#[test]
fn p_244_main_plays_vemmon_from_hand_free_then_places_self() {
    let mut r = builder()
        .hand(0, &[CARD_ID, "VEM", "VTXT"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    r.place_on_field(0, "ANCHOR", Some(0));
    r.game.enter_main_phase();
    let mem = r.memory();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let view = r.pending_selection_view().expect("union pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert!(r.pending_is_optional(), "'you may play'");
    assert_eq!(view.valid_action_ids.len(), 1, "only [Vemmon] (not [Vemmon]-text cards)");
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick Vemmon");
    r.auto_resolve().expect("resolve");
    assert!(on_field(&r, 0, "VEM"));
    assert_eq!(r.memory(), mem - 3, "only the Option's own cost is paid");
    assert!(matches!(
        attainer_state(&r, 0),
        Some(OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnAddDigivolutionCards),
            ..
        })
    ));
}

#[test]
fn p_244_main_plays_zenith_or_xeno_alias_from_trash() {
    let mut r = builder()
        .dsl_card("EX11-066")
        .expect("Xeno")
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    r.inject_trash(0, "ZENITH");
    r.inject_trash(0, XENO);
    r.inject_trash(0, "VTXT");
    r.place_on_field(0, "ANCHOR", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let view = r.pending_selection_view().expect("union pick");
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "[Zenith] and Xeno (also treated as [Zenith]) are both eligible"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[1])
        .expect("pick");
    r.auto_resolve().expect("resolve");
    assert!(
        on_field(&r, 0, XENO) || on_field(&r, 0, "ZENITH"),
        "a [Zenith] was played from trash"
    );
    assert!(attainer_state(&r, 0).is_some());
}

#[test]
fn p_244_main_decline_still_places_self() {
    let mut r = builder()
        .hand(0, &[CARD_ID, "VEM"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    r.place_on_field(0, "ANCHOR", Some(0));
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    let view = r.pending_selection_view().expect("union pick");
    r.execute_action(view.selecting_player, PASS).expect("decline");
    r.auto_resolve().expect("resolve");
    assert!(in_hand(&r, 0, "VEM"));
    assert!(attainer_state(&r, 0).is_some(), "placed even when declined");
}

// ═══ <Delay> ═════════════════════════════════════════════════════════════════

/// Stage: P-244 placed on turn 1, advance to player 0's next main phase, then
/// place Xeno and play a [Vemmon]-text Digimon so Xeno's effect places a
/// revealed [Vemmon] under it.
fn stage_delay(after_placing_turn: bool) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .dsl_card("EX11-066")
        .expect("Xeno")
        .hand(0, &[CARD_ID, "VTXT", "EVO"])
        // top of deck = last. Draws on turn 3 consume the top card.
        .deck(0, &["FILL", "FILL", "OTHER", "VEM", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    r.place_on_field(0, "ANCHOR", Some(0));
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    r.auto_resolve().expect("resolve [Main] (no target)");
    assert!(attainer_state(&r, 0).is_some());
    if after_placing_turn {
        r.end_turn();
        r.game.enter_main_phase();
        r.end_turn();
        assert_eq!(r.game.turn_player(), 0);
        r.game.enter_main_phase();
        // the turn-3 draw happened in end_turn → the deck top is now VEM,OTHER
    }
    // Make the reveal deterministic: top two = VEM (top), OTHER.
    r.game.memory = 10;
    let xeno = r.place_on_field(0, XENO, Some(0));
    let idx = hand_index(&r, 0, "VTXT");
    let _ = r.play(0, idx);
    // Xeno's [All Turns] activation (optional) — accept.
    r.accept_optional_trigger().expect("accept Xeno");
    // Xeno's mandatory "place all [Vemmon]" reveal pick(s).
    while matches!(r.pending_kind(), Some(SelectionKind::Reveal)) {
        let v = r.pending_selection_view().unwrap();
        r.execute_action(v.selecting_player, v.valid_action_ids[0])
            .expect("place revealed Vemmon");
    }
    (r, xeno)
}

#[test]
fn p_244_delay_vemmon_placed_by_effect_digivolves_with_cost_minus_3() {
    let (mut r, _xeno) = stage_delay(true);
    // Xeno placed a [Vemmon] under VTXT → P-244's <Delay> is offered.
    let view = r.pending_selection_view().expect("<Delay> activation prompt");
    assert_eq!(view.kind, SelectionKind::Replacement, "16-16-2 accept/decline");
    assert!(r.pending_is_optional());
    r.execute_action(view.selecting_player, REPLACEMENT_ACCEPT)
        .expect("accept Delay");
    assert!(in_trash(&r, 0, CARD_ID), "Delay cost trashes P-244");

    let view = r.pending_selection_view().expect("own Digimon pick");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(r.pending_is_optional(), "'may digivolve'");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the [Vemmon]-text Digimon with a legal [Vemmon]-text evolution"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick VTXT");
    let mem_before = r.memory();
    let view = r.pending_selection_view().expect("evolution card pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert_eq!(view.valid_action_ids.len(), 1, "EVO in hand");
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick EVO");
    r.auto_resolve().expect("resolve digivolve");
    assert!(on_field(&r, 0, "EVO"), "digivolved into EVO");
    assert_eq!(r.memory(), mem_before - 1, "evo cost 4 reduced by 3 → 1");
}

#[test]
fn p_244_delay_decline_keeps_option_parked() {
    let (mut r, _xeno) = stage_delay(true);
    let view = r.pending_selection_view().expect("<Delay> activation prompt");
    r.execute_action(view.selecting_player, PASS).expect("decline");
    r.auto_resolve().expect("settle");
    assert!(attainer_state(&r, 0).is_some(), "declined Delay stays parked");
    assert!(in_hand(&r, 0, "EVO"));
}

#[test]
fn p_244_delay_does_not_fire_on_placing_turn() {
    let (mut r, _xeno) = stage_delay(false);
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "no Delay on the placing turn"
    );
    r.auto_resolve().expect("settle");
    assert!(attainer_state(&r, 0).is_some());
    assert!(in_hand(&r, 0, "EVO"));
}

#[test]
fn p_244_delay_not_triggered_by_normal_digivolution_onto_vemmon() {
    // Digivolving ON TOP of a [Vemmon] puts it into the digivolution cards by
    // the digivolve rule, not by an effect → no trigger.
    let mut r = builder()
        .hand(0, &[CARD_ID, "VTXT"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    r.place_on_field(0, "ANCHOR", Some(0));
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    r.auto_resolve().expect("resolve");
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    r.game.enter_main_phase();
    let vem = r.place_on_field(0, "VEM", Some(0));
    let idx = hand_index(&r, 0, "VTXT");
    let ok = r.game.digivolve_from_hand(
        0,
        idx,
        vem.index as usize,
        digimon_engine::enums::PlaySource::ByDigivolve,
    );
    assert!(ok);
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "rule-driven placement must not offer the Delay"
    );
    r.auto_resolve().expect("settle");
    assert!(attainer_state(&r, 0).is_some());
}

// ═══ [Security] ══════════════════════════════════════════════════════════════

#[test]
fn p_244_security_runs_main_then_places_self() {
    let mut atk = make_test_card("ATK", "Attacker");
    atk.card_kind = CardKind::Digimon;
    atk.level = Some(5);
    atk.dp = Some(9000);
    let mut r = builder()
        .add_card(atk)
        .hand(1, &["VEM"])
        .security(1, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(0)
        .start();
    let a = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(a, 1, false);
    let view = r.pending_selection_view().expect("security union pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick Vemmon");
    r.auto_resolve().expect("resolve");
    assert!(on_field(&r, 1, "VEM"), "Vemmon played free");
    assert!(attainer_state(&r, 1).is_some(), "P-244 placed in the battle area");
}


// ═══ <Delay> trigger gates — inline effect-placement fixture ═════════════════

/// Inline fixture: "[On Play] Place 1 card from your trash as this Digimon's
/// bottom digivolution card." An EFFECT-driven placement (fires
/// OnAddDigivolutionCards), fired manually so it can run on either turn.
const PLACER_YAML: &str = r#"
card: TEST-PLACER
name: Placermon
kind: digimon
level: 4
color: [black]
cost: 4
dp: 4000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Place 1 card from your trash as this Digimon's bottom digivolution card"
    process:
      - select_trash:
          of: you
          bind_as: placed
          filter: {}
          prompt: "Place 1 card from your trash under this Digimon"
      - place_as_bottom_source:
          source: placed
          target: source
"#;

/// P-244 parked on turn 1; returns a runner advanced to `opponents_turn`
/// (true → player 1's turn 2; false → player 0's turn 3), with PLACER on
/// player 0's field and `trash_card` in player 0's trash.
fn stage_gate(opponents_turn: bool, trash_card: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .from_dsl_yaml(PLACER_YAML)
        .expect("placer fixture")
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    r.place_on_field(0, "ANCHOR", Some(0));
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, 0);
    r.auto_resolve().expect("resolve [Main] (no target)");
    assert!(attainer_state(&r, 0).is_some());
    r.end_turn();
    r.game.enter_main_phase();
    if !opponents_turn {
        r.end_turn();
        r.game.enter_main_phase();
        assert_eq!(r.game.turn_player(), 0);
    } else {
        assert_eq!(r.game.turn_player(), 1);
    }
    r.game.memory = 10;
    r.inject_trash(0, trash_card);
    let placer = r.place_on_field(0, "TEST-PLACER", Some(0));
    r.game.enqueue_triggered(
        EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(placer),
    );
    r.game.drain_effect_queue();
    // Drive the fixture's mandatory trash pick.
    let view = r.pending_selection_view().expect("fixture trash pick");
    assert_eq!(view.kind, SelectionKind::Trash);
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("place the trash card");
    (r, placer)
}

fn placer_sources(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "TEST-PLACER")
        .map(|p| {
            p.card_sources
                .iter()
                .map(|c| c.card_id(&r.game.card_data).to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn still_delayed(r: &DebugRunner) -> bool {
    matches!(
        attainer_state(r, 0),
        Some(OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnAddDigivolutionCards),
            ..
        })
    )
}

/// Control: the same fixture placement on YOUR turn does offer the Delay —
/// proves the staging below is a real effect placement.
#[test]
fn p_244_delay_offered_for_fixture_vemmon_placement_on_your_turn() {
    let (r, _) = stage_gate(false, "VEM");
    assert_eq!(placer_sources(&r).first().map(String::as_str), Some("VEM"));
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "effect placed [Vemmon] on your turn → <Delay> offered"
    );
}

#[test]
fn p_244_delay_not_offered_on_opponents_turn() {
    let (mut r, _) = stage_gate(true, "VEM");
    assert_eq!(
        placer_sources(&r).first().map(String::as_str),
        Some("VEM"),
        "the effect did place [Vemmon] under your Digimon"
    );
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "[Your Turn] only — no <Delay> prompt on the opponent's turn"
    );
    r.auto_resolve().expect("settle");
    assert!(still_delayed(&r), "P-244 stays parked as a Delayed Option");
    assert!(!in_trash(&r, 0, CARD_ID));
}

#[test]
fn p_244_delay_not_offered_when_placed_card_is_not_vemmon() {
    // VTXT has [Vemmon] in its TEXT but is not named [Vemmon].
    let (mut r, _) = stage_gate(false, "VTXT");
    assert_eq!(placer_sources(&r).first().map(String::as_str), Some("VTXT"));
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::Replacement)),
        "only a card NAMED [Vemmon] triggers the <Delay>"
    );
    r.auto_resolve().expect("settle");
    assert!(still_delayed(&r), "P-244 stays parked as a Delayed Option");
}
