//! BT5-106 Demonic Disaster - Option, Purple, Cost 1.
//!
//! # Card text (cards.json)
//!
//! [Main] You may delete 1 of your Digimon to unsuspend 1 of your purple Digimon.
//!
//! Security Effect [Security] You may play 1 level 3 purple Digimon card from
//! your trash without paying its memory cost. Any [On Play] effects on Digimon
//! played with this effect don't activate.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT5/Purple/BT5_106.cs (submodule not initialized)
//!
//! # Patterns this test covers
//! - Option [Main] with a visible sacrifice selection.
//! - Follow-up visible own purple Digimon unsuspend selection.
//! - Security free-play from trash with played-Digimon [On Play] suppression
//!   (PUPPETS-G030).

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, color: CardColor) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card
}

fn option_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT5-106")
        .expect("BT5-106 YAML parses and compiles")
        .add_card(digimon("PURPLE-COST", CardColor::Purple))
        .add_card(digimon("PURPLE-TARGET", CardColor::Purple))
        .add_card(digimon("YELLOW-TARGET", CardColor::Yellow))
        .hand(0, &["BT5-106"])
        .memory(10)
        .start()
}

#[test]
fn bt5_106_is_purple_option_cost_1() {
    let runner = option_runner();
    let compiled = runner
        .compiled_card("BT5-106")
        .expect("BT5-106 compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(1));
    assert!(
        compiled
            .color
            .contains(&digimon_dsl::compiled::CompiledColor::Purple),
        "Demonic Disaster must be a purple Option"
    );
}

#[test]
fn bt5_106_has_main_and_security_clauses() {
    let runner = option_runner();
    let compiled = runner
        .compiled_card("BT5-106")
        .expect("BT5-106 compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "BT5-106 ships both the [Main] slice and the [Security] slice (PUPPETS-G030)"
    );

    let main = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("[Main] clause present");
    assert_eq!(main.scope, CompiledScope::FaceUp);

    let security = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("[Security] clause present");
    assert_eq!(
        security.scope,
        CompiledScope::Inherited,
        "the Security clause rides on the option's inherited surface"
    );
    assert!(
        security.optional,
        "the printed 'You may play' makes the Security clause optional"
    );
}

#[test]
fn bt5_106_main_prompts_for_sacrifice_then_purple_unsuspend_target() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let target = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(target);
    runner.game.suspend(yellow);

    assert!(runner.game.activate_hand_main(0, 0));

    let first = runner
        .pending_selection_view()
        .expect("Main effect must first prompt for the Digimon to delete");
    assert_eq!(first.kind, SelectionKind::OwnField);
    assert!(
        first.is_optional,
        "the sacrifice selection is optional because the card says 'You may'"
    );
    assert_eq!(
        first.valid_action_ids,
        vec![
            encode_attack(0, 0),
            encode_attack(0, 1),
            encode_attack(0, 2)
        ],
        "all own Digimon should be legal sacrifice choices"
    );

    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-COST to delete");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "the selected sacrifice Digimon must be deleted before the unsuspend target is chosen"
    );

    let second = runner
        .pending_selection_view()
        .expect("after paying the cost, Main must prompt for a purple Digimon to unsuspend");
    assert_eq!(second.kind, SelectionKind::OwnField);
    assert!(
        !second.is_optional,
        "the follow-up target choice is mandatory after cost payment"
    );
    assert_eq!(
        second.valid_action_ids,
        vec![encode_attack(0, 0)],
        "only the remaining suspended purple Digimon should be a legal unsuspend target"
    );

    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-TARGET to unsuspend");

    assert!(
        !runner.game.player(0).battle_area[target.index as usize - 1].is_suspended,
        "the chosen purple Digimon should be unsuspended"
    );
    assert!(
        runner.game.player(0).battle_area[yellow.index as usize - 1].is_suspended,
        "non-purple Digimon must not be affected"
    );
}

#[test]
fn bt5_106_main_decline_sacrifice_short_circuits_effect() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let target = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(target);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .execute_action(0, PASS)
        .expect("decline the optional sacrifice");

    assert!(
        runner.pending_selection().is_none(),
        "declining the optional sacrifice must not continue to the unsuspend choice"
    );
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "no Digimon should be deleted"
    );
    assert!(
        runner.game.player(0).battle_area[target.index as usize].is_suspended,
        "target stays suspended when the effect is declined"
    );
}

#[test]
fn bt5_106_main_requires_suspended_purple_unsuspend_target() {
    let mut runner = option_runner();
    let cost = runner.place_on_field(0, "PURPLE-COST", Some(0));
    let purple = runner.place_on_field(0, "PURPLE-TARGET", Some(0));
    let yellow = runner.place_on_field(0, "YELLOW-TARGET", Some(0));
    runner.game.suspend(cost);
    runner.game.suspend(yellow);

    assert!(runner.game.activate_hand_main(0, 0));
    runner
        .execute_action(0, encode_attack(0, 0))
        .expect("choose PURPLE-COST to delete");

    assert!(
        runner.pending_selection().is_none(),
        "no follow-up target prompt should install when no suspended purple Digimon remains"
    );
    assert!(
        !runner.game.player(0).battle_area[purple.index as usize - 1].is_suspended,
        "unsuspended purple Digimon should not be offered or changed"
    );
    assert!(
        runner.game.player(0).battle_area[yellow.index as usize - 1].is_suspended,
        "suspended non-purple Digimon should not be offered"
    );
}

// ─── PUPPETS-G030 — [Security] free play from trash with On Play suppression ──

/// An inline DSL purple level-3 Digimon whose `[On Play]` effect draws 1 card
/// for its controller. The draw is the observable signal: when BT5-106's
/// Security clause plays this card with `suppress_on_play: true`, the OnPlay
/// `draw` must NOT fire. Played by any other path, the `draw` fires normally.
const ONPLAY_PURPLE_L3_YAML: &str = r#"
card: ONPLAY-PURPLE-L3
name: OnPlay Purple Imp
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 2000
effects:
  - when: on_play
    summary: "[On Play] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;

/// A purple level-4 Digimon (wrong level) — must NOT be a legal Security
/// trash-play candidate (the clause filters on level 3).
fn purple_l4(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(4);
    card
}

/// A yellow level-3 Digimon (wrong color) — must NOT be a legal Security
/// trash-play candidate (the clause filters on purple).
fn yellow_l3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card
}

/// Build a runner where P0 attacks P1, whose single Security card is BT5-106.
/// P1's trash is seeded with the supplied card ids. P1's deck is stocked so a
/// draw is observable. Returns the runner with an attacker already on P0's
/// field (turn 0 → no summoning sickness).
fn security_runner(
    trash_ids: &[&str],
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT5-106")
        .expect("BT5-106 YAML parses and compiles")
        .from_dsl_yaml(ONPLAY_PURPLE_L3_YAML)
        .expect("ONPLAY-PURPLE-L3 inline DSL compiles")
        .add_card(attacker)
        .add_card(purple_l4("PURPLE-L4"))
        .add_card(yellow_l3("YELLOW-L3"))
        .security(1, &["BT5-106"])
        .deck(1, &["ATTACKER", "ATTACKER", "ATTACKER"])
        .memory(10)
        .start();

    // Seed P1's trash with the requested cards.
    for id in trash_ids {
        let card = {
            let data_idx = runner
                .game
                .card_data
                .iter()
                .position(|c| &c.card_id == id)
                .unwrap_or_else(|| panic!("security_runner: unknown card_id {id}"));
            let next_idx = runner.game.next_card_index();
            digimon_engine::card_source::CardSource::new(data_idx, 1, next_idx)
        };
        runner.game.players[1].trash.push(card);
    }

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    (runner, attacker)
}

/// G030: BT5-106's [Security] clause prompts the defender to optionally play a
/// **level 3 purple** Digimon from trash. Wrong-level and wrong-color cards in
/// trash must not be offered as candidates.
#[test]
fn bt5_106_security_prompts_for_level_3_purple_digimon_in_trash() {
    let (mut runner, attacker) = security_runner(&["ONPLAY-PURPLE-L3", "PURPLE-L4", "YELLOW-L3"]);

    runner.attack_player(attacker, 1, false);

    let sel = runner
        .pending_selection_view()
        .expect("BT5-106 Security clause must install a trash-selection prompt");
    assert_eq!(
        sel.kind,
        SelectionKind::Trash,
        "Security clause selects from the defender's own trash"
    );
    assert_eq!(
        sel.selecting_player, 1,
        "the defender controls the Security selection"
    );
    assert!(
        sel.is_optional,
        "the printed 'You may play' makes the trash play optional"
    );
    assert_eq!(
        sel.valid_action_ids.len(),
        1,
        "only the level-3 purple Digimon is a legal candidate \
         (the level-4 purple and level-3 yellow cards are filtered out)"
    );
}

/// G030: BT5-106's [Security] clause plays the chosen level-3 purple Digimon
/// from trash for free, and the `suppress_on_play: true` flag stops THAT
/// Digimon's own `[On Play]` effect from activating. The On Play here is a
/// `draw: 1` — under suppression the defender draws nothing from it.
#[test]
fn bt5_106_security_suppresses_on_play_effects_of_played_digimon() {
    let (mut runner, attacker) = security_runner(&["ONPLAY-PURPLE-L3"]);

    let hand_before = runner.hand_size(1);
    let deck_before = runner.deck_size(1);

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve Security trash play");

    // The level-3 purple Digimon was played from trash into P1's battle area.
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ONPLAY-PURPLE-L3"),
        "the level-3 purple Digimon must enter the defender's battle area"
    );
    // The played Digimon left the trash (BT5-106 itself lands in the trash as
    // the spent security card, so only it remains there).
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ONPLAY-PURPLE-L3"),
        "the played Digimon must have left the trash"
    );

    // suppress_on_play: true — the played Digimon's [On Play] draw must NOT
    // fire. Hand and deck are unchanged by the OnPlay (the play itself moves
    // no cards into hand or deck).
    assert_eq!(
        runner.hand_size(1),
        hand_before,
        "suppressed [On Play] draw must not add a card to the defender's hand"
    );
    assert_eq!(
        runner.deck_size(1),
        deck_before,
        "suppressed [On Play] draw must not consume a card from the defender's deck"
    );
}

/// G030 scoping proof: the suppression is scoped to the BT5-106 Security play
/// event only. The SAME OnPlay Digimon, played through an ordinary
/// effect-play-from-trash path (no `suppress_on_play`), DOES fire its
/// `[On Play]` draw. This guards against a global On Play disable.
#[test]
fn bt5_106_on_play_suppression_does_not_leak_to_normal_plays() {
    // No BT5-106 involved — just the OnPlay Digimon, played the ordinary way.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ONPLAY_PURPLE_L3_YAML)
        .expect("ONPLAY-PURPLE-L3 inline DSL compiles")
        .deck(0, &["ONPLAY-PURPLE-L3", "ONPLAY-PURPLE-L3"])
        .memory(10)
        .start();

    // Seed one copy into P0's trash.
    let card = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "ONPLAY-PURPLE-L3")
            .expect("ONPLAY-PURPLE-L3 in card_data");
        let next_idx = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx)
    };
    runner.game.players[0].trash.push(card);

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    // Ordinary effect-driven play from trash — no suppression.
    runner
        .game
        .play_from_trash(0, 0)
        .expect("OnPlay Digimon plays from trash");
    runner.auto_resolve().expect("resolve [On Play] draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "an unsuppressed [On Play] draw must add a card to the hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "an unsuppressed [On Play] draw must consume a card from the deck"
    );
}

// ─── Failure-mode audit (PR #456 cluster 3f — effect-driven play / sacrifice) ──

/// **Adjacent edge-case (cluster 3f):** BT5-106 [Main] is a "you may"
/// sacrifice. With NO own Digimon on field, the optional sacrifice
/// pool is empty. The effect must short-circuit cleanly: no
/// pending_selection installed, no field changes, no crash. Guards
/// against the empty-candidate-pool branch silently keeping the
/// selection alive.
#[test]
fn bt5_106_main_with_no_own_digimon_short_circuits_without_panic() {
    let mut runner = option_runner();
    // No place_on_field calls — P0 has no Digimon.
    assert_eq!(runner.battle_area_size(0), 0);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "[Main] still fires; the optional inner step is what's empty"
    );

    // Selection should not park (no targets to pick from).
    assert!(
        runner.game.pending_selection.is_none(),
        "no pending_selection should be installed when sacrifice candidate pool is empty"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "no field state can change when the optional sacrifice has no targets"
    );
}
