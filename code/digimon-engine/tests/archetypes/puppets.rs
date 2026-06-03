//! Puppets — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/Puppets-model.md`. Each `#[test]` maps 1:1 to a
//! named combo in that model and asserts the combo's *claimed mechanical
//! outcome* — the cross-card behaviour the per-card `cards_behavioral/` tests
//! (which pin each card in isolation) cannot see.
//!
//! The deck's identity is a **self-deletion value loop**: it floods the board
//! with [Familiar] Tokens (Yellow/3000 DP / `[On Deletion]` -3000 DP), then
//! deliberately deletes its own tokens via the **Overclock** keyword to draw
//! cards (Arisa), refill the board (Cendrillmon refire), and stack -DP onto the
//! opponent until things hit 0 DP and die (rule 17-1-3-1). Deletion is a
//! *resource*, not a cost — so the high-value bugs live at the seams where one
//! card's deletion feeds another card's trigger.
//!
//! Sources cited per combo: DCGO C# under
//! `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`, card text
//! from `data/cards.json`, rules from `general_rule.pdf`.

#![allow(dead_code)]

use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START,
    PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Shared fixtures ─────────────────────────────────────────────────────────

/// A neutral Lv.5 Yellow base so a Lv.6 payoff can sit in a legal stack.
fn lv5_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(5);
    c.dp = Some(6000);
    c
}

/// A neutral Lv.6 Yellow base for a Lv.7 payoff stack.
fn lv6_base(id: &str) -> CardData {
    let mut c = lv5_base(id);
    c.level = Some(6);
    c.dp = Some(10000);
    c
}

/// A plain own Digimon used only to widen the board for "for each of your
/// Digimon" counting (real `kind: digimon`, so it counts; tokens do not).
fn own_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// An opponent Digimon target with a chosen DP.
fn opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

fn field_index_of_name(r: &DebugRunner, player: u8, name: &str) -> usize {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&r.game.card_data) == name)
        .unwrap_or_else(|| panic!("{name} must be on player {player}'s field"))
}

fn familiar_count(r: &DebugRunner, player: usize) -> usize {
    r.game.players[player]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_name(&r.game.card_data) == "Familiar Token")
        .count()
}

fn handle_of_name(r: &DebugRunner, player: u8, name: &str) -> PermanentHandle {
    PermanentHandle {
        player,
        index: field_index_of_name(r, player, name) as u8,
    }
}

fn encode_field_effect(handle: PermanentHandle, slot: u16) -> u16 {
    FIELD_EFFECT_START + handle.index as u16 * EFFECTS_PER_PERMANENT + slot
}

/// Fire a permanent's `[When Digivolving]` timing (direct enqueue + drain — the
/// proven harness pattern; `place_stack` builds the stack without firing the
/// trigger, so we fire it explicitly to model the digivolve).
fn fire_when_digivolving(r: &mut DebugRunner, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

/// Drive every parked selection to completion, order-independently. Optional
/// triggers ("you may") are accepted or declined per `accept_optional`;
/// `EffectChoice` takes its affirmative index-0 branch; any other selection
/// (an opponent/permanent target) takes its first concrete target. This walks a
/// deletion-driven trigger *cascade* — where one deletion queues several cards'
/// triggers in an order the test must not assume (e.g. a Familiar's On-Deletion
/// target pick interleaving with Cendrillmon's refire) — to completion.
fn resolve_cascade(r: &mut DebugRunner, accept_optional: bool) {
    for _ in 0..20 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        let action = if r.pending_is_optional() {
            if accept_optional {
                view.valid_action_ids
                    .iter()
                    .copied()
                    .find(|&a| a != PASS)
                    .unwrap_or(PASS)
            } else {
                PASS
            }
        } else {
            match view.kind {
                SelectionKind::EffectChoice => view.valid_action_ids[0],
                _ => view
                    .valid_action_ids
                    .iter()
                    .copied()
                    .find(|&a| a != PASS)
                    .unwrap_or(PASS),
            }
        };
        if r.execute_action(view.selecting_player, action).is_err() {
            break;
        }
    }
    let _ = r.auto_resolve();
}

/// Accept everything (the common happy-path cascade).
fn resolve_taking_affirmative(r: &mut DebugRunner) {
    resolve_cascade(r, true);
}

/// Fire `overclock`'s end-of-turn Overclock, paying its cost by deleting the
/// `sacrifice` permanent. Mirrors the proven path in
/// `cards_behavioral/ex11/ex11_060.rs`.
fn fire_overclock_deleting(r: &mut DebugRunner, overclock: PermanentHandle, sacrifice_index: u16) {
    r.game.current_phase = GamePhase::EndOfTurnAction;
    let action = encode_field_effect(overclock, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    r.game.decode_action(action, 0);
    r.game
        .resolve_selection(0, encode_attack(0, sacrifice_index))
        .expect("Overclock cost: choose the Token/[Puppet] sacrifice");
}

// ─── C1 — Overclock value loop (Cendrillmon + Arisa + Familiar Token) ─────────
//
// ST19-12 Cendrillmon (Overclock + When-Dig plays 2 Familiar Tokens),
// EX11-060 Arisa Kinosaki, the [Familiar] Token. At end of turn Cendrillmon's
// Overclock deletes 1 Familiar Token → that one deletion fires BOTH the
// Familiar's `[On Deletion]` -3000 DP onto an opponent Digimon AND Arisa's
// `[All Turns]` suspend-to-Draw-1. The signature engine of every Puppets deck.
//
// Sources: $BASE_DCGO/.../ST19/Yellow/ST19_12.cs, EX11/Yellow/EX11_060.cs;
// deletion → on-deletion timing (general_rule.pdf §6, §16). Per-card coverage:
// cards_behavioral/st19/st19_12.rs + ex11/ex11_060.rs.

fn c1_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST19-12")
        .expect("ST19-12 loads")
        .dsl_card("EX11-060")
        .expect("EX11-060 loads")
        .add_card(lv5_base("C1-BASE"))
        .add_card(opp_digimon("C1-OPP", "OppTarget", 10000))
        .add_card(make_test_card("C1-FILL", "Fill"))
        .deck(0, &["C1-FILL"; 10])
        .deck(1, &["C1-FILL"; 10])
        .security(0, &["C1-FILL"; 5])
        .security(1, &["C1-FILL"; 5])
        .memory(10)
        .start()
}

#[test]
fn c1_overclock_token_sacrifice_draws_for_arisa_and_reduces_opponent_dp() {
    let mut r = c1_runner();
    r.game.turn_count = 1;

    // Cendrillmon as a face-up Lv.6; fire its When-Dig to make 2 real Familiar
    // Tokens (the loop's fuel).
    let cendrill = r.place_stack(0, &["C1-BASE", "ST19-12"]);
    fire_when_digivolving(&mut r, cendrill);
    r.auto_resolve().expect("accept ST19-12's 2 Familiar Tokens");
    assert_eq!(familiar_count(&r, 0), 2, "ST19-12 When-Dig plays 2 Familiar Tokens");

    r.place_on_field(0, "EX11-060", Some(0)); // Arisa, unsuspended
    let opp = r.place_on_field(1, "C1-OPP", Some(0));

    let hand_before = r.hand_size(0);
    let opp_dp_before = r.effective_dp(opp).expect("opp DP");
    let token_index = field_index_of_name(&r, 0, "Familiar Token") as u16;

    // End-of-turn Overclock deletes one Familiar Token as its cost.
    fire_overclock_deleting(&mut r, cendrill, token_index);
    resolve_taking_affirmative(&mut r);

    // (a) Arisa's deletion observer drew exactly 1.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "Arisa suspends to Draw 1 when the Overclock deletes your Familiar Token"
    );
    // (b) The deleted Familiar's On-Deletion put -3000 DP on the opponent.
    let opp = handle_of_name(&r, 1, "OppTarget");
    let opp_dp_after = r.effective_dp(opp).expect("opp DP after");
    assert_eq!(
        opp_dp_after,
        opp_dp_before - 3000,
        "the sacrificed Familiar's On-Deletion reduces an opponent Digimon by 3000 DP"
    );
    // One token was consumed by the Overclock cost.
    assert_eq!(familiar_count(&r, 0), 1, "exactly one Familiar Token was deleted as the cost");
}

#[test]
fn c1_value_loop_yields_no_draw_when_arisa_cannot_pay_the_suspend_cost() {
    // Unhappy path: a pre-suspended Arisa cannot pay "by suspending this Tamer",
    // so the deletion still fires the Familiar's -3000 DP but yields NO draw.
    let mut r = c1_runner();
    r.game.turn_count = 1;

    let cendrill = r.place_stack(0, &["C1-BASE", "ST19-12"]);
    fire_when_digivolving(&mut r, cendrill);
    r.auto_resolve().expect("accept tokens");

    let arisa = r.place_on_field(0, "EX11-060", Some(0));
    r.game.players[0].battle_area[arisa.index as usize].is_suspended = true; // already tapped
    let opp = r.place_on_field(1, "C1-OPP", Some(0));

    let hand_before = r.hand_size(0);
    let opp_dp_before = r.effective_dp(opp).expect("opp DP");
    let token_index = field_index_of_name(&r, 0, "Familiar Token") as u16;

    fire_overclock_deleting(&mut r, cendrill, token_index);
    resolve_taking_affirmative(&mut r);

    assert_eq!(
        r.hand_size(0),
        hand_before,
        "a suspended Arisa cannot pay the suspend cost, so no card is drawn"
    );
    let opp = handle_of_name(&r, 1, "OppTarget");
    assert_eq!(
        r.effective_dp(opp).expect("opp DP after"),
        opp_dp_before - 3000,
        "the Familiar's On-Deletion -3000 DP still fires independent of Arisa"
    );
}

// ─── C2 — Self-sustaining token refill (Overclock deleter + BT22-040) ─────────
//
// ST19-12 Cendrillmon is the Overclock *deleter*; BT22-040 Cendrillmon is the
// *observer* (When-Dig plays a Familiar Token; `[All Turns][OPT]` "when any of
// your OTHER Digimon are deleted, you may re-activate this Digimon's When-Dig").
// ST19-12's Overclock deletes a Familiar Token → BT22-040 refire-replays a
// fresh Familiar, so the board count is restored. The cross-card wiring
// (Overclock-deletes → refire-replays) is the system fact under test.
//
// Sources: $BASE_DCGO/.../BT22/Yellow/BT22_040.cs, ST19/Yellow/ST19_12.cs.
// Per-card coverage: cards_behavioral/bt22/bt22_040.rs.

fn c2_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-040")
        .expect("BT22-040 loads")
        .dsl_card("ST19-12")
        .expect("ST19-12 loads")
        .add_card(lv5_base("C2-BASE-A"))
        .add_card(lv5_base("C2-BASE-B"))
        .add_card(opp_digimon("C2-OPP", "OppTarget", 10000))
        .add_card(make_test_card("C2-FILL", "Fill"))
        .deck(0, &["C2-FILL"; 10])
        .deck(1, &["C2-FILL"; 10])
        .security(0, &["C2-FILL"; 5])
        .security(1, &["C2-FILL"; 5])
        .memory(10)
        .start()
}

/// Build the C2 board: BT22-040 (observer) with one Familiar Token it made via
/// When-Dig, plus ST19-12 (the Overclock deleter) and an opponent target.
/// Returns (st19_12 handle as the deleter).
fn c2_setup(r: &mut DebugRunner) -> PermanentHandle {
    r.game.turn_count = 1;

    // BT22-040 face-up; fire its When-Dig and accept the Familiar Token.
    let bt040 = r.place_stack(0, &["C2-BASE-A", "BT22-040"]);
    fire_when_digivolving(r, bt040);
    let choice = r
        .pending_selection_view()
        .expect("BT22-040 When-Dig offers Play/Decline Familiar Token");
    r.execute_action(0, choice.valid_action_ids[0])
        .expect("play the Familiar Token");
    r.auto_resolve().expect("finish BT22-040 token play");
    assert_eq!(familiar_count(r, 0), 1, "BT22-040 made one Familiar Token");

    // ST19-12 as the separate Overclock deleter.
    let st12 = r.place_stack(0, &["C2-BASE-B", "ST19-12"]);
    r.place_on_field(1, "C2-OPP", Some(0));
    st12
}

#[test]
fn c2_overclock_deletion_refires_bt22_040_to_replace_the_token() {
    let mut r = c2_runner();
    let st12 = c2_setup(&mut r);

    let before = familiar_count(&r, 0);
    assert_eq!(before, 1);
    let token_index = field_index_of_name(&r, 0, "Familiar Token") as u16;

    fire_overclock_deleting(&mut r, st12, token_index);
    // One deletion fans out into a cascade: the Familiar's On-Deletion target
    // pick AND BT22-040's optional refire, queued in an order the test must not
    // assume. Accept the whole cascade (incl. the refire and its token replay).
    resolve_taking_affirmative(&mut r);

    assert_eq!(
        familiar_count(&r, 0),
        before,
        "the Overclock-consumed Familiar is replaced by BT22-040's refire — board count restored"
    );
}

#[test]
fn c2_declined_refire_leaves_the_board_one_token_short() {
    let mut r = c2_runner();
    let st12 = c2_setup(&mut r);

    let before = familiar_count(&r, 0);
    let token_index = field_index_of_name(&r, 0, "Familiar Token") as u16;

    fire_overclock_deleting(&mut r, st12, token_index);
    // Resolve the mandatory parts of the cascade (the Familiar's -3000 DP target)
    // but DECLINE the optional refire — so the deleted token is not replaced.
    resolve_cascade(&mut r, /* accept_optional = */ false);

    assert_eq!(
        familiar_count(&r, 0),
        before - 1,
        "declining BT22-040's refire leaves the Overclock-deleted Familiar unreplaced"
    );
}

// ─── C3 — Nyabootmon mass-removal payoff (board-width-gated) ──────────────────
//
// BT22-042 Nyabootmon's `[When Digivolving]` gives "-3000 DP until their turn
// ends for each of your Digimon" to one opponent Digimon. Over a WIDE board the
// stacked -DP drives the target to <=0 → deleted (rule 17-1-3-1). The SAME
// effect over a NARROW board leaves it alive — the removal is gated on board
// width, the system-level fact a per-card test can't express.
//
// Sources: $BASE_DCGO/.../BT22/Yellow/BT22_042.cs; 0-DP deletion rule 17-1-3-1
// (memory project_dp_zero_deletion.md). Per-card: cards_behavioral/bt22/bt22_042.rs.

fn c3_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-042")
        .expect("BT22-042 loads")
        .add_card(lv6_base("C3-BASE"))
        .add_card(own_filler("C3-ALLY-1"))
        .add_card(own_filler("C3-ALLY-2"))
        .add_card(own_filler("C3-ALLY-3"))
        .add_card(opp_digimon("C3-OPP", "OppTarget", 12000))
        .add_card(make_test_card("C3-FILL", "Fill"))
        .deck(0, &["C3-FILL"; 10])
        .deck(1, &["C3-FILL"; 10])
        .security(0, &["C3-FILL"; 5])
        .security(1, &["C3-FILL"; 5])
        .memory(10)
        .start()
}

#[test]
fn c3_nyabootmon_deletes_a_12000_target_over_a_wide_board() {
    let mut r = c3_runner();
    r.game.turn_count = 1;

    // Wide board: Nyabootmon + 3 allies = 4 of your Digimon → -3000 * 4 = -12000.
    let nyaboot = r.place_stack(0, &["C3-BASE", "BT22-042"]);
    r.place_on_field(0, "C3-ALLY-1", Some(0));
    r.place_on_field(0, "C3-ALLY-2", Some(0));
    r.place_on_field(0, "C3-ALLY-3", Some(0));
    r.place_on_field(1, "C3-OPP", Some(0));

    let before = r.battle_area_size(1);
    fire_when_digivolving(&mut r, nyaboot);
    resolve_taking_affirmative(&mut r);

    assert_eq!(
        r.battle_area_size(1),
        before - 1,
        "with 4 of your Digimon the -12000 DP swing deletes the 12000-DP opponent Digimon"
    );
    assert!(
        !r.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&r.game.card_data) == "OppTarget"),
        "the opponent's 12000-DP Digimon must be gone"
    );
}

#[test]
fn c3_nyabootmon_spares_the_same_target_over_a_narrow_board() {
    let mut r = c3_runner();
    r.game.turn_count = 1;

    // Narrow board: Nyabootmon alone = 1 of your Digimon → only -3000.
    let nyaboot = r.place_stack(0, &["C3-BASE", "BT22-042"]);
    r.place_on_field(1, "C3-OPP", Some(0));

    let before = r.battle_area_size(1);
    fire_when_digivolving(&mut r, nyaboot);
    resolve_taking_affirmative(&mut r);

    assert_eq!(
        r.battle_area_size(1),
        before,
        "with only Nyabootmon the -3000 DP swing leaves the 12000-DP target alive"
    );
    let opp = handle_of_name(&r, 1, "OppTarget");
    assert_eq!(
        r.effective_dp(opp).expect("opp DP"),
        12000 - 3000,
        "the narrow-board swing is exactly -3000 DP (one Digimon)"
    );
}

// ─── C4 — Arisa replay & Shoemon recursion (BT22-088) ─────────────────────────
//
// BT22-088 Arisa's `[Start of Your Main Phase]`: by returning this Tamer to the
// bottom of the deck, play 1 [Arisa Kinosaki] from hand free; THEN if you don't
// have a Digimon, play 1 [Shoemon] from trash free. A multi-zone consistency
// engine (deck/hand/trash). The Shoemon clause is gated on having no Digimon.
//
// Sources: $BASE_DCGO/.../BT22/Yellow/BT22_088.cs. Per-card: bt22_088.rs.

fn c4_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 loads")
        .dsl_card("ST19-03")
        .expect("ST19-03 (Shoemon) loads")
        .add_card(own_filler("C4-DIGIMON"))
        .add_card(make_test_card("C4-FILL", "Fill"))
        .hand(0, &["BT22-088"]) // a second Arisa to replay
        .deck(0, &["C4-FILL"; 6])
        .deck(1, &["C4-FILL"; 6])
        .start()
}

/// Fire BT22-088's `[Start of Your Main Phase]` and accept the optional trigger.
/// Mirrors the per-card path in `cards_behavioral/bt22/bt22_088.rs`: the start
/// of the main phase surfaces a `TriggerOrder`; accepting it pays the
/// return-self-to-deck-bottom activation cost and parks at the next selection.
fn accept_start_of_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
    let view = r
        .pending_selection_view()
        .expect("[Start of Your Main Phase] must offer BT22-088's optional trigger");
    assert_eq!(
        view.kind,
        SelectionKind::TriggerOrder,
        "the start-of-main trigger surfaces as a TriggerOrder"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept BT22-088's start-of-main trigger");
}

/// Take the first concrete (non-PASS) option of the currently-parked selection.
fn pick_first_concrete(r: &mut DebugRunner, label: &str) {
    let view = r.pending_selection_view().unwrap_or_else(|| panic!("{label}"));
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or_else(|| panic!("{label}: a concrete option must be available"));
    r.execute_action(view.selecting_player, action)
        .unwrap_or_else(|_| panic!("{label}: execute"));
}

#[test]
fn c4_boardless_arisa_replays_itself_and_recurs_shoemon_from_trash() {
    let mut r = c4_runner();

    r.place_on_field(0, "BT22-088", Some(0));
    r.inject_trash(0, "ST19-03"); // a Shoemon waiting in the trash

    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);

    let _ = deck_before; // (the recurred Shoemon's [On Play] churns the deck —
                         // the clean tamer-return measure lives in the
                         // with-Digimon sibling test, where no Shoemon plays.)

    accept_start_of_main(&mut r);
    // Cost paid → select_hand for the exact-name [Arisa Kinosaki] in hand.
    pick_first_concrete(&mut r, "select_hand for a hand [Arisa Kinosaki]");
    // Boardless → the conditional select_trash for [Shoemon] appears.
    pick_first_concrete(&mut r, "select_trash for a [Shoemon] (boardless)");
    let _ = r.auto_resolve();

    // A new Arisa was played from hand, and (boardless) Shoemon recurred.
    let names: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&r.game.card_data).to_string())
        .collect();
    assert!(
        names.iter().any(|n| n == "Arisa Kinosaki"),
        "a fresh Arisa is played from hand free; field = {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Shoemon"),
        "with no Digimon, Shoemon is replayed from trash free; field = {names:?}"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before - 1,
        "the recurred Shoemon left the trash"
    );
}

#[test]
fn c4_with_a_digimon_in_play_arisa_replays_but_leaves_shoemon_in_trash() {
    let mut r = c4_runner();

    r.place_on_field(0, "BT22-088", Some(0));
    r.place_on_field(0, "C4-DIGIMON", Some(0)); // a Digimon is present
    r.inject_trash(0, "ST19-03");

    let deck_before = r.deck_size(0);
    let trash_before = r.trash_size(0);

    accept_start_of_main(&mut r);
    // Cost paid → replay the hand Arisa. The select_trash branch is gated off
    // (a Digimon is present), so the cascade resolves with no Shoemon prompt.
    pick_first_concrete(&mut r, "select_hand for a hand [Arisa Kinosaki]");
    let _ = r.auto_resolve();

    // The effect genuinely fired (tamer returned), proving the Shoemon clause
    // was *gated off* rather than the whole trigger being skipped.
    assert_eq!(
        r.deck_size(0),
        deck_before + 1,
        "the in-play Arisa still returns to the deck (the trigger fired)"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&r.game.card_data) == "Arisa Kinosaki"),
        "the hand Arisa is still replayed regardless of board state"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "the 'if you don't have a Digimon' gate keeps Shoemon in the trash"
    );
    assert!(
        !r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&r.game.card_data) == "Shoemon"),
        "Shoemon must not enter play when a Digimon is already present"
    );
}

// ─── More situations ─────────────────────────────────────────────────────────
//
// Additional cross-card situations beyond the four headline combos. Three are
// genuine coverage of distinct seams (S2, S4, S5); two are *finding* tests that
// pin the faithful behaviour DCGO + card text demand but the engine does not yet
// match — `#[ignore]`'d with a tracker code so they flip green when fixed.

/// A clean synthetic [Puppet] Digimon used as a combo's neutral target/filler
/// (no effect text, so it adds no resolution noise).
fn puppet_card(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = vec!["Puppet".to_string()];
    c
}

// ─── S2 — Karakurumon's temp-Puppet self-deletes at turn end -> Arisa draws ────
//
// EX11-022 Karakurumon `[When Digivolving]` plays a [Puppet] (DP <= 4000) free
// from hand/trash, then *deletes it at turn end*. That scheduled self-deletion
// is itself a [Puppet]-Digimon deletion you control -> it feeds EX11-060 Arisa's
// `[All Turns]` suspend-to-Draw-1. The combo turns Karakurumon's printed
// drawback (the temp play evaporating) into card advantage — a system fact
// neither per-card test sees.
//
// Sources: $BASE_DCGO/.../EX11/Yellow/EX11_022.cs, EX11/Yellow/EX11_060.cs.

#[test]
fn s2_karakurumon_temp_puppet_self_deletion_feeds_arisa_draw_at_turn_end() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-022")
        .expect("EX11-022 loads")
        .dsl_card("EX11-060")
        .expect("EX11-060 loads")
        .add_card(make_test_card("S2-BASE", "Base"))
        .add_card(puppet_card("S2-TEMP", 4, 4000))
        .add_card(make_test_card("S2-FILL", "Fill"))
        .hand(0, &["S2-TEMP"])
        .deck(0, &["S2-FILL"; 10])
        .deck(1, &["S2-FILL"; 10])
        .security(0, &["S2-FILL"; 5])
        .security(1, &["S2-FILL"; 5])
        .memory(10)
        .start();
    r.game.turn_count = 2;

    r.place_on_field(0, "EX11-060", Some(0)); // Arisa, unsuspended
    let karaku = r.place_stack(0, &["S2-BASE", "EX11-022"]);

    // Karakurumon's When-Dig: play the temp Puppet from hand for free.
    fire_when_digivolving(&mut r, karaku);
    resolve_taking_affirmative(&mut r);
    let temp = handle_of_name(&r, 0, "S2-TEMP");
    assert!(
        r.game.players[0].battle_area[temp.index as usize]
            .top_card()
            .card_name(&r.game.card_data)
            == "S2-TEMP",
        "Karakurumon plays the temp Puppet from hand for free"
    );

    // Model the temp Puppet's turn-end self-deletion with a direct delete: this
    // isolates the *cross-card draw* (the combo's claim) from `end_turn`'s
    // own fresh-turn draw, which would otherwise inflate the count. (The
    // turn-end scheduling itself is EX11-022's per-card concern.)
    let hand_before = r.hand_size(0);
    r.game
        .delete_permanent_with_cause(temp, ReplacementCause::OwnEffect);
    resolve_taking_affirmative(&mut r);

    assert!(
        !r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&r.game.card_data) == "S2-TEMP"),
        "the temp Puppet is deleted"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "the self-deletion of Karakurumon's temp [Puppet] Digimon triggers Arisa's Draw 1"
    );
}

// ─── S4 — ST19-14 Arisa grants Rush to an effect-played token ─────────────────
//
// ST19-14 Arisa `[Your Turn]` "when effects play your Token/[Puppet], by
// suspending this Tamer, 1 of those Digimon gains <Rush>." Pair it with ST19-12
// Cendrillmon, whose When-Dig *plays* 2 Familiar Tokens by effect: the first
// token play suspends Arisa and grants it Rush; the suspend cost then gates the
// second (Arisa is no longer unsuspended). Tempo seam: a freshly-played token
// can attack the turn it arrives.
//
// Sources: $BASE_DCGO/.../ST19/Yellow/ST19_14.cs, ST19/Yellow/ST19_12.cs.

#[test]
fn s4_st19_14_grants_rush_to_one_effect_played_familiar_token() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST19-12")
        .expect("ST19-12 loads")
        .dsl_card("ST19-14")
        .expect("ST19-14 loads")
        .add_card(make_test_card("S4-BASE", "Base"))
        .add_card(make_test_card("S4-FILL", "Fill"))
        .deck(0, &["S4-FILL"; 10])
        .deck(1, &["S4-FILL"; 10])
        .security(0, &["S4-FILL"; 5])
        .security(1, &["S4-FILL"; 5])
        .memory(10)
        .start();
    r.game.turn_count = 1;

    r.place_on_field(0, "ST19-14", Some(0)); // Arisa, unsuspended, your turn
    let cendrill = r.place_stack(0, &["S4-BASE", "ST19-12"]);

    fire_when_digivolving(&mut r, cendrill); // plays 2 Familiar Tokens by effect
    resolve_taking_affirmative(&mut r); // accept tokens + accept the Rush grant

    let rush_tokens = r.game.players[0]
        .battle_area
        .iter()
        .enumerate()
        .filter(|(_, p)| p.top_card().card_name(&r.game.card_data) == "Familiar Token")
        .filter(|(i, _)| {
            r.game.has_keyword(
                PermanentHandle {
                    player: 0,
                    index: *i as u8,
                },
                Keyword::Rush,
            )
        })
        .count();

    assert_eq!(
        familiar_count(&r, 0),
        2,
        "ST19-12 plays two Familiar Tokens"
    );
    assert_eq!(
        rush_tokens, 1,
        "ST19-14's suspend cost grants Rush to exactly one of the played tokens"
    );
}

// ─── S5 — BT22-088 Arisa draws when a token is *played* (play-side engine) ─────
//
// BT22-088 Arisa `[All Turns]` "when your Token/[Puppet] is played, by
// suspending this Tamer, Draw 1." This is the *play-side* draw engine — the
// mirror of C1's *deletion-side* draw. ST19-12 plays 2 Familiar Tokens, but the
// suspend cost limits the draw to one. Asserting +1 (not +2) pins that the cost
// genuinely gates repeat triggers.
//
// Sources: $BASE_DCGO/.../BT22/Yellow/BT22_088.cs, ST19/Yellow/ST19_12.cs.

#[test]
fn s5_bt22_088_draws_exactly_once_when_two_tokens_are_played() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST19-12")
        .expect("ST19-12 loads")
        .dsl_card("BT22-088")
        .expect("BT22-088 loads")
        .add_card(make_test_card("S5-BASE", "Base"))
        .add_card(make_test_card("S5-FILL", "Fill"))
        .deck(0, &["S5-FILL"; 10])
        .deck(1, &["S5-FILL"; 10])
        .security(0, &["S5-FILL"; 5])
        .security(1, &["S5-FILL"; 5])
        .memory(10)
        .start();
    r.game.turn_count = 1;

    r.place_on_field(0, "BT22-088", Some(0)); // Arisa, unsuspended
    let cendrill = r.place_stack(0, &["S5-BASE", "ST19-12"]);

    let hand_before = r.hand_size(0);
    fire_when_digivolving(&mut r, cendrill); // plays 2 Familiar Tokens
    resolve_taking_affirmative(&mut r);

    assert_eq!(familiar_count(&r, 0), 2, "ST19-12 plays two Familiar Tokens");
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "the suspend cost limits BT22-088's play-side draw to exactly one, not one-per-token"
    );
}

// ─── S1 — Pillomon's flood-gate must block Familiar Token plays (FINDING) ──────
//
// BT9-033 Pillomon: "Players can't play Digimon by effects." A Familiar Token is
// a Digimon (Digimon/Yellow/3000 DP), and DCGO blocks token plays under this
// lock: `CardEffectCommons.PlayToken` -> `CanPlayAsNewPermanent` ->
// `CanNotPutFieldClass` whose card-condition is `IsDigimon || IsDigiEgg` — true
// for the Familiar Token. So with Pillomon out, ST19-12's "play 2 Familiar
// Tokens" must place ZERO.
//
// CURRENT ENGINE: `EffectContext::play_token` pushes the token directly and does
// NOT consult `CannotPlayDigimonByEffect` (unlike `play_from_hand_free`), so the
// tokens still spawn. Confirmed against DCGO BT9_033.cs + CardEffectCommons.cs.
//
// Sources: $BASE_DCGO/.../BT9/Yellow/BT9_033.cs, Script/CardEffectCommons.cs
// (PlayToken/CanPlayAsNewPermanent); ST19/Yellow/ST19_12.cs.
// FIXED 2026-05-30 (G-PLAY-TOKEN-FLOODGATE): `EffectContext::play_token` now
// consults `CannotPlayDigimonByEffect`, so effect-played Digimon tokens are
// blocked under the lock — matching DCGO's `CanPlayAsNewPermanent` gate.
#[test]
fn s1_pillomon_floodgate_blocks_effect_token_plays() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST19-12")
        .expect("ST19-12 loads")
        .dsl_card("BT9-033")
        .expect("BT9-033 (Pillomon) loads")
        .add_card(make_test_card("S1-BASE", "Base"))
        .add_card(make_test_card("S1-FILL", "Fill"))
        .deck(0, &["S1-FILL"; 10])
        .deck(1, &["S1-FILL"; 10])
        .security(0, &["S1-FILL"; 5])
        .security(1, &["S1-FILL"; 5])
        .memory(10)
        .start();
    r.game.turn_count = 1;

    r.place_on_field(0, "BT9-033", Some(0)); // Pillomon installs the lock
    let cendrill = r.place_stack(0, &["S1-BASE", "ST19-12"]);
    // Pillomon's flood-gate is a continuous declarative effect; in live play the
    // declarative tick runs after every action. Materialize it explicitly here
    // (the raw enqueue/drain harness does not auto-tick) so the lock is active
    // when ST19-12 resolves.
    r.game.tick_declarative_effects();

    fire_when_digivolving(&mut r, cendrill);
    resolve_taking_affirmative(&mut r);

    assert_eq!(
        familiar_count(&r, 0),
        0,
        "Pillomon's 'can't play Digimon by effects' must block the Familiar Token plays"
    );
}

/// Paired control for S1: the SAME ST19-12 token play with NO Pillomon spawns
/// its 2 Familiar Tokens — pinning that the flood-gate (not some unrelated
/// failure) is what blocks above.
#[test]
fn s1b_without_pillomon_the_same_token_play_succeeds() {
    let mut r = DebugRunner::builder()
        .dsl_card("ST19-12")
        .expect("ST19-12 loads")
        .add_card(make_test_card("S1B-BASE", "Base"))
        .add_card(make_test_card("S1B-FILL", "Fill"))
        .deck(0, &["S1B-FILL"; 10])
        .deck(1, &["S1B-FILL"; 10])
        .security(0, &["S1B-FILL"; 5])
        .security(1, &["S1B-FILL"; 5])
        .memory(10)
        .start();
    r.game.turn_count = 1;

    let cendrill = r.place_stack(0, &["S1B-BASE", "ST19-12"]);
    r.game.tick_declarative_effects();
    fire_when_digivolving(&mut r, cendrill);
    resolve_taking_affirmative(&mut r);

    assert_eq!(
        familiar_count(&r, 0),
        2,
        "with no flood-gate, ST19-12's Familiar Token plays resolve normally"
    );
}

// ─── S3 — Kaguyamon's recursion must fire on Familiar Token deletion (FINDING) ─
//
// EX11-023 Kaguyamon `[All Turns][OPT]`: "When other Digimon are deleted, you
// may play 1 level 4 or lower [Puppet] Digimon from trash for free." A Familiar
// Token IS a Digimon, and DCGO gates this on `permanent.IsDigimon` (true for the
// token — `CardSource.IsDigimon => CardKinds.Contains(CardKind.Digimon)`), so a
// deleted Familiar Token must trigger the recursion.
//
// CURRENT ENGINE: the DSL condition is `event_target_kind: digimon` ONLY (no
// `token`), and `kind_matches_field(Digimon, Token)` is false — so a token
// deletion does NOT fire the recursion. Its sibling cards BT22-040 and EX11-060
// correctly use `any_of: [digimon, token]`; EX11-023 omits `token`.
//
// Sources: $BASE_DCGO/.../EX11/Yellow/EX11_023.cs (IsDigimon gate),
// Script/CardSource.cs (IsDigimon); card text cards.json EX11-023; sibling DSL
// specs cards/bt22/BT22-040.yaml, cards/ex11/EX11-060.yaml.
// FIXED 2026-05-30 (G-EX11-023-TOKEN-DELETION): EX11-023's recursion condition
// now matches `any_of: [digimon, token]`, so a Familiar Token deletion (a
// Digimon) triggers the trash-recursion — matching DCGO's `IsDigimon` gate.
#[test]
fn s3_kaguyamon_recursion_fires_on_familiar_token_deletion() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-023")
        .expect("EX11-023 loads")
        .dsl_card("ST19-12")
        .expect("ST19-12 loads")
        .add_card(make_test_card("S3-BASE-A", "BaseA"))
        .add_card(make_test_card("S3-BASE-B", "BaseB"))
        .add_card(puppet_card("S3-TRASH-PUP", 4, 4000))
        .add_card(opp_digimon("S3-OPP", "OppTarget", 10000))
        .add_card(make_test_card("S3-FILL", "Fill"))
        .deck(0, &["S3-FILL"; 10])
        .deck(1, &["S3-FILL"; 10])
        .security(0, &["S3-FILL"; 5])
        .security(1, &["S3-FILL"; 5])
        .memory(10)
        .start();
    r.game.turn_count = 1;

    r.place_on_field(0, "EX11-023", Some(0)); // Kaguyamon face-up (observer)
    r.inject_trash(0, "S3-TRASH-PUP"); // a Lv4 Puppet to recur from trash
    let st12 = r.place_stack(0, &["S3-BASE-B", "ST19-12"]); // Overclock deleter
    fire_when_digivolving(&mut r, st12);
    r.auto_resolve().expect("ST19-12 plays Familiar Tokens");
    r.place_on_field(1, "S3-OPP", Some(0));

    let token_index = field_index_of_name(&r, 0, "Familiar Token") as u16;
    fire_overclock_deleting(&mut r, st12, token_index);
    resolve_taking_affirmative(&mut r);

    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_name(&r.game.card_data) == "S3-TRASH-PUP"),
        "the deleted Familiar Token (a Digimon) must trigger Kaguyamon's trash recursion"
    );
}
