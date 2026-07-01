//! TEST-012: "On Play (Choose one): Gain 2 memory / Draw 2 cards."
//! Pilot for `select_effect_choice` — mandatory branch pick with no PASS.
//!
//! Clone-safety worked example (rule 28): the branch bodies are parked as
//! DATA (`EffectChoicePostAction::RunTailBranch` — one compiled tail per
//! label) alongside the coexistence closure, so a `Game` cloned at this
//! prompt resolves faithfully through the resumable VM.

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};

use crate::card_source::CardHandle;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::{CardEffect, Effect};
use crate::resume::{
    EffectChoicePostAction, ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind,
    ResumeStack,
};

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
                // Park the data frame alongside the closure: one compiled
                // tail per branch, dispatched on the chosen label index.
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
                            select_kind: ResumeSelectKind::EffectChoice {
                                post: Some(EffectChoicePostAction::RunTailBranch {
                                    branches: vec![
                                        Arc::new(vec![CompiledStep::GainMemory(2)]),
                                        Arc::new(vec![CompiledStep::Draw {
                                            of: CompiledPlayerRef::You,
                                            count: 2,
                                        }]),
                                    ],
                                }),
                            },
                            bind_as: None,
                            inner_tail: Arc::new(Vec::new()),
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
