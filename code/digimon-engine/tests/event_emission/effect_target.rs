//! `GameEvent::EffectTarget` emission tests. Covers the
//! `engine-event-emission` requirement "Rust engine emits EffectTarget
//! events at selection commit" (change `enrich-match-log-card-identity`),
//! including the forced single-target scenario.

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::events::GameEvent;

fn fighter(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.dp = Some(3000);
    card
}

/// On-play effect that selects one opponent Digimon (and does nothing with
/// it). The selection install + commit is all we need to observe.
struct TargetOpponent;
impl CardEffect for TargetOpponent {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Target an opponent Digimon")
            .process(|ctx| {
                ctx.select_opponent_permanent(
                    "Choose an opponent Digimon",
                    false,
                    |_g, _h| true,
                    |_ctx, _h| {},
                );
            })
            .build()]
    }
}

#[test]
fn forced_single_target_selection_emits_effect_target() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("SRC", "Greymon"))
        .add_card(fighter("TGT", "Tyrannomon"))
        .start();
    r.register_effect("SRC", Arc::new(TargetOpponent));

    let src = r.place_on_field(0, "SRC", Some(0));
    let _tgt = r.place_on_field(1, "TGT", Some(0));
    r.game.drain_events();

    // Install the selection (single legal target — no auto-pick).
    r.fire_on_play(0, src.index as usize);
    assert!(
        r.pending_selection().is_some(),
        "even a single legal target installs a selection (no-approximations)"
    );

    // Drain install-time events, then commit the pick.
    r.game.drain_events();
    r.auto_resolve().expect("resolve the forced single-target pick");

    let targets: Vec<GameEvent> = r
        .game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::EffectTarget { .. }))
        .collect();
    assert_eq!(
        targets.len(),
        1,
        "committing the target SHALL emit one EffectTarget; got {targets:?}"
    );

    let GameEvent::EffectTarget {
        source_card_id,
        source_card_name,
        targets: chosen,
        ..
    } = targets[0].clone()
    else {
        unreachable!()
    };
    assert_eq!(source_card_id, "SRC", "source effect card id");
    assert_eq!(source_card_name, "Greymon", "source effect card name");
    assert_eq!(chosen.len(), 1, "one target chosen");
    assert_eq!(chosen[0].card_id, "TGT", "target card id");
    assert_eq!(chosen[0].card_name, "Tyrannomon", "target card name");
}
