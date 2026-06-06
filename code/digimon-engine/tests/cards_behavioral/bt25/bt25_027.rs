//! BT25-027 MachGaogamon.
//!
//! Printed text (cards.json + image):
//!   [When Digivolving] [When Attacking] [Once Per Turn] You may return 1 of
//!   your opponent's level 4 or lower Digimon to the hand. Then, by trashing
//!   the bottom face-down card from under any of your Tamers, this Digimon
//!   unsuspends.
//!   [All Turns] [Once Per Turn] When this Digimon would leave the battle area,
//!   by trashing the bottom face-down card from under any of your Tamers, it
//!   doesn't leave.
//!   Inherited:
//!   [All Turns] [Once Per Turn] When this Digimon with [Gaogamon] in its name
//!   or the [DATA SQUAD] trait would leave the battle area, by trashing the
//!   bottom face-down card from under any of your Tamers, it doesn't leave.
//!
//! DCGO ref: DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_027.cs
//!   - Alt-digivolve: AddSelfDigivolutionRequirementStaticEffect(level 4,
//!     DATA SQUAD top-card, cost 3).
//!   - Shared WD/WA: bounce opp lvl<=4 (Mode.Bounce, canNoSelect) THEN select a
//!     Tamer with a face-down source, trash bottom face-down, then unsuspend
//!     self (if trashed & suspended). OPT, optional.
//!   - WhenRemoveField (main + inherited): pick Tamer, trash bottom face-down,
//!     set willBeRemoveField=false. OPT, optional.
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): alt-digivolve path; WD/WA shared
//!   trigger; optional bounce; Tamer face-down stash cost; self-unsuspend;
//!   replacement (leave-prevention) main + inherited; OPT lockout.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-027";

fn ds_tamer(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Blue];
    card.play_cost = 3;
    card.traits = vec!["DATA SQUAD".to_string()];
    card
}

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

/// MachGaogamon on field (player 0), suspended; a DATA SQUAD Tamer carrying a
/// face-down stash source; opponent lvl-4 + lvl-5 Digimon. Memory high.
fn runner_field_mach_suspended_with_stash_and_opp() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-027 YAML loads")
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(opp_digimon("OPP-L4", "Opp Lv4", 4))
        .add_card(opp_digimon("OPP-L5", "Opp Lv5", 5))
        .add_card(filler("STASH"))
        .memory(10)
        .start();
    let mach = runner.place_on_field(0, CARD_ID, None);
    runner.game.players[0].battle_area[mach.index as usize].is_suspended = true;
    let tamer = runner.place_stack(0, &["STASH", "DS-TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;
    runner.place_on_field(1, "OPP-L4", None);
    runner.place_on_field(1, "OPP-L5", None);
    runner
}

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

/// Walk any pending-selection chain, always taking the first non-PASS action
/// (falling back to PASS). Used to exercise the full accept path.
fn drive_take_targets(runner: &mut DebugRunner) {
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 32, "selection chain did not terminate");
        let actions: Vec<u32> = runner
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

// ---------------------------------------------------------------------------
// Section 1 — structural
// ---------------------------------------------------------------------------

#[test]
fn bt25_027_structural_clauses_and_alt_path() {
    let runner = runner_field_mach_suspended_with_stash_and_opp();
    let card = runner.compiled_card(CARD_ID).expect("BT25-027 present");

    assert_eq!(card.name, "MachGaogamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    assert!(!card.alt_paths.is_empty(), "alt-digivolve onto DATA SQUAD");

    let triggered: Vec<&_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered.iter().any(|t| t.when.contains(&CompiledTiming::WhenDigivolving)
            && t.when.contains(&CompiledTiming::WhenAttacking)),
        "WD/WA shared bounce+unsuspend clause"
    );

    // One main + one inherited leave-prevention replacement.
    let declaratives: Vec<&_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();
    assert!(
        declaratives.len() >= 2,
        "main + inherited leave-prevention replacements present (got {})",
        declaratives.len()
    );
    assert!(
        card.effects.iter().any(|c| matches!(c, CompiledClause::Declarative(_))
            && matches!(c, CompiledClause::Declarative(d) if format!("{d:?}").contains("Replacement"))),
        "at least one replacement clause"
    );
    // Inherited leave-prevention exists.
    assert!(
        card.effects.iter().any(|c| match c {
            CompiledClause::Declarative(d) => format!("{d:?}").contains("Inherited"),
            CompiledClause::Triggered(t) => t.scope == CompiledScope::Inherited,
            _ => false,
        }),
        "inherited leave-prevention clause present"
    );
}

// ---------------------------------------------------------------------------
// Section 3 — WD/WA: bounce opp lvl<=4, trash face-down, unsuspend self
// ---------------------------------------------------------------------------

#[test]
fn bt25_027_wa_bounce_lv4_trash_facedown_unsuspends_self() {
    let mut runner = runner_field_mach_suspended_with_stash_and_opp();
    let mach = find_field(&runner, 0, CARD_ID).expect("mach on field");

    let opp_hand_before = runner.hand_size(1);
    let trash_before = runner.trash_size(0);

    runner.attack_player(0, mach.index as usize);
    drive_take_targets(&mut runner);

    assert_eq!(
        runner.hand_size(1),
        opp_hand_before + 1,
        "opponent's lvl-4 Digimon returned to hand"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "bottom face-down stash source trashed as cost"
    );
    let mach2 = find_field(&runner, 0, CARD_ID).expect("mach still on field");
    assert!(
        !runner.game.players[0].battle_area[mach2.index as usize].is_suspended,
        "MachGaogamon unsuspends after paying the trash cost"
    );
}

#[test]
fn bt25_027_wa_no_stash_does_not_unsuspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-027 YAML loads")
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(opp_digimon("OPP-L4", "Opp Lv4", 4))
        .memory(10)
        .start();
    let mach = runner.place_on_field(0, CARD_ID, None);
    runner.game.players[0].battle_area[mach.index as usize].is_suspended = true;
    runner.place_stack(0, &["DS-TAMER"]); // face-up only, no stash
    runner.place_on_field(1, "OPP-L4", None);

    runner.attack_player(0, mach.index as usize);
    drive_take_targets(&mut runner);

    let mach2 = find_field(&runner, 0, CARD_ID).expect("mach on field");
    assert!(
        runner.game.players[0].battle_area[mach2.index as usize].is_suspended,
        "no face-down stash to trash → unsuspend cost unpaid → stays suspended"
    );
}

// ---------------------------------------------------------------------------
// Section 3b — leave-prevention replacement (main effect)
// ---------------------------------------------------------------------------

#[test]
fn bt25_027_leave_prevention_trashes_facedown_and_stays() {
    let mut runner = runner_field_mach_suspended_with_stash_and_opp();
    let mach = find_field(&runner, 0, CARD_ID).expect("mach on field");
    let trash_before = runner.trash_size(0);

    runner
        .game
        .delete_permanent_with_cause(mach, ReplacementCause::OpponentEffect);

    let mut guard = 0;
    let mut saw_replacement = false;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 32);
        let kind = runner.pending_selection().unwrap().kind.clone();
        if matches!(kind, SelectionKind::Replacement) {
            saw_replacement = true;
        }
        let actions: Vec<u32> = runner
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

    assert!(saw_replacement, "a leave-prevention replacement prompt was offered");
    assert!(
        find_field(&runner, 0, CARD_ID).is_some(),
        "MachGaogamon does not leave the battle area after paying the cost"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "bottom face-down stash source trashed to prevent leaving"
    );
}

#[test]
fn bt25_027_leave_prevention_declined_lets_it_leave() {
    let mut runner = runner_field_mach_suspended_with_stash_and_opp();
    let mach = find_field(&runner, 0, CARD_ID).expect("mach on field");

    runner
        .game
        .delete_permanent_with_cause(mach, ReplacementCause::OpponentEffect);

    if runner.pending_selection().is_some() {
        let _ = runner.execute_action(0, PASS);
    }
    let _ = runner.auto_resolve();

    assert!(
        find_field(&runner, 0, CARD_ID).is_none(),
        "declining leave-prevention lets MachGaogamon leave"
    );
}
