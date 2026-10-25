//! Effect-initiated App Fuse entry point (`initiate_effect_app_fuse`).
//!
//! Mirrors DCGO `BT23_079.cs` / `SelectAppFusionEffect`: a card effect lets one
//! of the controller's field Digimon "app fuse into" an App-Fusion-capable
//! Digimon card in hand/trash. Two RL-visible engine-driven selections:
//!   1. pick the field permanent to fuse onto (eligible = some result card's
//!      App-Fusion materials are present on it),
//!   2. pick the result card (eligible for the chosen permanent),
//! then commit via `Game::commit_effect_app_fuse` (stack on top, consume links).
//!
//! Eligibility reuses `Game::can_app_fuse_onto` (the alt-play app-fusion route);
//! the chained-selection install mirrors `link_cards`'s `to: own_digimon`
//! host-select flow (`dsl_cards/step/link_cards.rs`).

use digimon_dsl::compiled::{CompiledAppFuseZone, CompiledPredicate};

use crate::action::space::{encode_attack, HAND_EFFECT_START, TRASH_EFFECT_START};
use crate::card_source::{CardHandle, CardSource};
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::{EffectSourceKind, GamePhase, PlayerId};
use crate::game_actions::AppFuseSourceZone;
use crate::permanent::{OptionState, PermanentHandle};
use crate::selection::{PendingSelection, SelectionKind};

/// A result belongs to the initiating clause's lexical binding scope.
#[derive(Debug, Clone)]
pub(crate) struct AppFuseReceipt {
    pub(crate) name: String,
    pub(crate) bindings: crate::dsl_cards::bindings::Bindings,
}

#[derive(Debug, Clone)]
pub(crate) struct AppFuseHostSelectionState {
    pub(crate) controller: PlayerId,
    pub(crate) per_perm: Vec<(u16, PermanentHandle, u64, Vec<(u16, CardHandle)>)>,
    pub(crate) from_zone: CompiledAppFuseZone,
    pub(crate) optional: bool,
    pub(crate) selecting_player: PlayerId,
    pub(crate) source_card: CardHandle,
    pub(crate) source_permanent: Option<PermanentHandle>,
    pub(crate) source_kind: EffectSourceKind,
    pub(crate) receipt: Option<AppFuseReceipt>,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

#[derive(Debug, Clone)]
pub(crate) struct AppFuseResultSelectionState {
    pub(crate) controller: PlayerId,
    pub(crate) host: PermanentHandle,
    pub(crate) host_identity: u64,
    pub(crate) results: Vec<(u16, CardHandle)>,
    pub(crate) zone: CompiledAppFuseZone,
    pub(crate) receipt: Option<AppFuseReceipt>,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

impl EffectContext<'_> {
    /// Effect-initiated App Fuse. Installs selection #1 over the controller's
    /// field Digimon that some eligible result card can app-fuse onto; on pick,
    /// installs selection #2 over the result cards eligible for that permanent;
    /// on pick, commits via `Game::commit_effect_app_fuse`. `optional` makes
    /// PASS legal at each step ("may"). No eligible permanent ⇒ silent no-op
    /// (matches DCGO's `HasMatchConditionOwnersPermanent` guard).
    pub fn initiate_effect_app_fuse(
        &mut self,
        from_zone: CompiledAppFuseZone,
        result_filter: Option<&CompiledPredicate>,
        optional: bool,
    ) {
        self.initiate_effect_app_fuse_with_receipt(from_zone, result_filter, optional, None);
    }

    /// Receipt-aware DSL entry point. Only the authored binding opts into
    /// running the enclosing tail after a declined selection.
    pub(crate) fn initiate_effect_app_fuse_with_receipt(
        &mut self,
        from_zone: CompiledAppFuseZone,
        result_filter: Option<&CompiledPredicate>,
        optional: bool,
        receipt: Option<AppFuseReceipt>,
    ) {
        let controller = self.player;
        let source_card = self.source_card;
        let source_kind = self.source_kind;
        let source_permanent = self.source_permanent;
        let selecting_player = self.override_selecting_player().unwrap_or(controller);

        // 1. Result cards in the source zone passing the optional filter.
        //    Clone each CardSource so the per-permanent route check below does
        //    not hold a borrow of the zone vector.
        let result_cards: Vec<(u16, CardHandle, CardSource)> = {
            let read_ctx = EffectReadContext::new_with_source_kind(
                self.game,
                source_card,
                source_permanent,
                source_kind,
                controller,
            );
            let (zone_cards, base_action): (&Vec<CardSource>, u16) = match from_zone {
                CompiledAppFuseZone::Hand => {
                    (&self.game.player(controller).hand, HAND_EFFECT_START)
                }
                CompiledAppFuseZone::Trash => {
                    (&self.game.player(controller).trash, TRASH_EFFECT_START)
                }
            };
            zone_cards
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    let handle = c.handle();
                    let passes = match result_filter {
                        Some(p) => eval_predicate(p, &read_ctx, PredicateSubject::Card(handle)),
                        None => true,
                    };
                    passes.then(|| (base_action + i as u16, handle, c.clone()))
                })
                .collect()
        };

        if result_cards.is_empty() {
            return;
        }

        // 2. For each own standing-Digimon permanent, which result cards fuse?
        let n = self.game.player(controller).battle_area.len();
        let mut per_perm: Vec<(u16, PermanentHandle, u64, Vec<(u16, CardHandle)>)> = Vec::new();
        for idx in 0..n {
            let host = PermanentHandle {
                player: controller,
                index: idx as u8,
            };
            if !self.game.permanent_is_digimon_for_rules(host) {
                continue;
            }
            if !matches!(
                self.game.player(controller).battle_area[idx].option_state,
                OptionState::Standard
            ) {
                continue;
            }
            let fusable: Vec<(u16, CardHandle)> = result_cards
                .iter()
                .filter(|(_, _, cs)| self.game.can_app_fuse_onto(cs, host))
                .map(|(aid, h, _)| (*aid, *h))
                .collect();
            if !fusable.is_empty() {
                let identity = self
                    .game
                    .capture_source_play_host_identity(host)
                    .expect("enumerated battle-area host exists");
                per_perm.push((
                    encode_attack(controller as u16, idx as u16),
                    host,
                    identity,
                    fusable,
                ));
            }
        }

        if per_perm.is_empty() {
            return;
        }

        // 3. Install selection #1 over the eligible permanents.
        let valid_action_ids: Vec<u16> = per_perm.iter().map(|(a, _, _, _)| *a).collect();
        let previous_phase = self.game.current_phase;
        let state = AppFuseHostSelectionState {
            controller,
            per_perm,
            from_zone,
            optional,
            selecting_player,
            source_card,
            source_permanent,
            source_kind,
            receipt,
            outer_conts: Vec::new(),
        };
        let callback_state = state.clone();
        self.game.current_phase = GamePhase::SelectTarget;
        self.game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Target,
            selecting_player,
            previous_phase,
            valid_action_ids,
            is_optional: optional,
            prompt: "Select 1 of your Digimon to app fuse".to_string(),
            effect_choices: None,
            source_card,
            source_permanent,
            source_kind,
            card_choice: None,
            zone_owner: None,
            callback: Box::new(move |game, action_id| {
                run_app_fuse_host_selection_step(game, callback_state, action_id);
            }),
            on_decline: state.receipt.clone().map(|receipt| {
                Box::new(move |game: &mut crate::game::Game| {
                    publish_app_fuse_receipt(game, Some(&receipt), None);
                }) as _
            }),
        });
        self.game.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::AppFuseHostSelection(state)],
        });
    }
}

pub(crate) fn run_app_fuse_host_selection_step(
    game: &mut crate::game::Game,
    state: AppFuseHostSelectionState,
    action_id: u16,
) {
    let picked = state
        .per_perm
        .iter()
        .find(|(a, _, _, _)| *a == action_id)
        .cloned()
        .and_then(|(_, host, identity, results)| {
            game.resolve_source_play_host_identity(host, identity)
                .map(|host| (host, identity, results))
        });
    let Some((host, identity, results)) = picked else {
        publish_app_fuse_receipt(game, state.receipt.as_ref(), None);
        return;
    };
    install_result_selection(
        game,
        state.controller,
        host,
        identity,
        results,
        state.from_zone,
        state.optional,
        state.selecting_player,
        state.source_card,
        state.source_permanent,
        state.source_kind,
        state.receipt,
    );
}

pub(crate) fn run_app_fuse_result_selection_step(
    game: &mut crate::game::Game,
    state: AppFuseResultSelectionState,
    action_id: u16,
) {
    let result = (|| {
        let (_, card) = state
            .results
            .iter()
            .find(|(a, _)| *a == action_id)
            .copied()?;
        // Field indices are presentation coordinates, not stable identities.
        let host = game.resolve_source_play_host_identity(state.host, state.host_identity)?;
        let cards = match state.zone {
            CompiledAppFuseZone::Hand => &game.player(state.controller).hand,
            CompiledAppFuseZone::Trash => &game.player(state.controller).trash,
        };
        let result = cards.iter().find(|c| c.handle() == card)?;
        if !game.can_app_fuse_onto(result, host) {
            return None;
        }
        game.commit_effect_app_fuse(
            state.controller,
            host,
            card,
            app_fuse_source_zone(state.zone),
        )
        .then_some(card)
    })();
    publish_app_fuse_receipt(game, state.receipt.as_ref(), result);
}

/// Install selection #2: the result card (in `zone`) to app-fuse onto `host`.
/// On resolution, commit the app fuse.
#[allow(clippy::too_many_arguments)]
fn install_result_selection(
    game: &mut crate::game::Game,
    controller: crate::enums::PlayerId,
    host: PermanentHandle,
    host_identity: u64,
    results: Vec<(u16, CardHandle)>,
    zone: CompiledAppFuseZone,
    optional: bool,
    selecting_player: crate::enums::PlayerId,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
    receipt: Option<AppFuseReceipt>,
) {
    if results.is_empty() {
        return;
    }
    let valid_action_ids: Vec<u16> = results.iter().map(|(a, _)| *a).collect();
    let previous_phase = game.current_phase;
    let state = AppFuseResultSelectionState {
        controller,
        host,
        host_identity,
        results,
        zone,
        receipt,
        outer_conts: Vec::new(),
    };
    let callback_state = state.clone();
    game.current_phase = GamePhase::SelectTarget;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Target,
        selecting_player,
        previous_phase,
        valid_action_ids,
        is_optional: optional,
        prompt: "Select a Digimon card to app fuse into".to_string(),
        effect_choices: None,
        source_card,
        source_permanent,
        source_kind,
        card_choice: None,
        zone_owner: None,
        callback: Box::new(move |game, action_id| {
            run_app_fuse_result_selection_step(game, callback_state, action_id);
        }),
        on_decline: state.receipt.clone().map(|receipt| {
            Box::new(move |game: &mut crate::game::Game| {
                publish_app_fuse_receipt(game, Some(&receipt), None);
            }) as _
        }),
    });
    game.pending_selection_resume = Some(crate::resume::ResumeStack {
        frames: vec![crate::resume::ResumeFrame::AppFuseResultSelection(state)],
    });
}

/// Terminal publication replaces the channel with this receipt's own scope.
/// Synchronous no-pair clears never enter the channel; enclosing tails receive
/// those directly from the interpreter's returned lexical bindings.
pub(crate) fn publish_app_fuse_receipt(
    game: &mut crate::game::Game,
    receipt: Option<&AppFuseReceipt>,
    result: Option<CardHandle>,
) {
    let Some(receipt) = receipt else {
        return;
    };
    let mut bindings = receipt.bindings.clone();
    if let Some(card) = result {
        bindings.insert_card(&receipt.name, card);
    } else {
        bindings.clear_for_merge(&receipt.name);
    }
    game.dsl_resolved_tail_bindings = Some(bindings);
}

fn app_fuse_source_zone(zone: CompiledAppFuseZone) -> AppFuseSourceZone {
    match zone {
        CompiledAppFuseZone::Hand => AppFuseSourceZone::Hand,
        CompiledAppFuseZone::Trash => AppFuseSourceZone::Trash,
    }
}
