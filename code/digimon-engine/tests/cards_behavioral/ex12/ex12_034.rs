use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::enums::{CardColor, CardKind};

use super::support::{
    field_contains, hand_index, plain_digimon, select_first_non_pass, DebugRunner,
};

const CARD_ID: &str = "EX12-034";

#[test]
fn ex12_034_leave_replacement_is_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled EX12-034");
    let replacement = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                once_per_turn,
                ..
            }) if trigger == "when_would_leave_battle_area" => Some(*once_per_turn),
            _ => None,
        })
        .expect("printed leave-area replacement");

    assert!(replacement, "printed leave replacement is [Once Per Turn]");
}

#[test]
fn ex12_034_on_play_may_play_kotenken_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .hand(0, &[CARD_ID])
        .memory(12)
        .start();

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-034");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve token play");

    assert!(field_contains(&runner, 0, "TOKEN_KOTENKEN"));
    let token = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "TOKEN_KOTENKEN")
        .expect("Kotenken token");
    assert_eq!(
        token.top_card().card_kind(&runner.game.card_data),
        CardKind::Token
    );
}

#[test]
fn ex12_034_ally_played_bottom_decks_opponent_lowest_level_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .add_card(plain_digimon("ALLY", CardColor::Black, 3, 3000))
        .add_card(plain_digimon("LOW", CardColor::Black, 3, 3000))
        .add_card(plain_digimon("HIGH", CardColor::Black, 5, 7000))
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner.fire_play_event_triggers(ally.player, ally.index as usize, true, false);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve bottom-deck");

    assert_eq!(runner.battle_area_size(1), 1);
    assert_eq!(
        runner.game.players[1]
            .deck
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "LOW"
    );
}

// --- Leave replacement: offered zones + OPT refund (deck audit 2026-11-10) ---

fn leave_setup(hand: &[&str]) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .add_card(super::support::red_sw("SW-5", 5))
        .add_card(plain_digimon("VICTIM-A", CardColor::Red, 4, 4000))
        .add_card(plain_digimon("VICTIM-B", CardColor::Red, 4, 4000))
        .hand(0, hand)
        .memory(10)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let a = runner.place_on_field(0, "VICTIM-A", Some(0));
    runner.place_on_field(0, "VICTIM-B", Some(0));
    (runner, a)
}

fn delete_own(runner: &mut DebugRunner, card_id: &str) {
    let h = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .map(|i| digimon_engine::permanent::PermanentHandle { player: 0, index: i as u8 })
        .expect("own Digimon on field");
    runner
        .game
        .delete_permanent_with_cause(h, digimon_engine::replacement::ReplacementCause::OwnEffect);
}

fn choice_labels(runner: &DebugRunner) -> Vec<String> {
    runner
        .pending_selection()
        .and_then(|p| p.effect_choices.as_ref())
        .map(|c| c.iter().map(|e| e.label.clone()).collect())
        .unwrap_or_default()
}

/// DCGO EX12_034.cs offers "from hand" only when the hand holds a level 5 or
/// lower [SW] card and "from digivolution cards" only when this Digimon's
/// sources do, always with "Don't play".
#[test]
fn ex12_034_leave_offers_only_zones_with_a_playable_card() {
    let (mut runner, _) = leave_setup(&["SW-5"]);
    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    let labels = choice_labels(&runner);
    assert_eq!(labels, vec!["Hand".to_string(), "Don't play".to_string()]);
}

/// Playing nothing refunds the [Once Per Turn] (DCGO RemoveUse), so the next
/// leave this turn offers the replacement again.
#[test]
fn ex12_034_leave_playing_nothing_refunds_once_per_turn() {
    let (mut runner, _) = leave_setup(&["SW-5"]);
    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    runner.execute_branch(1).expect("choose Don't play");
    let _ = runner.auto_resolve();
    assert!(!field_contains(&runner, 0, "VICTIM-A"), "the first Digimon left");

    delete_own(&mut runner, "VICTIM-B");
    assert!(
        runner.pending_selection().is_some(),
        "the refunded [Once Per Turn] replacement is offered again"
    );
}

/// After choosing "Hand" the card pick can't be passed: declining there would
/// spend the [Once Per Turn] without the refund. "Don't play" is the opt-out
/// (DCGO's decline-then-RemoveUse has the same outcome).
#[test]
fn ex12_034_leave_hand_pick_after_choosing_hand_is_mandatory() {
    let (mut runner, _) = leave_setup(&["SW-5"]);
    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    runner.execute_branch(0).expect("choose Hand");
    let sel = runner.pending_selection().expect("hand pick pending");
    assert_eq!(sel.kind, digimon_engine::selection::SelectionKind::Hand);
    assert!(!sel.is_optional, "the hand pick has no PASS");
}

/// With nothing playable anywhere, the choice is only "Don't play" (it
/// auto-binds, no prompt) and the [Once Per Turn] is refunded.
#[test]
fn ex12_034_leave_with_no_playable_card_plays_nothing() {
    let (mut runner, _) = leave_setup(&[]);
    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    let _ = runner.auto_resolve();
    assert!(!field_contains(&runner, 0, "VICTIM-A"));
    assert!(runner.pending_selection().is_none());
}

/// "Play ... without paying the cost" stays free for a DigiXros card: Erlangmon
/// playing Sagomon (EX12-029, cost 7, DigiXros) from hand pays no memory.
/// Game audit 2026-11-11: the DigiXros transaction charged 5.
#[test]
fn ex12_034_leave_free_play_of_a_digixros_card_pays_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .dsl_card("EX12-029")
        .expect("EX12-029 YAML loads")
        .add_card(plain_digimon("VICTIM-A", CardColor::Red, 4, 4000))
        .hand(0, &["EX12-029"])
        .memory(3)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "VICTIM-A", Some(0));
    let memory_before = runner.memory();

    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    assert_eq!(choice_labels(&runner)[0], "Hand");
    runner.execute_branch(0).expect("play from hand");
    // Pick Sagomon, then decline any DigiXros material pick.
    let mut guard = 0;
    while let Some(sel) = runner.pending_selection() {
        guard += 1;
        assert!(guard < 16, "selection chain did not terminate");
        let player = sel.selecting_player;
        let prompt = format!("{:?} {}", sel.kind, sel.prompt);
        // Take Sagomon from hand; decline optional picks (DigiXros materials);
        // otherwise (trigger ordering) take the first choice.
        let pick = if sel.kind == digimon_engine::selection::SelectionKind::Hand {
            sel.valid_action_ids[0]
        } else if sel.is_optional {
            digimon_engine::action::space::PASS
        } else {
            sel.valid_action_ids[0]
        };
        runner
            .execute_action(player, pick)
            .unwrap_or_else(|e| panic!("{e:?} at {prompt}"));
    }
    assert!(field_contains(&runner, 0, "EX12-029"), "Sagomon was played");
    // Sagomon's own effects may gain memory; the play itself must not charge.
    assert!(runner.memory() >= memory_before, "the free play paid no memory");
}

// --- Leave replacement: candidate legality (select-hand playability audit) ---
// DCGO EX12_034.cs:146-152 `CanSelectCardCondition`: `HasLevel && Level <= 5
// && EqualsTraits("SW") && CanPlayAsNewPermanent(cardSource, payCost: false,
// activateClass)`, used both to decide which zones are offered
// (EX12_034.cs:158-171) and for the pick itself (EX12_034.cs:186-194). A card
// the free play would refuse (play prohibition, no free frame) is never
// offered, and a zone holding only such cards is not offered either.

fn ex12_034_dp_lock(runner: &mut DebugRunner, ceiling: i32) {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonWithDPAtMost,
            ceiling,
            Expiry::Permanent,
            None,
            1,
        ),
    );
}

fn ex12_034_hand_pick_ids(runner: &DebugRunner) -> Vec<String> {
    use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
    let view = runner.pending_selection_view().expect("hand pick pending");
    assert_eq!(view.kind, digimon_engine::selection::SelectionKind::Hand);
    let hand_ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut offered: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| {
            let slot = if id >= HAND_EFFECT_START { id - HAND_EFFECT_START } else { id - PLAY_HAND_START };
            hand_ids[slot as usize].clone()
        })
        .collect();
    offered.sort();
    offered
}

/// "You can't play Digimon with 4000 DP or less": of two level 5 [SW] cards in
/// hand only the 7000 DP one is offered.
#[test]
fn ex12_034_leave_hand_pick_excludes_card_under_play_prohibition() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .add_card(super::support::red_sw("SW-5", 5))
        .add_card(super::support::digimon("SW-5-HIGH", CardColor::Red, 5, 4, 7000, &["SW"]))
        .add_card(plain_digimon("VICTIM-A", CardColor::Red, 4, 4000))
        .hand(0, &["SW-5", "SW-5-HIGH"])
        .memory(10)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "VICTIM-A", Some(0));
    ex12_034_dp_lock(&mut runner, 4000);

    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    assert_eq!(choice_labels(&runner), vec!["Hand".to_string(), "Don't play".to_string()]);
    runner.execute_branch(0).expect("choose Hand");
    assert_eq!(ex12_034_hand_pick_ids(&runner), vec!["SW-5-HIGH".to_string()]);
}

/// A full battle area (the leaving Digimon still occupies its frame while this
/// effect resolves): nothing can be played, so "Hand" isn't offered, only
/// "Don't play" remains (auto-bound), and the [SW] card stays in hand.
#[test]
fn ex12_034_leave_offers_no_hand_zone_when_battle_area_full() {
    let (mut runner, _) = leave_setup(&["SW-5"]);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    assert!(
        !choice_labels(&runner).contains(&"Hand".to_string()),
        "no playable hand card: {:?}",
        choice_labels(&runner)
    );
    let _ = runner.auto_resolve();
    assert!(super::support::hand_contains(&runner, 0, "SW-5"), "SW-5 stays in hand");
}

/// The digivolution-card zone uses the same filter: with a play prohibition
/// covering Erlangmon's only level 5 [SW] source, "Sources" isn't offered.
#[test]
fn ex12_034_leave_offers_no_sources_zone_when_its_cards_cannot_be_played() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .add_card(super::support::red_sw("SW-5", 5))
        .add_card(plain_digimon("VICTIM-A", CardColor::Red, 4, 4000))
        .memory(10)
        .start();
    runner.place_stack(0, &["SW-5", CARD_ID]);
    runner.place_on_field(0, "VICTIM-A", Some(0));
    ex12_034_dp_lock(&mut runner, 4000);

    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    assert!(
        !choice_labels(&runner).contains(&"Sources".to_string()),
        "no playable source: {:?}",
        choice_labels(&runner)
    );
    let _ = runner.auto_resolve();
    assert!(!field_contains(&runner, 0, "SW-5"), "SW-5 was not played");
}

/// Positive control for the test above: the same source with no play
/// prohibition is playable, so "Sources" is offered and choosing it plays it.
#[test]
fn ex12_034_leave_offers_sources_zone_when_a_source_can_be_played() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-034 YAML loads")
        .add_card(super::support::red_sw("SW-5", 5))
        .add_card(plain_digimon("VICTIM-A", CardColor::Red, 4, 4000))
        .memory(10)
        .start();
    runner.place_stack(0, &["SW-5", CARD_ID]);
    runner.place_on_field(0, "VICTIM-A", Some(0));

    delete_own(&mut runner, "VICTIM-A");
    runner.accept_optional_trigger().expect("accept the replacement");
    assert_eq!(
        choice_labels(&runner),
        vec!["Sources".to_string(), "Don't play".to_string()]
    );
    runner.execute_branch(0).expect("choose Sources");
    let _ = select_first_non_pass(&mut runner);
    let _ = runner.auto_resolve();
    assert!(field_contains(&runner, 0, "SW-5"), "SW-5 was played from the sources");
}
