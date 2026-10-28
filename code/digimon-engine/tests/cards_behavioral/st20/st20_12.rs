//! ST20-12 Sora Takenouchi & Kari Kamiya — Tamer, Red+Yellow, Cost 3,
//! traits: [ADVENTURE].
//!
//! # Card text (official bundle `data/card_bundles/ST20-12.md`)
//!
//! ```text
//! [Start of Your Main Phase] If you have a Digimon with the [ADVENTURE] trait,
//! gain 1 memory.
//! [Your Turn] When any Digimon cards with the [ADVENTURE] trait would be played
//! from your hand, by suspending this Tamer, reduce the play cost by 1.
//! Security Effect: [Security] Play this card without paying the cost.
//! ```
//!
//! Official Q&A (bundle): with two such Tamers "the play cost will be reduced
//! by a total of 2".
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Red/ST20_12.cs` — OnStartMainPhase
//! (non-optional, `HasMatchConditionOwnersPermanent(IsDigimon && TopCard
//! HasAdventureTraits)`), BeforePayCost (isOptional, `IsOwnerTurn`,
//! `CanActivateSuspendCostEffect`, `PlayCardCondition = IsExistOnHand &&
//! IsDigimon && HasAdventureTraits && Owner == card.Owner`; suspend self then
//! `ChangeCostClass -1`), `PlaySelfTamerSecurityEffect`.
//!
//! ST21-12 (Joe & Mimi) and ST20-13 (Tai & Izzy, cost-reduction clause) print
//! the identical "would be played ... by suspending this Tamer" clause; they
//! reuse the `pub(crate)` suite below with their own card id.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::build_action_mask;
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "ST20-12";

// ─── Shared fixtures (reused by ST21-12 / ST20-13) ──────────────────────────

pub(crate) fn digimon(id: &str, level: u8, cost: u16, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Red];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        vec![]
    };
    c
}

pub(crate) fn tamer(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        vec![]
    };
    c
}

/// Builder with `ids` (DSL cards) + the synthetic pool.
pub(crate) fn builder(ids: &[&str]) -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder();
    let mut seen: Vec<&str> = Vec::new();
    for id in ids {
        if seen.contains(id) {
            continue;
        }
        seen.push(id);
        b = b.dsl_card(id).expect("card YAML in the embedded DSL pack");
    }
    // Lv.4 ADVENTURE that digivolves from a red Lv.3 for 2.
    let mut evo = digimon("ADV-EVO", 4, 6, true);
    evo.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 2 }];
    b.add_card(digimon("ADV-DIGI", 4, 5, true))
        .add_card(digimon("PLAIN-DIGI", 4, 5, false))
        .add_card(digimon("BASE", 3, 3, false))
        .add_card(evo)
        .add_card(tamer("ADV-TAMER", true))
        .add_card(make_test_card("FILL", "Fill"))
}

fn start(ids: &[&str], hand: &[&str]) -> DebugRunner {
    let mut r = builder(ids)
        .hand(0, hand)
        .hand(1, hand)
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();
    r
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn hand_slot(r: &DebugRunner, player: usize, id: &str) -> usize {
    r.game.players[player]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn on_field(r: &DebugRunner, player: usize, id: &str) -> bool {
    r.game.players[player]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

/// Count of optional "use this reducer?" prompts accepted.
fn accept_all(r: &mut DebugRunner) -> usize {
    let mut n = 0;
    while let Some(view) = r.pending_selection_view() {
        assert!(view.is_optional, "the reduction is a 'by suspending' opt-in: {view:?}");
        assert!(
            build_action_mask(&r.game, view.selecting_player)[PASS as usize] > 0.5,
            "decline must be legal in the mask: {view:?}"
        );
        let a = *view
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("an accept action");
        r.execute_action(view.selecting_player, a).expect("accept");
        n += 1;
    }
    n
}

// ─── Shared cost-reduction suite ─────────────────────────────────────────────

/// Accept → cost 5 - 1 = 4, the Tamer suspends, the Digimon is played.
pub(crate) fn reduction_accept_pays_one_less_and_suspends(id: &str) {
    let mut r = start(&[id], &["ADV-DIGI"]);
    let t = r.place_on_field(0, id, Some(0));
    let before = r.memory();
    let _ = r.play(0, 0);
    assert!(r.pending_selection().is_some(), "{id}: optional reduction prompt");
    assert_eq!(accept_all(&mut r), 1);
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 4, "{id}: 5 - 1");
    assert!(suspended(&r, t), "{id}: suspend cost paid");
    assert!(on_field(&r, 0, "ADV-DIGI"));
}

/// Decline (PASS) → full cost, Tamer stays unsuspended.
pub(crate) fn reduction_decline_pays_full(id: &str) {
    let mut r = start(&[id], &["ADV-DIGI"]);
    let t = r.place_on_field(0, id, Some(0));
    let before = r.memory();
    let _ = r.play(0, 0);
    let view = r.pending_selection_view().expect("reduction prompt");
    assert!(view.is_optional);
    r.execute_action(view.selecting_player, PASS).expect("decline");
    assert!(r.pending_selection().is_none());
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 5, "{id}: declined → full cost");
    assert!(!suspended(&r, t));
    assert!(on_field(&r, 0, "ADV-DIGI"));
}

/// Non-ADVENTURE Digimon and ADVENTURE Tamer → no prompt, full cost.
pub(crate) fn reduction_only_for_adventure_digimon(id: &str) {
    for (card, cost) in [("PLAIN-DIGI", 5), ("ADV-TAMER", 3)] {
        let mut r = start(&[id], &[card]);
        let t = r.place_on_field(0, id, Some(0));
        let before = r.memory();
        assert!(r.play(0, 0).is_some(), "{id}/{card}: plays with no prompt");
        assert!(r.pending_selection().is_none());
        let _ = r.auto_resolve();
        assert_eq!(before - r.memory(), cost, "{id}/{card}: full cost");
        assert!(!suspended(&r, t));
    }
}

/// An already-suspended Tamer cannot pay → not offered.
pub(crate) fn reduction_not_offered_when_suspended(id: &str) {
    let mut r = start(&[id], &["ADV-DIGI"]);
    let t = r.place_on_field(0, id, Some(0));
    r.game.players[0].battle_area[t.index as usize].is_suspended = true;
    let before = r.memory();
    assert!(r.play(0, 0).is_some(), "{id}: no prompt for a suspended Tamer");
    assert!(r.pending_selection().is_none());
    assert_eq!(before - r.memory(), 5);
}

/// [Your Turn]: the opponent's ADVENTURE play on its turn is not reduced by
/// our Tamer (also owner-scoped: "your hand").
pub(crate) fn reduction_not_on_opponents_turn(id: &str) {
    let mut r = start(&[id], &["ADV-DIGI"]);
    let t = r.place_on_field(0, id, Some(0));
    r.game.memory = 0;
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    let before = r.memory();
    let slot = hand_slot(&r, 1, "ADV-DIGI");
    assert!(r.play(1, slot).is_some(), "{id}: P1 plays without P0's prompt");
    assert!(r.pending_selection().is_none());
    let _ = r.auto_resolve();
    assert_eq!((r.memory() - before).abs(), 5, "{id}: P1 pays full");
    assert!(!suspended(&r, t));
}

/// Q&A: two such Tamers each reduce → total -2, both suspended. A third,
/// already-suspended Tamer adds nothing.
pub(crate) fn reduction_stacks_across_tamers(a: &str, b: &str) {
    let mut r = start(&[a, b], &["ADV-DIGI"]);
    let ta = r.place_on_field(0, a, Some(0));
    let tb = r.place_on_field(0, b, Some(0));
    let tc = r.place_on_field(0, a, Some(0));
    r.game.players[0].battle_area[tc.index as usize].is_suspended = true;
    let before = r.memory();
    let _ = r.play(0, 0);
    assert_eq!(accept_all(&mut r), 2, "{a}+{b}: one prompt per payable Tamer");
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 3, "{a}+{b}: 5 - 2");
    assert!(suspended(&r, ta) && suspended(&r, tb));
    assert!(on_field(&r, 0, "ADV-DIGI"));
}

/// Taking one reduction and declining the other → -1, only one suspended.
pub(crate) fn reduction_each_tamer_is_its_own_choice(a: &str, b: &str) {
    let mut r = start(&[a, b], &["ADV-DIGI"]);
    let ta = r.place_on_field(0, a, Some(0));
    let tb = r.place_on_field(0, b, Some(0));
    let before = r.memory();
    let _ = r.play(0, 0);
    let view = r.pending_selection_view().expect("first prompt");
    let acc = *view.valid_action_ids.iter().find(|&&x| x != PASS).unwrap();
    r.execute_action(view.selecting_player, acc).unwrap();
    let view = r.pending_selection_view().expect("second Tamer is offered too");
    assert!(view.is_optional);
    r.execute_action(view.selecting_player, PASS).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 4, "{a}+{b}: one accepted → 5 - 1");
    assert_eq!(
        [suspended(&r, ta), suspended(&r, tb)].iter().filter(|s| **s).count(),
        1
    );
}

/// "would be PLAYED": digivolving into an ADVENTURE Digimon from hand is not
/// a play → no prompt, printed digivolve cost.
pub(crate) fn reduction_not_on_digivolve(id: &str) {
    let mut r = start(&[id], &["ADV-EVO"]);
    let t = r.place_on_field(0, id, Some(0));
    let base = r.place_on_field(0, "BASE", Some(0));
    let before = r.memory();
    let slot = hand_slot(&r, 0, "ADV-EVO");
    let mask = build_action_mask(&r.game, 0);
    assert!(mask[encode_digivolve(slot as u16, base.index as u16) as usize] > 0.5);
    let done = r.game.digivolve_from_hand(0, slot, base.index as usize, PlaySource::ByHand);
    assert!(
        r.pending_selection().is_none(),
        "{id}: no reduction prompt on digivolve: {:?}",
        r.pending_selection_view()
    );
    assert!(done, "digivolve resolves");
    assert!(r.pending_selection().is_none(), "{id}: no reduction on digivolve");
    let _ = r.auto_resolve();
    assert_eq!(before - r.memory(), 2, "{id}: printed digivolve cost");
    assert!(!suspended(&r, t));
}

/// `[Security] Play this card without paying the cost.`
pub(crate) fn security_plays_self_free(id: &str) {
    let mut r = builder(&[id])
        .security(0, &[id])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(4)
        .start();
    let attacker = r.place_on_field(1, "PLAIN-DIGI", Some(0));
    r.game.turn_player_idx = 1;
    r.game.turn_count = 3;
    r.game.current_phase = GamePhase::Main;
    let memory = r.memory();
    let _ = r.attack_player(attacker, 0, false);
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 0, id), "{id}: played from security");
    assert_eq!(r.memory(), memory, "{id}: without paying the cost");
}

/// `[Start of Your Main Phase] If you have a Digimon with the [ADVENTURE]
/// trait, gain 1 memory.` Own ADVENTURE Digimon only (not a Tamer, not the
/// opponent's, not a non-ADVENTURE Digimon).
pub(crate) fn start_of_main_gains_one_with_adventure_digimon(id: &str) {
    for (own, opp, expected) in [
        (Some("ADV-DIGI"), None, 1),
        (None, None, 0),
        (Some("PLAIN-DIGI"), None, 0),
        (Some("ADV-TAMER"), None, 0),
        (None, Some("ADV-DIGI"), 0),
    ] {
        let mut r = builder(&[id]).deck(0, &["FILL"; 4]).deck(1, &["FILL"; 4]).memory(0).start();
        r.place_on_field(0, id, Some(0));
        if let Some(c) = own {
            r.place_on_field(0, c, Some(0));
        }
        if let Some(c) = opp {
            r.place_on_field(1, c, Some(0));
        }
        r.game.memory = 0;
        r.game.enter_main_phase();
        let _ = r.auto_resolve();
        assert_eq!(r.memory(), expected, "{id}: own={own:?} opp={opp:?}");
    }
}

pub(crate) fn assert_adventure_tamer_identity(id: &str, colors: [CompiledColor; 2]) {
    let r = builder(&[id]).start();
    let card = r.compiled_card(id).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3));
    for c in colors {
        assert!(card.color.contains(&c), "{id} color {c:?}");
    }
    assert!(card.traits.iter().any(|t| t == "ADVENTURE"));
    let cr = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                optional,
                once_per_turn,
                amount,
                ..
            }) => Some((*optional, *once_per_turn, *amount)),
            _ => None,
        })
        .expect("cost_reduction clause");
    assert_eq!(cr, (true, false, Some(1)), "{id}: optional, no OPT, -1");
}

// ─── ST20-12 tests ───────────────────────────────────────────────────────────

#[test]
fn st20_12_identity_and_clauses() {
    assert_adventure_tamer_identity(CARD_ID, [CompiledColor::Red, CompiledColor::Yellow]);
    let r = builder(&[CARD_ID]).start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let timings: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.when.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        timings,
        vec![
            vec![CompiledTiming::StartOfYourMainPhase],
            vec![CompiledTiming::OnSecurity]
        ]
    );
}

#[test]
fn st20_12_start_of_main_gains_one_with_adventure_digimon() {
    start_of_main_gains_one_with_adventure_digimon(CARD_ID);
}

#[test]
fn st20_12_reduction_accept_pays_one_less_and_suspends() {
    reduction_accept_pays_one_less_and_suspends(CARD_ID);
}

#[test]
fn st20_12_reduction_decline_pays_full() {
    reduction_decline_pays_full(CARD_ID);
}

#[test]
fn st20_12_reduction_only_for_adventure_digimon() {
    reduction_only_for_adventure_digimon(CARD_ID);
}

#[test]
fn st20_12_reduction_not_offered_when_suspended() {
    reduction_not_offered_when_suspended(CARD_ID);
}

#[test]
fn st20_12_reduction_not_on_opponents_turn() {
    reduction_not_on_opponents_turn(CARD_ID);
}

#[test]
fn st20_12_reduction_stacks_with_second_copy_qa() {
    reduction_stacks_across_tamers(CARD_ID, CARD_ID);
}

#[test]
fn st20_12_reduction_each_tamer_is_its_own_choice() {
    reduction_each_tamer_is_its_own_choice(CARD_ID, CARD_ID);
}

#[test]
fn st20_12_reduction_not_on_digivolve() {
    reduction_not_on_digivolve(CARD_ID);
}

#[test]
fn st20_12_security_plays_self_free() {
    security_plays_self_free(CARD_ID);
}
