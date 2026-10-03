//! EX13-039 Coredramon — Digimon, Lv.4, Green/Red, DP 5000, Cost 5.
//! Traits: Dragon. Form: Champion. Attribute: Virus.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-039.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-039.md` agrees)
//!
//! ```text
//! Digivolve: Green Lv.3 / cost 3; Red Lv.3 / cost 3.
//! [Digivolve] Lv.3 w/[Dracomon] in name: Cost 2
//!
//! Effect:
//! [On Play] [When Digivolving] You may return 1 non-Digi-Egg card with
//! [Dracomon] or [Examon] in its text from your trash to the hand.
//! [Your Turn] When any of your other Digimon with [Dracomon] or [Examon] in
//! their texts are played, this Digimon may digivolve into a Digimon card
//! with [Examon] in its text in the hand with the cost reduced by 2.
//!
//! Inherited Effect:
//! [Your Turn] This Digimon gets +2000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Green/EX13_039.cs
//! - Alt digivolve: `ContainsCardName("Dracomon")`, level 3, cost 2.
//! - Shared [OP]/[WD]: skippable, gated on an eligible trash card;
//!   `SelectCardEffect(Root.Trash, AddHand, maxCount 1, canNoSelect: true)`,
//!   `!IsDigiEgg && (HasText("Dracomon") || HasText("Examon"))`.
//! - [Your Turn] `OnEnterFieldAnyone` observer — identical to EX13-018.
//! - Inherited `ChangeSelfDPStaticEffect(+2000, IsOwnerTurn)`.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - B trash recursion ("you may return 1 non-Digi-Egg [X]-text card").
//! - F observer: own-ally-played → self may digivolve (cost −2), [Your Turn].
//! - D1 inherited [Your Turn] +2000 DP self-aura.
//! - Named alt digivolve (Lv.3 w/[Dracomon] in name / cost 2).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{
    encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START, TRASH_EFFECT_END, TRASH_EFFECT_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-039";

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
        .expect("EX13-039 YAML parses, compiles and is in the embedded pack")
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
        .add_card({
            let mut egg = card("DRACO-EGG", "Dracomon Egg", 2, "[Dracomon] hatchling.");
            egg.card_kind = CardKind::DigiEgg;
            egg
        })
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
fn ex13_039_has_two_circles_and_the_dracomon_name_alt_digivolve() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 3, "Green Lv.3/3 + Red Lv.3/3 + Lv.3 [Dracomon]-name/2");
}

#[test]
fn ex13_039_clause_shape_matches_printed_text() {
    let runner = builder().start();
    let t = triggered(&runner);
    let face: Vec<_> = t.iter().filter(|c| c.scope == CompiledScope::FaceUp).collect();
    assert_eq!(face.len(), 2, "[OP]/[WD] trash recursion + [Your Turn] digivolve observer");
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

// ─── Section 2/3 — [On Play]/[When Digivolving] trash recursion ─────────────

fn trash_actions(runner: &DebugRunner, id: &str) -> u16 {
    let pos = runner.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in trash"));
    TRASH_EFFECT_START + pos as u16
}

/// Walk past any accept gate to the trash prompt; return its view.
fn to_trash_prompt(runner: &mut DebugRunner) -> Option<digimon_engine::selection::PendingSelectionView> {
    for _ in 0..4 {
        let view = runner.pending_selection_view()?;
        if view.valid_action_ids.iter().any(|&a| a >= TRASH_EFFECT_START && a < TRASH_EFFECT_END) {
            return Some(view);
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
    None
}

fn play_with_trash(trash: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    for id in trash {
        runner.inject_trash(0, id);
    }
    let slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, slot).expect("Coredramon plays");
    runner
}

#[test]
fn ex13_039_on_play_returns_a_text_card_from_trash_to_hand() {
    let mut runner = play_with_trash(&["PLAIN-L3", "DRACO-L3"]);
    let view = to_trash_prompt(&mut runner).expect("trash prompt");
    let action = trash_actions(&runner, "DRACO-L3");
    assert!(view.valid_action_ids.contains(&action));
    runner.execute_action(view.selecting_player, action).expect("pick");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["DRACO-L3"]);
    assert_eq!(trash_ids(&runner, 0), vec!["PLAIN-L3"]);
}

#[test]
fn ex13_039_on_play_accepts_an_examon_text_option_card() {
    let mut runner = play_with_trash(&["OPTION-TEXT"]);
    let view = to_trash_prompt(&mut runner).expect("trash prompt");
    let action = trash_actions(&runner, "OPTION-TEXT");
    assert!(view.valid_action_ids.contains(&action), "non-Digi-Egg includes Options");
    runner.execute_action(view.selecting_player, action).expect("pick");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec!["OPTION-TEXT"]);
}

#[test]
fn ex13_039_on_play_excludes_digi_eggs_and_plain_cards() {
    let mut runner = play_with_trash(&["DRACO-EGG", "PLAIN-L3", "EXAMON-TEXT-L3"]);
    let view = to_trash_prompt(&mut runner).expect("trash prompt");
    let egg = trash_actions(&runner, "DRACO-EGG");
    let plain = trash_actions(&runner, "PLAIN-L3");
    let ok = trash_actions(&runner, "EXAMON-TEXT-L3");
    assert!(!view.valid_action_ids.contains(&egg), "Digi-Egg excluded");
    assert!(!view.valid_action_ids.contains(&plain), "no [Dracomon]/[Examon] text");
    assert!(view.valid_action_ids.contains(&ok));
}

#[test]
fn ex13_039_on_play_return_is_optional() {
    let mut runner = play_with_trash(&["DRACO-L3"]);
    for _ in 0..4 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        assert!(view.is_optional, "\"you may return\"");
        runner.execute_action(view.selecting_player, PASS).expect("decline");
    }
    assert!(hand_ids(&runner, 0).is_empty());
    assert_eq!(trash_ids(&runner, 0), vec!["DRACO-L3"]);
}

#[test]
fn ex13_039_on_play_with_no_eligible_trash_card_installs_nothing() {
    let runner = play_with_trash(&["PLAIN-L3", "DRACO-EGG"]);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn ex13_039_when_digivolving_from_dracomon_costs_2_and_fires_the_recursion() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.inject_trash(0, "EXAMON-TEXT-L3");
    let base = runner.place_on_field(0, "DRACO-L3", Some(0));
    let memory_before = runner.memory();
    let slot = hand_index(&runner, 0, CARD_ID) as u16;
    runner.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
    assert_eq!(memory_before - runner.memory(), 2, "Lv.3 w/[Dracomon] in name: Cost 2");
    let view = to_trash_prompt(&mut runner).expect("[When Digivolving] trash prompt");
    let action = trash_actions(&runner, "EXAMON-TEXT-L3");
    runner.execute_action(view.selecting_player, action).expect("pick");
    let _ = runner.auto_resolve();
    let hand = hand_ids(&runner, 0);
    assert!(hand.contains(&"EXAMON-TEXT-L3".to_string()), "returned from trash");
    assert!(runner.game.players[0].trash.is_empty());
}

#[test]
fn ex13_039_digivolving_from_a_plain_red_lv3_costs_3() {
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
fn ex13_039_ally_with_dracomon_text_played_lets_coredramon_digivolve_for_2_less() {
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
fn ex13_039_observer_only_offers_examon_text_digimon() {
    let (mut runner, _core) = observer_setup(&["DRACO-L3", "EXAMON-EVO", "PLAIN-EVO"]);
    let slot = hand_index(&runner, 0, "DRACO-L3");
    runner.play(0, slot).expect("ally plays");
    assert!(!hand_pick_offered(&mut runner, "PLAIN-EVO"), "no [Examon] in its text");
}

#[test]
fn ex13_039_observer_digivolve_is_optional() {
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
fn ex13_039_ally_without_dracomon_or_examon_text_does_not_trigger() {
    let (mut runner, core) = observer_setup(&["PLAIN-L3", "EXAMON-EVO"]);
    let slot = hand_index(&runner, 0, "PLAIN-L3");
    runner.play(0, slot).expect("plain ally plays");
    assert!(runner.pending_selection().is_none());
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, core), CARD_ID);
}

#[test]
fn ex13_039_observer_does_not_fire_on_the_opponents_turn() {
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
fn ex13_039_inherited_gives_plus_2000_dp_on_your_turn_only() {
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
