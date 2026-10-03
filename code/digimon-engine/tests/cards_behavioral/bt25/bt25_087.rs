//! BT25-087 Thomas H. Norstein — Tamer, Blue, Cost 4. Traits: DATA SQUAD.
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/BT25-087.md)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When effects add cards to your opponent's hand, by suspending
//!   this Tamer, you may place the top 2 cards of your deck face down under
//!   this Tamer.
//! [Your Turn] [Once Per Turn] When any of your Digimon would digivolve into a
//!   [DATA SQUAD] trait Digimon card, by trashing the bottom face-down card from
//!   under any of your Tamers, reduce the cost by 1.
//! [Security] Play this card without paying the cost.
//! Official Q&A: the placed cards go on the BOTTOM of the cards under the Tamer.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_087.cs
//!   - OnStartTurn: SetMemoryTo3TamerEffect.
//!   - OnAddHand: CanTriggerWhenAddHand(player == Enemy, cardEffect != null)
//!     (effect-only gate), CanActivateSuspendCostEffect (must be unsuspended),
//!     no OPT; suspend → AddDigivolutionCardsBottom(top 2, face down).
//!   - BeforePayCost: OPT (hash BT25_087_YT), IsOwnerTurn, any own Digimon
//!     would digivolve into a DATA SQUAD Digimon card; needs a Tamer with a
//!     face-down source; Tamer pick canNoSelect:true → trash its bottom FD → -1.
//!   - SecuritySkill: PlaySelfTamerSecurityEffect.
//!
//! # Patterns this test covers
//! set-memory (positive / boundary / negative), OnAddToHand observer gated on
//! opponent + effect-initiated (bounce = effect; draw-phase draw = not), the
//! optional suspend activation cost (decline, already-suspended), [All Turns]
//! (fires on the opponent's turn), no OPT on clause 2, the interactive
//! digivolve cost reducer over ANY ally Digimon with a pick across ANY Tamer,
//! the OPT lockout, face-up sources ineligible, non-DATA-SQUAD negative, and
//! the [Security] free self-play.

#![allow(dead_code, unused_imports)]

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-087";
const EVO_BLUE: u8 = 1;

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = level as u16;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Lv.5 digivolve TARGET; printed evo cost 4 from a Lv.4 Blue.
fn lv5_target(id: &str, traits: &[&str]) -> CardData {
    let mut c = digimon(id, 5, traits);
    c.play_cost = 7;
    c.evo_costs = vec![EvoCost {
        level: 4,
        card_color: EVO_BLUE,
        memory_cost: 4,
    }];
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Blue];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-087 DSL spec")
        .add_card(digimon("FILLER", 3, &[]))
        .add_card(digimon("DECK-A", 3, &[]))
        .add_card(digimon("OPP-DIGI", 3, &[]))
        .add_card(digimon("BASE4", 4, &[]))
        .add_card(digimon("STASH", 3, &[]))
        .add_card(lv5_target("DS-LV5", &["DATA SQUAD"]))
        .add_card(lv5_target("DS-LV5-B", &["DATA SQUAD"]))
        .add_card(lv5_target("PLAIN-LV5", &["Beast"]))
        .add_card(tamer("TAMER"))
}

fn perm(r: &DebugRunner, h: PermanentHandle) -> &digimon_engine::permanent::Permanent {
    &r.game.players[h.player as usize].battle_area[h.index as usize]
}

fn face_down_sources(r: &DebugRunner, h: PermanentHandle) -> usize {
    perm(r, h).digivolution_cards().iter().filter(|s| s.face_down).count()
}

fn source_count(r: &DebugRunner, h: PermanentHandle) -> usize {
    perm(r, h).card_sources.len() - 1
}

/// An EFFECT returns `h` to its owner's hand (effect controlled by
/// `effect_player`), then the surrounding effect-resolution loop drains the
/// enqueued `OnAddToHand` observer (as it would inside a real card effect).
fn bounce(r: &mut DebugRunner, h: PermanentHandle, effect_player: u8) -> bool {
    let moved = r.game.return_to_hand_from_effect(h, effect_player);
    r.game.drain_effect_queue();
    moved
}

/// Seed a stack `[STASH(face-down), top]` and return it.
fn stack_with_face_down(r: &mut DebugRunner, player: u8, top: &str) -> PermanentHandle {
    let h = r.place_stack(player, &["STASH", top]);
    r.game.players[player as usize].battle_area[h.index as usize].card_sources[0].face_down = true;
    h
}

// ════════════════════════════════════════════════════════════════════════════
// Clause 1 — [Start of Your Turn] memory <= 2 → set to 3
// ════════════════════════════════════════════════════════════════════════════

fn sot_runner(mem: i16) -> DebugRunner {
    let mut runner = builder()
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .memory(mem)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = mem;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner
}

#[test]
fn bt25_087_start_of_turn_sets_one_memory_to_three() {
    assert_eq!(sot_runner(1).memory(), 3, "memory 1 (<=2) set to 3");
}

#[test]
fn bt25_087_start_of_turn_sets_two_memory_to_three_boundary() {
    assert_eq!(sot_runner(2).memory(), 3, "memory 2 (<=2) set to 3");
}

#[test]
fn bt25_087_start_of_turn_leaves_three_memory() {
    assert_eq!(sot_runner(3).memory(), 3, "memory 3 (>2) unchanged");
}

#[test]
fn bt25_087_start_of_turn_leaves_high_memory() {
    assert_eq!(sot_runner(5).memory(), 5, "memory 5 (>2) unchanged");
}

// ════════════════════════════════════════════════════════════════════════════
// Clause 2 — [All Turns] effect adds to opponent's hand → may suspend → place
// top 2 deck cards face down under this Tamer
// ════════════════════════════════════════════════════════════════════════════

/// Thomas on P0's field, P1 has two Digimon to be bounced; P0's deck is
/// [.., DECK-A, DECK-A] (distinct from filler so placement is observable).
fn obs_runner() -> (DebugRunner, PermanentHandle, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .deck(0, &["DECK-A"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    runner.set_first_player(0);
    let thomas = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-DIGI", Some(0));
    (runner, thomas, opp_a, opp_b)
}

#[test]
fn bt25_087_effect_bounce_to_opponent_hand_accept_suspends_and_places_two_face_down() {
    let (mut runner, thomas, opp_a, _) = obs_runner();
    let deck_before = runner.game.players[0].deck.len();
    assert!(bounce(&mut runner, opp_a, 0));
    assert!(
        runner.pending_selection().is_some() && runner.pending_is_optional(),
        "an effect adding to the opponent's hand offers the optional clause"
    );
    runner.accept_optional_trigger().expect("accept");
    let _ = runner.auto_resolve();
    assert!(perm(&runner, thomas).is_suspended, "suspended as the cost");
    assert_eq!(source_count(&runner, thomas), 2, "two cards placed under");
    assert_eq!(face_down_sources(&runner, thomas), 2, "both face down");
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before - 2,
        "taken from the top of the deck"
    );
}

#[test]
fn bt25_087_effect_bounce_decline_does_nothing() {
    let (mut runner, thomas, opp_a, _) = obs_runner();
    let deck_before = runner.game.players[0].deck.len();
    bounce(&mut runner, opp_a, 0);
    assert!(runner.pending_is_optional());
    runner.decline_optional_trigger().expect("decline");
    assert!(runner.pending_selection().is_none());
    assert!(!perm(&runner, thomas).is_suspended, "declined → not suspended");
    assert_eq!(source_count(&runner, thomas), 0);
    assert_eq!(runner.game.players[0].deck.len(), deck_before);
}

#[test]
fn bt25_087_own_hand_gain_by_effect_does_not_trigger() {
    let (mut runner, thomas, _, _) = obs_runner();
    let own = runner.place_on_field(0, "OPP-DIGI", Some(0));
    bounce(&mut runner, own, 0);
    assert!(
        runner.pending_selection().is_none(),
        "own-hand gain must not trigger (opponent's hand only)"
    );
    assert!(!perm(&runner, thomas).is_suspended);
    assert_eq!(source_count(&runner, thomas), 0);
}

#[test]
fn bt25_087_opponent_draw_phase_draw_does_not_trigger() {
    let (mut runner, thomas, _, _) = obs_runner();
    let opp_hand_before = runner.hand_size(1);
    // P0 ends turn → P1's turn starts with its normal (non-effect) draw.
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before + 1,
        "precondition: the opponent drew in its draw phase"
    );
    assert!(runner.pending_selection().is_none(), "rule draw is not an effect");
    assert!(!perm(&runner, thomas).is_suspended);
    assert_eq!(source_count(&runner, thomas), 0, "nothing placed");
}

#[test]
fn bt25_087_already_suspended_cannot_pay_cost() {
    let (mut runner, thomas, opp_a, _) = obs_runner();
    runner.game.players[0].battle_area[thomas.index as usize].is_suspended = true;
    bounce(&mut runner, opp_a, 0);
    let _ = runner.auto_resolve();
    assert_eq!(
        source_count(&runner, thomas),
        0,
        "a suspended Tamer cannot pay the suspend cost → nothing placed"
    );
}

#[test]
fn bt25_087_clause_two_has_no_once_per_turn() {
    let (mut runner, thomas, opp_a, _) = obs_runner();
    bounce(&mut runner, opp_a, 0);
    runner.accept_optional_trigger().expect("accept #1");
    let _ = runner.auto_resolve();
    assert_eq!(source_count(&runner, thomas), 2);
    // Unsuspend and trigger again in the same turn.
    runner.game.players[0].battle_area[thomas.index as usize].is_suspended = false;
    // opp_b shifted down to index 0 after opp_a left.
    let opp_b = PermanentHandle { player: 1, index: 0 };
    bounce(&mut runner, opp_b, 0);
    assert!(
        runner.pending_is_optional(),
        "no OPT on the [All Turns] clause → offered again"
    );
    runner.accept_optional_trigger().expect("accept #2");
    let _ = runner.auto_resolve();
    assert_eq!(source_count(&runner, thomas), 4, "placed two more");
    assert_eq!(face_down_sources(&runner, thomas), 4);
}

#[test]
fn bt25_087_all_turns_fires_on_opponents_turn() {
    let (mut runner, thomas, _, _) = obs_runner();
    // pass_turn (memory → 3 on P1's side), not a raw end_turn (which would
    // leave P1 starting on the opponent's memory side and end its turn at
    // the first effect resolution).
    runner.pass_turn(); // → P1's turn
    let _ = runner.auto_resolve();
    // P1's own effect bounces P1's Digimon → P1 (Thomas's opponent) gains.
    let opp = PermanentHandle { player: 1, index: 0 };
    bounce(&mut runner, opp, 1);
    assert!(
        runner.pending_is_optional(),
        "[All Turns] → offered on the opponent's turn too"
    );
    runner.accept_optional_trigger().expect("accept");
    let _ = runner.auto_resolve();
    assert!(perm(&runner, thomas).is_suspended);
    assert_eq!(face_down_sources(&runner, thomas), 2);
}

// ════════════════════════════════════════════════════════════════════════════
// Clause 3 — [Your Turn][OPT] any ally digivolves into [DATA SQUAD] → by
// trashing the bottom FD card from under ANY of your Tamers → cost -1
// ════════════════════════════════════════════════════════════════════════════

struct DvSetup {
    runner: DebugRunner,
    thomas: PermanentHandle,
    other_tamer: PermanentHandle,
    base: PermanentHandle,
}

/// P0: Thomas (with a FD source iff `thomas_fd`), a vanilla Tamer (with a FD
/// source iff `tamer_fd`), and a Lv.4 BASE4 (NOT Thomas — "any of your
/// Digimon"). Hand: `target`.
fn dv_setup(thomas_fd: bool, tamer_fd: bool, target: &str) -> (DvSetup, usize) {
    let mut runner = builder()
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    runner.set_first_player(0);
    let thomas = if thomas_fd {
        stack_with_face_down(&mut runner, 0, CARD_ID)
    } else {
        runner.place_on_field(0, CARD_ID, Some(0))
    };
    let other_tamer = if tamer_fd {
        stack_with_face_down(&mut runner, 0, "TAMER")
    } else {
        runner.place_on_field(0, "TAMER", Some(0))
    };
    let base = runner.place_on_field(0, "BASE4", Some(0));
    let hand_idx = runner.add_to_hand(0, target);
    (
        DvSetup {
            runner,
            thomas,
            other_tamer,
            base,
        },
        hand_idx,
    )
}

#[test]
fn bt25_087_reducer_any_ally_into_data_squad_picks_any_tamer_and_credits_minus_one() {
    let (mut s, hand_idx) = dv_setup(true, true, "DS-LV5");
    let mem_before = s.runner.memory();
    let trash_before = s.runner.trash_size(0);
    let ok = s
        .runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    assert!(!ok, "digivolve parks on the reducer prompt");
    assert!(s.runner.pending_is_optional(), "the reducer is optional");
    s.runner.accept_optional_trigger().expect("accept reducer");

    // Two Tamers carry a face-down source → the player must choose which.
    let sel = s.runner.pending_selection().expect("Tamer pick is surfaced");
    let n_opts = sel.valid_action_ids.len();
    assert!(
        n_opts >= 2,
        "both Tamers with a face-down source must be selectable (got {n_opts})"
    );
    // Pick the OTHER Tamer (not Thomas): choose the last option.
    let (p, last) = (sel.selecting_player, *sel.valid_action_ids.last().unwrap());
    s.runner.execute_action(p, last).expect("pick a Tamer");
    let _ = s.runner.auto_resolve();
    assert!(s.runner.pending_selection().is_none(), "digivolution done");

    let fd_thomas = face_down_sources(&s.runner, s.thomas);
    let fd_other = face_down_sources(&s.runner, s.other_tamer);
    assert_eq!(fd_thomas + fd_other, 1, "exactly one FD source trashed");
    assert_eq!(s.runner.trash_size(0), trash_before + 1);
    assert_eq!(mem_before - s.runner.memory(), 3, "cost 4 - 1 = 3 paid");
    assert_eq!(
        perm(&s.runner, s.base).top_card().card_id(&s.runner.game.card_data),
        "DS-LV5",
        "the ally digivolved"
    );
}

#[test]
fn bt25_087_reducer_can_trash_from_a_non_thomas_tamer() {
    // Only the vanilla Tamer has a FD source — still payable ("any of your Tamers").
    let (mut s, hand_idx) = dv_setup(false, true, "DS-LV5");
    let mem_before = s.runner.memory();
    s.runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    s.runner.accept_optional_trigger().expect("accept");
    let _ = s.runner.auto_resolve();
    assert_eq!(face_down_sources(&s.runner, s.other_tamer), 0, "trashed from the other Tamer");
    assert_eq!(mem_before - s.runner.memory(), 3);
}

#[test]
fn bt25_087_reducer_decline_pays_full_cost() {
    let (mut s, hand_idx) = dv_setup(true, false, "DS-LV5");
    let mem_before = s.runner.memory();
    s.runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    assert!(s.runner.pending_is_optional());
    s.runner.decline_optional_trigger().expect("decline");
    let _ = s.runner.auto_resolve();
    assert!(s.runner.pending_selection().is_none());
    assert_eq!(face_down_sources(&s.runner, s.thomas), 1, "FD stash untouched");
    assert_eq!(mem_before - s.runner.memory(), 4, "full cost");
}

#[test]
fn bt25_087_reducer_inactive_for_non_data_squad_target() {
    let (mut s, hand_idx) = dv_setup(true, false, "PLAIN-LV5");
    let mem_before = s.runner.memory();
    let ok = s
        .runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    assert!(ok, "no reducer prompt → completes synchronously");
    assert!(s.runner.pending_selection().is_none());
    assert_eq!(face_down_sources(&s.runner, s.thomas), 1);
    assert_eq!(mem_before - s.runner.memory(), 4);
}

#[test]
fn bt25_087_reducer_unpayable_without_face_down_source() {
    let (mut s, hand_idx) = dv_setup(false, false, "DS-LV5");
    let mem_before = s.runner.memory();
    let _ = s
        .runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    let _ = s.runner.auto_resolve();
    assert_eq!(mem_before - s.runner.memory(), 4, "no FD source → full cost");
}

#[test]
fn bt25_087_reducer_face_up_source_is_not_eligible() {
    let (mut s, hand_idx) = dv_setup(false, false, "DS-LV5");
    // A FACE-UP source under the Tamer does not satisfy "face-down card".
    let t = s.runner.place_stack(0, &["STASH", "TAMER"]);
    let mem_before = s.runner.memory();
    let _ = s
        .runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    let _ = s.runner.auto_resolve();
    assert_eq!(source_count(&s.runner, t), 1, "face-up source not trashed");
    assert_eq!(mem_before - s.runner.memory(), 4, "full cost");
}

#[test]
fn bt25_087_reducer_once_per_turn_lockout() {
    let (mut s, hand_idx) = dv_setup(true, true, "DS-LV5");
    let second_base = s.runner.place_on_field(0, "BASE4", Some(0));
    let mem0 = s.runner.memory();
    s.runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    s.runner.accept_optional_trigger().expect("accept #1");
    let _ = s.runner.auto_resolve();
    assert_eq!(mem0 - s.runner.memory(), 3, "first digivolve reduced");

    let hand2 = s.runner.add_to_hand(0, "DS-LV5-B");
    let mem1 = s.runner.memory();
    let ok = s
        .runner
        .game
        .digivolve_from_hand(0, hand2, second_base.index as usize, PlaySource::ByHand);
    assert!(ok, "OPT used → no second reducer prompt");
    assert!(s.runner.pending_selection().is_none());
    assert_eq!(mem1 - s.runner.memory(), 4, "second digivolve pays full cost");
    assert_eq!(
        face_down_sources(&s.runner, s.thomas) + face_down_sources(&s.runner, s.other_tamer),
        1,
        "only one FD source consumed this turn"
    );
}

#[test]
fn bt25_087_reducer_once_per_turn_clears_next_turn() {
    let (mut s, hand_idx) = dv_setup(true, true, "DS-LV5");
    s.runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    s.runner.accept_optional_trigger().expect("accept turn 1");
    let _ = s.runner.auto_resolve();
    let fd_after_first =
        face_down_sources(&s.runner, s.thomas) + face_down_sources(&s.runner, s.other_tamer);
    assert_eq!(fd_after_first, 1, "one FD card used on turn 1");

    // Back to P0's next turn.
    s.runner.end_turn();
    let _ = s.runner.auto_resolve();
    s.runner.end_turn();
    let _ = s.runner.auto_resolve();
    assert_eq!(s.runner.game.turn_player(), 0, "back on P0's turn");

    let fresh_base = s.runner.place_on_field(0, "BASE4", Some(0));
    let hand2 = s.runner.add_to_hand(0, "DS-LV5-B");
    let mem_before = s.runner.memory();
    let trash_before = s.runner.trash_size(0);
    let ok = s.runner.game.digivolve_from_hand(
        0,
        hand2,
        fresh_base.index as usize,
        PlaySource::ByHand,
    );
    assert!(!ok, "digivolve parks on the reducer prompt again");
    assert!(
        s.runner.pending_is_optional(),
        "OPT lockout cleared → optional reducer prompt offered on the new turn"
    );
    s.runner.accept_optional_trigger().expect("accept turn 3");
    let _ = s.runner.auto_resolve();
    assert!(s.runner.pending_selection().is_none());
    assert_eq!(mem_before - s.runner.memory(), 3, "cost 4 - 1 = 3 paid");
    assert_eq!(s.runner.trash_size(0), trash_before + 1);
    assert_eq!(
        face_down_sources(&s.runner, s.thomas) + face_down_sources(&s.runner, s.other_tamer),
        0,
        "one more FD card trashed"
    );
}

#[test]
fn bt25_087_reducer_inactive_on_opponents_turn() {
    let (mut s, hand_idx) = dv_setup(true, true, "DS-LV5");
    s.runner.pass_turn(); // → P1's turn
    let _ = s.runner.auto_resolve();
    assert_eq!(s.runner.game.turn_player(), 1, "on P1's turn");
    let fd_before =
        face_down_sources(&s.runner, s.thomas) + face_down_sources(&s.runner, s.other_tamer);
    let cp = s.runner.event_checkpoint();
    let ok = s
        .runner
        .game
        .digivolve_from_hand(0, hand_idx, s.base.index as usize, PlaySource::ByHand);
    assert!(ok, "no reducer prompt → digivolve completes synchronously");
    assert!(
        s.runner.pending_selection().is_none(),
        "[Your Turn] → no reducer prompt on the opponent's turn"
    );
    assert_eq!(
        face_down_sources(&s.runner, s.thomas) + face_down_sources(&s.runner, s.other_tamer),
        fd_before,
        "no FD card trashed"
    );
    // The cost is read from the Digivolve event, not by diffing the memory
    // gauge: paying 4 from P1's +3 crosses to P0's side, which ends P1's turn,
    // and Thomas's [Start of Your Turn] then sets P0's memory to 3, so the
    // before/after gauge reading (even with .abs()) shows 0.
    let paid: Vec<i16> = s
        .runner
        .events_since(cp)
        .iter()
        .filter_map(|e| match e {
            digimon_engine::events::GameEvent::Digivolve { player: 0, memory_paid, .. } => {
                Some(*memory_paid)
            }
            _ => None,
        })
        .collect();
    assert_eq!(paid, vec![4], "full digivolve cost 4 paid (no -1 reduction)");
}

// ════════════════════════════════════════════════════════════════════════════
// Clause 4 — [Security] play this card without paying the cost
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_087_security_plays_self_free() {
    let mut runner = builder()
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &[CARD_ID])
        .memory(0)
        .start();
    runner.set_first_player(0);
    runner.end_turn(); // → P1's turn
    let _ = runner.auto_resolve();
    let attacker = runner.place_on_field(1, "BASE4", Some(0));
    let mem_before = runner.memory();
    let _ = runner.attack_player(attacker, 0, false);
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Thomas was played from security"
    );
    assert_eq!(runner.memory(), mem_before, "played without paying the cost");
}
