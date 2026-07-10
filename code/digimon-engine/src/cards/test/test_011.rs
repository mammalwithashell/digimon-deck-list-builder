//! TEST-011: "On Play: Trash 1 card from your hand, then draw 2."
//! Pilot for `select_hand` — mandatory (caller must pick a hand card),
//! with a trash-then-draw side effect on the resolved pick.
//!
//! Clone-safety worked example (rule 28): the continuation is parked as a
//! DATA `ResumeFrame` alongside the coexistence closure. `run_resume`'s
//! `Hand` arm binds the picked hand index (`insert_hand_index`), then the
//! compiled tail trashes the bound card and draws 2 — so a `Game` cloned at
//! this prompt resolves faithfully through the resumable VM.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::{CardEffect, Effect};
use crate::resume::{ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack};

pub struct Test011;

impl CardEffect for Test011 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Trash 1 from hand, draw 2")
            .process(|ctx| {
                let controller = ctx.player;
                ctx.select_hand(
                    controller,
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
                // Park the data frame alongside the closure: bind the picked
                // hand index, trash it, then draw 2 — all as compiled steps.
                if ctx.game.pending_selection.is_some() {
                    ctx.game.pending_selection_resume = Some(ResumeStack {
                        frames: vec![ResumeFrame::RunTail {
                            prov: ResumeProvenance {
                                source_card: ctx.source_card,
                                source_permanent: ctx.source_permanent,
                                source_kind: ctx.source_kind,
                                controller,
                                override_pin: ctx.override_selecting_player(),
                            },
                            select_kind: ResumeSelectKind::Hand {
                                of_player: controller,
                            },
                            bind_as: Some("test011_pick".to_string()),
                            inner_tail: Arc::new(vec![
                                CompiledStep::TrashFromHandByIndex {
                                    of: CompiledPlayerRef::You,
                                    hand_index: CompiledBindingRef::Named(
                                        "test011_pick".to_string(),
                                    ),
                                },
                                CompiledStep::Draw {
                                    of: CompiledPlayerRef::You,
                                    count: 2,
                                },
                            ]),
                            outer_conts: Vec::new(),
                            bindings: Bindings::new(),
                            runtime: StepRuntime::default(),
                            trigger_context: ctx.game.current_trigger_context.clone(),
                            decline: ResumeDecline::None,
                        }],
                    });
                }
            })
            .build()]
    }
}
