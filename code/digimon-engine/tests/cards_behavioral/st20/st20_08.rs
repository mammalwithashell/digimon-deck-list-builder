//! ST20-08 Kabuterimon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Insectoid / ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (official bundle `data/card_bundles/ST20-08.md`)
//!
//! ```text
//! Digivolve: Green Lv.3 / cost 2.
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2
//!
//! [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer card
//! with the [ADVENTURE] trait from your hand without paying the cost.
//!
//! Inherited: <Piercing>
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Green/ST20_08.cs` — WD optional,
//! CanActivate: ADVENTURE Tamer in hand && own battle-area Tamer count <= 1;
//! `SelectHandEffect(canNoSelect: true, maxCount 1)` → `PlayPermanentCards(
//! payCost: false)`. Inherited `PierceSelfEffect(isInheritedEffect: true)`.
//! Same [When Digivolving] body as ST21-05 Angemon (st21/st21_05.rs).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST20-08";

fn tamer(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn lv3(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut adv_digi = make_test_card("ADV-DIGI", "Adv Digimon");
    adv_digi.traits = vec!["ADVENTURE".to_string()];
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(5);
    carrier.dp = Some(6000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-08 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("ADV-TAMER", true))
        .add_card(tamer("ADV-TAMER-2", true))
        .add_card(tamer("PLAIN-TAMER", false))
        .add_card(lv3("ADV-LV3", CardColor::Purple, &["ADVENTURE"]))
        .add_card(lv3("GREEN-LV3", CardColor::Green, &["Insectoid"]))
        .add_card(lv3("PURPLE-LV3", CardColor::Purple, &["Beast"]))
        .add_card(adv_digi)
        .add_card(carrier)
        .add_card(make_test_card("FILL", "Fill"))
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn hand_actions(runner: &DebugRunner, id: &str) -> [u16; 2] {
    let slot = hand_index(runner, id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
}

fn walk_to_hand_prompt(runner: &mut DebugRunner) -> Option<digimon_engine::selection::PendingSelectionView> {
    for _ in 0..4 {
        let view = runner.pending_selection_view()?;
        if view.kind == SelectionKind::Hand {
            return Some(view);
        }
        let action = view.valid_action_ids.iter().copied().find(|&a| a != PASS)?;
        runner.execute_action(view.selecting_player, action).expect("accept gate");
    }
    None
}

fn on_field(runner: &DebugRunner, id: &str) -> bool {
    runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

fn put_in_hand(runner: &mut DebugRunner, id: &str) -> usize {
    let data_idx = runner.game.card_data.iter().position(|c| c.card_id == id).unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[0].hand.push(CardSource::new(data_idx, 0, card_index));
    runner.game.player(0).hand.len() - 1
}

/// Digivolve Kabuterimon onto an ADVENTURE Lv.3 with `tamers` already in play.
fn digivolve_with(tamers: &[&str], hand: &[&str]) -> DebugRunner {
    let mut all = vec![CARD_ID];
    all.extend_from_slice(hand);
    let mut runner = builder()
        .hand(0, &all)
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    for t in tamers {
        runner.place_on_field(0, t, None);
    }
    let base = runner.place_on_field(0, "ADV-LV3", Some(0));
    let slot = hand_index(&runner, CARD_ID) as u16;
    runner.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
    assert_eq!(
        runner.game.players[base.player as usize].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
    runner
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st20_08_identity_paths_and_clauses() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Kabuterimon");
    assert_eq!((card.level, card.dp, card.cost), (Some(4), Some(4000), Some(4)));
    for t in ["Insectoid", "ADVENTURE"] {
        assert!(card.traits.iter().any(|x| x == t), "missing trait {t}");
    }
    let paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();
    assert_eq!(paths.len(), 2, "green Lv.3 circle + Lv.3 [ADVENTURE] box");
    assert!(paths.iter().all(|p| p.cost == Some(CompiledCost::Literal(2))));
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].when, vec![CompiledTiming::WhenDigivolving], "[When Digivolving] only");
}

/// Digivolve box: a non-green Lv.3 [ADVENTURE] Digimon → Kabuterimon for 2;
/// a non-green non-[ADVENTURE] Lv.3 cannot.
#[test]
fn st20_08_digivolve_box_lv3_adventure_for_cost_2() {
    let mut runner = builder().memory(10).start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "ADV-LV3", Some(0));
    let idx = put_in_hand(&mut runner, CARD_ID);
    let mem = runner.game.memory;
    assert!(runner.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    assert_eq!(runner.game.memory, mem - 2);

    let mut runner = builder().memory(10).start();
    runner.game.turn_count = 1;
    let base = runner.place_on_field(0, "PURPLE-LV3", Some(0));
    let idx = put_in_hand(&mut runner, CARD_ID);
    assert!(
        !runner.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand),
        "purple non-[ADVENTURE] Lv.3 matches neither the circle nor the box"
    );
}

#[test]
fn st20_08_inherited_piercing() {
    let mut runner = builder().start();
    runner.game.turn_count = 1;
    let host = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(runner.game.has_keyword(host, Keyword::Piercing), "carrier inherits <Piercing>");
    let bare = runner.place_on_field(0, "CARRIER", Some(0));
    assert!(!runner.game.has_keyword(bare, Keyword::Piercing));
}

// ─── Section 2 — [When Digivolving] free-play an ADVENTURE Tamer ─────────────

#[test]
fn st20_08_with_no_tamers_plays_an_adventure_tamer_for_free() {
    let mut runner = digivolve_with(&[], &["ADV-TAMER"]);
    let before = runner.memory();
    let view = walk_to_hand_prompt(&mut runner).expect("play prompt");
    assert!(view.is_optional, "'you may'");
    let action = hand_actions(&runner, "ADV-TAMER")
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .expect("ADV-TAMER is offered");
    runner.execute_action(view.selecting_player, action).expect("play tamer");
    let _ = runner.auto_resolve();
    assert!(on_field(&runner, "ADV-TAMER"));
    assert_eq!(before, runner.memory(), "without paying the cost");
}

#[test]
fn st20_08_with_one_tamer_still_fires() {
    let mut runner = digivolve_with(&["PLAIN-TAMER"], &["ADV-TAMER"]);
    let view = walk_to_hand_prompt(&mut runner).expect("play prompt");
    assert!(hand_actions(&runner, "ADV-TAMER").iter().any(|a| view.valid_action_ids.contains(a)));
}

#[test]
fn st20_08_with_two_tamers_does_not_fire() {
    let mut runner = digivolve_with(&["PLAIN-TAMER", "ADV-TAMER-2"], &["ADV-TAMER"]);
    assert!(walk_to_hand_prompt(&mut runner).is_none(), "2 Tamers: condition fails");
    let _ = runner.auto_resolve();
    assert!(!on_field(&runner, "ADV-TAMER"));
}

#[test]
fn st20_08_only_adventure_tamers_are_offered() {
    let mut runner = digivolve_with(&[], &["ADV-TAMER", "PLAIN-TAMER", "ADV-DIGI"]);
    let view = walk_to_hand_prompt(&mut runner).expect("play prompt");
    assert!(!hand_actions(&runner, "PLAIN-TAMER").iter().any(|a| view.valid_action_ids.contains(a)));
    assert!(!hand_actions(&runner, "ADV-DIGI").iter().any(|a| view.valid_action_ids.contains(a)));
}

#[test]
fn st20_08_play_is_optional() {
    let mut runner = digivolve_with(&[], &["ADV-TAMER"]);
    let mut prompted = false;
    for _ in 0..4 {
        let Some(view) = runner.pending_selection_view() else { break };
        prompted = true;
        assert!(view.is_optional);
        runner.execute_action(view.selecting_player, PASS).expect("decline");
    }
    assert!(prompted, "the choice must surface");
    assert!(!on_field(&runner, "ADV-TAMER"));
}

#[test]
fn st20_08_on_play_does_not_fire() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "ADV-TAMER"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    runner.play(0, 0).expect("Kabuterimon plays");
    assert!(runner.pending_selection().is_none(), "no [On Play] effect");
}
