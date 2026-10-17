//! A used Option that a `WhenWouldBeTrashed` replacement sends to its owner's
//! hand or deck instead of the trash (§9-1-5's trash is the original event)
//! logs a `GameEvent::CardToHand` / `GameEvent::CardToDeck`, so the event log
//! accounts for the card the way `GameEvent::Trash` does for the ordinary
//! outcome (F-ENGINE-EFFECT-USED-OPTION-MAIN-IS-ORDERABLE residual: "a used
//! Option redirected to hand or deck emits no event").

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, Zone};
use digimon_engine::events::GameEvent;
use digimon_engine::replacement::ReplacementSubject;

fn option_card(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = 0;
    card
}

/// "[Main] (nothing)" -- the Option's body.
struct MainNoop;

impl CardEffect for MainNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("[Main] noop")
            .option_main()
            .process(|_ctx| {})
            .build()]
    }
}

/// "When a card would be trashed, it goes to `zone` instead" -- a mandatory
/// replacement on the owner's Digimon, scoped to CARD subjects (a used
/// Option), never to a permanent's deletion.
struct RedirectCardTrash(Zone);

impl CardEffect for RedirectCardTrash {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let zone = self.0;
        vec![Effect::when_would_be_trashed(card)
            .name("used card goes elsewhere instead")
            .replacement_condition(|_ctx, subject| {
                matches!(subject, ReplacementSubject::Card(_, _))
            })
            .replacement_process(move |ctx| ctx.redirect_to(zone))
            .build()]
    }
}

fn use_redirected_option(zone: Zone) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ANCHOR", "Anchormon"))
        .add_card(make_test_card("GUARD", "Guardmon"))
        .add_card(option_card("OPT-USED"))
        .hand(0, &["OPT-USED"])
        .deck(0, &["ANCHOR"; 5])
        .deck(1, &["ANCHOR"; 5])
        .memory(10)
        .start();
    r.register_effect("OPT-USED", Arc::new(MainNoop));
    r.register_effect("GUARD", Arc::new(RedirectCardTrash(zone)));
    r.place_on_field(0, "ANCHOR", None);
    r.place_on_field(0, "GUARD", None);
    let cp = r.event_checkpoint();
    let _ = r.game.play_option_from_hand(0, 0);
    assert!(r.pending_selection().is_none(), "a mandatory replacement asks nothing");
    assert!(
        r.events_since(cp)
            .iter()
            .all(|e| !matches!(e, GameEvent::Trash { card_id, .. } if card_id == "OPT-USED")),
        "the trash was replaced: no GameEvent::Trash for the Option"
    );
    assert!(
        r.game.player(0).trash.iter().all(|c| c.card_id(&r.game.card_data) != "OPT-USED"),
        "not in the trash"
    );
    r
}

#[test]
fn used_option_redirected_to_hand_logs_card_to_hand() {
    let r = use_redirected_option(Zone::Hand);
    assert_eq!(r.hand_size(0), 1, "back in the hand");
    let event = r.game.events.iter().find(|e| matches!(e, GameEvent::CardToHand { .. }));
    match event {
        Some(GameEvent::CardToHand {
            player,
            card_id,
            card_name,
            source,
            ..
        }) => {
            assert_eq!(*player, 0);
            assert_eq!(card_id, "OPT-USED");
            assert_eq!(card_name, "OPT-USED");
            assert_eq!(source, "option_use");
        }
        other => panic!("expected GameEvent::CardToHand, got {other:?}"),
    }
}

#[test]
fn used_option_redirected_to_deck_logs_card_to_deck() {
    let r = use_redirected_option(Zone::Deck);
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(
        r.game.player(0).deck.first().map(|c| c.card_id(&r.game.card_data).to_string()),
        Some("OPT-USED".to_string()),
        "at the bottom of the deck"
    );
    let event = r.game.events.iter().find(|e| matches!(e, GameEvent::CardToDeck { .. }));
    match event {
        Some(GameEvent::CardToDeck {
            player,
            card_id,
            position,
            source,
            ..
        }) => {
            assert_eq!(*player, 0);
            assert_eq!(card_id, "OPT-USED");
            assert_eq!(position, "bottom");
            assert_eq!(source, "option_use");
        }
        other => panic!("expected GameEvent::CardToDeck, got {other:?}"),
    }
}
