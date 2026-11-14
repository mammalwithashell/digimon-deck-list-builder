//! BT15-096 Supreme Connection! - Option, Black, Cost 3.
//!
//! Printed text (official Bandai DB bundle `data/card_bundles/BT15-096.md`,
//! card image, and `data/cards.json` all agree):
//! - [Main] Reveal the top 5 cards of your deck. Among them, add 1 card with the
//!   [Machine] or [Cyborg] trait to the hand and trash 1 such card. Return the
//!   rest to the top of the deck. Then, place this card in the battle area.
//! - [Main] <Delay> - You may play 1 level 5 or higher Digimon card with the
//!   [Machine] or [Cyborg] trait from your hand with the play cost reduced by 3.
//! - [Security] Reveal the top 5 cards of your deck. Among them, add 1 card with
//!   the [Machine] or [Cyborg] trait to the hand and trash 1 such card. Return
//!   the rest to the top of the deck. Then, place this card in the battle area.
//!
//! CORRECTION (2026-11-13 select-hand audit): the previous YAML and this file
//! implemented a different text ("reveal top 3", Digimon-only buckets, a
//! Delay that first chose one of your Machine/Cyborg Digimon and capped the
//! hand card's play cost at its play cost, and a Security that only placed the
//! card). None of that is printed. DCGO BT15_096.cs matches the printed text:
//! - [Main] OptionSkill (:14-73): `SimplifiedRevealDeckTopCardsAndSelect(
//!   revealCount: 5, ...)`, add + trash buckets over `CardTraits.Contains(
//!   "Machine" | "Cyborg")` (any card kind), remainder `DeckTop`, then
//!   `PlaceDelayOptionCards`.
//! - [Main] <Delay> OnDeclaration (:76-282): delete self (the Delay cost),
//!   register a -3 play-cost `ChangeCostClass` (`isCheckAvailability: () =>
//!   false`, so availability checks include it, CardSource.cs:780-850), then a
//!   `SelectHandEffect` (canNoSelect: true) over CanSelectCardCondition
//!   (:88-112) = `Level >= 5 && HasLevel && IsDigimon &&
//!   CanPlayAsNewPermanent(cardSource, payCost: true, activateClass) &&
//!   (Machine || Cyborg)` — `payCost: true` refuses a card whose reduced cost
//!   exceeds `MaxMemoryCost` (CardSource.cs:1150-1160). The play is
//!   `PlayPermanentCards(payCost: true)`.
//! - [Security] SecuritySkill (:284-345): the same reveal-5 + place.

use std::fs;
use std::path::PathBuf;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledCostDelta, CompiledDeclarativeClause, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;

fn yaml() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/bt15/BT15-096.yaml");
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("BT15-096 YAML should exist at {}: {err}", path.display()))
}

fn option_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Black];
    card.play_cost = 3;
    card
}

fn digimon(id: &str, traits: &[&str], level: u8, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.colors = vec![CardColor::Black];
    card.traits = traits.iter().map(|s| (*s).to_string()).collect();
    card.play_cost = play_cost;
    card
}

fn tamer(id: &str, traits: &[&str], play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Black];
    card.traits = traits.iter().map(|s| (*s).to_string()).collect();
    card.play_cost = play_cost;
    card
}

fn runner_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT15-096 YAML must parse and compile")
        .add_card(option_card("BT15-096"))
        .add_card(digimon("MACHINE-HAND", &["Machine"], 5, 7))
        .add_card(digimon("CYBORG-HAND", &["Cyborg"], 6, 8))
        .add_card(digimon("MACHINE-LV4", &["Machine"], 4, 5))
        .add_card(digimon("MACHINE-COST13", &["Machine"], 6, 13))
        .add_card(digimon("MACHINE-COST14", &["Machine"], 6, 14))
        .add_card(digimon("PLAIN-DIGIMON", &[], 5, 4))
        .add_card(tamer("MACHINE-TAMER", &["Machine"], 3))
        .add_card(digimon("ATTACKER", &[], 3, 3))
        .add_card(digimon("FILL", &[], 3, 3))
}

fn runner() -> DebugRunner {
    runner_builder().memory(10).start()
}

fn place_bt15_096_as_mature_delay(runner: &mut DebugRunner) {
    let handle = runner.place_on_field(0, "BT15-096", Some(0));
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            // Printed timing is [Main] and DCGO registers this at
            // EffectTiming.OnDeclaration, so the Delay is player-activated,
            // not turn-scheduled. MainPhaseActivated parks indefinitely
            // (trash_on_turn = u16::MAX, see Game::compute_delay_trash_turn)
            // and is offered as a [Main] mask bit.
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: 0,
        };
}

/// Activate the staged `[Main]` `<Delay>` the way a player does (CR 4.3
/// 16-17-1: available while it sits in the battle area; activating it is the
/// 16-17-2 opt-in).
fn activate_staged_delay(runner: &mut DebugRunner) {
    let handle = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT15-096")
        .map(|index| digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: index as u8,
        })
        .expect("BT15-096 waits on the field for its [Main] activation");
    assert!(
        runner.game.activate_delayed_option_main(handle),
        "the [Main] <Delay> activation must be available"
    );
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn on_field(runner: &DebugRunner, p: usize, id: &str) -> bool {
    runner.game.players[p]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

/// Card ids offered by the pending HAND pick (PASS excluded); empty when no
/// hand pick is pending.
fn offered_hand_ids(runner: &DebugRunner) -> Vec<String> {
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if view.kind != digimon_engine::selection::SelectionKind::Hand {
        return Vec::new();
    }
    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
        .collect();
    ids.sort();
    ids
}

fn delayed_on_field(runner: &DebugRunner, p: usize) -> bool {
    runner.game.players[p].battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "BT15-096"
            && matches!(perm.option_state, OptionState::Delayed { .. })
    })
}

fn has_reveal_5_and_buckets(process: &[CompiledStep]) -> bool {
    process
        .iter()
        .any(|step| matches!(step, CompiledStep::RevealTopDeck { count: 5, .. }))
        && process
            .iter()
            .any(|step| matches!(step, CompiledStep::AddToHandFromReveal { .. }))
        && process
            .iter()
            .any(|step| matches!(step, CompiledStep::TrashFromReveal { .. }))
        && process.iter().any(|step| {
            matches!(
                step,
                CompiledStep::PlaceRemainderOnDeck {
                    position: digimon_dsl::compiled::CompiledStackPosition::Top,
                    ..
                }
            )
        })
        && process
            .iter()
            .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption))
}

#[test]
fn bt15_096_yaml_compiles_to_printed_main_delay_and_security_clauses() {
    let runner = runner();
    let card = runner.compiled_card("BT15-096").expect("BT15-096 compiled");

    assert_eq!(card.name, "Supreme Connection!");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.effects.len(), 3);

    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] clause exists");
    assert!(
        has_reveal_5_and_buckets(&main.process),
        "[Main] reveals the top 5, adds 1 + trashes 1, tops the rest, places itself"
    );

    let delay = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("[Delay] clause exists");
    assert_eq!(*delay.0, CompiledTiming::Delayed);
    assert!(
        !delay
            .1
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectOwnPermanent { .. })),
        "the printed Delay chooses no field Digimon"
    );
    assert!(
        delay.1.iter().any(|step| matches!(
            step,
            CompiledStep::PlayFromHand {
                cost_delta: Some(CompiledCostDelta::Reduce(3)),
                ..
            }
        )),
        "[Delay] plays the hand Digimon with the play cost reduced by 3"
    );

    let security = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Security] clause exists");
    assert!(
        has_reveal_5_and_buckets(&security.process),
        "[Security] reveals the top 5, adds 1 + trashes 1, tops the rest, places itself"
    );
}

/// Top 5 (deck top is the END of the list): FILL, CYBORG-HAND, FILL,
/// MACHINE-TAMER, PLAIN-DIGIMON. MACHINE-HAND is 6th and stays unrevealed. A
/// Tamer with the [Machine] trait IS a "card with the [Machine] trait".
fn reveal_deck() -> [&'static str; 7] {
    [
        "FILL",
        "MACHINE-HAND",
        "PLAIN-DIGIMON",
        "MACHINE-TAMER",
        "FILL",
        "CYBORG-HAND",
        "FILL",
    ]
}

fn first_pick_count(runner: &DebugRunner) -> usize {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.iter().filter(|&&id| id != PASS).count())
        .unwrap_or(0)
}

#[test]
fn bt15_096_main_reveals_top_5_and_offers_any_machine_or_cyborg_card() {
    let mut runner = runner_builder()
        .hand(0, &["BT15-096"])
        .deck(0, &reveal_deck())
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        first_pick_count(&runner),
        2,
        "the add bucket offers CYBORG-HAND and the Machine Tamer from the top 5 \
         (the 6th card, MACHINE-HAND, is not revealed)"
    );
    runner.auto_resolve().expect("pick add/trash reveal buckets");

    let hand = zone_ids(&runner.game.player(0).hand, &runner.game.card_data);
    let trash = zone_ids(&runner.game.player(0).trash, &runner.game.card_data);
    let picked: Vec<&String> = hand
        .iter()
        .chain(trash.iter())
        .filter(|id| *id == "CYBORG-HAND" || *id == "MACHINE-TAMER")
        .collect();
    assert_eq!(picked.len(), 2, "one is added, the other trashed: hand={hand:?} trash={trash:?}");
    assert!(
        hand.iter().any(|id| id == "CYBORG-HAND" || id == "MACHINE-TAMER"),
        "one revealed Machine/Cyborg card enters the hand"
    );
    assert!(
        trash.iter().any(|id| id == "CYBORG-HAND" || id == "MACHINE-TAMER"),
        "the other one is trashed"
    );
    let deck = zone_ids(&runner.game.player(0).deck, &runner.game.card_data);
    assert_eq!(deck.len(), 5, "3 revealed cards return to the deck: {deck:?}");
    assert_eq!(deck[1], "MACHINE-HAND", "the unrevealed 6th card is untouched");
    assert!(delayed_on_field(&runner, 0), "then this card is placed in the battle area");
}

#[test]
fn bt15_096_delay_offers_only_level_5_plus_machine_or_cyborg_digimon_and_reduces_cost() {
    let mut runner = runner_builder()
        .hand(0, &["MACHINE-HAND", "CYBORG-HAND", "MACHINE-LV4", "PLAIN-DIGIMON", "MACHINE-TAMER"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_bt15_096_as_mature_delay(&mut runner);

    activate_staged_delay(&mut runner);

    assert_eq!(
        offered_hand_ids(&runner),
        vec!["CYBORG-HAND".to_string(), "MACHINE-HAND".to_string()],
        "level 5+ Digimon with [Machine] or [Cyborg] only"
    );
    assert!(runner.pending_is_optional(), "printed 'You may play'");
    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    let pick = PLAY_HAND_START + hand.iter().position(|id| id == "MACHINE-HAND").unwrap() as u16;
    let memory_before = runner.memory();
    runner.execute_action(0, pick).expect("play MACHINE-HAND");
    let _ = runner.auto_resolve();

    assert!(on_field(&runner, 0, "MACHINE-HAND"));
    assert_eq!(runner.memory(), memory_before - 4, "play cost 7 reduced by 3");
    assert!(
        zone_ids(&runner.game.player(0).trash, &runner.game.card_data)
            .iter()
            .any(|id| id == "BT15-096"),
        "the Delay cost trashed this card"
    );
}

#[test]
fn bt15_096_delay_can_decline_the_play_after_paying_delay_cost() {
    let mut runner = runner_builder()
        .hand(0, &["MACHINE-HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_bt15_096_as_mature_delay(&mut runner);

    activate_staged_delay(&mut runner);
    assert!(runner.pending_is_optional(), "Delay play is printed as may");
    runner.execute_action(0, PASS).expect("decline the play");
    let _ = runner.auto_resolve();

    assert!(!on_field(&runner, 0, "MACHINE-HAND"));
    assert!(!delayed_on_field(&runner, 0));
    assert!(zone_ids(&runner.game.player(0).trash, &runner.game.card_data)
        .iter()
        .any(|id| id == "BT15-096"));
}

// ─── Delay hand-pick legality (DCGO CanPlayAsNewPermanent(payCost: true)).
// CR 4.3 1-3-2: an impossible play is not carried out, so a card the paid play
// would refuse is not a legal pick (CR 4.3 15-10-2-1: as many as possible).

/// "You can't play Digimon by effects": nothing is offered.
#[test]
fn bt15_096_delay_does_not_offer_digimon_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut runner = runner_builder()
        .hand(0, &["MACHINE-HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    place_bt15_096_as_mature_delay(&mut runner);
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );

    activate_staged_delay(&mut runner);

    assert!(offered_hand_ids(&runner).is_empty(), "a prohibited play is not offered");
    let _ = runner.auto_resolve();
    assert!(!on_field(&runner, 0, "MACHINE-HAND"));
}

/// A full battle area once the Delay cost has trashed this card: nothing is
/// offered. (DCGO selects after the Delay deletion, so the freed slot counts;
/// the area is set to the fillers' count to make it full at selection time.)
#[test]
fn bt15_096_delay_does_not_offer_digimon_when_battle_area_full() {
    let mut runner = runner_builder()
        .hand(0, &["MACHINE-HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "FILL", Some(0));
    runner.place_on_field(0, "FILL", Some(0));
    place_bt15_096_as_mature_delay(&mut runner);
    runner.game.rules.field_slots = 2;

    activate_staged_delay(&mut runner);

    assert!(offered_hand_ids(&runner).is_empty(), "no free slot: nothing is offered");
    let _ = runner.auto_resolve();
    assert!(!on_field(&runner, 0, "MACHINE-HAND"));
}

/// A level 5 [Cyborg] DUAL card (shape of Monarchlizamon BT25-057) whose
/// Option use requirement is met is NOT offered: the printed text is "play",
/// and a DUAL is digivolved or used, never played (CR 4.3 4-6-2, 7-1-1).
/// DCGO `CanPlayAsNewPermanent(payCost: true)` requires `isPlayOption ||
/// !IsOption`, and a DUAL's kinds contain Option. The plain Machine Digimon is
/// still offered.
#[test]
fn bt15_096_delay_does_not_offer_dual_card() {
    use digimon_engine::card_data::{DualCardData, DualDigimonFace, DualOptionFace};
    let mut dual = make_test_card("CYBORG-DUAL", "CYBORG-DUAL");
    dual.card_kind = CardKind::Dual;
    dual.level = Some(5);
    dual.dp = Some(8000);
    dual.colors = vec![CardColor::Black];
    dual.traits = vec!["Cyborg".to_string()];
    dual.play_cost = 4;
    dual.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 5,
            dp: 8000,
            colors: vec![CardColor::Black],
            traits: vec!["Cyborg".to_string()],
            evo_costs: vec![],
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost: 4,
            colors: vec![CardColor::Black],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: vec![],
        },
    });
    let mut runner = runner_builder()
        .add_card(dual)
        .hand(0, &["CYBORG-DUAL", "MACHINE-HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    // A black Digimon stays on the field after the Delay cost trashes this
    // card, so the DUAL's Option colour requirement is met (CR 4.3 4-22).
    runner.place_on_field(0, "FILL", Some(0));
    place_bt15_096_as_mature_delay(&mut runner);

    activate_staged_delay(&mut runner);

    assert_eq!(
        offered_hand_ids(&runner),
        vec!["MACHINE-HAND".to_string()],
        "a DUAL card is never played, so it is not a candidate"
    );
}

/// Memory 0: cost 14 - 3 = 11 would take memory to -11, past the -10 floor
/// (DCGO `MaxMemoryCost < cost`), so it is not offered; cost 13 - 3 = 10 is.
/// Also pins that the -3 reduction is credited in the availability check.
#[test]
fn bt15_096_delay_offers_only_affordable_cards_after_the_reduction() {
    let mut runner = runner_builder()
        .hand(0, &["MACHINE-COST13", "MACHINE-COST14"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();
    place_bt15_096_as_mature_delay(&mut runner);

    activate_staged_delay(&mut runner);

    assert_eq!(
        offered_hand_ids(&runner),
        vec!["MACHINE-COST13".to_string()],
        "only the card whose reduced cost is payable is offered"
    );
}

#[test]
fn bt15_096_security_reveals_top_5_then_places_self_in_battle_area() {
    let mut runner = runner_builder()
        .security(1, &["BT15-096"])
        .deck(0, &["FILL"])
        .deck(1, &reveal_deck())
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let _ = runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.security_count(1), 0);
    let hand = zone_ids(&runner.game.player(1).hand, &runner.game.card_data);
    let trash = zone_ids(&runner.game.player(1).trash, &runner.game.card_data);
    assert!(
        hand.iter().any(|id| id == "CYBORG-HAND" || id == "MACHINE-TAMER"),
        "one revealed Machine/Cyborg card enters the defender's hand: {hand:?}"
    );
    assert!(
        trash.iter().any(|id| id == "CYBORG-HAND" || id == "MACHINE-TAMER"),
        "the other one is trashed: {trash:?}"
    );
    assert!(!trash.iter().any(|id| id == "BT15-096"), "this card is not trashed");
    assert!(runner.game.player(1).battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "BT15-096"
            && matches!(
                perm.option_state,
                OptionState::Delayed {
                    trigger: DelayTrigger::MainPhaseActivated,
                    ..
                }
            )
    }));
}
