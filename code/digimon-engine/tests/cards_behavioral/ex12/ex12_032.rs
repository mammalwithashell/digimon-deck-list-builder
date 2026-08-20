use digimon_engine::enums::{CardColor, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::{enums::EffectTiming, permanent::PermanentHandle};

use super::support::{
    plain_digimon, push_to_trash, select_first_non_pass, tamer, vb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-032";

#[test]
fn ex12_032_dna_materials_are_printed_level4_pairs() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .start();
    let card = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == CARD_ID)
        .expect("EX12-032 card data registered");
    let dna = card
        .dna_costs
        .first()
        .expect("EX12-032 should expose its DNA digivolve route");

    assert_eq!(dna.requirement1.level, 4);
    assert_eq!(dna.requirement2.level, 4);
    assert_eq!(dna.memory_cost, 0);
    assert!(dna.requirement1.card_colors.contains(&CardColor::Blue));
    assert!(dna.requirement1.card_colors.contains(&CardColor::Yellow));
    assert!(dna.requirement2.card_colors.contains(&CardColor::Purple));
    assert!(dna.requirement2.card_colors.contains(&CardColor::Red));
}

fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

#[test]
fn ex12_032_on_play_locks_opponent_tamer_from_suspending() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(tamer("OPP-TAMER", CardColor::Blue))
        .start();

    let weregarurumon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.fire_on_play(0, weregarurumon.index as usize);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve EX12-032 lock");

    assert!(
        runner
            .game
            .modifiers
            .has(opp_tamer, ModifierType::CannotSuspend),
        "chosen opponent Tamer should receive CannotSuspend"
    );
}

#[test]
fn ex12_032_when_attacking_requires_same_level_source_pair() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(plain_digimon("SRC-L3", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("SRC-L4", CardColor::Blue, 4, 4000))
        .add_card(vb_digimon("TRASH-GARURUMON", CardColor::Blue, 6, 12000))
        .start();

    let weregarurumon = runner.place_stack(0, &["SRC-L3", "SRC-L4", CARD_ID]);
    push_to_trash(&mut runner, 0, "TRASH-GARURUMON");

    fire_when_attacking(&mut runner, weregarurumon);

    assert!(
        runner.pending_selection_view().is_none(),
        "no same-level source pair means the trash-digivolve prompt must not install"
    );
}

#[test]
fn ex12_032_when_attacking_prompts_with_same_level_source_pair() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(plain_digimon("SRC-L4A", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("SRC-L4B", CardColor::Purple, 4, 4000))
        .add_card(vb_digimon("TRASH-GARURUMON", CardColor::Blue, 6, 12000))
        .start();

    let weregarurumon = runner.place_stack(0, &["SRC-L4A", "SRC-L4B", CARD_ID]);
    push_to_trash(&mut runner, 0, "TRASH-GARURUMON");

    fire_when_attacking(&mut runner, weregarurumon);

    let offer = runner
        .pending_selection_view()
        .expect("same-level source pair should offer trash digivolution");
    assert_eq!(offer.kind, SelectionKind::Replacement);
    assert!(
        offer.is_optional,
        "printed 'may digivolve' must be declinable"
    );
    select_first_non_pass(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("accepting the optional trigger should expose trash candidates");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert!(
        !view.valid_action_ids.is_empty(),
        "matching trash Digimon should be selectable"
    );
}

/// A REAL attack must open the `[When Attacking]` window before the security
/// check resolves.
///
/// Every other `[When Attacking]` test in this file fires the trigger by hand
/// (`enqueue_triggered` + `drain_effect_queue`), which proves the effect works
/// but says nothing about *when* the attack flow reaches it. The DCGO parity
/// corpus found the gap: recording `20260820T154511Z` has EX12-032 (7000 DP)
/// attack security, survive, and digivolve — while replaying the same actions
/// through this engine deleted it, leaving the attacker and its whole
/// digivolution stack in the trash and the follow-up digivolve illegal.
///
/// A 7000 DP attacker into a 12000 DP security Digimon is the discriminating
/// case: if the security check resolves first the attacker is already dead, and
/// the window it was entitled to never opens.
#[test]
fn ex12_032_when_attacking_window_opens_before_the_security_check() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(plain_digimon("SRC-L4A", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("SRC-L4B", CardColor::Purple, 4, 4000))
        .add_card(vb_digimon("TRASH-VB-L6", CardColor::Blue, 6, 12000))
        .add_card(plain_digimon("SEC-BIG", CardColor::Red, 6, 12000))
        .security(1, &["SEC-BIG"])
        .start();

    let attacker = runner.place_stack(0, &["SRC-L4A", "SRC-L4B", CARD_ID]);
    push_to_trash(&mut runner, 0, "TRASH-VB-L6");

    assert_eq!(
        runner.effective_dp(attacker),
        Some(7000),
        "attacker should be the printed 7000 DP before any digivolution"
    );

    let _ = runner.attack_player(attacker, 1, false);

    let attacker_alive = runner
        .game
        .player(0)
        .battle_area
        .get(attacker.index as usize)
        .is_some();

    assert!(
        attacker_alive,
        "the security check deleted the attacker before its [When Attacking] \
         window opened. P0 trash now holds {:?}. DCGO resolves the trigger \
         first, which is what lets the attacker digivolve out of lethal range.",
        runner
            .game
            .player(0)
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect::<Vec<_>>()
    );

    let offer = runner.pending_selection_view().expect(
        "a live same-level source pair must offer the [When Attacking] \
         digivolution during a real attack, not only when the trigger is \
         enqueued by hand",
    );
    assert!(
        offer.is_optional,
        "printed 'may digivolve' must stay declinable in the attack flow"
    );
}

/// Control for the test above: with no same-level source pair the trigger's
/// `active_when` fails, so no window is owed and the 7000 DP attacker really
/// should die to a 12000 DP security Digimon.
///
/// Without this, the test above could pass for the wrong reason — an attacker
/// that survives because the security check never happened at all would look
/// identical to one that survived because it got its window.
#[test]
fn ex12_032_without_a_same_level_pair_the_security_check_deletes_the_attacker() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-032 YAML loads")
        .add_card(plain_digimon("SRC-L3", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("SRC-L4A", CardColor::Blue, 4, 4000))
        .add_card(vb_digimon("TRASH-VB-L6", CardColor::Blue, 6, 12000))
        .add_card(plain_digimon("SEC-BIG", CardColor::Red, 6, 12000))
        .security(1, &["SEC-BIG"])
        .start();

    // Levels 3 and 4 — no pair, so `self_same_level_source_pairs_gte: 1` fails.
    let attacker = runner.place_stack(0, &["SRC-L3", "SRC-L4A", CARD_ID]);
    push_to_trash(&mut runner, 0, "TRASH-VB-L6");

    let _ = runner.attack_player(attacker, 1, false);

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .get(attacker.index as usize)
            .is_none(),
        "with no [When Attacking] window owed, a 7000 DP attacker must lose to \
         a 12000 DP security Digimon — if it survives here the security check \
         is not running and the sibling test proves nothing"
    );
}
