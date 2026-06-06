//! Thomas / DATA SQUAD (BT25 slice) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/thomas-data-squad-model.md`. Slice cards named for
//! this slice: BT25-087 Thomas H. Norstein, BT25-096 Mirage Beast Knight,
//! BT25-002 Wanyamon, BT25-027 MachGaogamon, BT25-029 MirageGaogamon,
//! BT25-104 ShineGreymon: Burst Mode.
//!
//! Phase-4 precondition gate (`qa/qa-reports/validated_cards_dsl.json`,
//! 2026-06-06): only **BT25-002** (Wanyamon) and **BT25-027** (MachGaogamon)
//! are IMPLEMENTED. BT25-087 / BT25-096 / BT25-029 / BT25-104 are BLOCKED on
//! engine gaps (OnAddToHand inert; BeforePayCost selection-bearing pay_cost
//! Parked-drop; Option-Main cross-side activation; treated-as-Digimon
//! name-overlay aura — all in `docs/RUST_ENGINE_GAPS.md`). Per the skill, every
//! combo naming a BLOCKED card is reported blocked and NOT authored here — the
//! dropped combos (Thomas stash-build chain, MirageGaogamon double-bounce loop,
//! Mirage Beast Knight shortcut, ShineGreymon BM Option-Main + Marcus aura) are
//! enumerated in the model doc's "Blocked combos" section.
//!
//! The unifying archetype resource is the **face-down stash under a Tamer**:
//! Thomas (087) builds it, the Gaogamon line spends it. With 087 BLOCKED, the
//! only IMPLEMENTED stash *consumer* is MachGaogamon (027); these tests seed
//! the face-down stash directly on a synthesized DATA SQUAD Tamer (the cross-set
//! evolution / Tamer prerequisites are **synthesized fixtures** — neutral cards
//! carrying only the level / trait / colour a combo needs). No combo fires a
//! cross-set card's *printed* effect (the DATA SQUAD Tamer in C3 is a
//! name/trait-only target whose own effect never resolves), so no cross-set
//! implementation is pulled (lazy closure).
//!
//! Source priority (CLAUDE.md): printed card face + DCGO C#
//! (`$BASE_DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_0{02,27}.cs`) outrank
//! cards.json; each `code/digimon-engine/cards/bt25/BT25-0xx.yaml` header carries
//! the per-card DCGO crosscheck each combo below relies on. `general_rule.pdf`
//! §16 governs digivolution / WD-WA / replacement timing.
//!
//! Combos authored (ranked by play-frequency × payoff-centrality):
//!   C1 — MachGaogamon (027) WA: bounce an opp lvl≤4 AND, by trashing the
//!        face-down stash under a Tamer, unsuspend itself (the core engine).
//!   C2 — the C1 mirror with NO face-down stash: the bounce still resolves but
//!        the self-unsuspend cost is unpaid → it stays suspended (the gate).
//!   C3 — Wanyamon (002) buried as the inherited bottom source under a REAL
//!        MachGaogamon (027): playing a DATA SQUAD Tamer makes BOTH players
//!        draw 1 — the inherited trigger firing from inside a real two-card stack.
//!   C4 — MachGaogamon (027) stash contention: the WA self-unsuspend and the
//!        leave-prevention replacement draw from the SAME single face-down
//!        source; once the WA chain consumes it, a later leave attempt cannot pay.

#![allow(dead_code)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const WANYAMON: &str = "BT25-002";
const MACHGAOGAMON: &str = "BT25-027";

// ─── Synthesized combo-piece fixtures ────────────────────────────────────────
//
// Cross-set evolution / Tamer prerequisites are neutral cards carrying ONLY the
// level / trait / colour / evo-cost a combo needs — none has a printed effect
// that fires, so no cross-set implementation is pulled (lazy closure).

/// A synthetic DATA SQUAD Tamer. NAME/TRAIT-ONLY: it is only ever a stash
/// carrier (C1/C2/C4) or the "DATA SQUAD Tamer played" trigger source for
/// Wanyamon (C3); its own printed effect never resolves, so the real Thomas
/// (BT25-087, BLOCKED) is NOT pulled.
fn ds_tamer(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Blue];
    card.play_cost = 3;
    card.traits = vec!["DATA SQUAD".to_string()];
    card
}

/// A neutral lvl-4 DATA SQUAD Digimon — the legal alt-digivolve BASE for
/// MachGaogamon's `AddSelfDigivolutionRequirementStaticEffect(level 4, DATA
/// SQUAD, cost 3)`. Synthetic: no printed effect of its own.
fn ds_lv4_base(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 4;
    card.traits = vec!["DATA SQUAD".to_string()];
    card
}

/// A neutral opponent Digimon at the given level — the C1/C4 bounce target.
fn opp_digimon(card_id: &str, name: &str, level: u8) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(level);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn find_field(runner: &DebugRunner, player: u8, card_id: &str) -> Option<PermanentHandle> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .map(|i| PermanentHandle {
            player,
            index: i as u8,
        })
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let iid = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, iid));
    runner.game.players[player as usize].hand.len() - 1
}

/// Walk any pending-selection chain, always taking the first non-PASS action
/// (falling back to PASS) — exercises the full accept path. Mirrors the
/// per-card `bt25_027` driver.
fn drive_take_targets(runner: &mut DebugRunner) {
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 32, "selection chain did not terminate");
        let actions: Vec<u16> = runner
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .collect();
        if let Some(a) = actions.iter().copied().find(|a| *a != PASS) {
            if runner.execute_action(0, a).is_ok() {
                continue;
            }
        }
        let _ = runner.execute_action(0, PASS);
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C1 — MachGaogamon (027) WA: bounce opp lvl≤4 AND self-unsuspend by
//            trashing the face-down stash under a Tamer (the core engine)
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-027 [WD][WA][OPT]: "You may return 1 of your opponent's level 4 or lower
// Digimon to the hand. Then, by trashing the bottom face-down card from under
// any of your Tamers, this Digimon unsuspends." This stages the WHOLE archetype
// engine in one go: a Gaogamon-line body spends a Thomas-style face-down stash to
// both clear the opponent's board AND ready itself to attack again. The per-card
// `bt25_027` test pins the WA path too, but only ever beside a bare synthetic
// Tamer; here the stash sits under a DATA SQUAD Tamer (the real archetype
// carrier) and we assert the *combined* outcome: bounce (opp hand +1) AND
// stash-trash (own trash +1) AND self-unsuspend in a single resolution.
//
// Sources: BT25-027.yaml (WD/WA: select_opponent_permanent lvl_lte 4 →
// return_to_hand; trash_bottom_face_down_source_under_tamer → unsuspend source)
// + DCGO BT25_027.cs (Mode.Bounce then `if (trashed)` unsuspend).
// general_rule.pdf §16 WD/WA + digivolve-cost timing.

/// Happy path: MachGaogamon attacks; it bounces the opponent's lvl-4 Digimon
/// AND trashes the face-down stash under the DATA SQUAD Tamer to unsuspend
/// itself — the full bounce + stash-spend + ready-up chain in one resolution.
#[test]
fn c1_mach_wa_bounces_then_trashes_stash_to_unsuspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card(MACHGAOGAMON)
        .expect("BT25-027 (MachGaogamon) in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(opp_digimon("OPP-L4", "Opp Lv4", 4))
        .add_card(filler("STASH"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 3; // no summoning sickness on the attacker
    runner.game.turn_player_idx = 0;
    runner.game.players[1].security.clear(); // attack resolves cleanly

    // MachGaogamon on field, placed on turn 0 (no summoning sickness at turn 3)
    // and unsuspended so it can declare an attack (attacking re-suspends it; the
    // WA effect then unsuspends it again after paying the stash cost).
    let mach = runner.place_on_field(0, MACHGAOGAMON, Some(0));
    runner.game.players[0].battle_area[mach.index as usize].is_suspended = false;

    // A DATA SQUAD Tamer carrying a single face-down stash source — the fuel.
    let tamer = runner.place_stack(0, &["STASH", "DS-TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;

    // Opponent lvl-4 Digimon — the bounce target.
    runner.place_on_field(1, "OPP-L4", None);

    let opp_hand_before = runner.hand_size(1);
    let trash_before = runner.trash_size(0);

    runner.attack_player(mach, 1, false);
    drive_take_targets(&mut runner);

    assert_eq!(
        runner.hand_size(1),
        opp_hand_before + 1,
        "C1: the opponent's lvl-4 Digimon is bounced to hand"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "C1: the bottom face-down stash under the DATA SQUAD Tamer is trashed as the unsuspend cost"
    );
    let mach2 = find_field(&runner, 0, MACHGAOGAMON).expect("MachGaogamon still on field");
    assert!(
        !runner.game.players[0].battle_area[mach2.index as usize].is_suspended,
        "C1: MachGaogamon unsuspends after paying the face-down stash cost"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C2 — the C1 mirror with NO face-down stash (the gate)
// ══════════════════════════════════════════════════════════════════════════
//
// The "Then, ... this Digimon unsuspends" tail is gated on actually paying the
// trash cost (DCGO's `if (trashed)`). The system fact that makes the archetype
// resource-bounded: without a face-down stash the bounce STILL happens, but the
// self-unsuspend never does. This is the cross-card proof that Thomas's stash
// (BLOCKED here, so absent) is what powers the recurring attacks — strip it and
// MachGaogamon bounces once then sits suspended.

/// Unhappy path: a DATA SQUAD Tamer is present but carries NO face-down stash.
/// The WA bounce still resolves, but with nothing to trash MachGaogamon stays
/// suspended — the stash gates the self-unsuspend, not the bounce.
#[test]
fn c2_mach_wa_without_stash_bounces_but_stays_suspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card(MACHGAOGAMON)
        .expect("BT25-027 (MachGaogamon) in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(opp_digimon("OPP-L4", "Opp Lv4", 4))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;
    runner.game.players[1].security.clear();

    let mach = runner.place_on_field(0, MACHGAOGAMON, Some(0));
    runner.game.players[0].battle_area[mach.index as usize].is_suspended = false;
    // Face-UP Tamer only — no face-down stash source to pay the unsuspend cost.
    runner.place_stack(0, &["DS-TAMER"]);
    runner.place_on_field(1, "OPP-L4", None);

    let opp_hand_before = runner.hand_size(1);
    let trash_before = runner.trash_size(0);

    runner.attack_player(mach, 1, false);
    drive_take_targets(&mut runner);

    assert_eq!(
        runner.hand_size(1),
        opp_hand_before + 1,
        "C2: the bounce resolves regardless of the stash (no face-down to trash)"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "C2: nothing is trashed — there is no face-down stash to pay the unsuspend cost"
    );
    let mach2 = find_field(&runner, 0, MACHGAOGAMON).expect("MachGaogamon on field");
    assert!(
        runner.game.players[0].battle_area[mach2.index as usize].is_suspended,
        "C2: with no stash to trash the unsuspend cost is unpaid → MachGaogamon stays suspended"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C3 — Wanyamon (002) inherited draw firing from under a REAL
//            MachGaogamon (027) stack on a DATA SQUAD Tamer play
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-002 inherited [Your Turn][OPT]: "When any of your [DATA SQUAD] trait
// Tamers are played, both players <Draw 1>." Wanyamon is an inherited-source
// body that rides under the Gaogamon line. The cross-card system fact: the
// inherited trigger fires from the BOTTOM of a real two-card digivolution stack
// (Wanyamon under a real MachGaogamon), not just under a synthetic carrier as the
// per-card `bt25_002` test stages. Playing a DATA SQUAD Tamer draws BOTH players 1.
//
// Sources: BT25-002.yaml (scope: inherited, on_ally_played, your_turn,
// event_target {tamer, DATA SQUAD} → draw you + opponent) + DCGO BT25_002.cs
// (OnEnterFieldAnyone, owner-turn, Players_ForTurnPlayer draw loop).
// general_rule.pdf §16 inherited-effect activation from a digivolution source.

/// Wanyamon buried as the bottom inherited source under a real MachGaogamon;
/// playing a DATA SQUAD Tamer on your turn draws 1 for BOTH players.
#[test]
fn c3_wanyamon_under_mach_draws_both_players_on_ds_tamer_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(WANYAMON)
        .expect("BT25-002 (Wanyamon) in embedded DSL pack")
        .dsl_card(MACHGAOGAMON)
        .expect("BT25-027 (MachGaogamon) in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(filler("MY-DRAW"))
        .add_card(filler("OPP-DRAW"))
        .hand(0, &["DS-TAMER"])
        .deck(0, &["MY-DRAW"; 4])
        .deck(1, &["OPP-DRAW"; 4])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;

    // A real two-card digivolution stack: Wanyamon (inherited source) at the
    // bottom, a real MachGaogamon on top.
    runner.place_stack(0, &[WANYAMON, MACHGAOGAMON]);

    let my_deck_before = runner.deck_size(0);
    let opp_deck_before = runner.deck_size(1);
    let my_hand_before = runner.hand_size(0); // includes the Tamer about to leave hand

    let h = push_to_hand(&mut runner, 0, "DS-TAMER");
    // (DS-TAMER was seeded both via hand() and pushed; play the freshly-pushed one.)
    runner.play(0, h).expect("play the DATA SQUAD Tamer");
    runner.auto_resolve().expect("resolve the inherited both-players draw");

    assert_eq!(
        runner.deck_size(0),
        my_deck_before - 1,
        "C3: you draw 1 from Wanyamon's inherited trigger under the real MachGaogamon stack"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before - 1,
        "C3: the opponent also draws 1 (both players draw)"
    );
    // Net own hand: +1 pushed Tamer, then −1 played, +1 drawn = my_hand_before+1.
    assert_eq!(
        runner.hand_size(0),
        my_hand_before + 1,
        "C3: own hand net = (played Tamer −1) + (Wanyamon draw +1) over the pushed-Tamer baseline"
    );
}

/// Negative leg: on the OPPONENT's turn the inherited [Your Turn] trigger must
/// not fire even when a DATA SQUAD Tamer enters — the cross-card timing gate that
/// keeps the symmetric draw from leaking onto the opponent's plays.
#[test]
fn c3_wanyamon_under_mach_does_not_draw_on_opponent_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(WANYAMON)
        .expect("BT25-002 (Wanyamon) in embedded DSL pack")
        .dsl_card(MACHGAOGAMON)
        .expect("BT25-027 (MachGaogamon) in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(filler("MY-DRAW"))
        .add_card(filler("OPP-DRAW"))
        .deck(0, &["MY-DRAW"; 4])
        .deck(1, &["OPP-DRAW"; 4])
        .memory(10)
        .start();
    runner.place_stack(0, &[WANYAMON, MACHGAOGAMON]);

    runner.end_turn(); // now player 1's turn
    assert_eq!(runner.turn_player(), 1, "it is the opponent's turn");

    let my_deck_before = runner.deck_size(0);
    // Player 1 plays a DATA SQUAD Tamer of their own; player 0's inherited
    // [Your Turn] effect must NOT fire on the opponent's turn.
    let p1_idx = push_to_hand(&mut runner, 1, "DS-TAMER");
    let _ = runner.play(1, p1_idx);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        my_deck_before,
        "C3: player 0's inherited [Your Turn] Wanyamon draw must not fire on the opponent's turn"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Combo C4 — MachGaogamon (027) stash contention: WA self-unsuspend and
//            leave-prevention draw from the SAME single face-down source
// ══════════════════════════════════════════════════════════════════════════
//
// BT25-027 has TWO clauses that both pay "trash the bottom face-down card from
// under any of your Tamers": the WD/WA self-unsuspend AND the [All Turns]
// leave-prevention replacement. They share the same finite stash. The cross-clause
// system fact a single-trigger per-card test cannot express: with exactly ONE
// face-down source, the WA chain consumes it, so a SUBSEQUENT deletion attempt
// finds no stash to pay and MachGaogamon actually leaves. (Per-card bt25_027
// tests each clause in isolation, each beside its own fresh single stash.)
//
// Sources: BT25-027.yaml (both the WD/WA tail and the replacement clause call
// trash_bottom_face_down_source_under_tamer) + DCGO BT25_027.cs (shared WD/WA tail
// + WhenRemoveField replacement both `TrashDigivolutionCardsFromTopOrBottom(... ,
// isFromTop:false, CanSelectTrashSourceCardCondition=IsFlipped)`).
// general_rule.pdf §16 replacement timing + cost payment.

/// With a single face-down stash, the WA self-unsuspend consumes it; a later
/// deletion attempt then has no stash to pay the leave-prevention, so
/// MachGaogamon leaves the battle area.
#[test]
fn c4_mach_wa_consumes_only_stash_leaving_leave_prevention_unpayable() {
    let mut runner = DebugRunner::builder()
        .dsl_card(MACHGAOGAMON)
        .expect("BT25-027 (MachGaogamon) in embedded DSL pack")
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(opp_digimon("OPP-L4", "Opp Lv4", 4))
        .add_card(filler("STASH"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 0;
    runner.game.players[1].security.clear();

    let mach = runner.place_on_field(0, MACHGAOGAMON, Some(0));
    runner.game.players[0].battle_area[mach.index as usize].is_suspended = false;
    // EXACTLY ONE face-down stash source — the contested resource.
    let tamer = runner.place_stack(0, &["STASH", "DS-TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;
    runner.place_on_field(1, "OPP-L4", None);

    // Step 1 — WA chain consumes the only face-down stash to unsuspend.
    let trash_before = runner.trash_size(0);
    runner.attack_player(mach, 1, false);
    drive_take_targets(&mut runner);
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "C4: the WA self-unsuspend trashes the single face-down stash"
    );
    let mach2 = find_field(&runner, 0, MACHGAOGAMON).expect("MachGaogamon still on field after WA");

    // Step 2 — now delete MachGaogamon. The leave-prevention clause would trash a
    // bottom face-down source, but the WA chain already consumed the only one.
    let trash_after_wa = runner.trash_size(0);
    runner
        .game
        .delete_permanent_with_cause(mach2, ReplacementCause::OpponentEffect);

    // Drive any replacement prompt (taking non-PASS first) — but with no stash to
    // pay, the replacement cannot complete and MachGaogamon leaves.
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 32, "selection chain did not terminate");
        // The leave-prevention is a Replacement; with no face-down stash left to
        // trash its only legal action is PASS. Take the first non-PASS where one
        // exists (to fully exercise any accept path), else PASS.
        let actions: Vec<u16> = runner
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .copied()
            .collect();
        let pick = actions.iter().copied().find(|a| *a != PASS).unwrap_or(PASS);
        if runner.execute_action(0, pick).is_err() {
            let _ = runner.execute_action(0, PASS);
        }
    }

    assert!(
        find_field(&runner, 0, MACHGAOGAMON).is_none(),
        "C4: the single stash was already spent on the WA unsuspend, so the leave-prevention \
         cannot pay its trash cost and MachGaogamon leaves the battle area"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_after_wa + 1,
        "C4: only the MachGaogamon body reaches trash on deletion — no additional face-down \
         stash is trashed (there was none left to pay the leave-prevention)"
    );
}
