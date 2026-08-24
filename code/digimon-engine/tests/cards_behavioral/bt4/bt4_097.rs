//! BT4-097 Kari Kamiya
//!
//! Printed text (card face + official Bandai DB, `data/card_bundles/BT4-097.md`):
//! [All Turns] When a card is removed from your security stack, you may suspend
//! this Tamer to gain 1 memory.
//! (Older/JP-derived wording of the same clause: "by suspending this Tamer,
//! gain 1 memory" — the shape §15-7-1 names an OPTIONAL PROCESSING CONDITION.)
//!
//! [Security] Play this card without paying its memory cost.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT4/Purple/BT4_097.cs
//! (`BT4_097.cs:19` — `SetUpActivateClass(..., -1, true, ...)`, i.e.
//! isOptional = true: DCGO asks the player whether to pay the suspend.)

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

const KARI_YAML: &str = include_str!("../../../cards/bt4/BT4-097.yaml");

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn kari_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(KARI_YAML)
        .expect("BT4-097 YAML parses")
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SEC"))
        .add_card(make_filler("FILLER-DECK"))
        .security(1, &["SEC", "SEC"])
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start()
}

#[test]
fn bt4_097_has_own_security_removed_and_security_clauses() {
    let runner = kari_runner();
    let compiled = runner.compiled_card("BT4-097").expect("compiled Kari");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 2);
    let own_sec = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("own security removed clause");
    assert_eq!(own_sec.scope, CompiledScope::FaceUp);
    assert!(
        own_sec.active_when.is_some(),
        "Kari's [All Turns] clause should carry an active_when gate"
    );
    // §15-7-1: "Optional processing conditions include text such as 'by X, Y.'"
    // Kari's "you may suspend this Tamer to gain 1 memory" is exactly that
    // shape, so §15-7-4 gives the player the choice of whether to pay it.
    // DCGO agrees (BT4_097.cs:19 passes isOptional = true). Rule 17 requires
    // the decline to be reachable from the action space, which means the
    // clause must be `optional: true` + `outer_prompt: true` (the body's first
    // step, a bare `suspend:`, is not itself declinable).
    assert!(
        own_sec.optional,
        "Kari's suspend cost is an optional processing condition (§15-7-1/§15-7-4)"
    );

    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnSecurity)),
        "Kari should retain her [Security] play clause"
    );
}

#[test]
fn bt4_097_own_security_removed_suspends_kari_and_gains_memory() {
    let mut runner = kari_runner();
    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    assert!(
        !runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "Kari starts unsuspended"
    );
    let before = runner.memory();

    runner.attack_player(attacker, 1, true);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "Kari should suspend herself as the activation cost"
    );
    assert_eq!(
        runner.memory(),
        before - 1,
        "P1 gaining 1 memory moves the memory counter one step toward P1"
    );
}

#[test]
fn bt4_097_does_not_fire_for_opponents_security_removed() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(KARI_YAML)
        .expect("BT4-097 YAML parses")
        .add_card(make_filler("ATTACKER-P1"))
        .add_card(make_filler("SEC-P0"))
        .add_card(make_filler("FILLER-DECK"))
        .security(0, &["SEC-P0"])
        .deck(0, &["FILLER-DECK"])
        .deck(1, &["FILLER-DECK"])
        .memory(5)
        .start();

    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    let before = runner.memory();

    runner.attack_player(attacker, 0, true);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "Kari should not react when the opponent's security is removed"
    );
    assert_eq!(runner.memory(), before);
    assert!(
        !matches!(runner.pending_kind(), Some(SelectionKind::OwnField)),
        "no hidden Kari follow-up selection should be pending"
    );
}

#[test]
fn bt4_097_already_suspended_does_not_fire() {
    let mut runner = kari_runner();
    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    runner.game.players[1].battle_area[kari.index as usize].is_suspended = true;
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let before = runner.memory();

    runner.attack_player(attacker, 1, true);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        before,
        "suspended Kari cannot pay the suspend cost and should not gain memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Optional processing condition (§15-7) — accept AND decline must be reachable
// ═══════════════════════════════════════════════════════════════════════════════

/// Drain pending selections until Kari's outer accept/decline confirm is on
/// top. Returns `true` when it was found (the engine formats the prompt as
/// "You may activate BT4-097's triggered effect"). Bounded so a mis-wired
/// prompt loops finitely instead of hanging the suite.
fn advance_to_kari_optional_prompt(runner: &mut DebugRunner) -> bool {
    for _ in 0..64 {
        let Some(view) = runner.pending_selection_view() else {
            return false;
        };
        if view.is_optional && view.prompt.contains("BT4-097") {
            return true;
        }
        // Same fallback `DebugRunner::auto_resolve` uses: an optional
        // selection with no listed action is answered with PASS.
        let action = match view.valid_action_ids.first().copied() {
            Some(id) => id,
            None if view.is_optional => digimon_engine::action::space::PASS,
            None => return false,
        };
        if runner.execute_action(view.selecting_player, action).is_err() {
            return false;
        }
    }
    false
}

/// §15-7-4: "A player can choose whether or not to execute the content of
/// optional processing conditions." Kari's "you may suspend this Tamer to gain
/// 1 memory" is that condition (§15-7-1's "by X, Y" shape), so DECLINING must
/// leave BOTH halves undone — Kari stays unsuspended AND no memory is gained,
/// because §15-7-2 says that when the condition's content isn't executed,
/// "the processing after the conditions can't be executed".
///
/// Before this fix the clause fired unconditionally: the engine auto-paid the
/// suspend and handed over the memory, so this branch was unreachable from the
/// action space (a rule-17 no-approximations violation). DCGO has always
/// offered the choice — `BT4_097.cs:19` passes `isOptional: true`.
#[test]
fn bt4_097_optional_suspend_cost_may_be_declined() {
    let mut runner = kari_runner();
    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    assert!(
        !runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "Kari starts unsuspended"
    );
    let before = runner.memory();

    runner.attack_player(attacker, 1, true);

    assert!(
        advance_to_kari_optional_prompt(&mut runner),
        "the optional processing condition must surface an accept/decline prompt (rule 17)"
    );
    let view = runner
        .pending_selection_view()
        .expect("prompt is on top after advance_to_kari_optional_prompt");
    runner
        .execute_action(view.selecting_player, digimon_engine::action::space::PASS)
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "declining the optional processing condition must NOT suspend Kari (§15-7-4)"
    );
    assert_eq!(
        runner.memory(),
        before,
        "with the condition declined, the processing after it can't execute (§15-7-2) — no memory gain"
    );
}

/// The accept half of the same choice, driven explicitly through the prompt
/// (rather than relying on `auto_resolve` picking the accept action): paying
/// the suspend must still suspend Kari and gain her controller 1 memory.
#[test]
fn bt4_097_optional_suspend_cost_may_be_accepted() {
    let mut runner = kari_runner();
    let kari = runner.place_on_field(1, "BT4-097", Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let before = runner.memory();

    runner.attack_player(attacker, 1, true);

    assert!(
        advance_to_kari_optional_prompt(&mut runner),
        "the optional processing condition must surface an accept/decline prompt (rule 17)"
    );
    let view = runner
        .pending_selection_view()
        .expect("prompt is on top after advance_to_kari_optional_prompt");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("the accept branch must be offered");
    runner
        .execute_action(view.selecting_player, accept)
        .expect("accept the suspend cost");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area[kari.index as usize].is_suspended,
        "accepting pays the cost: Kari suspends herself"
    );
    assert_eq!(
        runner.memory(),
        before - 1,
        "P1 gaining 1 memory moves the memory counter one step toward P1"
    );
}
