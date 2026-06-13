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
//! the chained-selection install mirrors `link_card_to_self`'s
//! `to: chosen_own_digimon` flow (`dsl_cards/step/link_card.rs`).

use digimon_dsl::compiled::{CompiledAppFuseZone, CompiledPredicate};

use crate::action::space::{encode_attack, HAND_EFFECT_START, TRASH_EFFECT_START};
use crate::card_source::{CardHandle, CardSource};
use crate::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use crate::effect_context::{EffectContext, EffectReadContext};
use crate::enums::GamePhase;
use crate::game_actions::AppFuseSourceZone;
use crate::permanent::{OptionState, PermanentHandle};
use crate::selection::{PendingSelection, SelectionKind};

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
        let mut per_perm: Vec<(u16, PermanentHandle, Vec<(u16, CardHandle)>)> = Vec::new();
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
                per_perm.push((encode_attack(controller as u16, idx as u16), host, fusable));
            }
        }

        if per_perm.is_empty() {
            return;
        }

        // 3. Install selection #1 over the eligible permanents.
        let valid_action_ids: Vec<u16> = per_perm.iter().map(|(a, _, _)| *a).collect();
        let previous_phase = self.game.current_phase;
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
            callback: Box::new(move |game, action_id| {
                let Some((_, host, results)) = per_perm
                    .iter()
                    .find(|(a, _, _)| *a == action_id)
                    .cloned()
                else {
                    return;
                };
                install_result_selection(
                    game,
                    controller,
                    host,
                    results,
                    from_zone,
                    optional,
                    selecting_player,
                    source_card,
                    source_permanent,
                    source_kind,
                );
            }),
            on_decline: None,
        });
    }
}

/// Install selection #2: the result card (in `zone`) to app-fuse onto `host`.
/// On resolution, commit the app fuse.
#[allow(clippy::too_many_arguments)]
fn install_result_selection(
    game: &mut crate::game::Game,
    controller: crate::enums::PlayerId,
    host: PermanentHandle,
    results: Vec<(u16, CardHandle)>,
    zone: CompiledAppFuseZone,
    optional: bool,
    selecting_player: crate::enums::PlayerId,
    source_card: CardHandle,
    source_permanent: Option<PermanentHandle>,
    source_kind: crate::enums::EffectSourceKind,
) {
    if results.is_empty() {
        return;
    }
    let valid_action_ids: Vec<u16> = results.iter().map(|(a, _)| *a).collect();
    let previous_phase = game.current_phase;
    let gz = match zone {
        CompiledAppFuseZone::Hand => AppFuseSourceZone::Hand,
        CompiledAppFuseZone::Trash => AppFuseSourceZone::Trash,
    };
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
        callback: Box::new(move |game, action_id| {
            let Some((_, card)) = results.iter().find(|(a, _)| *a == action_id).copied() else {
                return;
            };
            game.commit_effect_app_fuse(controller, host, card, gz);
        }),
        on_decline: None,
    });
}
