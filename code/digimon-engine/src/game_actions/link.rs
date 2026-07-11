//! DigiLink Shape-B (Digimon-link) — `impl Game`.
//!
//! An un-linked standing Appmon Link Digimon (e.g. BT21-009 Gatchmon) activates
//! its printed `[Main]` Link ability to attach itself onto one of the
//! controller's other Digimon; plus the facet-#9 chosen-card link path and the
//! per-link-card trash used by the Gap-3a leave-replacement cost. Mirrors DCGO
//! `CardEffectFactory.LinkEffect` + `ILinkCard.LinkCard`.

#![allow(unused_imports)]
use super::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

impl Game {
    /// DigiLink Shape-B: if `handle` is an un-linked standing Digimon that
    /// carries a `LinkCondition` self-effect (an Appmon Link Digimon like
    /// BT21-009 Gatchmon), return its link cost and the set of legal hosts it
    /// may link onto right now. Returns `None` when the permanent has no self
    /// link-condition. Mirrors DCGO `LinkEffect`'s `CanUseCondition` +
    /// `CanSelectPermanentCondition`: the linking Digimon is excluded as its
    /// own host, and each host is filtered through the printed link filter and
    /// the shared `link_host_candidates` eligibility (Digimon, Standard state,
    /// link-max). Cost is the printed cost before `ChangeLinkCost` modifiers,
    /// which the initiation applies at pay time via `link_cost_delta_for_player`.
    pub fn digimon_link_condition_targets(
        &self,
        handle: PermanentHandle,
    ) -> Option<(u16, Vec<PermanentHandle>)> {
        let owner = handle.player;
        let perm = self.player(owner).battle_area.get(handle.index as usize)?;
        // Only an un-linked standing Digimon can be a link SOURCE.
        if !matches!(perm.option_state, crate::permanent::OptionState::Standard) {
            return None;
        }
        if !self.permanent_is_digimon_for_rules(handle) {
            return None;
        }
        let top = perm.top_card();
        let source_card = top.handle();
        let effects = self.effects_for_card(top.card_id(&self.card_data), source_card)?;
        let cost = effects
            .iter()
            .find(|e| e.timing == EffectTiming::LinkCondition && e.link_cost.is_some())
            .and_then(|e| e.link_cost)?;
        let mut hosts = self.link_host_candidates(owner, source_card, &effects);
        // A Digimon cannot link onto itself.
        hosts.retain(|h| *h != handle);
        Some((cost, hosts))
    }

    // ───────────────────── DigiLink Shape-B (Digimon-link) ─────────────────
    //
    // An un-linked standing Appmon Link Digimon (e.g. BT21-009 Gatchmon)
    // activates its printed `[Main]` Link ability to attach itself onto one of
    // the controller's other Digimon. Mirrors DCGO `CardEffectFactory.LinkEffect`
    // + `ILinkCard.LinkCard` (root `None`, the standing-permanent path): fire
    // `WhenWouldLink`, pay the link cost, then absorb the linking permanent —
    // its digivolution sources are trashed (DCGO `DiscardEvoRoots`) and only its
    // top card becomes a single linked card on the host — then fire `OnLink`.

    /// Non-mutating affordability of a link cost against the shared memory
    /// gauge (mirrors `pay_memory`'s floor check).
    fn can_afford_link_cost(&self, cost: u16) -> bool {
        (self.memory as i32 - cost as i32) >= self.rules.memory_range.0 as i32
    }

    /// Decode entry for the FIELD_EFFECT link sub-slot: the standing Digimon at
    /// `perm_idx` declares a link. Installs a host-selection prompt whose pick
    /// drives `begin_digimon_link`. No-op if the permanent has no self
    /// link-condition, no legal host, or the cost is unaffordable (the mask
    /// already guards all three; this re-checks defensively).
    pub(crate) fn activate_field_link(&mut self, player: PlayerId, perm_idx: usize) {
        let source = PermanentHandle {
            player,
            index: perm_idx as u8,
        };
        let Some((cost, hosts)) = self.digimon_link_condition_targets(source) else {
            return;
        };
        if hosts.is_empty() || !self.can_afford_link_cost(cost) {
            return;
        }
        let Some(source_card) = self
            .player(player)
            .battle_area
            .get(perm_idx)
            .map(|p| p.top_card().handle())
        else {
            return;
        };
        self.install_digimon_link_host_selection(player, source, source_card, cost, hosts);
    }

    /// Install the host-selection prompt for a Digimon-link. Reuses the
    /// attack-id encoding convention shared with `install_link_host_selection`;
    /// the resolved pick drives `begin_digimon_link`.
    pub(crate) fn install_digimon_link_host_selection(
        &mut self,
        owner: PlayerId,
        source: PermanentHandle,
        source_card: crate::card_source::CardHandle,
        cost: u16,
        candidates: Vec<PermanentHandle>,
    ) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};
        use crate::selection::SelectionKind;

        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|h| encode_attack(0, h.index as u16))
            .collect();
        let candidate_snapshot = candidates.clone();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::OwnField,
            selecting_player: owner,
            previous_phase,
            valid_action_ids,
            is_optional: false,
            prompt: "Choose a Digimon to link this Digimon to".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: Some(source),
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let picked = candidate_snapshot
                    .iter()
                    .copied()
                    .find(|h| h.index == target_index)
                    .unwrap_or(PermanentHandle {
                        player: owner,
                        index: target_index,
                    });
                game.begin_digimon_link(source, picked, cost);
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DigimonLinkHostSelection(
                DigimonLinkHostSelectionState {
                    owner,
                    source,
                    cost,
                    candidates,
                },
            )],
        });
    }

    pub(crate) fn run_digimon_link_host_selection_step(
        &mut self,
        state: DigimonLinkHostSelectionState,
        action_id: u16,
    ) {
        use crate::action::space::{ATTACK_START, TARGETS_PER_ATTACKER};

        let offset = action_id.saturating_sub(ATTACK_START);
        let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
        let picked = state
            .candidates
            .iter()
            .copied()
            .find(|h| h.index == target_index)
            .unwrap_or(PermanentHandle {
                player: state.owner,
                index: target_index,
            });
        self.begin_digimon_link(state.source, picked, state.cost);
    }

    /// Begin the link attach: fire the `WhenWouldLink` replacement window on the
    /// linking card, then commit (or park if the replacement installs an
    /// interactive selection, resumed via `commit_digimon_link`).
    pub(crate) fn begin_digimon_link(
        &mut self,
        source: PermanentHandle,
        host: PermanentHandle,
        cost: u16,
    ) {
        use crate::enums::Zone;
        use crate::replacement::{ReplacementCause, ReplacementSubject};

        let Some(src_perm) = self
            .player(source.player)
            .battle_area
            .get(source.index as usize)
        else {
            return;
        };
        if !matches!(
            src_perm.option_state,
            crate::permanent::OptionState::Standard
        ) {
            return;
        }
        let source_card = src_perm.top_card().handle();
        let host_live = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map(|p| {
                self.permanent_is_digimon_for_rules(host)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false);
        if !host_live {
            return;
        }

        self.pending_digimon_link = Some(crate::game::PendingDigimonLink {
            source,
            host,
            cost,
            card: source_card,
        });
        // Expose the link host for the duration of the `WhenWouldLink` window so
        // a host-side reducer effect can verify "...link to THIS Digimon" via
        // `EffectContext::pending_link_host` (Gap 5). It is cleared by
        // `commit_digimon_link` (synchronous and parked paths both route there).
        self.pending_link_host = Some(host);
        let outcome = self.try_replace(
            EffectTiming::WhenWouldLink,
            ReplacementSubject::Card(source_card, Zone::BattleArea),
            ReplacementCause::OwnEffect,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            // An optional/interactive replacement parked — `pending_link_host`
            // stays live until the resumed `commit_digimon_link` clears it.
            return;
        }
        self.commit_digimon_link(outcome);
    }

    /// Commit (or abort) a parked Digimon-link after its `WhenWouldLink`
    /// replacement resolves. Pays the cost and absorbs the linking permanent.
    pub(crate) fn commit_digimon_link(&mut self, outcome: crate::replacement::ReplacementOutcome) {
        use crate::permanent::OptionState;
        use crate::replacement::ReplacementOutcome;

        let Some(p) = self.pending_digimon_link.take() else {
            return;
        };
        // The `WhenWouldLink` window is closing — the host is no longer "about
        // to be linked onto". Clear it on every path below (commit or abort).
        self.pending_link_host = None;
        if !matches!(outcome, ReplacementOutcome::None) {
            // Cancelled / redirected / substituted — link aborted, source stays
            // standing, no cost paid.
            self.check_turn_end();
            return;
        }
        // Re-validate both permanents are still live (an interactive
        // replacement may have moved things mid-window).
        let source_live = self
            .player(p.source.player)
            .battle_area
            .get(p.source.index as usize)
            .map(|perm| {
                perm.top_card().handle() == p.card
                    && matches!(perm.option_state, OptionState::Standard)
            })
            .unwrap_or(false);
        let host_live = self
            .player(p.host.player)
            .battle_area
            .get(p.host.index as usize)
            .map(|perm| {
                self.permanent_is_digimon_for_rules(p.host)
                    && matches!(perm.option_state, OptionState::Standard)
            })
            .unwrap_or(false);
        if !source_live || !host_live {
            self.check_turn_end();
            return;
        }
        let effective = (p.cost as i32 + self.modifiers.link_cost_delta_for_player(p.source.player))
            .max(0) as u16;
        if !self.pay_memory(effective) {
            self.check_turn_end();
            return;
        }
        self.absorb_standing_digimon_as_link(p.source, p.host);
        self.check_turn_end();
    }

    /// Absorb a standing Digimon `source` into `host`'s linked cards. The
    /// digivolution sources under `source`'s top card are trashed (DCGO
    /// `DiscardEvoRoots`); only the top card becomes a single linked card.
    /// Follows the canonical removal sequence (`clear_permanent_full` →
    /// `delete slot` → `shift_after_battle_area_remove`) and fixes the host
    /// handle with `shift_handle_after_soft_remove` before attaching.
    pub(crate) fn absorb_standing_digimon_as_link(
        &mut self,
        source: PermanentHandle,
        host: PermanentHandle,
    ) {
        // Pre-clear modifiers + granted bodies for the leaving permanent.
        self.clear_permanent_full(source);
        self.modifiers.expire_player_on_permanent_leave(source);
        // Remove the slot, taking ownership of the permanent.
        let perm = {
            let ba = &mut self.player_mut(source.player).battle_area;
            if (source.index as usize) >= ba.len() {
                return;
            }
            ba.remove(source.index as usize)
        };
        self.modifiers
            .shift_after_battle_area_remove(source.player, source.index);
        self.shift_pending_attack_after_battle_area_remove(source.player, source.index);

        let mut sources = perm.card_sources;
        let Some(top) = sources.pop() else {
            return;
        };
        let top_handle = top.handle();
        let top_owner = top.owner;
        // Digivolution sources under the top → owner's trash + trigger.
        for card in sources {
            let scard = card.handle();
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
            self.fire_digivolution_card_trashed(
                source.player,
                source,
                top_handle,
                scard,
                crate::trigger_context::EventCause::OwnEffect,
            );
        }
        // The source's own linked cards (rare) → trash + OnLinkedCardTrashed.
        let had_linked = !perm.linked_cards.is_empty();
        for card in perm.linked_cards {
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
        }
        if had_linked {
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
            self.drain_effect_queue();
        }
        // Fix the host handle after the Vec shift, then attach the top card.
        let host_adj = Game::shift_handle_after_soft_remove(source, host);
        match self
            .player_mut(host_adj.player)
            .battle_area
            .get_mut(host_adj.index as usize)
        {
            Some(hp) => hp.linked_cards.push(top),
            None => {
                // Host vanished — route the top card to its owner's trash.
                self.player_mut(top_owner).trash.push(top);
                return;
            }
        }
        // Fire OnLink so the linked card's WhenLinked + ESS resolve. The
        // `Linked` trigger carries the just-linked card for the self-filter.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnLink,
                TriggerSource::Linked {
                    player: pid as PlayerId,
                    host: host_adj,
                    card: top_handle,
                },
            );
        }
        self.drain_effect_queue();
    }

    /// Facet #9 — link a **chosen card** from a non-battle-area zone (hand,
    /// trash, or another permanent's digivolution sources) onto `host`'s
    /// linked cards. This is the DCGO `ILinkCard.LinkCard` path with
    /// `root != None` (`Permanent.AddLinkCard`): a single card is moved out of
    /// its zone and attached sideways — no standing permanent is absorbed and
    /// no `DiscardEvoRoots` runs (the chosen card was never a stack top).
    ///
    /// Cost payment and `WhenWouldLink` are the calling effect's
    /// responsibility (effect-driven links register their own cost reduction
    /// and pay via the effect body) — the standing-permanent activate path
    /// (`begin_digimon_link`) owns the interactive `WhenWouldLink` replacement
    /// window. After the move, `OnLink` fires globally so the linked card's
    /// `[When Linking]` self-filter, the host's `[When Linked]`, and the
    /// linked-card ESS all resolve, identical to every other attach site.
    ///
    /// Returns `true` if the card was found in `from` and attached.
    pub fn link_chosen_card_into_host(
        &mut self,
        host: PermanentHandle,
        card: crate::card_source::CardHandle,
        from: crate::enums::LinkCardSource,
    ) -> bool {
        use crate::enums::LinkCardSource;
        use crate::permanent::OptionState;

        // Host must be a live standing Digimon.
        let host_ok = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map(|p| {
                self.permanent_is_digimon_for_rules(host)
                    && matches!(p.option_state, OptionState::Standard)
            })
            .unwrap_or(false);
        if !host_ok {
            return false;
        }

        // Lift the chosen card out of its source zone.
        let moved = match from {
            LinkCardSource::Hand(owner) => {
                let pos = self
                    .player(owner)
                    .hand
                    .iter()
                    .position(|c| c.handle() == card);
                pos.map(|i| self.player_mut(owner).hand.remove(i))
            }
            LinkCardSource::Trash(owner) => {
                let pos = self
                    .player(owner)
                    .trash
                    .iter()
                    .position(|c| c.handle() == card);
                pos.map(|i| self.player_mut(owner).trash.remove(i))
            }
            LinkCardSource::DigivolutionSource(src) => {
                // A digivolution source under another permanent's top card.
                // It must not be the stack top (the top is the live Digimon).
                match self.player(src.player).battle_area.get(src.index as usize) {
                    Some(perm) => {
                        let top_pos = perm.card_sources.len().saturating_sub(1);
                        let pos = perm
                            .card_sources
                            .iter()
                            .position(|c| c.handle() == card)
                            .filter(|&i| i != top_pos);
                        pos.and_then(|i| {
                            self.player_mut(src.player)
                                .battle_area
                                .get_mut(src.index as usize)
                                .map(|p| p.card_sources.remove(i))
                        })
                    }
                    None => None,
                }
            }
            LinkCardSource::OptionInPlay(owner) => {
                // Gap 3b — lift the in-play Option out of `pending_option` so
                // its Standard dispose finds nothing to trash. The card must be
                // the one currently held in `pending_option` and owned by
                // `owner`; otherwise no-op (defensive).
                match self.pending_option.as_ref() {
                    Some(pending) if pending.owner == owner && pending.card.handle() == card => {
                        self.pending_option.take().map(|p| p.card)
                    }
                    _ => None,
                }
            }
        };
        let Some(moved) = moved else {
            return false;
        };
        let moved_handle = moved.handle();
        let moved_owner = moved.owner;

        // Attach onto the host. If the host vanished between the validity
        // check and here (it cannot in synchronous flow, but be defensive),
        // route the lifted card to its owner's trash.
        match self
            .player_mut(host.player)
            .battle_area
            .get_mut(host.index as usize)
        {
            Some(hp) => hp.linked_cards.push(moved),
            None => {
                self.player_mut(moved_owner).trash.push(moved);
                return false;
            }
        }

        // Fire OnLink globally with the just-linked card identity.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnLink,
                TriggerSource::Linked {
                    player: pid as PlayerId,
                    host,
                    card: moved_handle,
                },
            );
        }
        self.maybe_drain_effect_queue();
        true
    }

    /// Trash one specific link card off `host` (addressed by `card` handle),
    /// routing it to its owner's trash and firing `OnLinkedCardTrashed` globally.
    /// Returns `true` if the card was found among `host.linked_cards` and
    /// trashed; `false` otherwise (host gone / card not linked there).
    ///
    /// Used by the Gap-3a leave-replacement cost ("by trashing 1 of its link
    /// cards, it doesn't leave"). The trashed card leaves the host but the host
    /// itself stays — this is NOT a host-leave path, so only the single chosen
    /// link card moves. DCGO ref: `TrashLinkedCards.cs` (per-card trash + the
    /// `OnLinkedCardTrashed` observer dispatch).
    pub fn trash_specific_link_card(
        &mut self,
        host: PermanentHandle,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(perm) = self
            .player_mut(host.player)
            .battle_area
            .get_mut(host.index as usize)
        else {
            return false;
        };
        let Some(pos) = perm.linked_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let removed = perm.linked_cards.remove(pos);
        let owner = removed.owner;
        self.player_mut(owner).trash.push(removed);

        // Fire OnLinkedCardTrashed globally — mirrors the host-leave linked-card
        // disposition at game.rs:3749 and place_permanent_on_security_observed.
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnLinkedCardTrashed,
                TriggerSource::PlayerBattleArea(pid as crate::PlayerId),
            );
        }
        self.maybe_drain_effect_queue();
        true
    }

    /// Trash one specific DIGIVOLUTION SOURCE off `host` (addressed by `card`
    /// handle — a source BELOW the stack top), routing it to its owner's trash
    /// and firing `OnDigivolutionCardTrashed` globally. Returns `true` if the
    /// card was found among `host.card_sources` (below the top) and trashed;
    /// `false` otherwise (host gone / card not a below-top source / card IS the
    /// top card — never decapitate the host).
    ///
    /// The digivolution-source sibling of [`Self::trash_specific_link_card`].
    /// Used by the `trash_option_from_own_stacks` activation cost (BT25-085
    /// BeelStarmon) and the EX7-048 protect-others leave cost, both of which
    /// trash an Option from a permanent's digivolution cards. DCGO trashes a
    /// `DigivolutionCards` entry via `ITrashDigivolutionCards` (fires the
    /// `OnDigivolutionCardTrashed` observer). Routes through
    /// [`Self::trash_source_and_fire`] so the `host_card` (the top card AFTER
    /// removal) and the standard effect cause are derived uniformly.
    pub fn trash_specific_source_card(
        &mut self,
        host: PermanentHandle,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(perm) = self
            .player_mut(host.player)
            .battle_area
            .get_mut(host.index as usize)
        else {
            return false;
        };
        let Some(pos) = perm.card_sources.iter().position(|c| c.handle() == card) else {
            return false;
        };
        // Never pull the TOP card (that is the Digimon itself, not a
        // digivolution source) — decapitating the host is not a source trash.
        if pos + 1 == perm.card_sources.len() {
            return false;
        }
        let removed = perm.card_sources.remove(pos);
        let owner = removed.owner;
        // The host card AFTER removal is the (possibly promoted) top card.
        let host_card = perm.top_card().handle();
        self.trash_source_and_fire(owner, host, removed, host_card);
        true
    }

    /// Move one specific link card off `host` (addressed by `card` handle) into
    /// `host`'s digivolution sources as its BOTTOM card (DCGO
    /// `Permanent.AddDigivolutionCardsBottom`). Unlike `trash_specific_link_card`
    /// the card is NOT trashed — it leaves `linked_cards` and becomes a
    /// digivolution source under the carrier — so no `OnLinkedCardTrashed`
    /// fires. Returns `true` if the card was found among `host.linked_cards` and
    /// moved; `false` otherwise (host gone / card not linked there).
    ///
    /// Used by the EX11-027 leave-replacement cost ("by placing 1 of its link
    /// cards as its bottom digivolution card, it doesn't leave"). The host
    /// itself stays — only the single chosen link card relocates.
    /// DCGO ref: `EX11_027.cs` Link-Effect region (`AddDigivolutionCardsBottom`).
    pub fn place_specific_link_card_as_bottom_source(
        &mut self,
        host: PermanentHandle,
        card: crate::card_source::CardHandle,
    ) -> bool {
        let Some(perm) = self
            .player_mut(host.player)
            .battle_area
            .get_mut(host.index as usize)
        else {
            return false;
        };
        let Some(pos) = perm.linked_cards.iter().position(|c| c.handle() == card) else {
            return false;
        };
        let moved = perm.linked_cards.remove(pos);
        // Place it as the carrier's bottom digivolution source (under the stack).
        perm.push_under(moved);
        true
    }
}
