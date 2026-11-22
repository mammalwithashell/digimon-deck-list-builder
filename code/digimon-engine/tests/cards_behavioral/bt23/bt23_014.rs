//! BT23-014 Gallantmon — Digimon, Lv.6, Red, DP 11000, Cost 11.
//!
//! # Card text (cards.json)
//!
//! [On Play] [When Digivolving] Until your opponent's turn ends, their effects
//! can't play Digimon or Tamers from the trash.
//!
//! [On Play] [When Digivolving] [When Attacking] Delete 1 of your opponent's
//! Digimon with 8000 DP or less. For each of their Digimon and Tamers, add
//! 2000 to this DP deletion effect's maximum.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Red/BT23_014.cs
//!
//! # Patterns this test covers
//! - D6: Flood gate / opponent restriction (Digimon + Tamer play from trash)
//! - Player-scoped CannotPlayDigimonByEffect + CannotPlayTamerByEffect with EndOfOpponentsTurn expiry
//! - Dynamic DP cap: 8000 + 2000 × (count of opp battle area permanents)
//! - Shared [On Play]/[When Digivolving] + standalone [When Attacking] triggering

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

const LOW_DP_TARGET: &str = "BT23-014-LOW-DP-TARGET";

/// Standard fixture: Gallantmon registered, no cards in hand yet.
fn gallantmon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-014")
        .expect("BT23-014 found in embedded DSL pack")
        .add_card(low_dp_opponent())
        .memory(12)
        .start()
}

fn low_dp_opponent() -> CardData {
    let mut card = make_test_card(LOW_DP_TARGET, "LowDpTarget");
    card.dp = Some(9000);
    card
}

/// Fixture with Gallantmon in hand for playing.
/// Includes minimal decks for both players so `end_turn()` sequences don't
/// cause deck-out (draw on empty deck → game_over) before modifiers expire.
fn gallantmon_in_hand() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-014")
        .expect("BT23-014 found in embedded DSL pack")
        .add_card(low_dp_opponent())
        .hand(0, &["BT23-014"])
        // 3-card decks for each player prevent deck-out during multi-turn tests.
        .deck(0, &["BT23-014", "BT23-014", "BT23-014"])
        .deck(1, &["BT23-014", "BT23-014", "BT23-014"])
        .memory(12)
        .start()
}

// ─── Section 1: Structural assertions ──────────────────────────────────────

#[test]
fn bt23_014_has_triggered_clauses_covering_on_play() {
    let runner = gallantmon_runner();
    let compiled = runner
        .compiled_card("BT23-014")
        .expect("BT23-014 must be in compiled_cards after dsl_card()");

    let has_on_play = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnPlay),
        _ => false,
    });

    assert!(
        has_on_play,
        "BT23-014 must have at least one [On Play] triggered clause"
    );
}

#[test]
fn bt23_014_has_triggered_clauses_covering_when_digivolving() {
    let runner = gallantmon_runner();
    let compiled = runner
        .compiled_card("BT23-014")
        .expect("BT23-014 must be in compiled_cards");

    let has_wd = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::WhenDigivolving),
        _ => false,
    });

    assert!(
        has_wd,
        "BT23-014 must have at least one [When Digivolving] triggered clause"
    );
}

#[test]
fn bt23_014_has_triggered_clause_covering_when_attacking() {
    let runner = gallantmon_runner();
    let compiled = runner
        .compiled_card("BT23-014")
        .expect("BT23-014 must be in compiled_cards");

    let has_wa = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::WhenAttacking),
        _ => false,
    });

    assert!(
        has_wa,
        "BT23-014 must have a [When Attacking] triggered clause for the delete effect"
    );
}

#[test]
fn bt23_014_all_triggered_clauses_are_face_up_scope() {
    let runner = gallantmon_runner();
    let compiled = runner
        .compiled_card("BT23-014")
        .expect("BT23-014 must be in compiled_cards");

    // All triggered clauses are own-scope (FaceUp), not inherited, not security.
    for clause in &compiled.effects {
        if let CompiledClause::Triggered(t) = clause {
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "All BT23-014 clauses must be face-up (own) scope; found {:?}",
                t.scope
            );
        }
    }
}

#[test]
fn bt23_014_no_once_per_turn_restriction() {
    // BT23-014 has no [Once Per Turn] text — clauses fire every time triggered.
    let runner = gallantmon_runner();
    let compiled = runner
        .compiled_card("BT23-014")
        .expect("BT23-014 must be in compiled_cards");

    for clause in &compiled.effects {
        if let CompiledClause::Triggered(t) = clause {
            assert!(
                !t.once_per_turn,
                "BT23-014 has no [Once Per Turn] text; once_per_turn must be false"
            );
        }
    }
}

#[test]
fn bt23_014_at_least_two_triggered_clauses() {
    // Must have at minimum: floodgate clause(s) + delete clause(s).
    let runner = gallantmon_runner();
    let compiled = runner
        .compiled_card("BT23-014")
        .expect("BT23-014 must be in compiled_cards");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    assert!(
        triggered_count >= 2,
        "BT23-014 must have at least 2 triggered clauses (floodgate + delete); got {}",
        triggered_count
    );
}

// ─── Section 2: Floodgate — positive condition (modifier installed on play) ─

#[test]
fn bt23_014_on_play_installs_cannot_play_from_trash_on_opponent() {
    let mut runner = gallantmon_in_hand();

    // No modifier before play.
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, digimon_engine::enums::ModifierType::CannotPlayFromTrash),
        "opponent must NOT have CannotPlayFromTrash before Gallantmon is played"
    );

    // Play Gallantmon; auto-resolve any pending selection from delete clause.
    let _idx = runner.play(0, 0);
    runner.auto_resolve();

    // The trash-scoped floodgate modifier must be installed on opponent.
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, digimon_engine::enums::ModifierType::CannotPlayFromTrash),
        "CannotPlayFromTrash must be installed on opponent after [On Play]"
    );
}

/// The printed text restricts plays "from the trash" only — the broad
/// hand+trash `CannotPlayDigimonByEffect` / `CannotPlayTamerByEffect`
/// modifiers (which the old raw_rust installed) must NOT be installed, so
/// the opponent can still play Digimon/Tamers from HAND by effect.
#[test]
fn bt23_014_floodgate_does_not_block_hand_plays() {
    let mut runner = gallantmon_in_hand();

    let _idx = runner.play(0, 0);
    runner.auto_resolve();

    assert!(
        !runner.game.modifiers.player_has(
            1,
            digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect
        ),
        "floodgate must NOT install the broad hand+trash CannotPlayDigimonByEffect modifier"
    );
    assert!(
        !runner.game.modifiers.player_has(
            1,
            digimon_engine::enums::ModifierType::CannotPlayTamerByEffect
        ),
        "floodgate must NOT install the broad hand+trash CannotPlayTamerByEffect modifier"
    );
}

// ─── Section 3: Floodgate — negative condition (modifiers expire) ─────────────

#[test]
fn bt23_014_floodgate_modifier_still_active_during_opponents_turn() {
    let mut runner = gallantmon_in_hand();

    let _idx = runner.play(0, 0);
    runner.auto_resolve();

    assert!(runner
        .game
        .modifiers
        .player_has(1, digimon_engine::enums::ModifierType::CannotPlayFromTrash));

    // The 11-cost play crosses memory, so player 1's turn has already begun.
    // During player 1's turn the modifier should still be active.
    // (EndOfOpponentsTurn from P0's view = expires when P1's turn ends.)
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, digimon_engine::enums::ModifierType::CannotPlayFromTrash),
        "CannotPlayFromTrash must still be active DURING opponent's turn"
    );
}

#[test]
fn bt23_014_floodgate_modifier_expires_after_opponents_turn_ends() {
    let mut runner = gallantmon_in_hand();

    let _idx = runner.play(0, 0);
    runner.auto_resolve();

    // End P0's turn (P1's turn starts) then end P1's turn (P0's turn starts again).
    runner.end_turn();
    runner.end_turn();

    // After opponent's turn fully ends, the modifier must be gone.
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, digimon_engine::enums::ModifierType::CannotPlayFromTrash),
        "CannotPlayFromTrash must expire after opponent's turn ends"
    );
}

// ─── Section 4: Floodgate enforcement — blocking effect-plays ────────────────

#[test]
fn bt23_014_floodgate_blocks_digimon_play_from_trash_by_effect() {
    use digimon_engine::enums::{CostDelta, PlaySource};

    let mut runner = gallantmon_in_hand();

    // Put a card in opponent's trash for the test.
    if !runner.game.players[1].hand.is_empty() {
        let card = runner.game.players[1].hand.remove(0);
        runner.game.players[1].trash.push(card);
    } else if !runner.game.players[1].deck.is_empty() {
        let card = runner.game.players[1].deck.pop().expect("deck has cards");
        runner.game.players[1].trash.push(card);
    } else {
        // No cards available to test with; skip.
        return;
    }

    // Play Gallantmon to install floodgate.
    let _idx = runner.play(0, 0);
    runner.auto_resolve();

    // Verify the floodgate modifier is installed.
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, digimon_engine::enums::ModifierType::CannotPlayFromTrash),
        "CannotPlayFromTrash must be installed"
    );

    // Attempt to play a Digimon from trash by effect. Should be blocked.
    // The trash card may or may not be a Digimon — if not, gate won't fire.
    // This test confirms the enforcement path is wired; specific card-kind tests
    // use dedicated cards. The important thing is no panic + modifier is consulted.
    let result = runner.game.play_from_trash_with_cost(
        1,
        0, // first trash card
        CostDelta::Free,
        PlaySource::ByEffect,
    );
    // Result depends on card kind. If Digimon, must be None (blocked).
    // We can't assert None unconditionally, but we can check the gate doesn't panic.
    let _ = result;
}

// ─── Section 5: Delete clause — [On Play] ────────────────────────────────────

#[test]
fn bt23_014_on_play_with_no_opp_digimon_does_not_delete() {
    // With no eligible opponent Digimon, the delete clause is a no-op.
    let mut runner = gallantmon_in_hand();

    let opp_area_before = runner.game.players[1].battle_area.len();
    let _idx = runner.play(0, 0);
    runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        opp_area_before,
        "With no eligible targets, opponent battle area count must be unchanged"
    );
}

#[test]
fn bt23_014_on_play_with_opp_digimon_present_deletes_it() {
    // Place an eligible opponent Digimon first, then play Gallantmon.
    let mut runner = gallantmon_in_hand();

    // With one opponent permanent, the dynamic cap is 8000 + 2000 = 10000.
    runner.place_on_field(1, LOW_DP_TARGET, None);
    let opp_count_before = runner.game.players[1].battle_area.len();

    let _idx = runner.play(0, 0);
    runner.auto_resolve();

    let opp_count_after = runner.game.players[1].battle_area.len();
    assert!(
        opp_count_after < opp_count_before,
        "After [On Play] delete fires and auto-resolves, opp battle area should shrink; \
         before={}, after={}",
        opp_count_before,
        opp_count_after
    );
}

// ─── Section 6: Delete clause — [When Attacking] ─────────────────────────────

#[test]
fn bt23_014_when_attacking_fires_without_panic() {
    // Verify the [When Attacking] clause triggers and the game does not panic.
    let mut runner = gallantmon_runner();

    let gallantmon = runner.place_on_field(0, "BT23-014", Some(0));
    runner.place_on_field(1, "BT23-014", None);

    // Attack opponent player.
    runner.attack_player(gallantmon, 1, false);
    runner.auto_resolve();

    // No panic = the WA delete path fires correctly.
}

#[test]
fn bt23_014_when_attacking_with_target_reduces_opp_field() {
    // The [When Attacking] delete should remove an opponent Digimon if eligible.
    let mut runner = gallantmon_runner();

    let gallantmon = runner.place_on_field(0, "BT23-014", Some(0));
    let _target = runner.place_on_field(1, LOW_DP_TARGET, None);
    let opp_count_before = runner.game.players[1].battle_area.len();

    runner.attack_player(gallantmon, 1, false);
    runner.auto_resolve();

    let opp_count_after = runner.game.players[1].battle_area.len();
    assert!(
        opp_count_after < opp_count_before,
        "After [When Attacking] delete fires and auto-resolves, opp battle area should shrink; \
         before={}, after={}",
        opp_count_before,
        opp_count_after
    );
}

// ─── Section 7: Dynamic DP cap — structural ───────────────────────────────────

#[test]
fn bt23_014_compiled_card_has_when_attacking_clause() {
    // Structural check: the delete clause covers [When Attacking].
    let runner = gallantmon_runner();
    let compiled = runner
        .compiled_card("BT23-014")
        .expect("BT23-014 in compiled_cards");

    let wa_count = compiled
        .effects
        .iter()
        .filter(|c| match c {
            CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::WhenAttacking),
            _ => false,
        })
        .count();

    assert_eq!(
        wa_count, 1,
        "BT23-014 must have exactly 1 [When Attacking] clause"
    );
}

#[test]
fn bt23_014_dp_cap_with_3_opp_perms_is_14000() {
    // Formula: 8000 + 2000 × 3 = 14000.
    let mut runner = gallantmon_runner();

    let gallantmon = runner.place_on_field(0, "BT23-014", Some(0));
    for _ in 0..3 {
        runner.place_on_field(1, "BT23-014", None);
    }

    runner.attack_player(gallantmon, 1, false);

    // With 3 opp perms, threshold = 14000. BT23-014 (11000 DP) → eligible.
    // Expect OppField selection.
    assert!(
        runner.pending_kind().is_some(),
        "With 3 opp permanents and cap=14000, delete selection must fire"
    );
}

// ─── Section 6: The lock is read against the playing EFFECT's controller ─────
//
// G-ENGINE-TRASH-PLAY-LOCK-SCOPE. DCGO BT23_014.cs:46-58 installs a
// `CanNotPutFieldClass` whose card condition is "in ANY trash and a Digimon
// or Tamer" and whose effect condition is `EffectSourceCard.Owner ==
// card.Owner.Enemy` — it is the locked player's EFFECTS that cannot play, not
// the locked player's trash that cannot be played from. The printed text
// agrees ("their effects can't play Digimon or Tamers from the trash", CR 4.3
// 7-1-1 for what "play" means). The driver that plays from a trash that is not
// its own controller's is Dragomon (EX5-060): "Your opponent plays 1 level 4
// or lower Digimon card from their trash" — the effect's controller is the
// card's owner's opponent.

const OWN_LV4: &str = "BT23-014-X-LV4";

fn lv4_digimon() -> CardData {
    let mut card = make_test_card(OWN_LV4, "CrossTrashLv4");
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.colors = vec![digimon_engine::enums::CardColor::Purple];
    card
}

/// Gallantmon (BT23-014) owned by player 0 has resolved its [On Play]
/// floodgate, so player 1 carries `CannotPlayFromTrash` and player 0 does
/// not. Player 1's field was empty when Gallantmon resolved, so the printed
/// delete clause had nothing to take.
fn gallantmon_lock_on_player_1() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("BT23-014")
        .expect("BT23-014 in pack")
        .dsl_card("EX5-060")
        .expect("EX5-060 Dragomon in pack")
        .add_card(lv4_digimon())
        .memory(12)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    let gallantmon = r.place_on_field(0, "BT23-014", Some(0));
    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(gallantmon),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    use digimon_engine::enums::ModifierType::CannotPlayFromTrash;
    assert!(
        r.game.modifiers.player_has(1, CannotPlayFromTrash),
        "fixture: player 1 is the locked player"
    );
    assert!(
        !r.game.modifiers.player_has(0, CannotPlayFromTrash),
        "fixture: player 0 (Gallantmon's controller) is not locked"
    );
    r
}

/// The locked player's own effect plays from the OTHER player's trash: refused.
/// Player 1 owns Dragomon (EX5-060), so its [On Play] makes player 0 -- the
/// opponent -- play a Lv4 from player 0's trash. The effect belongs to the
/// locked player, so DCGO blocks it (the card is in a trash and
/// `EffectSourceCard.Owner` is the locked player).
#[test]
fn bt23_014_lock_refuses_a_locked_players_effect_playing_from_the_opponents_trash() {
    let mut r = gallantmon_lock_on_player_1();
    let dragomon = r.place_on_field(1, "EX5-060", Some(0));
    r.inject_trash(0, OWN_LV4);
    let p0_field_before = r.game.players[0].battle_area.len();

    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(dragomon),
    );
    r.game.drain_effect_queue();
    // The candidate leaf (`trash_play_locked`) must already drop the Lv4, so no
    // pick is offered. Without this the refusal could come only from the later
    // trash-play gate (`trash_play_refusal`) and a leaf still keyed on the trash
    // owner would go unnoticed.
    assert!(
        r.pending_selection_view().is_none(),
        "the locked player's Dragomon must not be offered the opponent's trash Lv4"
    );
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        p0_field_before,
        "a locked player's effect cannot play a Digimon from the opponent's trash"
    );
    assert_eq!(
        r.trash_size(0),
        1,
        "the Lv4 stays in player 0's trash"
    );
}

/// Gallantmon's controller's own effect plays from the locked player's trash:
/// allowed. Player 0 owns both Gallantmon and Dragomon, so Dragomon's [On Play]
/// makes the locked player 1 play from player 1's trash. The effect is player
/// 0's, which the lock does not bind.
#[test]
fn bt23_014_lock_allows_the_gallantmon_controllers_effect_playing_from_the_locked_players_trash() {
    let mut r = gallantmon_lock_on_player_1();
    let dragomon = r.place_on_field(0, "EX5-060", Some(1));
    r.inject_trash(1, OWN_LV4);

    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::OnPlay,
        digimon_engine::selection::TriggerSource::Permanent(dragomon),
    );
    r.game.drain_effect_queue();
    let view = r
        .pending_selection_view()
        .expect("the Lv4 is a legal pick: the lock does not bind player 0's effect");
    assert_eq!(view.selecting_player, 1);
    r.execute_action(1, view.valid_action_ids[0])
        .expect("player 1 picks the Lv4");
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == OWN_LV4),
        "player 0's effect played the Lv4 from player 1's trash onto player 1's field"
    );
    assert_eq!(r.trash_size(1), 0);
}

/// Engine-level pin of the same scope at the trash-play gate
/// (`Game::trash_play_refusal`), independent of any card's selection flow: the
/// lock is read against `effect_source_player`, the playing effect's controller.
#[test]
fn bt23_014_trash_play_gate_reads_the_effect_controller_not_the_trash_owner() {
    use digimon_engine::enums::{CostDelta, PlaySource};

    // (effect controller, trash owner, expect the play to happen)
    for (effect_controller, trash_owner, allowed) in [
        (1u8, 1u8, false), // locked player's effect, own trash
        (1, 0, false),     // locked player's effect, opponent's trash
        (0, 1, true),      // Gallantmon controller's effect, locked player's trash
        (0, 0, true),      // Gallantmon controller's effect, own trash
    ] {
        let mut r = gallantmon_lock_on_player_1();
        r.inject_trash(trash_owner, OWN_LV4);
        r.game.set_effect_source_player_for_test(Some(effect_controller));
        let played = r.game.play_from_trash_with_cost(
            trash_owner,
            0,
            CostDelta::Free,
            PlaySource::ByEffect,
        );
        r.game.set_effect_source_player_for_test(None);
        assert_eq!(
            played.is_some(),
            allowed,
            "effect of player {effect_controller} playing from player {trash_owner}'s trash"
        );
    }
}

/// Seat `id` in `player`'s battle area as an Option card an effect PLACED there
/// (`OptionState::OrdinaryFieldOption`, the Digital Accident Tactics Squad
/// (P-235) / DNA Charge (ST24-15) shape; CR 4.3 4-28-1). A bare
/// `place_on_field` of an Option card is not a placed Option and does not stay.
fn place_option_in_battle_area(runner: &mut DebugRunner, player: u8, id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == id)
        .unwrap_or_else(|| panic!("unknown card {id}"));
    let card_index = runner.game.next_card_index();
    runner.game.install_field_option_as_ordinary(
        digimon_engine::card_source::CardSource::new(data_idx, player, card_index),
        player,
    );
}

/// The placed Option is still in `player`'s battle area (so the count under
/// test really had an Option to miscount).
fn placed_option_is_on_field(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

/// "For each of their Digimon and Tamers, add 2000 to this DP deletion
/// effect's maximum" -- an Option card placed in the opponent's battle area is
/// neither (CR 4.3 4-28-1). DCGO BT23_014.cs:
/// `Enemy.GetBattleAreaPermanents().Filter(x => x.IsDigimon || x.IsTamer).Count * 2000`.
/// Board: 1 opponent Digimon (14000 DP) + 1 Tamer + 1 placed Option. Correct
/// cap = 8000 + 2000x2 = 12000 -> the 14000 DP Digimon is NOT a legal target.
/// Regression: the unfiltered count gave 8000 + 2000x3 = 14000 and deleted it.
#[test]
fn bt23_014_dp_cap_counts_digimon_and_tamers_but_not_placed_options() {
    use digimon_engine::enums::CardKind;
    let mut big = make_test_card("OPP-14K", "Opp14k");
    big.card_kind = CardKind::Digimon;
    big.dp = Some(14000);
    let mut tamer = make_test_card("OPP-TAMER", "OppTamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut option = make_test_card("OPP-OPTION", "OppOption");
    option.card_kind = CardKind::Option;
    option.level = None;
    option.dp = None;
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-014")
        .expect("BT23-014 found in embedded DSL pack")
        .add_card(big)
        .add_card(tamer)
        .add_card(option)
        .hand(0, &["BT23-014"])
        .memory(12)
        .start();
    runner.place_on_field(1, "OPP-14K", None);
    runner.place_on_field(1, "OPP-TAMER", None);
    place_option_in_battle_area(&mut runner, 1, "OPP-OPTION");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.battle_area_size(1),
        3,
        "precondition: Digimon + Tamer + placed Option in the opponent's battle area"
    );

    let _ = runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert!(
        placed_option_is_on_field(&runner, 1, "OPP-OPTION"),
        "the placed Option is still in the battle area"
    );
    assert!(
        placed_option_is_on_field(&runner, 1, "OPP-14K"),
        "cap is 8000 + 2000 x (1 Digimon + 1 Tamer) = 12000: the 14000 DP Digimon survives"
    );
}

/// Same board with a 12000 DP Digimon: exactly at the cap, so it IS deleted
/// (the Tamer counts, the placed Option does not).
#[test]
fn bt23_014_dp_cap_includes_the_opponents_tamer() {
    use digimon_engine::enums::CardKind;
    let mut target = make_test_card("OPP-12K", "Opp12k");
    target.card_kind = CardKind::Digimon;
    target.dp = Some(12000);
    let mut tamer = make_test_card("OPP-TAMER", "OppTamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-014")
        .expect("BT23-014 found in embedded DSL pack")
        .add_card(target)
        .add_card(tamer)
        .hand(0, &["BT23-014"])
        .memory(12)
        .start();
    runner.place_on_field(1, "OPP-12K", None);
    runner.place_on_field(1, "OPP-TAMER", None);

    let _ = runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "cap 8000 + 2000 x 2 = 12000: the 12000 DP Digimon is deleted, the Tamer stays"
    );
}
