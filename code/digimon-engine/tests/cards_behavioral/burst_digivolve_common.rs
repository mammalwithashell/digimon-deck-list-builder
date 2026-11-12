//! Shared [Burst Digivolve] scenario for the six printed Burst Mode cards
//! (BT13-020, BT13-033, BT13-060, BT13-092, BT25-104, BT26-050).
//!
//! CR 4.3 (`docs/digimon-rules/cr-4.3.txt`):
//! - 6-5-1-2-3: burst digivolve is the hand-digivolve action, paying its cost.
//! - 8-3-3: declare + reveal, choose the requirement and the base, return the
//!   specified Tamer from the battle area to the hand, pay the cost, place on
//!   top and draw 1.
//! - 8-3-2-1: pending processing trashes the top card of the digivolved
//!   Digimon's stack at the end of that turn.
//! - 8-3-2-2: an effect that digivolves can't burst digivolve.
//!
//! DCGO: `SelectBurstDigivolutionEffect.cs` (Tamer pick + `BounceTamer` +
//! `AddTrashTopCardAtTurnEnd`) and each card's `AddBurstDigivolutionConditionClass`.
//!
//! Every scenario drives the REAL action surface: the digivolve bit of
//! `build_action_mask`, `decode_action`, and the route `EffectChoice` prompt.

#![allow(dead_code)]

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, PlaySource};
use digimon_engine::permanent::PermanentHandle;

/// One printed Burst Mode card.
#[derive(Clone, Copy)]
pub struct BurstSpec {
    pub card: &'static str,
    /// "[Burst Digivolve] [<host>]".
    pub host_name: &'static str,
    /// A colour of one of the card's printed Lv.6 circles.
    pub host_color: CardColor,
    /// "By returning 1 [<tamer>] to the hand".
    pub tamer_name: &'static str,
}

pub const HOST: &str = "BURST-HOST";
pub const TAMER: &str = "BURST-TAMER";
pub const OTHER_TAMER: &str = "OTHER-TAMER";
pub const STACK_BASE: &str = "STACK-BASE";
pub const STACK_TOP: &str = "STACK-TOP";
pub const FILLER: &str = "BURST-FILLER";

fn digimon(id: &str, name: &str, color: CardColor, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c
}

/// How the burst host relates to the card's OTHER (normal) routes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Host {
    /// A Lv.5 White Digimon with the burst name: it meets no printed circle
    /// and no Lv.6 alt-digivolve, so burst is its only route.
    BurstOnly,
    /// A Lv.6 [DATA SQUAD] Digimon of a printed circle colour with the burst
    /// name: the normal cost-5 route applies as well.
    AlsoNormal,
}

/// Board: player 0 in its main phase with 8 memory (a cost-5 route keeps the turn); `[tamer, host, stack]`
/// in the battle area (the Tamer first, so returning it shifts the host's
/// slot), the Burst Mode card in hand. The 2-card `STACK` Digimon must be
/// untouched by the host's turn-end trash.
pub fn setup(spec: BurstSpec, host: Host, with_tamer: bool) -> (DebugRunner, PermanentHandle, usize) {
    let host_card = match host {
        Host::BurstOnly => digimon(HOST, spec.host_name, CardColor::White, 5, &[]),
        Host::AlsoNormal => digimon(HOST, spec.host_name, spec.host_color, 6, &["DATA SQUAD"]),
    };
    let mut r = DebugRunner::builder()
        .dsl_card(spec.card)
        .unwrap_or_else(|e| panic!("{}: {e}", spec.card))
        .add_card(host_card)
        .add_card(tamer(TAMER, spec.tamer_name))
        .add_card(tamer(OTHER_TAMER, "Some Other Tamer"))
        .add_card(digimon(STACK_BASE, "Stack Base", CardColor::Black, 4, &[]))
        .add_card(digimon(STACK_TOP, "Stack Top", CardColor::Black, 5, &[]))
        .add_card(digimon(FILLER, "Filler", CardColor::Black, 3, &[]))
        .deck(0, &[FILLER; 10])
        .deck(1, &[FILLER; 10])
        .security(0, &[FILLER; 5])
        .security(1, &[FILLER; 5])
        .memory(8)
        .start();
    r.set_first_player(0);
    if with_tamer {
        r.place_on_field(0, TAMER, Some(0));
    } else {
        r.place_on_field(0, OTHER_TAMER, Some(0));
    }
    let host_handle = r.place_on_field(0, HOST, Some(0));
    r.place_stack(0, &[STACK_BASE, STACK_TOP]);
    let hand_index = r.add_to_hand(0, spec.card);
    r.game.enter_main_phase();
    (r, host_handle, hand_index)
}

pub fn digivolve_action(host: PermanentHandle, hand_index: usize) -> u16 {
    encode_digivolve(hand_index as u16, host.index as u16)
}

pub fn offered(r: &DebugRunner, host: PermanentHandle, hand_index: usize) -> bool {
    build_action_mask(&r.game, 0)[digivolve_action(host, hand_index) as usize] == 1.0
}

/// The route prompt's entries, `(label, action_id)`, if one is pending.
pub fn route_choices(r: &DebugRunner) -> Option<Vec<(String, u16)>> {
    let view = r.pending_selection_view()?;
    if view.prompt != "Choose which digivolution cost to pay" {
        return None;
    }
    Some(
        view.effect_choices
            .unwrap_or_default()
            .into_iter()
            .map(|e| (e.label, e.action_id))
            .collect(),
    )
}

pub fn pick_route(r: &mut DebugRunner, burst: bool) {
    let choices = route_choices(r).expect("route prompt");
    let (_, action) = choices
        .iter()
        .find(|(label, _)| label.starts_with("Burst Digivolve") == burst)
        .unwrap_or_else(|| panic!("no {} route in {choices:?}", if burst { "burst" } else { "normal" }))
        .clone();
    r.execute_action(0, action).unwrap();
}

/// Resolve whatever the card's own triggers ask: decline every optional
/// prompt, take the first legal pick of a mandatory one.
pub fn settle(r: &mut DebugRunner) {
    for _ in 0..40 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        let p = view.selecting_player;
        // An optional prompt is declined with PASS (legal in the mask even
        // when `valid_action_ids` lists only the picks).
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .unwrap_or(PASS)
        };
        r.execute_action(p, action).unwrap();
    }
    panic!("selections did not settle");
}

pub fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    let card = r.game.players[h.player as usize].battle_area[h.index as usize].top_card().clone();
    r.game.card_data[card.data_index].card_id.clone()
}

pub fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| r.game.card_data[c.data_index].card_id.clone())
        .collect()
}

pub fn trash_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .trash
        .iter()
        .map(|c| r.game.card_data[c.data_index].card_id.clone())
        .collect()
}

fn slot_of(r: &DebugRunner, top: &str) -> Option<usize> {
    r.game.players[0]
        .battle_area
        .iter()
        .position(|p| r.game.card_data[p.top_card().data_index].card_id == top)
}

/// Perform the burst digivolve through the action surface and assert
/// 8-3-3: the Tamer is back in hand, 0 memory paid, 1 card drawn, the Burst
/// Mode card on top of the host.
pub fn burst_and_check(spec: BurstSpec, host: Host) -> DebugRunner {
    let (mut r, host_h, hand_index) = setup(spec, host, true);
    assert!(offered(&r, host_h, hand_index), "{}: burst digivolve must be offered", spec.card);
    let memory = r.game.memory;
    let hand = r.game.players[0].hand.len();
    let deck = r.game.players[0].deck.len();
    r.game.decode_action(digivolve_action(host_h, hand_index), 0);
    if host == Host::AlsoNormal {
        pick_route(&mut r, true);
    }
    assert!(
        route_choices(&r).is_none(),
        "{}: no further route prompt after the burst choice",
        spec.card
    );
    assert_eq!(r.game.memory, memory, "{}: burst cost 0 is paid", spec.card);
    assert_eq!(r.game.players[0].deck.len(), deck - 1, "{}: draws 1", spec.card);
    // -1 Burst Mode card, +1 returned Tamer, +1 draw.
    assert_eq!(r.game.players[0].hand.len(), hand + 1, "{}: hand", spec.card);
    assert!(hand_ids(&r).iter().any(|id| id == TAMER), "{}: Tamer returned", spec.card);
    assert_eq!(slot_of(&r, TAMER), None, "{}: Tamer left the battle area", spec.card);
    let host_slot = slot_of(&r, spec.card).expect("Burst Mode card on the field");
    assert_eq!(host_slot, 0, "{}: the host shifted into the Tamer's slot", spec.card);
    let stack = &r.game.players[0].battle_area[host_slot];
    assert_eq!(stack.card_sources.len(), 2, "{}: Burst Mode on top of the host", spec.card);
    assert_eq!(r.game.card_data[stack.card_sources[0].data_index].card_id, HOST);
    r
}

/// 8-3-2-1: at the end of the burst turn the host's top card (the Burst Mode
/// card) is trashed; the other 2-card stack is untouched.
pub fn assert_turn_end_trash(spec: BurstSpec, host: Host) {
    let mut r = burst_and_check(spec, host);
    settle(&mut r);
    let host_slot = slot_of(&r, spec.card).expect("still on the field before turn end");
    assert_eq!(top_id(&r, PermanentHandle { player: 0, index: host_slot as u8 }), spec.card);
    let turn = r.game.turn_count;
    r.end_turn();
    settle(&mut r);
    assert!(r.game.turn_count > turn, "{}: the turn ended", spec.card);
    assert_eq!(slot_of(&r, spec.card), None, "{}: Burst Mode card trashed", spec.card);
    assert!(trash_ids(&r).iter().any(|id| id == spec.card), "{}: in trash", spec.card);
    let host_slot = slot_of(&r, HOST).expect("the host remains");
    assert_eq!(r.game.players[0].battle_area[host_slot].card_sources.len(), 1);
    let other = slot_of(&r, STACK_TOP).expect("other stack");
    assert_eq!(
        r.game.players[0].battle_area[other].card_sources.len(),
        2,
        "{}: only the burst Digimon's top card is trashed",
        spec.card
    );
}

/// Without the specified Tamer the burst cost can't be paid: no burst route
/// (and for a burst-only host no digivolve at all).
pub fn assert_not_offered_without_tamer(spec: BurstSpec) {
    let (mut r, host_h, hand_index) = setup(spec, Host::BurstOnly, false);
    assert!(!offered(&r, host_h, hand_index), "{}: no Tamer, no burst", spec.card);
    let hand = r.game.players[0].hand.len();
    r.game.decode_action(digivolve_action(host_h, hand_index), 0);
    assert_eq!(r.game.players[0].hand.len(), hand, "{}: nothing happened", spec.card);
    assert_eq!(top_id(&r, host_h), HOST);
}

/// With a host that also meets a normal (cost 5) requirement the player
/// gets a real choice; the normal route keeps the Tamer and pays 5.
pub fn assert_route_choice(spec: BurstSpec) {
    let (mut r, host_h, hand_index) = setup(spec, Host::AlsoNormal, true);
    let memory = r.game.memory;
    r.game.decode_action(digivolve_action(host_h, hand_index), 0);
    let choices = route_choices(&r).expect("normal + burst routes => route prompt");
    assert_eq!(choices.len(), 2, "{}: {choices:?}", spec.card);
    assert!(choices.iter().any(|(l, _)| l.starts_with("Burst Digivolve for cost 0")));
    assert!(choices.iter().any(|(l, _)| l == "Digivolve for cost 5"));
    pick_route(&mut r, false);
    assert_eq!(r.game.memory, memory - 5, "{}: normal route pays 5", spec.card);
    assert_eq!(slot_of(&r, TAMER), Some(0), "{}: the Tamer stays", spec.card);
    assert_eq!(top_id(&r, host_h), spec.card);
    settle(&mut r);
    // No burst: the card stays at turn end.
    r.end_turn();
    settle(&mut r);
    assert_eq!(slot_of(&r, spec.card), Some(host_h.index as usize));
}

/// 8-3-2-2: an effect that digivolves can't use the burst requirement.
pub fn assert_effect_cannot_burst(spec: BurstSpec) {
    let (mut r, host_h, hand_index) = setup(spec, Host::BurstOnly, true);
    assert!(offered(&r, host_h, hand_index), "control: the action can burst");
    let ok = r.game.effect_initiated_digivolve(
        0,
        hand_index,
        host_h,
        CostDelta::Free,
        false,
        PlaySource::ByEffect,
    );
    assert!(!ok, "{}: effect digivolve must not burst", spec.card);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(top_id(&r, host_h), HOST);
    assert_eq!(slot_of(&r, TAMER), Some(0), "{}: the Tamer stays", spec.card);
}

/// Two qualifying Tamers: each is its own route, so the player chooses which
/// one returns (rule 17, no auto-selection).
pub fn assert_two_tamers_choice(spec: BurstSpec) {
    let (mut r, host_h, hand_index) = setup(spec, Host::BurstOnly, true);
    let second = r.place_on_field(0, TAMER, Some(0));
    r.game.decode_action(digivolve_action(host_h, hand_index), 0);
    let choices = route_choices(&r).expect("two Tamers => route prompt");
    assert_eq!(choices.len(), 2, "{choices:?}");
    assert!(choices.iter().all(|(l, _)| l.starts_with("Burst Digivolve")));
    let pick = choices
        .iter()
        .find(|(l, _)| l.ends_with(&format!("slot {} to the hand)", second.index)))
        .expect("route naming the second Tamer's slot")
        .1;
    r.execute_action(0, pick).unwrap();
    // The first Tamer (slot 0) stays; the second left.
    assert_eq!(
        r.game.card_data[r.game.players[0].battle_area[0].top_card().data_index].card_id,
        TAMER
    );
    assert_eq!(
        r.game.players[0]
            .battle_area
            .iter()
            .filter(|p| r.game.card_data[p.top_card().data_index].card_id == TAMER)
            .count(),
        1
    );
    assert!(hand_ids(&r).iter().any(|id| id == TAMER));
}
