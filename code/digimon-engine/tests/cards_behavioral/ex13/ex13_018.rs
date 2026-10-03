//! EX13-018 Coredramon — Digimon, Lv.4, Blue/Red, DP 5000, Cost 5.
//! Traits: Dragon. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-018.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-018.md` agrees)
//!
//! ```text
//! Digivolve: Blue Lv.3 / cost 3; Red Lv.3 / cost 3.
//! [Digivolve] Lv.3 w/[Dracomon] in name: Cost 2
//!
//! Effect:
//! [On Play] [When Digivolving] By trashing 1 card with [Dracomon] or [Examon]
//! in its text from your hand, <Draw 2> (Draw 2 cards from your deck.)
//! [Your Turn] When any of your other Digimon with [Dracomon] or [Examon] in
//! their texts are played, this Digimon may digivolve into a Digimon card
//! with [Examon] in its text in the hand with the cost reduced by 2.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_018.cs
//! - Alt digivolve: `ContainsCardName("Dracomon")`, level 3, cost 2,
//!   `ignoreDigivolutionRequirement: false`.
//! - Shared [OP]/[WD]: skippable, gated on a text card in hand;
//!   `SelectHandEffect(Discard, maxCount 1, canNoSelect: true)` → if a card was
//!   discarded, `DrawClass(2)`.
//! - [Your Turn] `OnEnterFieldAnyone` (no OPT, skippable):
//!   `CanTriggerOnPermanentPlay(own Digimon, != this, TopCard.HasText(...))` →
//!   `DigivolveIntoHandOrTrashCard(this, IsDigimon && HasText("Examon"),
//!   payCost: true, reduceCost: 2, isHand: true)`.
//! - Inherited `ChangeSelfDPStaticEffect(+2000, IsOwnerTurn)`.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - C-cost "By trashing 1 card from hand" optional cost → Draw 2 (BT24-008).
//! - F observer: own-ally-played ("[X] in its text") → self may digivolve into
//!   a hand card with cost reduced by 2 (EX12-002 idiom, [Your Turn], no OPT).
//! - D1 inherited [Your Turn] +2000 DP self-aura.
//! - Named alt digivolve (Lv.3 w/[Dracomon] in name / cost 2).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-018";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn card(id: &str, name: &str, level: u8, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.effect_text = text.to_string();
    c.play_cost = level as u16;
    c
}

/// Lv.5 red Digimon that digivolves from a red Lv.4 for 4.
fn examon_evo(id: &str, text: &str) -> CardData {
    let mut c = card(id, id, 5, text);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(7000);
    c.play_cost = 7;
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 4,
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut option_text = card("OPTION-TEXT", "Dragon Option", 3, "Choose 1 [Dracomon].");
    option_text.card_kind = CardKind::Option;
    option_text.level = None;
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-018 YAML parses, compiles and is in the embedded pack")
        .add_card({
            // Yellow: only the "Lv.3 w/[Dracomon] in name" route applies to it.
            let mut c = card("DRACO-L3", "Dracomon Variant", 3, "");
            c.colors = vec![CardColor::Yellow];
            c
        })
        .add_card(card("EXAMON-TEXT-L3", "Text Three", 3, "Treat as [Examon] material."))
        .add_card(card("PLAIN-L3", "Plain Three", 3, ""))
        .add_card(card("RED-L3", "Red Three", 3, ""))
        .add_card(option_text)
        .add_card(examon_evo("EXAMON-EVO", "[Examon] DNA material."))
        .add_card(examon_evo("PLAIN-EVO", ""))
        .add_card(card("CARRIER-L5", "Carrier Five", 5, ""))
        .add_card(card("FILL", "Fill", 3, ""))
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_index(runner: &DebugRunner, player: u8, id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player {player}'s hand"))
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn hand_actions(runner: &DebugRunner, player: u8, id: &str) -> [u16; 2] {
    let slot = hand_index(runner, player, id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
}

/// Accept any leading yes/no gate, then pick `id` from the hand prompt.
fn pick_hand(runner: &mut DebugRunner, id: &str) {
    for _ in 0..4 {
        let view = runner.pending_selection_view().expect("a prompt must be pending");
        if view.kind == SelectionKind::Hand {
            let action = hand_actions(runner, 0, id)
                .into_iter()
                .find(|a| view.valid_action_ids.contains(a))
                .unwrap_or_else(|| panic!("{id} must be selectable; view={view:?}"));
            runner
                .execute_action(view.selecting_player, action)
                .expect("pick hand card");
            return;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("accept gate");
        runner
            .execute_action(view.selecting_player, action)
            .expect("accept gate");
    }
    panic!("never reached the hand prompt");
}

/// Walk to the hand prompt and return whether `id` is a legal pick there.
fn hand_pick_offered(runner: &mut DebugRunner, id: &str) -> bool {
    for _ in 0..4 {
        let Some(view) = runner.pending_selection_view() else {
            return false;
        };
        if view.kind == SelectionKind::Hand {
            return hand_actions(runner, 0, id)
                .iter()
                .any(|a| view.valid_action_ids.contains(a));
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("accept gate");
        runner
            .execute_action(view.selecting_player, action)
            .expect("accept gate");
    }
    false
}

fn triggered(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn play_coredramon(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut all = vec![CARD_ID];
    all.extend_from_slice(hand);
    let mut runner = builder()
        .hand(0, &all)
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let slot = hand_index(&runner, 0, CARD_ID);
    let field = runner.play(0, slot).expect("Coredramon plays");
    (
        runner,
        PermanentHandle {
            player: 0,
            index: field as u8,
        },
    )
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_018_has_two_circles_and_the_dracomon_name_alt_digivolve() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 3, "Blue Lv.3/3 + Red Lv.3/3 + Lv.3 [Dracomon]-name/2");
}

#[test]
fn ex13_018_clause_shape_matches_printed_text() {
    let runner = builder().start();
    let t = triggered(&runner);
    let face: Vec<_> = t.iter().filter(|c| c.scope == CompiledScope::FaceUp).collect();
    assert_eq!(face.len(), 2, "[OP]/[WD] draw + [Your Turn] digivolve observer");
    let draw = face
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play] clause");
    assert!(draw.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!draw.once_per_turn);
    let observer = face
        .iter()
        .find(|c| !c.when.contains(&CompiledTiming::OnPlay))
        .expect("observer clause");
    assert!(!observer.once_per_turn, "no [Once Per Turn] printed");

    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                active_when,
                ..
            }) => Some((*scope, *dp_modifier, active_when.is_some())),
            _ => None,
        })
        .collect();
    assert_eq!(
        auras,
        vec![(CompiledScope::Inherited, Some(2000), true)],
        "inherited [Your Turn] +2000 DP aura"
    );
}

// ─── Section 2/3 — [On Play] trash-to-draw ───────────────────────────────────

#[test]
fn ex13_018_on_play_trashing_a_text_card_draws_two() {
    let (mut runner, _) = play_coredramon(&["DRACO-L3", "PLAIN-L3"]);
    pick_hand(&mut runner, "DRACO-L3");
    let _ = runner.auto_resolve();
    assert!(trash_ids(&runner, 0).contains(&"DRACO-L3".to_string()), "cost paid");
    let hand = hand_ids(&runner, 0);
    assert_eq!(hand.iter().filter(|h| *h == "FILL").count(), 2, "<Draw 2>");
    assert!(hand.contains(&"PLAIN-L3".to_string()));
}

#[test]
fn ex13_018_on_play_accepts_an_examon_text_option_card() {
    let (mut runner, _) = play_coredramon(&["OPTION-TEXT"]);
    pick_hand(&mut runner, "OPTION-TEXT");
    let _ = runner.auto_resolve();
    assert!(trash_ids(&runner, 0).contains(&"OPTION-TEXT".to_string()));
    assert_eq!(hand_ids(&runner, 0).len(), 2, "drew 2");
}

#[test]
fn ex13_018_on_play_plain_hand_card_is_not_a_legal_cost() {
    let (mut runner, _) = play_coredramon(&["DRACO-L3", "PLAIN-L3"]);
    assert!(!hand_pick_offered(&mut runner, "PLAIN-L3"));
}

#[test]
fn ex13_018_on_play_declining_the_cost_draws_nothing() {
    let (mut runner, _) = play_coredramon(&["DRACO-L3"]);
    for _ in 0..4 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        assert!(view.is_optional, "the trash cost is optional");
        runner
            .execute_action(view.selecting_player, PASS)
            .expect("decline");
    }
    assert_eq!(hand_ids(&runner, 0), vec!["DRACO-L3"], "no trash, no draw");
    assert!(trash_ids(&runner, 0).is_empty());
}

#[test]
fn ex13_018_on_play_without_a_text_card_in_hand_installs_nothing() {
    let (runner, _) = play_coredramon(&["PLAIN-L3"]);
    assert!(runner.pending_selection().is_none());
    assert_eq!(hand_ids(&runner, 0), vec!["PLAIN-L3"]);
}

#[test]
fn ex13_018_when_digivolving_from_dracomon_costs_2_and_fires_the_draw_clause() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "EXAMON-TEXT-L3"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "DRACO-L3", Some(0));
    let memory_before = runner.memory();
    let slot = hand_index(&runner, 0, CARD_ID) as u16;
    runner.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
    assert_eq!(memory_before - runner.memory(), 2, "Lv.3 w/[Dracomon] in name: Cost 2");
    assert_eq!(top_id(&runner, base), CARD_ID);
    pick_hand(&mut runner, "EXAMON-TEXT-L3");
    let _ = runner.auto_resolve();
    assert!(trash_ids(&runner, 0).contains(&"EXAMON-TEXT-L3".to_string()));
    assert_eq!(
        hand_ids(&runner, 0).iter().filter(|h| *h == "FILL").count(),
        1 + 2,
        "digivolution draw (rules) + <Draw 2>"
    );
}

#[test]
fn ex13_018_digivolving_from_a_plain_red_lv3_costs_3() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "RED-L3", Some(0));
    let memory_before = runner.memory();
    let slot = hand_index(&runner, 0, CARD_ID) as u16;
    runner.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
    assert_eq!(memory_before - runner.memory(), 3, "Red Lv.3 circle: cost 3");
}

// ─── Section 2/3 — [Your Turn] ally-played digivolve observer ────────────────

fn observer_setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let core = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, core)
}

#[test]
fn ex13_018_ally_with_dracomon_text_played_lets_coredramon_digivolve_for_2_less() {
    let (mut runner, core) = observer_setup(&["DRACO-L3", "EXAMON-EVO"]);
    let memory_before = runner.memory();
    let slot = hand_index(&runner, 0, "DRACO-L3");
    runner.play(0, slot).expect("ally plays");
    pick_hand(&mut runner, "EXAMON-EVO");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, core), "EXAMON-EVO", "Coredramon digivolved");
    assert_eq!(
        memory_before - runner.memory(),
        3 + 2,
        "ally play cost 3 + digivolve cost 4 reduced by 2"
    );
}

#[test]
fn ex13_018_observer_only_offers_examon_text_digimon() {
    let (mut runner, _core) = observer_setup(&["DRACO-L3", "EXAMON-EVO", "PLAIN-EVO"]);
    let slot = hand_index(&runner, 0, "DRACO-L3");
    runner.play(0, slot).expect("ally plays");
    assert!(!hand_pick_offered(&mut runner, "PLAIN-EVO"), "no [Examon] in its text");
}

#[test]
fn ex13_018_observer_digivolve_is_optional() {
    let (mut runner, core) = observer_setup(&["DRACO-L3", "EXAMON-EVO"]);
    let slot = hand_index(&runner, 0, "DRACO-L3");
    runner.play(0, slot).expect("ally plays");
    let view = runner.pending_selection_view().expect("may-digivolve prompt");
    assert!(view.is_optional);
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, core), CARD_ID);
    assert!(hand_ids(&runner, 0).contains(&"EXAMON-EVO".to_string()));
}

#[test]
fn ex13_018_ally_without_dracomon_or_examon_text_does_not_trigger() {
    let (mut runner, core) = observer_setup(&["PLAIN-L3", "EXAMON-EVO"]);
    let slot = hand_index(&runner, 0, "PLAIN-L3");
    runner.play(0, slot).expect("plain ally plays");
    assert!(runner.pending_selection().is_none());
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, core), CARD_ID);
}

#[test]
fn ex13_018_observer_does_not_fire_on_the_opponents_turn() {
    let (mut runner, core) = observer_setup(&["EXAMON-EVO"]);
    runner.game.players[1].deck.clear();
    runner.end_turn();
    let _ = runner.auto_resolve();
    // It is now player 1's turn: an own Dracomon-text Digimon entering play
    // (e.g. via an effect) must not trigger the [Your Turn] observer.
    let ally = runner.place_on_field(0, "DRACO-L3", None);
    runner.fire_play_event_triggers(0, ally.index as usize, true, false);
    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] gate: no prompt on the opponent's turn"
    );
    assert_eq!(top_id(&runner, core), CARD_ID);
}

// ─── Section 3 — inherited [Your Turn] +2000 DP ──────────────────────────────

#[test]
fn ex13_018_inherited_gives_plus_2000_dp_on_your_turn_only() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-L5"]);
    assert_eq!(runner.effective_dp(carrier), Some(2000 + 2000), "base 2000 + inherited 2000");
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.effective_dp(carrier), Some(2000), "no bonus on the opponent's turn");
}
