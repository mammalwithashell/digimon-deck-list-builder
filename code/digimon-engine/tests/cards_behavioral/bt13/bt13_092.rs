//! BT13-092 Ravemon: Burst Mode — Digimon, Lv.7, Purple, DP 15000, Cost 15,
//! Cyborg / Mega / Vaccine. (Not BT26-082 Ravemon.)
//!
//! Printed (official DB bundle + card image):
//! `[When Digivolving] Search your opponent's hand, and trash 1 card among it.`
//! `Then, if they have 7 or fewer cards in their hand, they add the top card`
//! `of their security stack to the hand.`
//! `[When Attacking] By returning 1 Digimon card from your opponent's trash to`
//! `the bottom of the deck, delete all of your opponent's Digimon with the`
//! `same name as that card.`
//! `[Burst Digivolve] 0 from [Ravemon] by returning 1 [Keenan Crier] to hand;`
//! `at the end of the burst turn trash the top card.`
//!
//! DCGO BT13_092.cs: WD — the OWNER picks 1 card of the enemy hand to discard
//! (mandatory while non-empty), then if the enemy hand is <= 7 and it has
//! security, the enemy adds its top security card to hand. WA — optional,
//! needs a Digimon card in the enemy trash; the owner picks up to 1 (may
//! decline), returns it to the deck bottom, then destroys every enemy Digimon
//! whose TOP card `HasSameCardName` as the returned card.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD: &str = "BT13-092";

fn digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c.level = Some(4);
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.dp = None;
    c.level = None;
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .expect("BT13-092 loads from the embedded DSL pack")
        .add_card(digimon("TWIN-A", "Twinmon", 3000))
        .add_card(digimon("TWIN-B", "Twinmon", 4000))
        .add_card(digimon("TWIN-TRASH", "Twinmon", 5000))
        .add_card(digimon("OTHER", "Othermon", 3000))
        .add_card(digimon("OTHER-TRASH", "Othermon", 3000))
        .add_card(digimon("BASE", "Basemon", 2000))
        .add_card(tamer("TRASH-TAMER", "Twinmon Tamer"))
        .add_card(make_test_card("OPP-HAND", "OppHandCard"))
        .add_card(make_test_card("OPP-PICK", "OppPickCard"))
        .add_card(make_test_card("OPP-DECK", "OppDeckCard"))
        .add_card(tamer("SEC-FILLER", "SecurityFiller"))
        .add_card(tamer("SEC-TOP", "SecurityTop"))
        .deck(0, &["OPP-DECK"; 5])
        .deck(1, &["OPP-DECK"; 5])
        .security(0, &["SEC-FILLER"; 5])
        .security(1, &["SEC-FILLER", "SEC-FILLER", "SEC-FILLER", "SEC-FILLER", "SEC-TOP"])
        .memory(3)
        .build();
    r.game.turn_count = 1;
    r
}

fn card(r: &mut DebugRunner, player: u8, card_id: &str) -> CardSource {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be registered"));
    let idx = r.game.next_card_index();
    CardSource::new(data_idx, player, idx)
}

fn push_to_hand(r: &mut DebugRunner, player: u8, card_id: &str) {
    let c = card(r, player, card_id);
    r.game.players[player as usize].hand.push(c);
}

fn push_to_trash(r: &mut DebugRunner, player: u8, card_id: &str) {
    let c = card(r, player, card_id);
    r.game.players[player as usize].trash.push(c);
}

fn ids(r: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| r.game.card_data[c.data_index].card_id.clone())
        .collect()
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| r.game.card_data[p.top_card().data_index].card_id.clone())
        .collect()
}

fn top_security_id(r: &DebugRunner) -> String {
    let top = r.game.players[1].security.last().expect("opponent security");
    r.game.card_data[top.data_index].card_id.clone()
}

/// Opponent hand: `fillers` OPP-HAND cards then one OPP-PICK at the end.
fn give_opp_hand(r: &mut DebugRunner, fillers: usize, with_pick: bool) {
    for _ in 0..fillers {
        push_to_hand(r, 1, "OPP-HAND");
    }
    if with_pick {
        push_to_hand(r, 1, "OPP-PICK");
    }
}

fn fire_when_digivolving(r: &mut DebugRunner) {
    let carrier = r.place_on_field(0, CARD, None);
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(carrier));
    r.game.drain_effect_queue();
}

/// Pick OPP-PICK from the opponent's hand in the mandatory search prompt.
fn pick_opp_card(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("hand search prompt");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert_eq!(v.selecting_player, 0, "the Ravemon player chooses");
    assert_eq!(v.zone_owner, Some(1), "from the opponent's hand");
    assert!(!v.valid_action_ids.contains(&PASS), "mandatory: trash 1");
    let n = r.game.players[1].hand.len();
    assert_eq!(v.valid_action_ids.len(), n, "every opponent hand card is selectable");
    let pick_index = r.game.players[1]
        .hand
        .iter()
        .position(|c| r.game.card_data[c.data_index].card_id == "OPP-PICK")
        .expect("OPP-PICK in hand");
    r.execute_action(0, PLAY_HAND_START + pick_index as u16).unwrap();
}

// ─── [When Digivolving] ─────────────────────────────────────────────────────

/// 8 cards: the chosen card is trashed (player choice), leaving 7 -> the
/// opponent adds its top security card to hand.
#[test]
fn bt13_092_when_digivolving_trash_then_seven_adds_top_security() {
    let mut r = runner();
    give_opp_hand(&mut r, 7, true);
    let sec_top = top_security_id(&r);
    fire_when_digivolving(&mut r);
    pick_opp_card(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), vec!["OPP-PICK".to_string()]);
    assert_eq!(r.game.players[1].security.len(), 4, "top security -> hand");
    assert_eq!(r.game.players[1].hand.len(), 8, "7 + the security card");
    assert!(ids(&r, &r.game.players[1].hand).contains(&sec_top));
}

/// 9 cards: after trashing, 8 remain (> 7) -> no security is added.
#[test]
fn bt13_092_when_digivolving_eight_after_trash_adds_nothing() {
    let mut r = runner();
    give_opp_hand(&mut r, 8, true);
    fire_when_digivolving(&mut r);
    pick_opp_card(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), vec!["OPP-PICK".to_string()]);
    assert_eq!(r.game.players[1].hand.len(), 8);
    assert_eq!(r.game.players[1].security.len(), 5, "8 cards: no security added");
}

/// Empty opponent hand: no prompt; 0 <= 7 -> the opponent still adds its top
/// security card (DCGO runs the second half independently).
#[test]
fn bt13_092_when_digivolving_empty_hand_still_adds_security() {
    let mut r = runner();
    fire_when_digivolving(&mut r);
    assert!(r.pending_selection_view().is_none(), "nothing to search");
    assert_eq!(r.game.players[1].hand.len(), 1);
    assert_eq!(r.game.players[1].security.len(), 4);
}

/// Hand <= 7 with an empty security stack: nothing to add, no failure.
#[test]
fn bt13_092_when_digivolving_no_security_is_a_no_op() {
    let mut r = runner();
    r.game.players[1].security.clear();
    give_opp_hand(&mut r, 2, true);
    fire_when_digivolving(&mut r);
    pick_opp_card(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].hand.len(), 2);
    assert_eq!(ids(&r, &r.game.players[1].trash), vec!["OPP-PICK".to_string()]);
}

// ─── [When Attacking] ───────────────────────────────────────────────────────

/// Attack the opponent player with BT13-092 (security is Tamer fillers, so the
/// check starts no battle).
fn attack(r: &mut DebugRunner) -> PermanentHandle {
    let attacker = r.place_on_field(0, CARD, Some(0));
    r.attack_player(attacker, 1, false);
    attacker
}

/// The return pick over the opponent's trash, reached after any activation
/// confirm. Returns the view.
fn trash_pick_view(r: &mut DebugRunner) -> digimon_engine::selection::PendingSelectionView {
    let mut v = r.pending_selection_view().expect("[When Attacking] prompt");
    if v.kind != SelectionKind::Trash {
        assert!(v.is_optional, "the [When Attacking] effect is optional");
        r.accept_optional_trigger().unwrap();
        v = r.pending_selection_view().expect("trash pick");
    }
    assert_eq!(v.kind, SelectionKind::Trash);
    assert_eq!(v.selecting_player, 0);
    assert_eq!(v.zone_owner, Some(1), "from the opponent's trash");
    v
}

fn trash_action(r: &DebugRunner, card_id: &str) -> u16 {
    let i = r.game.players[1]
        .trash
        .iter()
        .position(|c| r.game.card_data[c.data_index].card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in opponent trash"));
    TRASH_EFFECT_START + i as u16
}

/// Returning a [Twinmon] deletes BOTH opponent Digimon named Twinmon (and not
/// the Othermon); the returned card is on the bottom of the opponent's deck.
#[test]
fn bt13_092_when_attacking_deletes_all_with_the_returned_name() {
    let mut r = runner();
    r.place_on_field(1, "TWIN-A", None);
    r.place_on_field(1, "OTHER", None);
    r.place_on_field(1, "TWIN-B", None);
    push_to_trash(&mut r, 1, "TWIN-TRASH");
    push_to_trash(&mut r, 1, "TRASH-TAMER");
    attack(&mut r);
    let v = trash_pick_view(&mut r);
    assert!(v.is_optional, "may decline (PASS)");
    let tamer = trash_action(&r, "TRASH-TAMER");
    assert!(!v.valid_action_ids.contains(&tamer), "only Digimon cards may be returned");
    let a = trash_action(&r, "TWIN-TRASH");
    r.execute_action(0, a).unwrap();
    assert_eq!(field_ids(&r, 1), vec!["OTHER".to_string()], "both Twinmon deleted");
    assert_eq!(
        r.game.card_data[r.game.players[1].deck[0].data_index].card_id,
        "TWIN-TRASH",
        "returned card on the bottom of the opponent's deck"
    );
    let trash = ids(&r, &r.game.players[1].trash);
    assert!(trash.contains(&"TWIN-A".to_string()) && trash.contains(&"TWIN-B".to_string()));
    assert!(!trash.contains(&"TWIN-TRASH".to_string()));
    r.auto_resolve().unwrap();
}

/// The chosen card decides the name: returning [Othermon] deletes only it.
#[test]
fn bt13_092_when_attacking_choice_of_card_decides_the_name() {
    let mut r = runner();
    r.place_on_field(1, "TWIN-A", None);
    r.place_on_field(1, "OTHER", None);
    push_to_trash(&mut r, 1, "TWIN-TRASH");
    push_to_trash(&mut r, 1, "OTHER-TRASH");
    attack(&mut r);
    let v = trash_pick_view(&mut r);
    let a = trash_action(&r, "OTHER-TRASH");
    assert!(v.valid_action_ids.contains(&a));
    assert!(v.valid_action_ids.contains(&trash_action(&r, "TWIN-TRASH")));
    r.execute_action(0, a).unwrap();
    assert_eq!(field_ids(&r, 1), vec!["TWIN-A".to_string()]);
    r.auto_resolve().unwrap();
}

/// The name is the TOP card's: a stack topped by Twinmon is deleted, a
/// Basemon with a Twinmon digivolution card is not.
#[test]
fn bt13_092_when_attacking_matches_the_top_card_name() {
    let mut r = runner();
    r.place_stack(1, &["BASE", "TWIN-A"]);
    r.place_stack(1, &["TWIN-B", "BASE"]);
    push_to_trash(&mut r, 1, "TWIN-TRASH");
    attack(&mut r);
    trash_pick_view(&mut r);
    let a = trash_action(&r, "TWIN-TRASH");
    r.execute_action(0, a).unwrap();
    assert_eq!(field_ids(&r, 1), vec!["BASE".to_string()], "only the Twinmon-topped stack");
    r.auto_resolve().unwrap();
}

/// Declining returns nothing and deletes nothing.
#[test]
fn bt13_092_when_attacking_decline_deletes_nothing() {
    let mut r = runner();
    r.place_on_field(1, "TWIN-A", None);
    push_to_trash(&mut r, 1, "TWIN-TRASH");
    attack(&mut r);
    trash_pick_view(&mut r);
    r.execute_action(0, PASS).unwrap();
    assert!(
        r.pending_selection_view()
            .map_or(true, |v| v.kind != SelectionKind::Trash),
        "no second pick"
    );
    assert_eq!(field_ids(&r, 1), vec!["TWIN-A".to_string()]);
    assert!(
        ids(&r, &r.game.players[1].trash).contains(&"TWIN-TRASH".to_string()),
        "the Digimon card stays in the opponent's trash"
    );
    assert!(!ids(&r, &r.game.players[1].deck).contains(&"TWIN-TRASH".to_string()));
    r.auto_resolve().unwrap();
}

/// No Digimon card in the opponent's trash: the effect does not activate.
#[test]
fn bt13_092_when_attacking_needs_a_digimon_in_opponent_trash() {
    let mut r = runner();
    r.place_on_field(1, "TWIN-A", None);
    push_to_trash(&mut r, 1, "TRASH-TAMER");
    attack(&mut r);
    if let Some(v) = r.pending_selection_view() {
        assert_ne!(v.kind, SelectionKind::Trash, "nothing to return");
        assert!(
            !(v.is_optional && v.selecting_player == 0),
            "no activation prompt without a Digimon card in the opponent's trash"
        );
    }
    r.auto_resolve().unwrap();
    assert_eq!(field_ids(&r, 1), vec!["TWIN-A".to_string()]);
}

// ─────────────────────────────────────────────────────────────────────────────
// [Burst Digivolve] (CR 6-5-1-2-3, 8-3) through the real action surface.
// Shared scenario: `crate::burst_digivolve_common`.
// ─────────────────────────────────────────────────────────────────────────────

mod burst_digivolve {
    use crate::burst_digivolve_common::*;
    use digimon_engine::enums::CardColor;

    const SPEC: BurstSpec = BurstSpec {
        card: "BT13-092",
        host_name: "Ravemon",
        host_color: CardColor::Purple,
        tamer_name: "Keenan Crier",
    };

    /// 8-3-3: offered via the mask with [Keenan Crier] on the field; returns
    /// the Tamer, pays 0, draws 1, stacks on the [Ravemon].
    #[test]
    fn bt13_092_burst_offered_returns_tamer_costs_zero_draws_one() {
        burst_and_check(SPEC, Host::BurstOnly);
    }

    /// The burst cost can't be paid without a [Keenan Crier].
    #[test]
    fn bt13_092_burst_not_offered_without_tamer() {
        assert_not_offered_without_tamer(SPEC);
    }

    /// 8-3-2-1: the top card is trashed at the end of the burst turn,
    /// tracked by permanent identity (the host shifted slots).
    #[test]
    fn bt13_092_burst_top_card_trashed_at_turn_end() {
        assert_turn_end_trash(SPEC, Host::BurstOnly);
    }

    /// Normal (cost 5) and burst routes both legal: a real player choice.
    #[test]
    fn bt13_092_burst_vs_normal_route_is_a_player_choice() {
        assert_route_choice(SPEC);
    }

    /// Choosing the burst route from the route prompt, then the turn-end trash.
    #[test]
    fn bt13_092_burst_chosen_from_route_prompt_then_trashed() {
        assert_turn_end_trash(SPEC, Host::AlsoNormal);
    }

    /// 8-3-2-2: an effect that digivolves can't burst digivolve.
    #[test]
    fn bt13_092_effect_digivolve_cannot_burst() {
        assert_effect_cannot_burst(SPEC);
    }

    /// Two qualifying Tamers: the player picks which returns.
    #[test]
    fn bt13_092_burst_two_tamers_is_a_player_choice() {
        assert_two_tamers_choice(SPEC);
    }
}
