//! TEST-011: "On Play: Trash 1 card from your hand, then draw 2."
//! Pilot for `select_hand` — mandatory (caller must pick a hand card),
//! with a trash-then-draw side effect inside the callback.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test011;

impl CardEffect for Test011 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Trash 1 from hand, draw 2")
            .process(|ctx| {
                ctx.select_hand(
                    ctx.player,
                    "Trash a card from your hand",
                    /* is_optional = */ false,
                    |_game, _i| true, // any hand card is eligible
                    |ctx, hand_index| {
                        let me = ctx.player;
                        let player = ctx.game.player_mut(me);
                        if hand_index < player.hand.len() {
                            let card = player.hand.remove(hand_index);
                            player.trash.push(card);
                        }
                        ctx.draw(me, 2);
                    },
                );
            })
            .build()]
    }
}
