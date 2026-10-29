//! Hypothesis-owned hidden composition. No true opponent hidden identity is
//! used to choose, fabricate, or order a hypothetical card.
use super::beliefs::{BeliefEvidence, KnowledgeLedger, KnowledgeZone, WeightedDeckPrior};
use crate::{
    card_source::{CardHandle, CardSource},
    player::OriginalDeckCardCount,
    Game,
};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::collections::{BTreeMap, HashSet};

/// Validate continuation support before producing training data. Unreviewed
/// shapes fail explicitly instead of inheriting a true-hand candidate cache.
fn audit(root: &mut Game, viewer: u8, memory: &KnowledgeLedger) -> Result<(), String> {
    if super::beliefs::field_sources_with_controller(root)
        .any(|(controller, c)| c.face_down && c.owner != controller)
    {
        return Err("closed-list foreign-owned concealed field sources require an ownership-aware allocator".into());
    }
    if root.pending_effect_security_removal.iter().any(|s| {
        !matches!(
            s.destination,
            crate::selection::SecurityRemovalDestination::Trash
        )
    }) {
        return Err("closed-list concealed security transit requires destination rebinding".into());
    }
    if memory.concealed_positions().iter().any(|c| {
        !matches!(
            c.zone,
            KnowledgeZone::Deck
                | KnowledgeZone::Security
                | KnowledgeZone::Hand
                | KnowledgeZone::Field
        )
    }) {
        return Err("closed-list concealed security transit requires destination rebinding".into());
    }
    if root.players.iter().any(|p| {
        p.hand
            .iter()
            .chain(&p.deck)
            .chain(&p.security)
            .chain(&p.digitama_deck)
            .any(|c| c.owner != p.id)
    }) {
        return Err(
            "closed-list foreign-owned concealed zones require an ownership-aware allocator".into(),
        );
    }
    if root.players.iter().any(|p| {
        p.hand
            .iter()
            .chain(&p.deck)
            .chain(&p.security)
            .chain(&p.digitama_deck)
            .any(|c| {
                c.is_token
                    || !c.also_treated_as.is_empty()
                    || c.reveal_overlay.is_some()
                    || c.face_down
            })
    }) {
        return Err(
            "closed-list transformed concealed cards require entitled metadata rebinding".into(),
        );
    }
    let mut allowed = HashSet::new();
    for p in &root.players {
        for c in p
            .trash
            .iter()
            .chain(
                p.battle_area
                    .iter()
                    .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
            )
            .chain(
                p.breeding_area
                    .iter()
                    .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
            )
            .chain(p.commander_zone.iter())
            .chain(
                p.security
                    .iter()
                    .filter(|c| p.face_up_security.contains(&c.card_index)),
            )
        {
            allowed.insert(c.handle());
        }
    }
    allowed.extend(
        root.players[viewer as usize]
            .hand
            .iter()
            .map(|c| c.handle()),
    );
    allowed.extend(
        root.revealed_cards
            .iter()
            .chain(root.pending_option.iter().map(|p| &p.card))
            .chain(super::beliefs::public_pending_security(root))
            .map(|c| c.handle()),
    );
    allowed.extend(
        memory
            .hidden_instances(1 - viewer)
            .iter()
            .map(|c| CardHandle(c.0)),
    );
    allowed.extend(
        memory
            .hidden_instances(viewer)
            .iter()
            .map(|c| CardHandle(c.0)),
    );
    allowed.extend(
        memory
            .concealed_positions()
            .iter()
            .map(|c| CardHandle(c.instance)),
    );
    allowed.extend(memory.public_tokens().iter().copied().map(CardHandle));
    allowed.extend(
        root.digixros_leaving_limbo
            .iter()
            .flat_map(|(_, _, p)| p.card_sources.iter().chain(&p.linked_cards))
            .map(|c| c.handle()),
    );
    let mut identities = allowed.clone();
    let known_instances: HashSet<_> = memory
        .hidden_instances(0)
        .iter()
        .chain(memory.hidden_instances(1).iter())
        .map(|c| c.0)
        .chain(
            root.players[viewer as usize]
                .hand
                .iter()
                .map(|c| c.card_index),
        )
        .chain(
            super::beliefs::field_sources(root)
                .filter(|c| c.owner == viewer)
                .map(|c| c.card_index),
        )
        .collect();
    for c in memory.concealed_positions() {
        if !known_instances.contains(&c.instance) {
            identities.remove(&CardHandle(c.instance));
        }
    }
    let known_opponent = memory.hidden_instances(1 - viewer);
    for c in super::beliefs::field_sources(root).filter(|c| c.face_down && c.owner != viewer) {
        if !known_opponent.iter().any(|k| k.0 == c.card_index) {
            identities.remove(&c.handle());
        }
    }
    let context_safe = |c: &Option<crate::trigger_context::TriggerContext>| {
        c.as_ref()
            .is_none_or(|c| c.belief_safe(&allowed) && c.belief_snapshots_safe(&identities))
    };
    let outer_safe = |conts: &[crate::resume::OuterContinuation]| {
        conts.iter().all(|c| {
            identities.contains(&c.source_card)
                && c.bindings.belief_safe(viewer, &allowed)
                && context_safe(&c.trigger_context)
        })
    };
    if !context_safe(&root.current_trigger_context) {
        return Err("closed-list active trigger context refers to concealed metadata".into());
    }
    if root
        .effect_digivolve_scopes
        .iter()
        .any(|s| !s.declaration.belief_safe(viewer, &identities))
    {
        return Err("closed-list effect digivolution scope requires audit/rebinding".into());
    }
    if let Some(t) = &root.pending_digixros_transaction {
        let played = identities.contains(&t.played_card);
        let selected = t
            .selected_materials
            .iter()
            .chain(&t.pre_attached_materials)
            .all(|m| identities.contains(&m.origin.card()));
        let wildcards = t
            .wildcard_substitutions
            .iter()
            .all(|s| identities.contains(&s.material_card));
        if !(played && selected && wildcards) {
            return Err(format!("closed-list DigiXros transaction requires audit/rebinding (played={played}, selected={selected}, wildcards={wildcards})"));
        }
    }
    if !root.loop_watch.is_empty() {
        return Err("closed-list loop history requires hypothetical rebinding".into());
    }
    if root.pending_selection.is_some() && root.pending_selection_resume.is_none() {
        return Err("closed-list legacy callback prompt is unsupported".into());
    }
    if root
        .dsl_outer_tail
        .as_ref()
        .is_some_and(|(_, bindings, _)| !bindings.belief_safe(viewer, &allowed))
    {
        return Err("closed-list outer bindings refer to concealed cards".into());
    }
    // A queued security observer may outlive anonymous hand membership. Only
    // erase reference fields proven unused by its entire compiled clause;
    // never infer their identity/location from the real concealed card.
    for index in 0..root.effect_queue.len() {
        let queued = &root.effect_queue[index];
        let certified = queued.granted_effect_id.is_none() && queued.keyword_effect.is_none()
            && root.effects_for_card(&queued.card_id, queued.source_card)
                .and_then(|effects| effects.get(queued.effect_slot as usize)
                    .map(|e| e.ignores_event_card_identity)).unwrap_or(false);
        if certified {
            if let Some(context) = &mut root.effect_queue[index].trigger_context {
                if context.event_card.is_some_and(|h| !allowed.contains(&h)) { context.event_card = None; }
                if context.target_card.is_some_and(|h| !allowed.contains(&h)) { context.target_card = None; }
                if matches!(context.subject, Some(crate::trigger_context::EventSubject::Card { card, .. }) if !allowed.contains(&card)) {
                    context.subject = None;
                }
            }
        }
    }
    if root.effect_queue.iter().any(|effect| {
        !identities.contains(&effect.source_card)
            || !context_safe(&effect.trigger_context)
            || effect
                .attribution_source_card
                .is_some_and(|h| !allowed.contains(&h))
    }) {
        let details: Vec<_> = root
            .effect_queue
            .iter()
            .filter(|e| !identities.contains(&e.source_card) || !context_safe(&e.trigger_context))
            .map(|e| {
                (
                    &e.card_id,
                    identities.contains(&e.source_card),
                    context_safe(&e.trigger_context),
                    e.trigger_context.as_ref().map(|c| {
                        c.belief_missing_handles(&allowed)
                            .into_iter()
                            .map(|h| {
                                let location = root.players.iter().find_map(|p| {
                                    [
                                        ("hand", &p.hand),
                                        ("deck", &p.deck),
                                        ("security", &p.security),
                                        ("trash", &p.trash),
                                    ]
                                    .into_iter()
                                    .find_map(|(z, cards)| {
                                        cards
                                            .iter()
                                            .position(|c| c.handle() == h)
                                            .map(|i| (p.id, z, i))
                                    })
                                });
                                (h, location)
                            })
                            .collect::<Vec<_>>()
                    }),
                )
            })
            .collect();
        return Err(format!(
            "closed-list effect queue refers to concealed cards: {details:?}"
        ));
    }
    if root.pending_would_play_resume.as_ref().is_some_and(|p| {
        p.player != viewer
            && !(allowed.contains(&p.card)
                && matches!(p.origin, crate::game::PendingWouldPlayOrigin::Hand))
    }) {
        return Err("closed-list foreign pending play requires declaration rebinding".into());
    }
    for hook in &root.after_selection_resume_hooks.0 {
        use crate::resume::AfterSelectionHook;
        let safe = match hook {
            AfterSelectionHook::InteractiveOptionUseReducer { player_id, .. } => {
                *player_id == viewer
            }
            AfterSelectionHook::PlayCostContinuation {
                target, processed, ..
            } => {
                target.belief_safe(&identities)
                    && processed.iter().all(|k| k.belief_safe(&identities))
            }
            AfterSelectionHook::InteractiveDigivolveReducer { acting_player, .. } => {
                *acting_player == viewer
            }
            AfterSelectionHook::DigivolveReducerChain { state, .. } => {
                state.belief_safe(viewer, &identities)
            }
            AfterSelectionHook::PartitionSecondPlay {
                source_card, card, ..
            } => allowed.contains(source_card) && allowed.contains(card),
            AfterSelectionHook::DigiXrosLeaveContinuation { cont } => cont.belief_safe(&identities),
            _ => false,
        };
        if !safe {
            return Err(format!(
                "closed-list after-selection hook requires audit: {}",
                format!("{hook:?}")
                    .split([' ', '(', '{'])
                    .next()
                    .unwrap_or("unknown")
            ));
        }
    }
    if let Some(stack) = &root.pending_selection_resume {
        use crate::resume::ResumeFrame;
        let mut frames: Vec<_> = stack.frames.iter().collect();
        while let Some(frame) = frames.pop() {
            let safe = match frame {
                ResumeFrame::RunTail {
                    prov,
                    bindings,
                    outer_conts,
                    select_kind,
                    trigger_context,
                    ..
                } => {
                    identities.contains(&prov.source_card)
                        && bindings.belief_safe(viewer, &allowed)
                        && outer_safe(outer_conts)
                        && context_safe(trigger_context)
                        && !matches!(select_kind,
                        crate::resume::ResumeSelectKind::Hand {of_player} if *of_player != viewer)
                }
                ResumeFrame::BlockSelection(s) => outer_safe(&s.outer_conts),
                ResumeFrame::CounterSelection(s) => {
                    s.defender_player == viewer && outer_safe(&s.outer_conts)
                }
                ResumeFrame::RaidSelection(s) => outer_safe(&s.outer_conts),
                ResumeFrame::AllianceSelection(s) => outer_safe(&s.outer_conts),
                ResumeFrame::KeywordAscensionChoice(s) => {
                    allowed.contains(&s.prov.source_card)
                        && allowed.contains(&s.self_card)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::MayDnaPartnerSelection(s) => {
                    s.controller == viewer
                        && allowed.contains(&s.prov.source_card)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::MayDnaResultSelection(s) => {
                    s.controller == viewer
                        && allowed.contains(&s.prov.source_card)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::InteractiveOptionUseCostReductionPrompt(s) => {
                    s.belief_safe(viewer, &identities) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::DelayPlayFromHandSelection(s) => {
                    s.belief_safe(viewer, &identities) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::DelayFieldDnaSelection(s) => {
                    s.belief_safe(viewer, &identities) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::PlayOrUseDualChoice(s) => {
                    s.player == viewer
                        && identities.contains(&s.prov.source_card)
                        && s.source_ref.is_none_or(|s| identities.contains(&s.card))
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::DigiXrosMaterialSelection(s) => {
                    s.player_id == viewer
                        && allowed.contains(&s.target_card)
                        && s.candidates.iter().all(|c| identities.contains(&c.card()))
                        && s.selected_pins.iter().all(|p| {
                            use crate::owned_card_choice::CardChoicePin;
                            let h = match p {
                                CardChoicePin::Source(p) => p.card(),
                                CardChoicePin::Hand { card, .. }
                                | CardChoicePin::Trash { card, .. }
                                | CardChoicePin::Security { card, .. }
                                | CardChoicePin::BattleTop { card, .. } => *card,
                            };
                            identities.contains(&h)
                        })
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::TrashOptionFromStackSelection(s) => {
                    allowed.contains(&s.prov.source_card)
                        && s.cards.iter().all(|(h, _)| identities.contains(h))
                        && s.bindings.belief_safe(viewer, &allowed)
                        && context_safe(&s.trigger_context)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::ReplacementBody { inner, outer_conts } => {
                    frames.extend(inner.frames.iter());
                    outer_safe(outer_conts)
                }
                ResumeFrame::DelayBodyAfterCost {
                    inner,
                    continuation,
                    outer_conts,
                } => {
                    frames.extend(inner.frames.iter());
                    continuation.belief_safe(&identities) && outer_safe(outer_conts)
                }
                ResumeFrame::DnaDigivolveFirstMaterial(s) => {
                    s.player_id == viewer && allowed.contains(&s.source_card)
                }
                ResumeFrame::DnaDigivolveSecondMaterial(s) => s.player_id == viewer,
                ResumeFrame::ArtsDigivolveSelection(s) => outer_safe(&s.outer_conts),
                ResumeFrame::PlayFromHandCostReductionPrompt(s) => {
                    s.belief_safe(viewer, &identities) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::UseOptionFromHandStep(s) => {
                    s.of_player == viewer
                        && allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && context_safe(&s.trigger_context)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::UseOptionFromTrashStep(s) => {
                    allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && context_safe(&s.trigger_context)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::SourceMultiStep(s) => {
                    allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && s.filter_bindings.belief_safe(viewer, &allowed)
                        && s.picked_pins.iter().all(|p| allowed.contains(&p.card()))
                        && context_safe(&s.trigger_context)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::RefireEffectChoice(s) => {
                    s.effects.iter().all(|e| {
                        identities.contains(&e.source_card)
                            && e.attribution_source_card
                                .is_none_or(|h| allowed.contains(&h))
                    }) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::MultiPickStep(s) => {
                    use crate::effect_context::selections::CountCappedZone;
                    (!matches!(s.zone, CountCappedZone::Hand) || s.of_player == viewer)
                        && allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && context_safe(&s.trigger_context)
                        && s.accum.iter().all(|h| allowed.contains(h))
                        && s.source_pool
                            .as_ref()
                            .is_none_or(|p| p.iter().all(|p| allowed.contains(&p.card())))
                        && s.named_groups
                            .as_ref()
                            .is_none_or(|g| g.members.iter().all(|(h, _)| identities.contains(h)))
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::NonDslCountCappedStep(s) => {
                    use crate::{
                        effect_context::selections::CountCappedZone,
                        resume::NonDslCountCappedTerminal,
                    };
                    let terminal_safe = match &s.terminal {
                        NonDslCountCappedTerminal::KeywordPartitionSlots {
                            picked_so_far, ..
                        } => picked_so_far.iter().all(|h| allowed.contains(h)),
                        NonDslCountCappedTerminal::Assembly {
                            player,
                            target_card,
                            picked_so_far,
                            ..
                        } => {
                            *player == viewer
                                && allowed.contains(target_card)
                                && picked_so_far.iter().all(|h| allowed.contains(h))
                        }
                        _ => true,
                    };
                    terminal_safe
                        && (!matches!(s.zone, CountCappedZone::Hand) || s.of_player == viewer)
                        && allowed.contains(&s.prov.source_card)
                        && s.accum.iter().all(|h| allowed.contains(h))
                        && s.source_candidates
                            .as_ref()
                            .is_none_or(|p| p.iter().all(|p| allowed.contains(&p.card())))
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::CounterBlastDnaFieldMaterial(s) => {
                    s.defender == viewer
                        && allowed.contains(&s.source_card)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::CounterBlastDnaHandMaterial(s) => {
                    s.defender == viewer && outer_safe(&s.outer_conts)
                }
                // Route candidates depend on this seat's own hand and public
                // host, which are preserved. A foreign hand is not entitled.
                ResumeFrame::DigivolveCostChoice(s) => s.acting_player == viewer,
                ResumeFrame::DigivolveReducerPrompt(s)
                | ResumeFrame::DigivolveReducerSuspend(s) => s.belief_safe(viewer, &identities),
                ResumeFrame::EffectDigivolveCostChoice(s) => s.belief_safe(viewer, &identities),
                ResumeFrame::RevealBucketStep(s) => {
                    allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && context_safe(&s.trigger_context)
                        && s.buckets
                            .iter()
                            .flat_map(|b| &b.candidates)
                            .chain(s.picked_buckets.iter().flat_map(|(_, h)| h))
                            .chain(&s.current_bucket_picks)
                            .all(|h| allowed.contains(h))
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::ZoneCardPickStep(s) => {
                    allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && context_safe(&s.trigger_context)
                        && s.candidates
                            .iter()
                            .chain(&s.picked)
                            .all(|h| allowed.contains(h))
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::AttackDeclarationDecision(s) => {
                    allowed.contains(&s.source_card) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::OptionalReplacement(s) => {
                    s.belief_safe(viewer, &allowed) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::CountCappedPermanentsStep(s) => {
                    allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::PermutationStep(s) => {
                    allowed.contains(&s.prov.source_card)
                        && s.bindings.belief_safe(viewer, &allowed)
                        && s.remaining
                            .iter()
                            .chain(&s.accum)
                            .all(|h| allowed.contains(h))
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::OuterOptionalTrigger(s) => {
                    identities.contains(&s.queued_effect.source_card)
                        && context_safe(&s.queued_effect.trigger_context)
                        && outer_safe(&s.outer_conts)
                }
                ResumeFrame::TriggerOrderSelection(s) => outer_safe(&s.outer_conts),
                _ => false,
            };
            if !safe {
                let detail = if let ResumeFrame::RunTail {
                    prov,
                    bindings,
                    outer_conts,
                    trigger_context,
                    select_kind,
                    ..
                } = frame
                {
                    format!(
                        " (source={}, bindings={}, outer={}, context={}, selection={:?}{})",
                        allowed.contains(&prov.source_card),
                        bindings.belief_safe(viewer, &allowed),
                        outer_safe(outer_conts),
                        context_safe(trigger_context),
                        select_kind,
                        if context_safe(trigger_context) {
                            String::new()
                        } else {
                            let missing: Vec<_> = trigger_context
                                .as_ref()
                                .unwrap()
                                .belief_missing_handles(&allowed)
                                .into_iter()
                                .map(|h| {
                                    let location = root.players.iter().find_map(|p| {
                                        [
                                            ("hand", &p.hand),
                                            ("deck", &p.deck),
                                            ("security", &p.security),
                                            ("trash", &p.trash),
                                        ]
                                        .into_iter()
                                        .find_map(
                                            |(z, cards)| {
                                                cards
                                                    .iter()
                                                    .position(|c| c.handle() == h)
                                                    .map(|i| (p.id, z, i))
                                            },
                                        )
                                    });
                                    (h, location)
                                })
                                .collect();
                            format!(", missing={missing:?}, context_detail={trigger_context:?}")
                        }
                    )
                } else if let ResumeFrame::OuterOptionalTrigger(s) = frame {
                    format!(
                        " (source={}, outer={}, card={}, source_handle={:?}, controller={})",
                        allowed.contains(&s.queued_effect.source_card),
                        outer_safe(&s.outer_conts),
                        s.queued_effect.card_id,
                        s.queued_effect.source_card,
                        s.queued_effect.controller
                    )
                } else {
                    String::new()
                };
                return Err(format!(
                    "closed-list continuation requires audit: {}{}",
                    format!("{frame:?}")
                        .split([' ', '(', '{'])
                        .next()
                        .unwrap_or("unknown"),
                    detail
                ));
            }
        }
    }
    Ok(())
}

pub fn materialize_closed(
    root: &Game,
    viewer: u8,
    memory: &KnowledgeLedger,
    prior: &WeightedDeckPrior,
    seed: u64,
) -> Result<Game, String> {
    materialize_closed_with_diagnostics(root, viewer, memory, prior, seed).map(|(world, _)| world)
}

fn rebind_hand_indices(world: &mut Game, viewer: u8) {
    use crate::{
        dsl_cards::bindings::Bindings,
        resume::{OuterContinuation, ResumeFrame, ResumeStack},
    };
    let positions: std::collections::HashMap<_, _> = world
        .players
        .iter()
        .flat_map(|p| {
            p.hand
                .iter()
                .enumerate()
                .map(move |(i, c)| (c.handle(), (p.id, i as u16)))
        })
        .collect();
    let bind = |b: &mut Bindings| b.rebind_belief_hand_indices(viewer, &positions);
    fn outer(conts: &mut [OuterContinuation], bind: &impl Fn(&mut Bindings)) {
        for c in conts {
            bind(&mut c.bindings);
        }
    }
    fn walk(resume: &mut ResumeStack, bind: &impl Fn(&mut Bindings)) {
        for frame in &mut resume.frames {
            macro_rules! bound {
                ($s:expr) => {{
                    bind(&mut $s.bindings);
                    outer(&mut $s.outer_conts, bind);
                }};
            }
            macro_rules! tails {
                ($s:expr) => {
                    outer(&mut $s.outer_conts, bind)
                };
            }
            match frame {
                ResumeFrame::RunTail {
                    bindings,
                    outer_conts,
                    ..
                } => {
                    bind(bindings);
                    outer(outer_conts, bind);
                }
                ResumeFrame::ReplacementBody { inner, outer_conts }
                | ResumeFrame::DelayBodyAfterCost {
                    inner, outer_conts, ..
                } => {
                    walk(inner, bind);
                    outer(outer_conts, bind);
                }
                ResumeFrame::SourceMultiStep(s) => {
                    bound!(s);
                    bind(&mut s.filter_bindings);
                }
                ResumeFrame::MultiPickStep(s) => bound!(s),
                ResumeFrame::RevealBucketStep(s) => bound!(s),
                ResumeFrame::ZoneCardPickStep(s) => bound!(s),
                ResumeFrame::CountCappedPermanentsStep(s) => bound!(s),
                ResumeFrame::PermutationStep(s) => bound!(s),
                ResumeFrame::UseOptionFromHandStep(s) => bound!(s),
                ResumeFrame::UseOptionFromTrashStep(s) => bound!(s),
                ResumeFrame::NonDslCountCappedStep(s) => tails!(s),
                ResumeFrame::RefireEffectChoice(s) => tails!(s),
                ResumeFrame::BlockSelection(s) => tails!(s),
                ResumeFrame::CounterSelection(s) => tails!(s),
                ResumeFrame::RaidSelection(s) => tails!(s),
                ResumeFrame::AllianceSelection(s) => tails!(s),
                ResumeFrame::KeywordAscensionChoice(s) => tails!(s),
                ResumeFrame::MayDnaPartnerSelection(s) => tails!(s),
                ResumeFrame::MayDnaResultSelection(s) => tails!(s),
                ResumeFrame::InteractiveOptionUseCostReductionPrompt(s) => tails!(s),
                ResumeFrame::DelayPlayFromHandSelection(s) => tails!(s),
                ResumeFrame::DelayFieldDnaSelection(s) => tails!(s),
                ResumeFrame::PlayOrUseDualChoice(s) => tails!(s),
                ResumeFrame::DigiXrosMaterialSelection(s) => tails!(s),
                ResumeFrame::TrashOptionFromStackSelection(s) => bound!(s),
                ResumeFrame::CounterBlastDnaFieldMaterial(s) => tails!(s),
                ResumeFrame::CounterBlastDnaHandMaterial(s) => tails!(s),
                ResumeFrame::ArtsDigivolveSelection(s) => tails!(s),
                ResumeFrame::PlayFromHandCostReductionPrompt(s) => tails!(s),
                ResumeFrame::AttackDeclarationDecision(s) => tails!(s),
                ResumeFrame::OuterOptionalTrigger(s) => tails!(s),
                ResumeFrame::TriggerOrderSelection(s) => tails!(s),
                ResumeFrame::OptionalReplacement(s) => tails!(s),
                ResumeFrame::EffectDigivolveCostChoice(s) => tails!(s),
                ResumeFrame::DigivolveReducerPrompt(s)
                | ResumeFrame::DigivolveReducerSuspend(s) => {
                    if let Some(e) = &mut s.effect {
                        outer(&mut e.outer_conts, bind);
                    }
                }
                _ => {} // Audit rejects other shapes before this pass.
            }
        }
    }
    if let Some(s) = &mut world.pending_selection_resume {
        walk(s, &bind);
    }
    if let Some((_, b, _)) = &mut world.dsl_outer_tail {
        bind(b);
    }
    for scope in &mut world.effect_digivolve_scopes {
        outer(&mut scope.declaration.outer_conts, &bind);
    }
    for h in &mut world.after_selection_resume_hooks.0 {
        if let crate::resume::AfterSelectionHook::PlayCostContinuation {
            player_id,
            hand_index,
            target,
            ..
        } = h
        {
            if *player_id != viewer {
                *hand_index = positions
                    .get(&target.card_handle())
                    .filter(|(p, _)| *p == *player_id)
                    .map(|(_, i)| *i as usize)
                    .unwrap_or(usize::MAX);
            }
        }
        if let crate::resume::AfterSelectionHook::DigivolveReducerChain { state, .. } = h {
            if let Some(e) = &mut state.effect {
                outer(&mut e.outer_conts, &bind);
            }
        }
    }
    if let Some(t) = &mut world.pending_digixros_transaction {
        for m in t
            .selected_materials
            .iter_mut()
            .chain(&mut t.pre_attached_materials)
        {
            if let crate::digixros::DigiXrosMaterialOrigin::Hand {
                player,
                index,
                card,
            } = &mut m.origin
            {
                if *player != viewer {
                    *index = positions
                        .get(card)
                        .filter(|(p, _)| *p == *player)
                        .map(|(_, i)| *i as usize)
                        .unwrap_or(usize::MAX);
                }
            }
        }
    }
}

/// The diagnostic reports the selected mixture component, never true hidden data.
pub fn materialize_closed_with_diagnostics(
    root: &Game,
    viewer: u8,
    memory: &KnowledgeLedger,
    prior: &WeightedDeckPrior,
    seed: u64,
) -> Result<(Game, bool), String> {
    let evidence = BeliefEvidence::extract(root, viewer, memory)?;
    let mut world = root.clone();
    audit(&mut world, viewer, memory)?;
    let hypothesis = prior.sample(&evidence, seed)?;
    let opponent = 1 - viewer;
    let mut rng = StdRng::seed_from_u64(seed);
    let inspected = root
        .pending_selection
        .as_ref()
        .and_then(|s| s.card_choice.as_ref())
        .and_then(|c| c.inspected_security_owner(viewer));

    let mut pool: Vec<String> = hypothesis
        .hidden_main
        .iter()
        .flat_map(|(id, n)| std::iter::repeat(id.clone()).take(*n))
        .collect();
    let mut eggs: Vec<String> = hypothesis
        .hidden_eggs
        .iter()
        .flat_map(|(id, n)| std::iter::repeat(id.clone()).take(*n))
        .collect();
    pool.shuffle(&mut rng);
    eggs.shuffle(&mut rng);
    let mut known = memory.hidden_instances(opponent);
    known.retain(|(instance, _, zone, _)| {
        *zone != KnowledgeZone::Security
            || (inspected != Some(opponent)
                && !root.players[opponent as usize]
                    .face_up_security
                    .contains(instance))
    });
    let mut next = world
        .players
        .iter()
        .flat_map(|p| {
            p.trash
                .iter()
                .chain(
                    p.battle_area
                        .iter()
                        .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
                )
                .chain(
                    p.breeding_area
                        .iter()
                        .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
                )
                .chain(p.commander_zone.iter())
                .chain(p.security.iter().filter(|c| {
                    (inspected == Some(p.id) || p.face_up_security.contains(&c.card_index))
                }))
                .chain(if p.id == viewer {
                    p.hand.iter()
                } else {
                    p.hand[0..0].iter()
                })
        })
        .chain(world.revealed_cards.iter())
        .chain(
            world
                .digixros_leaving_limbo
                .iter()
                .flat_map(|(_, _, p)| p.card_sources.iter().chain(&p.linked_cards)),
        )
        .chain(world.pending_option.iter().map(|p| &p.card))
        .chain(super::beliefs::physical_security_transits(&world))
        .map(|c| c.card_index)
        .chain(known.iter().map(|c| c.0))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("hypothetical instance overflow")?;
    if let Some(last_token) = memory.public_tokens().iter().next_back() {
        next = next.max(
            last_token
                .checked_add(1)
                .ok_or("public token instance overflow")?,
        );
    }
    // Reserve own hidden handles too; their composition is entitled, and no
    // foreign identity participates in the allocation bound.
    for c in world.players[viewer as usize]
        .deck
        .iter()
        .chain(&world.players[viewer as usize].security)
        .chain(&world.players[viewer as usize].digitama_deck)
    {
        next = next.max(c.card_index.checked_add(1).ok_or("instance overflow")?);
    }
    for c in memory.concealed_positions() {
        next = next.max(c.instance.checked_add(1).ok_or("concealed role overflow")?);
    }
    // Unobserved own deck/security identities are a joint multiset. Native
    // aliases must not bind an unrevealed moved card to its true identity.
    resample_own(
        &mut world,
        viewer,
        memory,
        inspected == Some(viewer),
        &mut rng,
        &mut next,
    )?;
    let mut opaque: Vec<_> = memory
        .concealed_positions()
        .iter()
        .filter(|c| c.owner == opponent && !known.iter().any(|k| k.0 == c.instance))
        .cloned()
        .collect();
    opaque.sort_by_key(|c| c.position.is_none());
    let mut make = |id: &str, instance: Option<u16>| -> Result<CardSource, String> {
        let data_index = *world
            .card_id_index
            .get(id)
            .ok_or_else(|| format!("hypothesis card not loaded: {id}"))?;
        let index = if let Some(index) = instance {
            index
        } else {
            let index = next;
            next = next
                .checked_add(1)
                .ok_or("hypothetical instance overflow")?;
            index
        };
        Ok(CardSource::new(data_index, opponent, index))
    };
    fn zone(
        count: usize,
        kind: KnowledgeZone,
        known: &[(u16, String, KnowledgeZone, Option<usize>)],
        opaque: &[super::beliefs::ConcealedPosition],
        pool: &mut Vec<String>,
        make: &mut dyn FnMut(&str, Option<u16>) -> Result<CardSource, String>,
        rng: &mut StdRng,
        fixed: Vec<(usize, CardSource)>,
    ) -> Result<Vec<CardSource>, String> {
        let mut slots = vec![None; count];
        for (i, c) in fixed {
            slots[i] = Some(c);
        }
        let mut unpositioned = Vec::new();
        for (instance, id, _, position) in known.iter().filter(|c| c.2 == kind) {
            let idx = pool
                .iter()
                .position(|c| c == id)
                .ok_or("known card absent from hypothesis")?;
            pool.swap_remove(idx);
            let c = make(id, Some(*instance))?;
            if let Some(from_top) = position {
                let slot = count
                    .checked_sub(from_top + 1)
                    .ok_or("known position exceeds zone")?;
                if slots[slot].is_some() {
                    return Err("conflicting known positions".into());
                }
                slots[slot] = Some(c);
            } else {
                unpositioned.push(c);
            }
        }
        // The physical destination is known, the card's identity is not.
        // Deal an arbitrary hypothesis card into each such role.
        for role in opaque.iter().filter(|c| c.zone == kind) {
            let slot = if let Some(p) = role.position {
                count
                    .checked_sub(p + 1)
                    .ok_or("concealed role exceeds zone")?
            } else {
                let empty: Vec<_> = (0..count).filter(|i| slots[*i].is_none()).collect();
                *empty.choose(rng).ok_or("concealed roles exceed zone")?
            };
            if slots[slot].is_some() {
                return Err("conflicting concealed role positions".into());
            }
            let id = pool.pop().ok_or("concealed role pool exhausted")?;
            slots[slot] = Some(make(&id, Some(role.instance))?);
        }
        let mut empty: Vec<_> = (0..count).filter(|i| slots[*i].is_none()).collect();
        empty.shuffle(rng);
        for c in unpositioned {
            let i = empty.pop().ok_or("known cards exceed zone")?;
            slots[i] = Some(c);
        }
        for i in empty {
            slots[i] = Some(make(&pool.pop().ok_or("hypothesis pool exhausted")?, None)?);
        }
        Ok(slots.into_iter().map(Option::unwrap).collect())
    }
    // Remove memberships for ALL zones before filling arbitrary cards, so a
    // hand deal cannot consume a card pinned to the deck or security.
    let mut reserved: BTreeMap<u16, String> = BTreeMap::new();
    for (instance, id, kind, _) in &known {
        let p = if *kind == KnowledgeZone::Digitama {
            &mut eggs
        } else {
            &mut pool
        };
        let i = p
            .iter()
            .position(|c| c == id)
            .ok_or("remembered card absent from sample")?;
        reserved.insert(*instance, p.swap_remove(i));
    }
    let with_reserved = |kind: KnowledgeZone, p: &mut Vec<String>| {
        for (instance, _, _, _) in known.iter().filter(|c| c.2 == kind) {
            p.push(reserved[instance].clone());
        }
    };
    with_reserved(KnowledgeZone::Hand, &mut pool);
    let hand = zone(
        evidence.hand_count,
        KnowledgeZone::Hand,
        &known,
        &opaque,
        &mut pool,
        &mut make,
        &mut rng,
        Vec::new(),
    )?;
    let public_pins = root.players[opponent as usize].face_up_security.clone();
    with_reserved(KnowledgeZone::Security, &mut pool);
    let fixed_security = root.players[opponent as usize]
        .security
        .iter()
        .enumerate()
        .filter(|(_, c)| inspected == Some(opponent) || public_pins.contains(&c.card_index))
        .map(|(i, c)| (i, c.clone()))
        .collect();
    let security = zone(
        root.players[opponent as usize].security.len(),
        KnowledgeZone::Security,
        &known,
        &opaque,
        &mut pool,
        &mut make,
        &mut rng,
        fixed_security,
    )?;
    with_reserved(KnowledgeZone::Deck, &mut pool);
    let deck = zone(
        evidence.deck_count,
        KnowledgeZone::Deck,
        &known,
        &opaque,
        &mut pool,
        &mut make,
        &mut rng,
        Vec::new(),
    )?;
    with_reserved(KnowledgeZone::Digitama, &mut eggs);
    let digitama = zone(
        evidence.digitama_count,
        KnowledgeZone::Digitama,
        &known,
        &opaque,
        &mut eggs,
        &mut make,
        &mut rng,
        Vec::new(),
    )?;
    // Hidden field locations (including physical handles) are public. Allocate
    // only their identities from the hypothesis, never copy their real data.
    for c in world
        .players
        .iter_mut()
        .flat_map(|p| p.battle_area.iter_mut().chain(p.breeding_area.iter_mut()))
        .flat_map(|p| p.card_sources.iter_mut().chain(&mut p.linked_cards))
        .chain(
            world
                .digixros_leaving_limbo
                .iter_mut()
                .flat_map(|(_, _, p)| p.card_sources.iter_mut().chain(&mut p.linked_cards)),
        )
        .filter(|c| c.owner == opponent && c.face_down && !c.is_token)
    {
        let id = if let Some((_, id, _, _)) = known
            .iter()
            .find(|k| k.2 == KnowledgeZone::Field && k.0 == c.card_index)
        {
            // Kept out of the arbitrary pool from the initial reservation
            // pass, so an earlier unknown field slot cannot consume this ID.
            reserved
                .get(&c.card_index)
                .filter(|v| *v == id)
                .ok_or("known field source absent from hypothesis")?
                .clone()
        } else {
            pool.pop().ok_or("hypothesis field pool exhausted")?
        };
        let mut sampled = make(&id, Some(c.card_index))?;
        sampled.face_down = true;
        *c = sampled;
    }
    if !pool.is_empty() || !eggs.is_empty() {
        return Err("hypothesis cards left undealt".into());
    }
    let p = &mut world.players[opponent as usize];
    p.hand = hand;
    p.deck = deck;
    p.security = security;
    p.digitama_deck = digitama;
    let mut original = BTreeMap::new();
    for id in hypothesis.deck {
        *original.entry(id).or_insert(0u16) += 1;
    }
    p.original_deck = original
        .into_iter()
        .map(|(card_id, count)| OriginalDeckCardCount {
            is_digitama: world.card_data[*world.card_id_index.get(&card_id).unwrap()].card_kind
                == crate::enums::CardKind::DigiEgg,
            card_id,
            count,
        })
        .collect();
    if let Some(mut selection) = world.pending_selection.take() {
        if let Some(choice) = &mut selection.card_choice {
            choice.rebind_sampled_security(root, &world);
        }
        world.pending_selection = Some(selection);
    }
    rebind_hand_indices(&mut world, viewer);
    world.reset_hypothetical_indices(next);
    world.rng = StdRng::seed_from_u64(seed ^ 0xB311_EF00);
    world.reveal_source = None;
    world.events.clear();
    world.belief_knowledge = None;
    world.belief_knowledge_error = None;
    world.effects_cache = Default::default();
    *world.movement_observer.borrow_mut() = None;
    if crate::build_action_mask(root, viewer) != crate::build_action_mask(&world, viewer) {
        return Err(
            "hypothesis changes viewer root action mask; unsupported hidden dependency".into(),
        );
    }
    let actual = BeliefEvidence::extract(&world, viewer, memory)?;
    if actual != evidence {
        return Err("hypothesis changed entitled public state".into());
    }
    check_hypothesis(&world)?;
    Ok((world, hypothesis.used_fallback))
}

/// Conservation is against the sampled original list, never the true list.
pub fn check_hypothesis(world: &Game) -> Result<(), String> {
    let mut instances = BTreeMap::new();
    for c in world
        .players
        .iter()
        .flat_map(|p| {
            p.hand
                .iter()
                .chain(&p.deck)
                .chain(&p.security)
                .chain(&p.digitama_deck)
                .chain(&p.trash)
                .chain(
                    p.battle_area
                        .iter()
                        .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
                )
                .chain(
                    p.breeding_area
                        .iter()
                        .flat_map(|p| p.card_sources.iter().chain(&p.linked_cards)),
                )
                .chain(p.commander_zone.iter())
        })
        .chain(&world.revealed_cards)
        .chain(
            world
                .digixros_leaving_limbo
                .iter()
                .flat_map(|(_, _, p)| p.card_sources.iter().chain(&p.linked_cards)),
        )
    {
        if c.is_token {
            continue;
        }
        if c.is_opaque_placeholder {
            return Err("opaque hypothetical card".into());
        }
        if instances
            .insert(
                c.card_index,
                (c.owner, c.card_id(&world.card_data).to_string()),
            )
            .is_some()
        {
            return Err("duplicate physical instance in hypothetical zones".into());
        }
    }
    for c in world
        .pending_option
        .iter()
        .map(|p| &p.card)
        .chain(super::beliefs::physical_security_transits(world))
    {
        if !c.is_token {
            let expected = (c.owner, c.card_id(&world.card_data).to_string());
            if let Some(previous) = instances.insert(c.card_index, expected.clone()) {
                if previous != expected {
                    return Err("public pending snapshot identity mismatch".into());
                }
            }
        }
    }
    for owner in 0..2 {
        let mut actual = BTreeMap::new();
        for (p, id) in instances.values().filter(|(p, _)| *p == owner) {
            let _ = p;
            *actual.entry(id.clone()).or_insert(0u16) += 1;
        }
        let expected: BTreeMap<_, _> = world.players[owner as usize]
            .original_deck
            .iter()
            .map(|r| (r.card_id.clone(), r.count))
            .collect();
        if actual != expected {
            return Err(format!(
                "hypothesis conservation mismatch for player {owner}"
            ));
        }
    }
    Ok(())
}

fn resample_own(
    world: &mut Game,
    viewer: u8,
    memory: &KnowledgeLedger,
    inspected: bool,
    rng: &mut StdRng,
    next: &mut u16,
) -> Result<(), String> {
    let mut known = memory.hidden_instances(viewer);
    // Reserve exact positions before placing membership-only observations.
    // Otherwise a randomly placed known card can occupy a later pinned slot.
    known.sort_by_key(|(_, _, _, position)| position.is_none());
    let p = &mut world.players[viewer as usize];
    let mut pool = std::mem::take(&mut p.deck);
    let deck_count = pool.len();
    let mut security: Vec<Option<CardSource>> = std::mem::take(&mut p.security)
        .into_iter()
        .map(|c| {
            if inspected || p.face_up_security.contains(&c.card_index) {
                Some(c)
            } else {
                pool.push(c);
                None
            }
        })
        .collect();
    let mut deck = vec![None; deck_count];
    // Canonicalize before shuffling; the true assignment/order is irrelevant.
    pool.sort_by_key(|c| (c.card_id(&world.card_data).to_string(), c.card_index));
    pool.shuffle(rng);
    let identified: HashSet<_> = known.iter().map(|c| c.0).collect();
    let reserved_roles: Vec<_> = memory
        .concealed_positions()
        .iter()
        .filter(|c| c.owner == viewer && !identified.contains(&c.instance) && c.position.is_some())
        .collect();
    for (instance, _, zone, position) in known {
        let slots = match zone {
            KnowledgeZone::Deck => &mut deck,
            KnowledgeZone::Security => &mut security,
            _ => continue,
        };
        if slots.iter().flatten().any(|c| c.card_index == instance) {
            continue;
        }
        let i = pool
            .iter()
            .position(|c| c.card_index == instance)
            .ok_or("own knowledge refers to a departed hidden card")?;
        let card = pool.remove(i);
        let index = if let Some(from_top) = position {
            slots
                .len()
                .checked_sub(from_top + 1)
                .ok_or("own known position exceeds zone")?
        } else {
            let empty: Vec<_> = slots
                .iter()
                .enumerate()
                .filter(|(i, c)| {
                    c.is_none()
                        && !reserved_roles
                            .iter()
                            .any(|r| r.zone == zone && r.position == Some(slots.len() - 1 - *i))
                })
                .map(|(i, _)| i)
                .collect();
            *empty
                .choose(rng)
                .ok_or("own known memberships exceed zone")?
        };
        if slots[index].is_some() {
            return Err("conflicting own inspection positions".into());
        }
        slots[index] = Some(card);
    }
    let known_instances: HashSet<_> = memory
        .hidden_instances(viewer)
        .iter()
        .map(|c| c.0)
        .chain(deck.iter().chain(&security).flatten().map(|c| c.card_index))
        .collect();
    let mut roles: Vec<_> = memory
        .concealed_positions()
        .iter()
        .filter(|c| {
            c.owner == viewer
                && matches!(c.zone, KnowledgeZone::Deck | KnowledgeZone::Security)
                && !known_instances.contains(&c.instance)
        })
        .collect();
    roles.sort_by_key(|c| c.position.is_none());
    for role in roles {
        let slots = match role.zone {
            KnowledgeZone::Deck => &mut deck,
            KnowledgeZone::Security => &mut security,
            _ => return Err("own concealed role zone is unsupported".into()),
        };
        let slot = if let Some(p) = role.position {
            slots
                .len()
                .checked_sub(p + 1)
                .ok_or("own concealed role exceeds zone")?
        } else {
            let empty: Vec<_> = (0..slots.len()).filter(|i| slots[*i].is_none()).collect();
            *empty.choose(rng).ok_or("own concealed roles exceed zone")?
        };
        if slots[slot].is_some() {
            return Err(format!("conflicting own concealed role positions: viewer={viewer}, role={role:?}, occupied={:?}, known={:?}, anonymous={:?}",
                slots[slot].as_ref().map(|c| c.card_index), memory.hidden_instances(viewer), memory.concealed_positions()));
        }
        let mut card = pool.pop().ok_or("own concealed role pool exhausted")?;
        card.card_index = role.instance;
        slots[slot] = Some(card);
    }
    for slot in deck.iter_mut().chain(&mut security) {
        if slot.is_none() {
            let mut card = pool.pop().ok_or("own unseen pool exhausted")?;
            card.card_index = *next;
            *next = next
                .checked_add(1)
                .ok_or("own hypothetical instance overflow")?;
            *slot = Some(card);
        }
    }
    if !pool.is_empty() {
        return Err("own unseen pool has excess cards".into());
    }
    p.deck = deck.into_iter().map(Option::unwrap).collect();
    p.security = security.into_iter().map(Option::unwrap).collect();
    p.digitama_deck
        .sort_by_key(|c| (c.data_index, c.card_index));
    p.digitama_deck.shuffle(rng);
    Ok(())
}
