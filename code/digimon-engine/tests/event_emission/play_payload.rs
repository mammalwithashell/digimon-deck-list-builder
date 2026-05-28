//! `GameEvent::Play` payload tests for `cost_paid`, `cost_printed`, and
//! `via_alt_path`. Covers the scenarios in
//! `openspec/changes/add-reward-profiles/specs/engine-event-emission/spec.md`
//! under "GameEvent::Play carries cost-paid and alt-path payload".

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;

fn drained_plays(r: &mut DebugRunner) -> Vec<GameEvent> {
    r.game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::Play { .. }))
        .collect()
}

fn cheap_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(2000);
    card.play_cost = 3;
    card.level = Some(3);
    card
}

#[test]
fn effect_initiated_free_play_emits_cost_paid_zero_and_no_alt_path() {
    // Scenario: effect-initiated free play (Davis & Ken-style) emits a
    // Play event with cost_paid=0, cost_printed=<printed>, via_alt_path=None.
    // Goes through `Game::play_card_from_effect_without_cost`
    // (game.rs:1158-1197), which is the canonical effect-driven free
    // play path.
    let mut r = DebugRunner::builder()
        .add_card(cheap_digimon("PLAY-3"))
        .hand(0, &["PLAY-3"])
        .start();

    r.game.drain_events();

    let card_handle = r.game.players[0].hand[0].handle();
    let _entered = r
        .game
        .play_card_from_effect_without_cost(0, card_handle)
        .expect("effect free play should succeed");

    let plays = drained_plays(&mut r);
    assert_eq!(
        plays.len(),
        1,
        "exactly one Play event SHALL fire per effect-initiated play; got {plays:?}"
    );

    let GameEvent::Play {
        card_id,
        cost_paid,
        cost_printed,
        via_alt_path,
        ..
    } = plays[0].clone()
    else {
        unreachable!()
    };

    assert_eq!(card_id, "PLAY-3");
    assert_eq!(cost_paid, 0, "effect-initiated free play pays no memory");
    assert_eq!(
        cost_printed, 3,
        "cost_printed SHALL reflect CardData::play_cost"
    );
    assert_eq!(
        via_alt_path, None,
        "generic effect-initiated free plays SHALL NOT surface an alt-path key"
    );
}

#[test]
fn standard_play_from_hand_emits_cost_paid_post_discount_and_no_alt_path() {
    // Scenario: standard PLAY-from-hand path with a tamer-style cost
    // reduction. cost_paid reflects the post-discount memory paid;
    // cost_printed is the un-discounted CardData::play_cost; via_alt_path
    // is None (the standard PLAY action is not an alt-path).
    //
    // We exercise via `play_from_hand_with_cost(... CostDelta::Reduce(N))`
    // which is the engine-level entry point that
    // `commit_play_from_hand_card_no_replace` (site 3) ultimately calls.
    use digimon_engine::{CostDelta, PlaySource};

    let mut r = DebugRunner::builder()
        .add_card(cheap_digimon("PLAY-3"))
        .hand(0, &["PLAY-3"])
        .start();
    // Seed memory to cover the printed cost.
    r.game.set_memory(3);
    r.game.drain_events();

    // Play with a 1-memory discount; effective_cost = 3 - 1 = 2.
    let _entered = r
        .game
        .play_from_hand_with_cost(0, 0, CostDelta::Reduce(1), PlaySource::ByHand)
        .expect("standard PLAY with discount should succeed");

    let plays = drained_plays(&mut r);
    assert_eq!(
        plays.len(),
        1,
        "exactly one Play event SHALL fire per standard play"
    );

    let GameEvent::Play {
        card_id,
        cost_paid,
        cost_printed,
        via_alt_path,
        ..
    } = plays[0].clone()
    else {
        unreachable!()
    };

    assert_eq!(card_id, "PLAY-3");
    assert_eq!(
        cost_paid, 2,
        "cost_paid SHALL reflect post-discount memory (3 printed - 1 reduce = 2)"
    );
    assert_eq!(cost_printed, 3, "cost_printed SHALL equal the printed cost");
    assert_eq!(
        via_alt_path, None,
        "standard PLAY action (even with discount) SHALL NOT surface an alt-path key"
    );
}
