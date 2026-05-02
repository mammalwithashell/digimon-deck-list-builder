//! TEST-012: "On Play (Choose one): Gain 2 memory / Draw 2 cards."
//! Pilot for `select_effect_choice` — mandatory branch pick with no PASS.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test012;

impl CardEffect for Test012 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Choose: gain 2 memory / draw 2")
            .process(|ctx| {
                ctx.select_effect_choice(
                    "Choose one",
                    vec!["Gain 2 memory".to_string(), "Draw 2 cards".to_string()],
                    |ctx, choice| match choice {
                        0 => ctx.gain_memory(2),
                        1 => {
                            let me = ctx.player;
                            ctx.draw(me, 2);
                        }
                        _ => {}
                    },
                );
            })
            .build()]
    }
}
