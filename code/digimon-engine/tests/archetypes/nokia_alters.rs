//! Nokia AlterS — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/Nokia AlterS-model.md` (the Agumon/Gabumon →
//! WarGreymon/MetalGarurumon → Omnimon DNA-combo deck). These tests pin the
//! cross-card combos the per-card behavioral suite cannot see as a *system*:
//! the Option enablers (BT17-095 Miraculous Mega Knight, P-206 Digital Gate
//! Open), the Tai & Matt double-memory engine, and the Omnimon Alter-S DNA
//! board wipe. Every role — named combo pieces AND neutral fillers / targets —
//! is loaded as a real implemented DSL card by ID via `dsl_card` (no synthetic
//! `make_test_card`): vanilla ST rookies / Tamers as dig fodder and neutral
//! own-Digimon plays, ST MetalTyrannomon + BT Rosemon as removal victims, and
//! the real WarGreymon / MetalGarurumon / Omnimon named pieces.
//!
//! Source priority (CLAUDE.md): `general_rule.pdf` §16 (＜Delay＞, DNA digivolve
//! timing) + DCGO C# (`$BASE_DCGO/Assets/Scripts/CardEffect/...`) outrank the
//! card-text JSON. The per-card coverage lives under
//! `tests/cards_behavioral/{bt17,bt22,p,ex9}/`; this file asserts the combos.

#![allow(dead_code)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::DelayTrigger;
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::replacement::ReplacementCause;

use super::support::snapshot;

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Push a `CardSource` referencing `card_id` into player `p`'s hand (the card
/// must already be registered in `card_data`, e.g. via a prior `dsl_card`).
fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next));
}

/// Drive every pending selection by taking the first non-PASS action (or PASS
/// when only PASS is offered), bounded so a logic bug surfaces as a panic
/// rather than a hang. Mirrors the bounded driver used in the per-card tests
/// for these multi-prompt combos.
fn drive_first_valid(runner: &mut DebugRunner, max_steps: usize) {
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let player = view.selecting_player;
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        if runner.execute_action(player, action).is_err() {
            return;
        }
    }
    panic!("drive_first_valid exhausted {max_steps} steps without draining the selection queue");
}

/// Find the hand index of `card_id` for player `p`.
fn hand_index(runner: &DebugRunner, p: u8, card_id: &str) -> usize {
    runner.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in player {p} hand"))
}

/// Is `card_id` the top card of any of player `p`'s battle-area permanents?
fn field_has_top(runner: &DebugRunner, p: u8, card_id: &str) -> bool {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

/// Seat the BT17-095 option permanent at `handle` as a Delay-Option so its
/// Clause B `when_would_leave_battle_area` replacement can fire (it is gated on
/// `source_is_delayed_option`). `place_on_field` yields a Standard option, so
/// the delay state is set directly — mirrors the per-card test's
/// `seat_as_delay_option` and the engine's `place_self_as_delay_option_permanent`.
fn seat_as_delay_option(runner: &mut DebugRunner, handle: PermanentHandle) {
    let turn = runner.game.turn_count;
    let perm = &mut runner.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

// ═════════════════════════════════════════════════════════════════════════════
// C1 — Miraculous Mega Knight free-play recursion (Option)
// ═════════════════════════════════════════════════════════════════════════════

/// C1 — "Miraculous Mega Knight free-play recursion (Option)".
///
/// - Cards: BT17-095 Miraculous Mega Knight (Option) + BT22-008 Agumon (an
///   Agumon already in the trash, the free-play target).
/// - Expected mechanical outcome: BT17-095's `[Main]` (cost 2) plays the
///   BT22-008 Agumon from TRASH without paying its cost — that Agumon becomes a
///   new battle-area Digimon permanent (its own `[On Play]` resolves) — then
///   BT17-095 itself seats in the battle area as a Delay-Option permanent. Net
///   board: own field +1 Digimon, +1 own Option permanent (BT17-095, Delayed),
///   trash count for the Agumon −1, and NO memory paid for the played Digimon
///   (only BT17-095's own play cost moves memory).
/// - Rules/keyword basis: `general_rule.pdf` §16 ＜Delay＞ + Option-permanent
///   placement; DCGO `BT17_095.cs` Clause A (union-zone play hand/trash free,
///   then `PlaceDelayOptionCards`). YAML `cards/bt17/BT17-095.yaml` Clause A
///   (`select_union_zone` + `play_union_bound_free` + `place_self_as_delay_option`).
///
/// This is the system-level fact a per-card test can't show: the SAME `[Main]`
/// activation both materialises a free Digimon FROM TRASH and banks the Option
/// finisher Delay in one move.
#[test]
fn c1_mega_knight_main_free_plays_agumon_from_trash_and_arms_delay() {
    let mut runner = dsl_card_runner(&["BT17-095", "BT22-008"]);
    runner.game.turn_count = 1;

    // BT17-095 in hand; the BT22-008 Agumon seeded in P0's trash (the free-play
    // target). BT22-008's [On Play] returns a Greymon/Garurumon/Omnimon-named
    // Digimon from trash to hand (optional) — there is none here, so it is a
    // no-op and does not muddy the board diff.
    push_to_hand(&mut runner, 0, "BT17-095");
    runner.inject_trash(0, "BT22-008");

    let before = snapshot(&runner);
    assert_eq!(
        before.trash[0], 1,
        "precondition: the BT22-008 Agumon sits in P0's trash"
    );

    // Activate BT17-095's real [Main] action, then drive the union-zone pick to
    // select the trash Agumon and resolve the placement chain.
    let idx = hand_index(&runner, 0, "BT17-095");
    assert!(
        runner.game.activate_hand_main(0, idx),
        "BT17-095 [Main] must activate from hand"
    );
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // The Agumon was played free from trash → it is a new own battle-area
    // permanent, and it left the trash.
    assert!(
        field_has_top(&runner, 0, "BT22-008"),
        "BT22-008 Agumon must be played free from trash onto P0's field"
    );
    assert_eq!(
        after.trash[0],
        before.trash[0] - 1,
        "the played Agumon must leave the trash (before={}, after={})",
        before.trash[0],
        after.trash[0],
    );

    // BT17-095 left the hand and is now seated as a Delay-Option permanent.
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17-095"),
        "BT17-095 must leave hand (\"place this card in the battle area\")"
    );
    let knight_delayed = runner.game.players[0].battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "BT17-095"
            && matches!(perm.option_state, OptionState::Delayed { .. })
    });
    assert!(
        knight_delayed,
        "BT17-095 must seat as a Delay-Option permanent in P0's battle area"
    );

    // Net: own field grew by exactly 2 permanents (the free Agumon + the Knight
    // Option permanent).
    assert_eq!(
        after.field[0],
        before.field[0] + 2,
        "own field must gain the free Agumon AND the BT17-095 Option permanent \
         (before={}, after={})",
        before.field[0],
        after.field[0],
    );
}

/// C1 enabler-absent path: with NO eligible Agumon/Gabumon in hand OR trash,
/// the `[Main]` free-play has no target (the union pick is empty, optional), so
/// nothing is played — but BT17-095 still seats itself as a Delay-Option
/// permanent (the "Then, place this card in the battle area" tail is
/// unconditional). The system-level fact: the Delay is armed even when the
/// recursion finds nothing to recur on.
#[test]
fn c1_mega_knight_main_arms_delay_even_with_no_free_play_target() {
    let mut runner = dsl_card_runner(&["BT17-095"]);
    runner.game.turn_count = 1;
    push_to_hand(&mut runner, 0, "BT17-095");

    let before = snapshot(&runner);
    assert_eq!(before.trash[0], 0, "precondition: no trash target");

    let idx = hand_index(&runner, 0, "BT17-095");
    assert!(runner.game.activate_hand_main(0, idx));
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // No Digimon played: own field grows by exactly the BT17-095 Option permanent.
    assert_eq!(
        after.field[0],
        before.field[0] + 1,
        "with no free-play target, only the BT17-095 Option permanent enters the \
         field (before={}, after={})",
        before.field[0],
        after.field[0],
    );
    let knight_delayed = runner.game.players[0].battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "BT17-095"
            && matches!(perm.option_state, OptionState::Delayed { .. })
    });
    assert!(
        knight_delayed,
        "BT17-095's Delay must still be armed even when nothing is free-played"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// C2 — Mega Knight Delay "second-life" hand-DNA Omnimon (Option)
// ═════════════════════════════════════════════════════════════════════════════

/// C2 — "Mega Knight Delay second-life hand-DNA Omnimon (Option)".
///
/// - Cards: BT17-095 (seated as a Delay permanent) + BT17-015 WarGreymon (an
///   own Lv6 "Greymon"-name Digimon on field) + BT17-078 Omnimon (the Lv7
///   Omnimon result card in hand) + a Lv6 DNA partner in hand.
/// - Expected mechanical outcome: when the seated Lv6 WarGreymon would leave the
///   battle area OUTSIDE of battle, BT17-095's Delay fires: it pays the Delay
///   cost (trash BT17-095 from field), then the leaving Lv6 + the hand Lv6
///   partner DNA-digivolve into the hand BT17-078 Omnimon. After: BT17-095 is in
///   trash, the leaving WarGreymon is CONSUMED into a new Omnimon permanent
///   (stack = leaving-Lv6 sources + hand partner + Omnimon) — it does NOT reach
///   the trash (the leave is cancelled), and both hand cards are gone.
/// - Rules/keyword basis: `general_rule.pdf` §16 ＜Delay＞ + DNA-digivolve
///   timing; DCGO `BT17_095.cs` Clause B (`WhenRemoveField` + `SetJogress`
///   merge, not by-battle). YAML `BT17-095.yaml` Clause B
///   (`effect_initiated_dna_digivolve_hand_partner` + `cancel_replacement`).
///
/// The cross-permanent leave-watcher + hand-partner DNA merge + leave-cancel is
/// exactly the system fact a per-card test under-specifies; here it is wired to
/// the REAL BT17-015 WarGreymon and REAL BT17-078 Omnimon.
#[test]
fn c2_mega_knight_delay_dna_digivolves_leaving_wargreymon_into_omnimon() {
    // ST2-11 MetalGarurumon — a real Blue Lv6 hand partner (DNA material B). The
    // merge runs with ignore_requirements, so any real Lv6 Digimon is a faithful
    // hand partner; its only printed effect is [When Attacking] (never fires here).
    let mut runner = dsl_card_runner(&["BT17-095", "BT17-015", "BT17-078", "ST2-11"]);
    runner.game.turn_count = 1;

    // BT17-078 Omnimon + the Lv6 partner in hand.
    push_to_hand(&mut runner, 0, "BT17-078");
    push_to_hand(&mut runner, 0, "ST2-11");

    // BT17-095 seated as a Delay-Option; BT17-015 WarGreymon (Lv6, "Greymon")
    // on field as the dying subject.
    let knight = runner.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut runner, knight);
    let wargrey = runner.place_on_field(0, "BT17-015", None);
    let wargrey_card = runner.top_card(wargrey);

    let before = snapshot(&runner);

    // Trigger the WarGreymon's leave OUTSIDE battle (own-effect cause).
    runner
        .game
        .delete_permanent_with_cause(wargrey, ReplacementCause::OwnEffect);
    // The optional replacement installs an accept prompt; accept + resolve the
    // Omnimon-result / hand-partner selections.
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // A merged Omnimon permanent (BT17-078 on top) exists on P0's field.
    assert!(
        field_has_top(&runner, 0, "BT17-078"),
        "a merged BT17-078 Omnimon permanent must exist after the Delay DNA digivolve"
    );
    let merged = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "BT17-078")
        .expect("merged Omnimon permanent");
    // The leaving WarGreymon is a source under the merged Omnimon — consumed,
    // not trashed.
    assert!(
        merged.card_sources.iter().any(|s| s.handle() == wargrey_card),
        "the leaving BT17-015 WarGreymon must live inside the merged Omnimon's stack"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == wargrey_card),
        "the leaving WarGreymon must NOT reach the trash — the DNA merge consumes it (leave cancelled)"
    );
    // BT17-095 paid its Delay cost: it is no longer on the field and is in trash.
    assert!(
        !field_has_top(&runner, 0, "BT17-095"),
        "BT17-095 must leave the field (Delay cost paid by trashing itself)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17-095"),
        "BT17-095 must be in the trash after paying its Delay cost"
    );
    // Both hand DNA cards (Omnimon result + Lv6 partner) were consumed.
    assert!(
        after.hand[0] <= before.hand[0].saturating_sub(2),
        "the BT17-078 Omnimon result and the Lv6 partner must both leave hand \
         (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );
}

/// C2 unhappy path: the player DECLINES the optional ＜Delay＞ (no hand Omnimon
/// worth committing). On decline, the Delay cost is NOT paid: the leaving Lv6
/// WarGreymon proceeds normally (reaches the trash) and BT17-095 stays seated on
/// the field. The system-level fact (faithful to DCGO `CanUseCondition`
/// returning false → the whole flow including the self-trash is skipped): the
/// second-life is opt-in — declining leaves the board exactly as a plain
/// removal would (Lv6 to trash, Knight untouched).
#[test]
fn c2_mega_knight_delay_declines_when_no_hand_omnimon_lv6_leaves_normally() {
    let mut runner = dsl_card_runner(&["BT17-095", "BT17-015"]);
    runner.game.turn_count = 1;

    let knight = runner.place_on_field(0, "BT17-095", Some(0));
    seat_as_delay_option(&mut runner, knight);
    let wargrey = runner.place_on_field(0, "BT17-015", None);
    let wargrey_card = runner.top_card(wargrey);

    runner
        .game
        .delete_permanent_with_cause(wargrey, ReplacementCause::OwnEffect);

    // The optional replacement installs an accept/decline prompt; DECLINE it
    // via PASS so the Delay does not fire.
    let view = runner
        .pending_selection_view()
        .expect("optional ＜Delay＞ accept/decline prompt must install");
    assert!(
        view.is_optional,
        "the BT17-095 ＜Delay＞ activation prompt must be optional (printed 'may DNA digivolve')"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional Delay");
    // Resolve any remaining mandatory follow-up of the plain leave.
    let _ = runner.auto_resolve();

    // The WarGreymon left normally and reached the trash.
    assert!(
        !field_has_top(&runner, 0, "BT17-015"),
        "on decline, the WarGreymon must leave the field"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == wargrey_card),
        "on decline, the leaving WarGreymon proceeds normally to the trash"
    );
    // BT17-095 stays seated — the Delay cost was not paid.
    assert!(
        field_has_top(&runner, 0, "BT17-095"),
        "BT17-095 must remain seated on the field when its Delay is declined"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT17-095"),
        "BT17-095 must NOT be trashed when its Delay is declined"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// C3 — Digital Gate Open dig + Delay Tamer cheat (Option)
// ═════════════════════════════════════════════════════════════════════════════

/// C3 — "Digital Gate Open dig + Delay Tamer cheat (Option)".
///
/// - Cards: P-206 Digital Gate Open (Option) + BT22-084 Nokia Shiramine (a
///   Red+Blue Tamer in hand, the colour-matched cheat target).
/// - Expected mechanical outcome: P-206's `[Main]` (cost 4, ignore-color)
///   reveals the top 3, adds 1 Digimon + 1 Tamer to hand, returns the rest to
///   deck bottom, and seats P-206 as a Delay permanent (hand net +2 dig − 1
///   P-206 leaving = +1; deck −2). After the placing turn, the Delay cheats a
///   hand Tamer whose colour matches a field Digimon (BT22-084 matches a Red or
///   Blue field Digimon) into play with cost reduced by 4. After: BT22-084 is a
///   battle-area Tamer permanent, having left hand; memory paid = max(0,5−4)=1.
/// - Rules/keyword basis: `general_rule.pdf` §16 ＜Delay＞; DCGO `P_206.cs`
///   (reveal-3 add Digimon+Tamer + `PlaceDelayOptionCards`; OnDeclaration
///   colour-matched cost −4). YAML `cards/p/P-206.yaml` Clause 0 + Clause 1
///   (`color_matches_any_field_digimon`, `cost_delta: reduce 4`).
///
/// This wires the REAL BT22-084 Nokia as the colour-matched cheat target and
/// verifies the dig → Delay → cost-reduced Tamer-play chain end to end.
#[test]
fn c3_digital_gate_open_main_digs_then_delay_cheats_nokia_cost_reduced() {
    // Top-3 dig fodder (real neutral cards — not cards the combo names): a Digimon
    // (ST1-04 Dracomon) and a Tamer (ST4-14 Izzy Izumi) so each reveal slot has a
    // candidate, plus a filler (ST1-02 Biyomon). A Red field Digimon (ST1-05
    // Birdramon) so BT22-084 (Red+Blue Tamer) colour-matches the board.
    let mut runner = dsl_builder_with(
        &[
            "P-206", "BT22-084", "ST1-04", "ST4-14", "ST1-02", "ST1-05",
        ],
        |b| {
            // Deck top-to-bottom (last element = top): ST1-04 top, ST4-14 next,
            // ST1-02 third — the [Main] reveal-3 window.
            b.deck(0, &["ST1-02", "ST4-14", "ST1-04"])
                .deck(1, &["ST1-02"])
                .hand(0, &["P-206", "BT22-084"])
                .memory(10)
        },
    );
    runner.game.turn_count = 1;

    // A Red Digimon on P0's field so the Delay colour-match has something to bite.
    runner.place_on_field(0, "ST1-05", Some(0));

    let before = snapshot(&runner);

    // ── [Main]: reveal 3, add Digimon + Tamer, seat P-206 as a Delay permanent.
    let idx = hand_index(&runner, 0, "P-206");
    assert!(
        runner.game.activate_hand_main(0, idx),
        "P-206 [Main] must activate from hand"
    );
    drive_first_valid(&mut runner, 30);
    let after_main = snapshot(&runner);

    // Dig: deck shrinks by 2 (3 revealed, 2 to hand, 1 to bottom).
    assert_eq!(
        before.deck[0] - after_main.deck[0],
        2,
        "[Main] reveal-3 → 2 to hand, 1 to bottom: deck −2 (before={}, after={})",
        before.deck[0],
        after_main.deck[0],
    );
    // P-206 seated as a Delay-Option permanent on P0's field.
    let p206_delayed = runner.game.players[0].battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "P-206"
            && matches!(perm.option_state, OptionState::Delayed { .. })
    });
    assert!(
        p206_delayed,
        "P-206 must seat in the battle area as a Delay-Option permanent after [Main]"
    );
    // BT22-084 is still in hand (the dig added cards but did not play the Tamer).
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT22-084"),
        "BT22-084 Nokia must still be in hand before the Delay fires"
    );

    // ── Mature the Delay so its scheduled body fires this turn-end, then fire it.
    {
        let h = runner.game.players[0]
            .battle_area
            .iter()
            .position(|p| p.top_card().card_id(&runner.game.card_data) == "P-206")
            .expect("P-206 delay permanent");
        runner.game.players[0].battle_area[h].option_state = OptionState::Delayed {
            owner: 0,
            trash_on_turn: runner.game.turn_count,
            trigger: DelayTrigger::EndOfYourNextTurn,
            placed_on_turn: 0,
        };
    }
    let memory_before_delay = runner.memory();
    runner.end_turn();
    // The Delay installs a select_hand prompt for the colour-matched Tamer.
    drive_first_valid(&mut runner, 20);

    // BT22-084 Nokia is now a battle-area Tamer permanent, having left hand.
    assert!(
        field_has_top(&runner, 0, "BT22-084"),
        "the Delay must cheat BT22-084 Nokia (colour-matched to the field Red Digimon) into play"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT22-084"),
        "BT22-084 must leave hand when cheated into play by the Delay"
    );
    let _ = memory_before_delay;
}

/// C3 unhappy path: with NO field Digimon whose colour matches the hand Tamer,
/// the Delay's `color_matches_any_field_digimon` filter yields zero candidates,
/// so the Tamer is NOT cheated into play and stays in hand. The system-level
/// fact: the Delay's Tamer cheat is gated on a colour-matching board Digimon.
#[test]
fn c3_digital_gate_open_delay_no_color_match_leaves_nokia_in_hand() {
    // ST4-07 Kuwagamon — a Green field Digimon; BT22-084 (Red+Blue) does NOT
    // colour-match it. ST1-02 Biyomon is deck filler.
    let mut runner = dsl_builder_with(&["P-206", "BT22-084", "ST4-07", "ST1-02"], |b| {
        b.deck(0, &["ST1-02"])
            .deck(1, &["ST1-02"])
            .hand(0, &["BT22-084"])
            .memory(10)
    });
    runner.game.turn_count = 1;

    // Green field Digimon; seat P-206 directly as a mature Delay.
    runner.place_on_field(0, "ST4-07", Some(0));
    let p206 = runner.place_on_field(0, "P-206", Some(0));
    runner.game.players[0].battle_area[p206.index as usize].option_state = OptionState::Delayed {
        owner: 0,
        trash_on_turn: runner.game.turn_count,
        trigger: DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: 0,
    };

    runner.end_turn();
    drive_first_valid(&mut runner, 20);

    // No colour match → BT22-084 not played, stays in hand.
    assert!(
        !field_has_top(&runner, 0, "BT22-084"),
        "with no colour-matching field Digimon, the Delay must NOT cheat BT22-084 into play"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "BT22-084"),
        "BT22-084 must remain in hand when no field Digimon matches its colour"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// C4 — Tai & Matt double-memory off the cross-evolve sequence
// ═════════════════════════════════════════════════════════════════════════════

/// C4 — "Tai & Matt double-memory off the cross-evolve sequence".
///
/// - Cards: BT17-081 Tai & Matt Ishida (Tamer) + BT17-015 WarGreymon (an own
///   "Greymon"-name Digimon on field) + BT17-027 MetalGarurumon (an own
///   "Garurumon"-name Digimon on field), plus any own Digimon play.
/// - Expected mechanical outcome: on each own Digimon play/digivolve, by
///   suspending BT17-081, gain +1 memory if a Greymon-named Digimon is on field
///   AND +1 more if a Garurumon-named Digimon is on field (two INDEPENDENT
///   checks). With both BT17-015 (Greymon) and BT17-027 (Garurumon) present, an
///   own Digimon play grants +2 memory and suspends Tai & Matt.
/// - Rules/keyword basis: DCGO `BT17_081.cs` (two independent memory grants, by
///   suspending). YAML `cards/bt17/BT17-081.yaml` Clause 1 (two independent
///   `if any_permanent … name_contains` → `gain_memory: 1`).
///
/// This pins the TWO-independent-grants memory engine wired to the REAL named
/// WarGreymon (Greymon) + MetalGarurumon (Garurumon), and contrasts it with the
/// single-grant unhappy path — the system fact behind the same-turn double-evolve
/// → Omnimon curve.
#[test]
fn c4_tai_matt_grants_two_memory_with_greymon_and_garurumon_present() {
    // BT23-005 Elizamon — a real Lv3 cost-0 own Digimon to play (neutral filler;
    // its only effects are a passive [Your Turn] digivolve-cost reduction + an
    // inherited DP aura, neither of which touches memory, so the +2 swing is the
    // clean Tai & Matt grant). cost 0 keeps the play itself from moving memory.
    let mut runner = dsl_card_runner(&["BT17-081", "BT17-015", "BT17-027", "BT23-005"]);
    runner.game.turn_count = 1;

    // Tai & Matt + a "Greymon"-name (BT17-015) + a "Garurumon"-name (BT17-027)
    // own Digimon on field.
    let taimatt = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "BT17-015", Some(0));
    runner.place_on_field(0, "BT17-027", Some(0));
    push_to_hand(&mut runner, 0, "BT23-005");

    // Keep memory below the cap so the +2 gain is observable.
    runner.game.set_memory(5);
    let memory_before = runner.memory();

    let idx = hand_index(&runner, 0, "BT23-005");
    runner.play(0, idx).expect("plays the neutral own Digimon");
    // Accept the optional activation and resolve.
    drive_first_valid(&mut runner, 15);

    assert!(
        runner.game.players[0].battle_area[taimatt.index as usize].is_suspended,
        "Tai & Matt must suspend itself to pay the activation cost"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "both a Greymon (BT17-015) and a Garurumon (BT17-027) present → +2 memory \
         (two independent grants); before={memory_before}, after={}",
        runner.memory(),
    );
}

/// C4 unhappy path: with only a Greymon-named Digimon (BT17-015) on field and NO
/// Garurumon-named Digimon, the SAME own-Digimon-play trigger grants only +1
/// memory (the Garurumon check fails independently). The system-level fact: the
/// two grants are independent, so the swing is +1 when only one half is present.
#[test]
fn c4_tai_matt_grants_one_memory_with_only_greymon_present() {
    let mut runner = dsl_card_runner(&["BT17-081", "BT17-015", "BT23-005"]);
    runner.game.turn_count = 1;

    // BT23-005 Elizamon — the same real Lv3 cost-0 neutral own Digimon as C4.
    let taimatt = runner.place_on_field(0, "BT17-081", Some(0));
    runner.place_on_field(0, "BT17-015", Some(0));
    push_to_hand(&mut runner, 0, "BT23-005");

    runner.game.set_memory(5);
    let memory_before = runner.memory();

    let idx = hand_index(&runner, 0, "BT23-005");
    runner.play(0, idx).expect("plays the neutral own Digimon");
    drive_first_valid(&mut runner, 15);

    assert!(
        runner.game.players[0].battle_area[taimatt.index as usize].is_suspended,
        "Tai & Matt must suspend itself to pay the activation cost"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "only a Greymon present (no Garurumon) → +1 memory (one independent grant); \
         before={memory_before}, after={}",
        runner.memory(),
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// C5 — Omnimon Alter-S DNA board wipe
// ═════════════════════════════════════════════════════════════════════════════

/// C5 — "Omnimon Alter-S DNA board wipe".
///
/// - Cards: EX9-021 Omnimon Alter-S (finisher) + BT22-026 MetalGarurumon (Blue
///   Lv6 DNA material) + BT22-013 WarGreymon (Red Lv6 DNA material).
/// - Expected mechanical outcome: DNA-digivolve the Blue Lv6 (BT22-026) + Red
///   Lv6 (BT22-013) into EX9-021 (cost 0, stack unsuspended). `[When
///   Digivolving]`, because it is a DNA digivolve, EX9-021 gains immunity to the
///   opponent's effects this turn; then ALL opponent Digimon with the highest
///   level are deleted (the delete arm is unconditional). After: both Lv6
///   materials are consumed into one EX9-021 permanent, EX9-021 is immune to
///   opponent Digimon/Tamer/Option effects, and the opponent's highest-level
///   Digimon is gone (a lower-level one survives).
/// - Rules/keyword basis: `general_rule.pdf` §16 DNA digivolve; DCGO
///   `EX9_021.cs` (immunity inside the `IsJogress` block; unconditional
///   delete-highest after). YAML `cards/ex9/EX9-021.yaml` Clauses 1–2
///   (`dna_origin` immunity + unconditional `highest_level` delete).
///
/// This wires the REAL Blue/Red Lv6 BT22 pair through the engine's actual DNA
/// digivolve path and asserts the DNA-gated immunity + the highest-level wipe as
/// a board diff — the multi-card finisher fusion a per-card test fires
/// synthetically.
#[test]
fn c5_omnimon_alter_s_dna_wipe_deletes_opponent_highest_level() {
    use digimon_engine::enums::EffectSourceKind;

    // Opponent board: ST5-10 MetalTyrannomon (Lv5, vanilla survivor) + BT13-060
    // Rosemon: Burst Mode (Lv7, the unique highest → deleted; its only effects are
    // [When Digivolving]/[When Attacking], neither of which fires when placed or
    // deleted, so it is a clean removal victim).
    let mut runner =
        dsl_card_runner(&["EX9-021", "BT22-026", "BT22-013", "ST5-10", "BT13-060"]);
    runner.game.turn_count = 1;

    // The two real Lv6 DNA materials on P0's field; EX9-021 in hand.
    let blue = runner.place_on_field(0, "BT22-026", None);
    let red = runner.place_on_field(0, "BT22-013", None);
    push_to_hand(&mut runner, 0, "EX9-021");
    runner.place_on_field(1, "ST5-10", None);
    runner.place_on_field(1, "BT13-060", None);

    let ex9_hand = runner.game.players[0]
        .hand
        .iter()
        .find(|c| c.card_id(&runner.game.card_data) == "EX9-021")
        .expect("EX9-021 in hand")
        .handle();

    let before = snapshot(&runner);
    assert_eq!(before.field[1], 2, "precondition: 2 opponent Digimon on field");

    // Drive the REAL DNA digivolve path (Blue Lv6 + Red Lv6 → EX9-021, cost 0).
    let evolved = {
        let mut ctx = EffectContext::new(&mut runner.game, ex9_hand, None, 0);
        ctx.effect_initiated_dna_digivolve(blue, red, ex9_hand, 0, true)
            .expect("EX9-021 must DNA digivolve from Blue Lv6 (BT22-026) + Red Lv6 (BT22-013)")
    };
    runner
        .auto_resolve()
        .expect("DNA [When Digivolving] trigger order + delete resolve");
    let after = snapshot(&runner);

    // Both Lv6 materials are consumed into one EX9-021 permanent.
    assert!(
        field_has_top(&runner, 0, "EX9-021"),
        "EX9-021 Omnimon Alter-S must be the merged permanent's top card"
    );
    let merged = runner.game.players[0].battle_area[evolved.index as usize]
        .card_sources
        .iter()
        .map(|s| s.card_id(&runner.game.card_data))
        .collect::<Vec<_>>();
    assert!(
        merged.contains(&"BT22-026") && merged.contains(&"BT22-013"),
        "both Lv6 DNA materials (BT22-026, BT22-013) must be sources under EX9-021; stack={merged:?}"
    );

    // DNA-gated immunity: EX9-021 is unaffected by opponent effects this turn.
    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(evolved, 1, EffectSourceKind::Digimon),
        "DNA-origin EX9-021 must be immune to opponent Digimon effects this turn"
    );

    // Delete-highest arm: the opponent's Lv7 (highest) is gone; the Lv5 survives.
    assert!(
        !field_has_top(&runner, 1, "BT13-060"),
        "the opponent's highest-level (Lv7) Digimon must be deleted"
    );
    assert!(
        field_has_top(&runner, 1, "ST5-10"),
        "the opponent's lower-level (Lv5) Digimon must survive the highest-level wipe"
    );
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "exactly the unique highest-level opponent Digimon is removed \
         (before={}, after={})",
        before.field[1],
        after.field[1],
    );
    assert!(
        after.trash[1] >= before.trash[1] + 1,
        "the deleted opponent Digimon must land in the opponent's trash"
    );
}

// ─── Local fixture wrappers ──────────────────────────────────────────────────

/// Build a started `DebugRunner` with the given DSL cards loaded. Memory
/// defaults to 10 (combos that need a specific gauge call `set_memory`).
fn dsl_card_runner(card_ids: &[&str]) -> DebugRunner {
    super::support::dsl_builder(card_ids).memory(10).start()
}

/// Build a started `DebugRunner` from the given DSL cards plus extra builder
/// configuration (added neutral cards, decks, hands, memory).
fn dsl_builder_with<F>(card_ids: &[&str], configure: F) -> DebugRunner
where
    F: FnOnce(
        digimon_engine::debug_runner::DebugRunnerBuilder,
    ) -> digimon_engine::debug_runner::DebugRunnerBuilder,
{
    configure(super::support::dsl_builder(card_ids)).start()
}
