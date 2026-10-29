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
fn audit(root: &Game, viewer: u8, memory: &KnowledgeLedger) -> Result<(), String> {
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
            .chain(root.pending_security.iter().map(|p| &p.card))
            .map(|c| c.handle()),
    );
    allowed.extend(
        memory
            .hidden_instances(1 - viewer)
            .iter()
            .map(|c| CardHandle(c.0)),
    );
    let outer_safe = |conts: &[crate::resume::OuterContinuation]| {
        conts
            .iter()
            .all(|c| allowed.contains(&c.source_card) && c.bindings.belief_safe(viewer, &allowed))
    };
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
    if root.effect_queue.iter().any(|effect| {
        !allowed.contains(&effect.source_card)
            || effect
                .attribution_source_card
                .is_some_and(|h| !allowed.contains(&h))
    }) {
        return Err("closed-list effect queue refers to concealed cards".into());
    }
    if root
        .pending_would_play_resume
        .as_ref()
        .is_some_and(|p| p.player != viewer)
    {
        return Err("closed-list foreign pending play requires declaration rebinding".into());
    }
    if !root.after_selection_resume_hooks.0.is_empty() {
        return Err("closed-list continuation audit: after-selection hooks unsupported".into());
    }
    if let Some(stack) = &root.pending_selection_resume {
        use crate::resume::ResumeFrame;
        for frame in &stack.frames {
            let safe = match frame {
                ResumeFrame::RunTail {
                    prov,
                    bindings,
                    outer_conts,
                    select_kind,
                    ..
                } => {
                    allowed.contains(&prov.source_card)
                        && bindings.belief_safe(viewer, &allowed)
                        && outer_safe(outer_conts)
                        && !matches!(select_kind,
                        crate::resume::ResumeSelectKind::Hand {of_player} if *of_player != viewer)
                }
                ResumeFrame::BlockSelection(s) => outer_safe(&s.outer_conts),
                ResumeFrame::CounterSelection(s) => {
                    s.defender_player == viewer && outer_safe(&s.outer_conts)
                }
                ResumeFrame::RaidSelection(s) => outer_safe(&s.outer_conts),
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
                    allowed.contains(&s.queued_effect.source_card) && outer_safe(&s.outer_conts)
                }
                ResumeFrame::TriggerOrderSelection(s) => outer_safe(&s.outer_conts),
                _ => false,
            };
            if !safe {
                return Err(format!(
                    "closed-list continuation requires audit: {}",
                    format!("{frame:?}")
                        .split([' ', '(', '{'])
                        .next()
                        .unwrap_or("unknown")
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

/// The diagnostic reports the selected mixture component, never true hidden data.
pub fn materialize_closed_with_diagnostics(
    root: &Game,
    viewer: u8,
    memory: &KnowledgeLedger,
    prior: &WeightedDeckPrior,
    seed: u64,
) -> Result<(Game, bool), String> {
    let evidence = BeliefEvidence::extract(root, viewer, memory)?;
    audit(root, viewer, memory)?;
    let hypothesis = prior.sample(&evidence, seed)?;
    let mut world = root.clone();
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
        .chain(world.pending_option.iter().map(|p| &p.card))
        .chain(world.pending_security.iter().map(|p| &p.card))
        .map(|c| c.card_index)
        .chain(known.iter().map(|c| c.0))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("hypothetical instance overflow")?;
    // Preserve own instances but sample only their entitled joint composition.
    resample_own(
        &mut world,
        viewer,
        memory,
        inspected == Some(viewer),
        &mut rng,
    )?;
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
    let mut with_reserved = |kind: KnowledgeZone, p: &mut Vec<String>| {
        for (instance, _, _, _) in known.iter().filter(|c| c.2 == kind) {
            p.push(reserved[instance].clone());
        }
    };
    with_reserved(KnowledgeZone::Hand, &mut pool);
    let hand = zone(
        evidence.hand_count,
        KnowledgeZone::Hand,
        &known,
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
        &mut eggs,
        &mut make,
        &mut rng,
        Vec::new(),
    )?;
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
        .chain(world.pending_security.iter().map(|p| &p.card))
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
) -> Result<(), String> {
    let known = memory.hidden_instances(viewer);
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
                .filter(|(_, c)| c.is_none())
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
    for slot in deck.iter_mut().chain(&mut security) {
        if slot.is_none() {
            *slot = Some(pool.pop().ok_or("own unseen pool exhausted")?);
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
