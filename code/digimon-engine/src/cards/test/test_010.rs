//! TEST-010: "On Play: Delete 1 of your opponent's Digimon. (Optional)"
//! Pilot for the selection subsystem — exercises `select_opponent_permanent`
//! with a mandatory/optional flip and a delete on the resolved pick.
//!
//! Clone-safety worked example (rule 28): the continuation is parked as a
//! DATA `ResumeFrame` alongside the coexistence closure, so a `Game` cloned
//! at this prompt resolves faithfully through the resumable VM (`run_resume`'s
//! `FieldPermanent` arm decodes the pick, binds it, and runs the compiled
//! tail). Never install a closure-only `pending_selection`.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::{CardEffect, Effect};
use crate::resume::{ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack};

pub struct Test010;

impl CardEffect for Test010 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delete 1 opp Digimon (optional)")
            .process(|ctx| {
                let opponent = ctx.game.next_clockwise(ctx.player);
                ctx.select_opponent_permanent(
                    "Delete 1 of your opponent's Digimon",
                    /* is_optional = */ true,
                    // Only Digimon are valid targets — Tamer slots are skipped.
                    |game, handle| {
                        game.player(handle.player)
                            .battle_area
                            .get(handle.index as usize)
                            .map(|p| p.is_digimon(&game.card_data))
                            .unwrap_or(false)
                    },
                    |ctx, chosen| {
                        ctx.delete_permanent(chosen);
                    },
                );
                // Park the data frame alongside the closure: bind the pick,
                // delete it via a compiled step. The optional decline is a
                // clean no-op (empty tail — outer continuations still run).
                if ctx.game.pending_selection.is_some() {
                    ctx.game.pending_selection_resume = Some(ResumeStack {
                        frames: vec![ResumeFrame::RunTail {
                            prov: ResumeProvenance {
                                source_card: ctx.source_card,
                                source_permanent: ctx.source_permanent,
                                source_kind: ctx.source_kind,
                                controller: ctx.player,
                                override_pin: ctx.override_selecting_player(),
                            },
                            select_kind: ResumeSelectKind::FieldPermanent {
                                of_player: opponent,
                                post: None,
                            },
                            bind_as: Some("test010_target".to_string()),
                            inner_tail: Arc::new(vec![CompiledStep::DeletePermanent {
                                target: CompiledBindingRef::Named("test010_target".to_string()),
                            }]),
                            outer_conts: Vec::new(),
                            bindings: Bindings::new(),
                            runtime: StepRuntime::default(),
                            trigger_context: ctx.game.current_trigger_context.clone(),
                            decline: ResumeDecline::RunTail {
                                tail: Arc::new(Vec::new()),
                                aborts_clause: false,
                            },
                        }],
                    });
                }
            })
            .build()]
    }
}
