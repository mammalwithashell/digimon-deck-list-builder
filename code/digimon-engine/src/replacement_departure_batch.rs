// Included in replacement.rs. Reuses the event-member replacement machinery.
#[derive(Debug, Clone)]
struct DepartureWindow {
    receipt: u64,
    position: Option<crate::enums::StackPosition>,
    include_sources: bool,
    route_stage: bool,
    ordered: Vec<usize>,
    pending_order: Vec<(u16, usize)>,
}
pub(crate) fn start_departure_batch(
    game: &mut crate::game::Game,
    handles: Vec<PermanentHandle>,
    receipt: u64,
    position: Option<crate::enums::StackPosition>,
    include_sources: bool,
) {
    let members: Vec<_> = handles
        .iter()
        .filter_map(|h| {
            capture_subject_card(game, ReplacementSubject::Permanent(*h)).map(|identity| {
                DeletionWindowMember {
                    original: *h,
                    identity,
                    prevented: false,
                    terminal: false,
                    substituted: false,
                }
            })
        })
        .collect();
    let Some(first) = members.first().cloned() else {
        return;
    };
    let cause = game.infer_effect_cause(first.original.player);
    let initiating_source_permanent = game
        .effect_source_permanent
        .and_then(|h| capture_subject_card(game, ReplacementSubject::Permanent(h)))
        .map(|(p, _, id)| (p, id));
    let mut window = ReplacementWindow {
        timing: EffectTiming::WhenWouldLeaveBattleArea,
        subject: ReplacementSubject::Permanent(first.original),
        stable_subject: Some(first.identity),
        cause,
        destination: Some(if position.is_some() {
            Zone::Deck
        } else {
            Zone::Hand
        }),
        cause_player: game.effect_source_player,
        initiating_source_card: game.effect_source_card,
        effect_digivolve_card: None,
        play_declaration: None,
        event_cause: game.current_deletion_event_cause_override,
        trigger_context: game.current_trigger_context.clone(),
        deletion_receipt: game.current_deletion_receipt,
        turn_player: game.turn_player(),
        remaining: Vec::new(),
        cancelled: false,
        deletion_batch: Some(DeletionWindowBatch {
            members,
            pending_subject: None,
            substitution_depth: 0,
            initiating_source_permanent,
            active_paid_all: None,
        }),
        departure: Some(DepartureWindow {
            receipt,
            position,
            include_sources,
            route_stage: false,
            ordered: Vec::new(),
            pending_order: Vec::new(),
        }),
    };
    capture_departure_candidates(game, &mut window);
    advance_deletion_window(game, window, None, false);
}
fn capture_departure_candidates(game: &mut crate::game::Game, window: &mut ReplacementWindow) {
    window.remaining.clear();
    for i in 0..window.deletion_batch.as_ref().unwrap().members.len() {
        for c in deletion_member_candidates(game, window, i, false) {
            if !window
                .remaining
                .iter()
                .any(|old| window_candidate_matches(game, old, &c))
            {
                window.remaining.push(capture_window_candidate(game, c));
            }
        }
    }
}
fn finish_departure_window(game: &mut crate::game::Game, mut window: ReplacementWindow) {
    if !window.departure.as_ref().unwrap().route_stage {
        window.departure.as_mut().unwrap().route_stage = true;
        window.timing = if window.destination == Some(Zone::Hand) {
            EffectTiming::WhenWouldBeReturnedToHand
        } else {
            EffectTiming::WhenWouldBeReturnedToDeck
        };
        capture_departure_candidates(game, &mut window);
        advance_deletion_window(game, window, None, false);
        return;
    }
    let b = window.deletion_batch.as_ref().unwrap();
    let d = window.departure.as_ref().unwrap();
    let remaining: Vec<_> = b
        .members
        .iter()
        .enumerate()
        .filter(|(i, m)| !m.prevented && !m.terminal && !d.ordered.contains(i))
        .filter_map(|(i, m)| {
            deletion_member_handle(game, m).map(|h| {
                (
                    crate::action::space::encode_attack(h.player as u16, h.index as u16),
                    i,
                )
            })
        })
        .collect();
    if d.position.is_some() && remaining.len() > 1 {
        let actor = window.cause_player.unwrap_or(window.turn_player);
        let source_card = window
            .initiating_source_card
            .unwrap_or(b.members[0].identity.1);
        let prompt = if d.position == Some(crate::enums::StackPosition::Bottom) {
            "Choose the bottommost remaining card"
        } else {
            "Choose the topmost remaining card"
        }
        .into();
        let state = OptionalReplacementState {
            window: None,
            order: None,
            source_identity: None,
            deletion_receipt: window.deletion_receipt,
            timing: window.timing,
            candidate_kind: CandidateKind::PassiveCancel,
            source_card,
            source_permanent: None,
            controller: actor,
            subject: window.subject,
            stable_subject_card: window.stable_subject,
            cause: window.cause,
            event_cause_override: window.event_cause,
            original_destination: window.destination,
            outer_conts: Vec::new(),
        };
        let previous_phase = game.current_phase;
        game.current_phase = GamePhase::EffectChoice;
        game.replacement_pending_outcome = None;
        game.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::AnyField,
            selecting_player: actor,
            previous_phase,
            valid_action_ids: remaining.iter().map(|(a, _)| *a).collect(),
            is_optional: false,
            prompt,
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: crate::enums::EffectSourceKind::Rule,
            callback: Box::new(|_, _| panic!("departure ordering requires its typed continuation")),
            on_decline: None,
        });
        game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::OptionalReplacement(state)],
        });
        window.departure.as_mut().unwrap().pending_order = remaining;
        if let Some(crate::resume::ResumeFrame::OptionalReplacement(state)) = game
            .pending_selection_resume
            .as_mut()
            .and_then(|s| s.frames.last_mut())
        {
            state.window = Some(window);
        }
        return;
    }
    window
        .departure
        .as_mut()
        .unwrap()
        .ordered
        .extend(remaining.into_iter().map(|(_, i)| i));
    let d = window.departure.as_ref().unwrap();
    let b = window.deletion_batch.as_ref().unwrap();
    let handles = d
        .ordered
        .iter()
        .filter_map(|i| deletion_member_handle(game, &b.members[*i]))
        .collect();
    let old_player = game.effect_source_player;
    game.effect_source_player = window.cause_player;
    let old_card = game.effect_source_card;
    game.effect_source_card = window.initiating_source_card;
    let old_source = game.effect_source_permanent;
    game.effect_source_permanent = b
        .initiating_source_permanent
        .and_then(|(p, id)| game.resolve_digivolve_host_identity(p, id));
    let old_trigger = game.current_trigger_context.clone();
    game.current_trigger_context = window.trigger_context.clone();
    game.commit_simultaneous_returns(handles, d.position, d.include_sources, d.receipt);
    game.effect_source_player = old_player;
    game.effect_source_card = old_card;
    game.effect_source_permanent = old_source;
    game.current_trigger_context = old_trigger;
}
fn resume_departure_order(
    game: &mut crate::game::Game,
    mut window: ReplacementWindow,
    action: u16,
) {
    let d = window.departure.as_mut().unwrap();
    let Some((_, index)) = d.pending_order.iter().find(|(a, _)| *a == action).copied() else {
        return;
    };
    d.pending_order.clear();
    d.ordered.push(index);
    finish_departure_window(game, window);
}
