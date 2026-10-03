//! BT4-111 Jack Raid — Option, Purple, Cost 0.
//!
//! Printed text (official Bandai DB, `data/card_bundles/BT4-111.md`):
//!   [Main] Gain 1 memory for every 10 cards in your trash.
//!   [Security] Gain 2 memory.
//!
//! Official Q&A: Option cards go to the trash AFTER their effects activate, so
//! with 9 cards in your trash the [Main] effect still sees 9 cards and you
//! gain no memory.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT4/Purple/BT4_111.cs
//!   - OptionSkill: `Owner.AddMemory(Owner.TrashCards.Count / 10)` (integer
//!     division, computed at resolution).
//!   - SecuritySkill: `Owner.AddMemory(2)`.
//!
//! Patterns: option main_from_hand formula memory gain (floor_div over
//! card_count_in_zone trash); blue [Security] on_security memory gain.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "BT4-111";

fn purple_tamer() -> CardData {
    let mut c = make_test_card("PURPLE-TAMER", "Purple Tamer");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Purple];
    c
}

fn attacker() -> CardData {
    let mut c = make_test_card("ATTACKER", "Attacker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Red];
    c
}

fn filler() -> CardData {
    make_test_card("FILL", "Filler")
}

/// Player 0 holds Jack Raid with `trash_n` cards in trash and a purple Tamer
/// (option colour requirement) on the field; memory starts at 0.
fn runner_with_trash(trash_n: usize) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT4-111 DSL card")
        .add_card(purple_tamer())
        .add_card(filler())
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(0)
        .start();
    r.place_on_field(0, "PURPLE-TAMER", Some(0));
    for _ in 0..trash_n {
        r.inject_trash(0, "FILL");
    }
    r.game.enter_main_phase();
    r.game.set_memory(0);
    r
}

fn use_jack_raid(r: &mut DebugRunner) {
    let res = r.game.play_option_from_hand(0, 0);
    assert_ne!(res, OptionPlayResult::Invalid, "Jack Raid must be usable");
    let _ = r.auto_resolve();
}

// ── Section 1: structure ────────────────────────────────────────────────────

#[test]
fn bt4_111_structure_main_and_security() {
    let r = runner_with_trash(0);
    let card = r.compiled_card(CARD_ID).expect("BT4-111 compiles");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(0));
    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 2, "exactly a [Main] and a [Security] clause");
    let main = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("[Main] clause");
    assert!(!main.optional);
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::GainMemoryFn { .. })),
        "[Main] gains a formula amount of memory; got {:?}",
        main.process
    );
    let sec = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("[Security] clause");
    assert_eq!(sec.process, vec![CompiledStep::GainMemory(2)]);
}

// ── Section 2/3: [Main] scaling with trash size ────────────────────────────

#[test]
fn bt4_111_main_with_20_trash_gains_2_memory() {
    let mut r = runner_with_trash(20);
    use_jack_raid(&mut r);
    assert_eq!(r.memory(), 2, "20 trash cards → +2 memory");
    assert_eq!(r.trash_size(0), 21, "Jack Raid itself goes to trash afterwards");
}

#[test]
fn bt4_111_main_with_19_trash_gains_1_memory() {
    let mut r = runner_with_trash(19);
    use_jack_raid(&mut r);
    assert_eq!(r.memory(), 1, "19 trash cards → floor(19/10) = +1 memory");
}

#[test]
fn bt4_111_main_with_10_trash_gains_1_memory() {
    let mut r = runner_with_trash(10);
    use_jack_raid(&mut r);
    assert_eq!(r.memory(), 1, "exactly 10 trash cards → +1 memory");
}

/// Official Q&A: the Option is not yet in the trash when its effect counts.
#[test]
fn bt4_111_main_with_9_trash_gains_nothing_option_not_counted() {
    let mut r = runner_with_trash(9);
    use_jack_raid(&mut r);
    assert_eq!(
        r.memory(),
        0,
        "9 trash cards → no memory (Jack Raid itself is not counted)"
    );
    assert_eq!(r.trash_size(0), 10);
}

#[test]
fn bt4_111_main_with_empty_trash_gains_nothing() {
    let mut r = runner_with_trash(0);
    use_jack_raid(&mut r);
    assert_eq!(r.memory(), 0);
}

#[test]
fn bt4_111_main_counts_only_own_trash() {
    let mut r = runner_with_trash(0);
    for _ in 0..20 {
        r.inject_trash(1, "FILL");
    }
    use_jack_raid(&mut r);
    assert_eq!(r.memory(), 0, "opponent's trash does not count");
}

// ── Section 3: [Security] ──────────────────────────────────────────────────

#[test]
fn bt4_111_security_gains_owner_2_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT4-111 DSL card")
        .add_card(attacker())
        .add_card(filler())
        .security(1, &[CARD_ID])
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .memory(3)
        .start();
    let atk = r.place_on_field(0, "ATTACKER", Some(0));
    let before = r.memory();
    let res = r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(res, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        r.memory(),
        before - 2,
        "player 1 (security owner) gains 2 memory → turn player's gauge drops by 2"
    );
    assert_eq!(r.trash_size(1), 1, "Jack Raid is trashed after its security effect");
}
