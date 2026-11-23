//! BT22-096 Unique Emblem: Poseidia Lagoon — Option, Blue, Cost 3, LIBERATOR.
//!
//! Official text (data/card_bundles/BT22-096.md):
//!   [Main] You may play 1 [Sangomon] or [Yao Qinglan] from your hand or trash
//!   without paying the cost. Then, place this card in the battle area.
//!   [Your Turn] When any of your [Yao Qinglan] suspend, <Delay>
//!   ・1 of your Digimon with [Aqua] or [Sea Animal] in any of its traits may
//!     digivolve into an [Aquatic] and [LIBERATOR] trait Digimon card in the
//!     hand with the digivolution cost reduced by 3.
//!   [Security] Activate this card's [Main] effects.
//!
//! DCGO: BT22/Blue/BT22_096.cs.
//!
//! Regression origin: a real DCGO PvP game replayed through the engine. The card
//! had NO YAML, so it resolved as an effect-less Option (pay the cost, go to the
//! trash): no free play, no Delay placement, memory driven negative and the
//! owner's turn ended early. These tests use synthetic boards only.
#![allow(dead_code, unused_imports)]

use std::path::Path;

use digimon_dsl::compiled::{
    CompiledClause, CompiledCostDelta, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{self, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT22-096";

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt22/BT22-096.yaml"))
        .expect("BT22-096 YAML must exist at cards/bt22/BT22-096.yaml")
}

fn blue_digimon(id: &str, name: &str, level: u8, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A blue Lv.4 with a 5-memory digivolution cost from a blue Lv.3.
fn evo(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = blue_digimon(id, name, 4, 6, traits);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Blue as u8,
        level: 3,
        memory_cost: 5,
    }];
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT22-096 YAML parses and compiles")
        .add_card(filler("FILL"))
        // [Main] targets
        .add_card(blue_digimon("SANGO", "Sangomon", 3, 3, &["Mollusk"]))
        .add_card(tamer("YAO", "Yao Qinglan"))
        .add_card(tamer("YAO-X", "Yao Qinglan X"))
        .add_card(blue_digimon("OTHER-HAND", "Not A Target", 3, 3, &[]))
        // Delay bases
        .add_card(blue_digimon("SEA", "Sea Base", 3, 3, &["Sea Animal"]))
        .add_card(blue_digimon("AQUA", "Aqua Base", 3, 3, &["Aqua"]))
        .add_card(blue_digimon("BEAST", "Beast Base", 3, 3, &["Beast"]))
        // Delay evolution candidates (Lv.4, evo cost 5)
        .add_card(evo("EVO-OK", "Aquatic Liberator", &["Aquatic", "LIBERATOR"]))
        .add_card(evo("EVO-NOLIB", "Aquatic Only", &["Aquatic"]))
        .add_card(evo("EVO-NOAQ", "Liberator Only", &["LIBERATOR"]))
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
}

fn data_idx(r: &DebugRunner, card_id: &str) -> usize {
    r.game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"))
}

fn push_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .hand
        .push(CardSource::new(idx, p, next));
}

fn push_trash(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = data_idx(r, card_id);
    let next = r.game.next_card_index();
    r.game.players[p as usize]
        .trash
        .push(CardSource::new(idx, p, next));
}

fn hand_index(r: &DebugRunner, p: u8, card_id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"))
}

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn find_perm(r: &DebugRunner, p: u8, card_id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not on field"));
    PermanentHandle {
        player: p,
        index: index as u8,
    }
}

fn emblem_state(r: &DebugRunner, p: u8) -> Option<OptionState> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&r.game.card_data) == CARD_ID)
        .map(|perm| perm.option_state)
}

fn is_parked_on_suspend(r: &DebugRunner, p: u8) -> bool {
    matches!(
        emblem_state(r, p),
        Some(OptionState::Delayed {
            trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
            ..
        })
    )
}

/// The (is_trash, card id) rows the union pick offers, sorted.
fn union_offered(r: &DebugRunner, owner: u8) -> Vec<(bool, String)> {
    use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    let Some(choice) = pending.card_choice.as_ref() else {
        return Vec::new();
    };
    let mut rows: Vec<(bool, String)> = choice
        .view(&r.game, owner)
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .filter_map(|row| {
            let trash = matches!(row.zone, Some(CardChoiceZone::Trash));
            row.card_id.clone().map(|id| (trash, id))
        })
        .collect();
    rows.sort();
    rows
}

fn pick_union(r: &mut DebugRunner, p: u8, card_id: &str, trash: bool) {
    let action =
        owned_union_fixture::action(&r.game, p, card_id, trash).expect("eligible physical row");
    r.execute_action(p, action).unwrap();
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .map(|v| {
            v.valid_action_ids
                .into_iter()
                .filter(|&a| a != PASS)
                .collect()
        })
        .unwrap_or_default()
}

fn pick_own_field(r: &mut DebugRunner, selector: u8, h: PermanentHandle) {
    let action = space::encode_attack(0, h.index as u16);
    let v = r.pending_selection_view().expect("field selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{h:?} not a legal pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(selector, action).unwrap();
}

fn pick_hand(r: &mut DebugRunner, p: u8, card_id: &str) {
    let idx = hand_index(r, p, card_id);
    let action = space::PLAY_HAND_START + idx as u16;
    let v = r.pending_selection_view().expect("hand selection pending");
    assert!(
        v.valid_action_ids.contains(&action),
        "{card_id} not a legal hand pick (valid {:?})",
        v.valid_action_ids
    );
    r.execute_action(p, action).unwrap();
}

fn hand_pickable(r: &DebugRunner, p: u8, card_id: &str) -> bool {
    let Some(idx) = r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
    else {
        return false;
    };
    non_pass(r).contains(&(space::PLAY_HAND_START + idx as u16))
}

/// Player 0 uses BT22-096's [Main] with a blue Lv.3 `BASE_ID` on the field.
/// Returns after the [Main] has been parked (the union pick is pending).
fn start_main(r: &mut DebugRunner) -> OptionPlayResult {
    r.place_on_field(0, "SEA", Some(0));
    r.game.enter_main_phase();
    let idx = hand_index(r, 0, CARD_ID);
    r.game.play_option_from_hand(0, idx)
}

fn advance_to_next_own_main(r: &mut DebugRunner) {
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
}

fn accept_delay(r: &mut DebugRunner) {
    let v = r
        .pending_selection_view()
        .expect("<Delay> activation offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional(), "<Delay> is optional (16-17-2)");
    r.execute_action(v.selecting_player, REPLACEMENT_ACCEPT)
        .unwrap();
}

/// Park BT22-096 (declining the optional [Main] play), advance to the next own
/// main phase with `hand` in hand and 10 memory.
fn parked(hand: &[&str]) -> DebugRunner {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    for id in hand {
        push_hand(&mut r, 0, id);
    }
    let _ = start_main(&mut r);
    if r.pending_selection_view().is_some() {
        r.execute_action(0, PASS).expect("decline the optional play");
    }
    let _ = r.auto_resolve();
    assert!(is_parked_on_suspend(&r, 0), "emblem placed as a Delay");
    advance_to_next_own_main(&mut r);
    r.game.memory = 10;
    r
}

// ─── Structure ──────────────────────────────────────────────────────────────

#[test]
fn bt22_096_structure() {
    let r = builder().build();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.effects.len(), 3, "main, delay, security");
    let delay = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                active_when,
                ..
            }) => Some((trigger, active_when)),
            _ => None,
        })
        .expect("a <Delay> clause");
    assert_eq!(*delay.0, CompiledTiming::OnSuspend);
    let gate = format!("{:?}", delay.1);
    assert!(
        gate.contains("event_card_name_is: Some(\"Yao Qinglan\")"),
        "exact-name gate (not a substring): {gate}"
    );
    assert!(!gate.contains("event_card_name_contains: Some"));
}

// ─── Clause 1: [Main] ───────────────────────────────────────────────────────

#[test]
fn bt22_096_main_offers_exact_names_across_hand_and_trash() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    for id in ["SANGO", "YAO-X", "OTHER-HAND"] {
        push_hand(&mut r, 0, id);
    }
    push_trash(&mut r, 0, "YAO");
    push_trash(&mut r, 0, "BEAST");
    assert_eq!(start_main(&mut r), OptionPlayResult::Pending);
    assert!(r.pending_is_optional(), "'You may play'");
    assert_eq!(
        union_offered(&r, 0),
        vec![
            (false, "SANGO".to_string()),
            (true, "YAO".to_string())
        ],
        "[Sangomon] (hand) and [Yao Qinglan] (trash) only; [Yao Qinglan X] is a different name"
    );
}

#[test]
fn bt22_096_main_plays_trash_yao_free_then_places_as_delay() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "YAO");
    let mem = r.memory();
    assert_eq!(start_main(&mut r), OptionPlayResult::Pending);
    pick_union(&mut r, 0, "YAO", true);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"YAO".to_string()), "played");
    assert!(!trash_ids(&r, 0).contains(&"YAO".to_string()));
    assert_eq!(
        r.memory(),
        mem - 3,
        "only the Option's own cost 3 is paid, the free play adds nothing"
    );
    assert!(is_parked_on_suspend(&r, 0), "placed in the battle area");
    assert!(
        !trash_ids(&r, 0).contains(&CARD_ID.to_string()),
        "not trashed"
    );
}

#[test]
fn bt22_096_main_plays_hand_sangomon_free() {
    let mut r = builder().hand(0, &[CARD_ID, "SANGO"]).memory(10).start();
    r.set_first_player(0);
    assert_eq!(start_main(&mut r), OptionPlayResult::Pending);
    pick_union(&mut r, 0, "SANGO", false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"SANGO".to_string()));
    assert_eq!(r.memory(), 7, "10 - the Option's cost 3");
}

#[test]
fn bt22_096_main_decline_still_places() {
    let mut r = builder().hand(0, &[CARD_ID, "SANGO"]).memory(10).start();
    r.set_first_player(0);
    assert_eq!(start_main(&mut r), OptionPlayResult::Pending);
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"SANGO".to_string()), "kept");
    assert!(is_parked_on_suspend(&r, 0), "still placed");
}

#[test]
fn bt22_096_main_with_nothing_to_play_still_places() {
    let mut r = builder().hand(0, &[CARD_ID, "OTHER-HAND"]).memory(10).start();
    r.set_first_player(0);
    let _ = start_main(&mut r);
    let _ = r.auto_resolve();
    assert!(
        is_parked_on_suspend(&r, 0),
        "DCGO places the card outside the can-select guard"
    );
}

#[test]
fn bt22_096_cost_does_not_end_the_turn_when_memory_stays_non_negative() {
    // Regression: with no YAML the Option was effect-less and (with a Lv.3 free
    // play skipped) the owner's memory/turn handling diverged from DCGO.
    let mut r = builder().hand(0, &[CARD_ID, "SANGO"]).memory(3).start();
    r.set_first_player(0);
    assert_eq!(start_main(&mut r), OptionPlayResult::Pending);
    pick_union(&mut r, 0, "SANGO", false);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 0);
    assert_eq!(r.game.turn_player(), 0, "memory 0 keeps the turn");
}

// ─── Clause 2: [Your Turn] [Yao Qinglan] suspends -> <Delay> ─────────────────

#[test]
fn bt22_096_delay_digivolves_aqua_or_sea_animal_into_aquatic_liberator_for_3_less() {
    let mut r = parked(&["EVO-OK", "EVO-NOLIB", "EVO-NOAQ"]);
    let yao = r.place_on_field(0, "YAO", Some(1));
    r.place_on_field(0, "AQUA", Some(2));
    r.place_on_field(0, "BEAST", Some(3));
    r.game.suspend(yao);
    accept_delay(&mut r);
    // Accepting trashes the emblem (the Delay cost), shifting later permanents'
    // slots: resolve every handle AFTER the accept.
    let yao = find_perm(&r, 0, "YAO");
    let sea = find_perm(&r, 0, "SEA");
    let aqua = find_perm(&r, 0, "AQUA");
    let beast = find_perm(&r, 0, "BEAST");
    // Base pick: [Sea Animal] and [Aqua] Digimon only.
    let v = r.pending_selection_view().expect("base pick");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(r.pending_is_optional(), "'may digivolve'");
    let legal = non_pass(&r);
    assert!(legal.contains(&space::encode_attack(0, sea.index as u16)));
    assert!(legal.contains(&space::encode_attack(0, aqua.index as u16)));
    assert!(!legal.contains(&space::encode_attack(0, beast.index as u16)));
    assert!(!legal.contains(&space::encode_attack(0, yao.index as u16)));
    pick_own_field(&mut r, 0, sea);
    // Hand pick: [Aquatic] AND [LIBERATOR] only.
    assert!(hand_pickable(&r, 0, "EVO-OK"));
    assert!(!hand_pickable(&r, 0, "EVO-NOLIB"), "needs [LIBERATOR]");
    assert!(!hand_pickable(&r, 0, "EVO-NOAQ"), "needs [Aquatic]");
    let mem = r.memory();
    pick_hand(&mut r, 0, "EVO-OK");
    let _ = r.auto_resolve();
    assert_eq!(
        field_ids(&r, 0)[sea.index as usize],
        "EVO-OK",
        "digivolved"
    );
    assert_eq!(r.memory(), mem - 2, "cost 5 reduced by 3");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()), "Delay cost");
    assert!(emblem_state(&r, 0).is_none());
}

#[test]
fn bt22_096_delay_base_needs_a_legal_aquatic_liberator_card() {
    // Only an [Aquatic]-without-[LIBERATOR] card in hand: no Digimon can
    // digivolve, so no base is offered (the Delay still costs the emblem).
    let mut r = parked(&["EVO-NOLIB"]);
    push_trash(&mut r, 0, "YAO");
    let yao = r.place_on_field(0, "YAO", Some(1));
    r.place_on_field(0, "AQUA", Some(2));
    r.game.suspend(yao);
    accept_delay(&mut r);
    assert!(
        r.pending_selection_view().is_none(),
        "no legal digivolution target"
    );
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt22_096_delay_is_optional_to_activate() {
    let mut r = parked(&["EVO-OK"]);
    push_trash(&mut r, 0, "YAO");
    let yao = r.place_on_field(0, "YAO", Some(1));
    r.game.suspend(yao);
    let v = r.pending_selection_view().expect("offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    r.execute_action(v.selecting_player, PASS)
        .expect("decline <Delay>");
    let _ = r.auto_resolve();
    assert!(is_parked_on_suspend(&r, 0), "stays parked after a decline");
}

#[test]
fn bt22_096_delay_ignores_other_names_and_the_opponent() {
    let mut r = parked(&["EVO-OK"]);
    push_trash(&mut r, 0, "YAO-X");
    let yaox = r.place_on_field(0, "YAO-X", Some(1));
    let sea = find_perm(&r, 0, "SEA");
    r.game.suspend(sea);
    r.game.suspend(yaox);
    assert!(
        r.pending_selection_view().is_none(),
        "neither a non-[Yao Qinglan] nor [Yao Qinglan X] opens the <Delay>"
    );
    assert!(is_parked_on_suspend(&r, 0));
}

#[test]
fn bt22_096_delay_does_not_fire_on_the_placing_turn() {
    let mut r = builder().hand(0, &[CARD_ID, "EVO-OK"]).memory(10).start();
    r.set_first_player(0);
    let _ = start_main(&mut r);
    if r.pending_selection_view().is_some() {
        r.execute_action(0, PASS).unwrap();
    }
    let _ = r.auto_resolve();
    let yao = r.place_on_field(0, "YAO", Some(1));
    r.game.suspend(yao);
    assert!(
        r.pending_selection_view().is_none(),
        "<Delay> is only usable after the placing turn (16-17-3)"
    );
    assert!(is_parked_on_suspend(&r, 0));
}

// ─── Clause 3: [Security] ───────────────────────────────────────────────────

#[test]
fn bt22_096_security_activates_main_then_places() {
    let mut r = builder()
        .security(1, &[CARD_ID])
        .hand(1, &["SANGO"])
        .memory(0)
        .start();
    let attacker = r.place_on_field(0, "SEA", Some(0));
    let _ = r.attack_player(attacker, 1, false);
    assert!(r.pending_is_optional(), "'You may play'");
    pick_union(&mut r, 1, "SANGO", false);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"SANGO".to_string()));
    assert!(is_parked_on_suspend(&r, 1), "placed by the security effect");
    assert_eq!(r.security_count(1), 0);
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;

// ─── Play-legality of the union pick ────────────────────────────────────────
//
// While "can't play Digimon by effects" holds (Pomumon (BT9-047), Crimson Blaze
// (BT8-097)), the play of a [Sangomon] would be refused, so the pick must not
// offer it (bug scan 2026-11: the union pick fed `play_union_bound_free` with no
// play-legality leaf). The leaf is added by the DSL lowering
// (`digimon_dsl::play_candidates`). A Tamer stays playable under a Digimon-only
// lock; "Then, place this card" resolves either way.

fn lock_effect_digimon_plays(r: &mut DebugRunner, player: u8) {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    r.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1 - player,
        ),
    );
}

#[test]
fn bt22_096_main_does_not_offer_a_digimon_whose_play_by_effect_is_prohibited() {
    let mut r = builder().hand(0, &[CARD_ID, "SANGO"]).memory(10).start();
    r.set_first_player(0);
    push_trash(&mut r, 0, "YAO");
    lock_effect_digimon_plays(&mut r, 0);
    assert_eq!(start_main(&mut r), OptionPlayResult::Pending);
    assert_eq!(
        union_offered(&r, 0),
        vec![(true, "YAO".to_string())],
        "Sangomon can't be played by effects; the Tamer can"
    );
}

#[test]
fn bt22_096_main_opens_no_pick_when_nothing_is_playable_and_still_places() {
    let mut r = builder().hand(0, &[CARD_ID, "SANGO"]).memory(10).start();
    r.set_first_player(0);
    lock_effect_digimon_plays(&mut r, 0);
    let _ = start_main(&mut r);
    assert!(
        union_offered(&r, 0).is_empty(),
        "no pick for a card the play would refuse"
    );
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"SANGO".to_string()), "kept");
    assert!(is_parked_on_suspend(&r, 0), "\"Then, place this card\" still resolves");
}
