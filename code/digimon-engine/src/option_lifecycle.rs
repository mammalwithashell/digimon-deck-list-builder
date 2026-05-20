//! Explicit lifecycle taxonomy for Option cards that persist beyond immediate
//! resolution.
//!
//! Rules sources:
//! - `docs/RULES_CONTEXT.md` section 7: ordinary Options resolve and trash
//!   unless an effect places them elsewhere.
//! - `docs/RULES_CONTEXT.md` section 16-16: Delay Options stay in the battle
//!   area and activate by trashing themselves.
//! - `docs/RULES_CONTEXT.md` sections 4-7 and 8: Link/Plug-In cards attach to
//!   a carrier and are not ordinary standalone battle-area permanents while
//!   linked.

use crate::card_source::CardSource;
use crate::enums::{DelayTrigger, EffectTiming, PlayerId};
use crate::game::{Game, DelayedOptionLifecycleResume, DelayedOptionLifecycleResumeKind};
use crate::permanent::{OptionState, Permanent, PermanentHandle};
use crate::selection::TriggerSource;

/// Canonical, mutually-exclusive field state for persistent Option cards.
///
/// This is the public lifecycle vocabulary for card authors and DSL lowering.
/// `Permanent::option_state` remains the compact storage shape used by the
/// current engine, while `Game::option_field_state` and
/// `Game::linked_plug_in_field_state` expose this taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionFieldState {
    Delay {
        placed_turn: u32,
        can_activate_this_turn: bool,
    },
    LinkedPlugIn {
        carrier: PermanentHandle,
        link_index: u8,
    },
    OrphanedPlugIn {
        last_carrier_owner: PlayerId,
    },
    OrdinaryFieldOption,
}

/// Cause attached to an Option leaving its persistent lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionTrashCause {
    Effect,
    LeaveField,
    EndOfTurnDelayExpiry,
    PlugInCarrierLoss,
    SecurityActivation,
    Resolution,
}

impl Game {
    /// Query a standalone field Option's explicit lifecycle state.
    ///
    /// Returns `None` for linked Plug-In Options represented by
    /// `OptionState::Linked`, because this standalone storage shape does not
    /// carry the precise link slot. Callers that need linked Plug-In lifecycle
    /// details must use `linked_plug_in_field_state`.
    pub fn option_field_state(&self, option: PermanentHandle) -> Option<OptionFieldState> {
        let perm = self
            .player(option.player)
            .battle_area
            .get(option.index as usize)?;
        match perm.option_state {
            OptionState::Delayed { placed_on_turn, .. } => Some(OptionFieldState::Delay {
                placed_turn: placed_on_turn as u32,
                can_activate_this_turn: self.turn_count > placed_on_turn,
            }),
            OptionState::OrdinaryFieldOption => Some(OptionFieldState::OrdinaryFieldOption),
            OptionState::OrphanedPlugIn { last_carrier_owner } => {
                Some(OptionFieldState::OrphanedPlugIn { last_carrier_owner })
            }
            OptionState::Training { .. } => Some(OptionFieldState::OrdinaryFieldOption),
            OptionState::Linked { .. } => None,
            OptionState::Standard => {
                let card = perm.top_card();
                if card.is_option_card_for_search(&self.card_data) {
                    Some(OptionFieldState::OrdinaryFieldOption)
                } else {
                    None
                }
            }
        }
    }

    /// Query a Plug-In attached inside a carrier's linked-card list.
    pub fn linked_plug_in_field_state(
        &self,
        carrier: PermanentHandle,
        link_index: u8,
    ) -> Option<OptionFieldState> {
        self.player(carrier.player)
            .battle_area
            .get(carrier.index as usize)?
            .linked_cards
            .get(link_index as usize)?;
        Some(OptionFieldState::LinkedPlugIn {
            carrier,
            link_index,
        })
    }

    fn can_insert_plug_in_at(&self, carrier: PermanentHandle, link_index: u8) -> bool {
        let Some(host) = self
            .player(carrier.player)
            .battle_area
            .get(carrier.index as usize)
        else {
            return false;
        };
        let insert_at = link_index as usize;
        let link_max =
            (5 + self.modifiers.link_max_delta(carrier)).clamp(0, u8::MAX as i32) as usize;
        insert_at <= host.linked_cards.len() && host.linked_cards.len() < link_max
    }

    /// Install an Option as a Delay field permanent.
    pub fn install_field_option_as_delay(
        &mut self,
        card: CardSource,
        controller: PlayerId,
        placed_turn: u32,
    ) -> PermanentHandle {
        let placed_turn = placed_turn.min(u16::MAX as u32) as u16;
        let trigger = self
            .effects_for_card(card.card_id(&self.card_data), card.handle())
            .unwrap_or_default()
            .iter()
            .find_map(|effect| effect.delay_trigger)
            .unwrap_or(DelayTrigger::EndOfYourNextTurn);
        let mut permanent = Permanent::new(card, placed_turn);
        let placed_card = permanent.top_card().handle();
        permanent.option_state = OptionState::Delayed {
            owner: controller,
            trash_on_turn: self.compute_delay_trash_turn(controller, trigger),
            trigger,
            placed_on_turn: placed_turn,
        };
        self.player_mut(controller).battle_area.push(permanent);
        let handle = PermanentHandle {
            player: controller,
            index: (self.player(controller).battle_area.len() - 1) as u8,
        };
        self.enqueue_triggered(
            EffectTiming::OnOptionPlaced,
            TriggerSource::OptionPlaced {
                player: controller,
                permanent: Some(handle),
                linked_host: None,
                card: placed_card,
            },
        );
        self.drain_effect_queue();
        handle
    }

    /// Install an Option as an ordinary field Option.
    pub fn install_field_option_as_ordinary(
        &mut self,
        card: CardSource,
        controller: PlayerId,
    ) -> PermanentHandle {
        let mut permanent = Permanent::new(card, self.turn_count);
        let placed_card = permanent.top_card().handle();
        permanent.option_state = OptionState::OrdinaryFieldOption;
        self.player_mut(controller).battle_area.push(permanent);
        let handle = PermanentHandle {
            player: controller,
            index: (self.player(controller).battle_area.len() - 1) as u8,
        };
        self.enqueue_triggered(
            EffectTiming::OnOptionPlaced,
            TriggerSource::OptionPlaced {
                player: controller,
                permanent: Some(handle),
                linked_host: None,
                card: placed_card,
            },
        );
        self.drain_effect_queue();
        handle
    }

    /// Install a Plug-In from a source zone into a carrier's linked-card list.
    pub fn install_field_option_as_plug_in(
        &mut self,
        card: CardSource,
        carrier: PermanentHandle,
        link_index: u8,
    ) -> bool {
        if !self.can_insert_plug_in_at(carrier, link_index) {
            return false;
        }
        let Some(host) = self
            .player_mut(carrier.player)
            .battle_area
            .get_mut(carrier.index as usize)
        else {
            return false;
        };
        let insert_at = link_index as usize;
        let placed_card = card.handle();
        host.linked_cards.insert(insert_at, card);
        self.enqueue_triggered(
            EffectTiming::OnOptionPlaced,
            TriggerSource::OptionPlaced {
                player: carrier.player,
                permanent: None,
                linked_host: Some(carrier),
                card: placed_card,
            },
        );
        self.drain_effect_queue();
        for pid in 0..self.players.len() {
            self.enqueue_triggered(
                EffectTiming::OnLink,
                TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.drain_effect_queue();
        true
    }

    /// Convert an attached Plug-In into an orphaned field Option.
    pub fn orphan_linked_plug_in(
        &mut self,
        carrier: PermanentHandle,
        link_index: u8,
        last_carrier_owner: PlayerId,
    ) -> Option<PermanentHandle> {
        let card = {
            let host = self
                .player_mut(carrier.player)
                .battle_area
                .get_mut(carrier.index as usize)?;
            if link_index as usize >= host.linked_cards.len() {
                return None;
            }
            host.linked_cards.remove(link_index as usize)
        };
        let owner = card.owner;
        let mut permanent = Permanent::new(card, self.turn_count);
        permanent.option_state = OptionState::OrphanedPlugIn { last_carrier_owner };
        self.player_mut(owner).battle_area.push(permanent);
        Some(PermanentHandle {
            player: owner,
            index: (self.player(owner).battle_area.len() - 1) as u8,
        })
    }

    /// Mark an existing standalone Plug-In permanent as orphaned.
    pub fn orphan_plug_in(
        &mut self,
        option_handle: PermanentHandle,
        last_carrier_owner: PlayerId,
    ) -> bool {
        let Some(perm) = self
            .player_mut(option_handle.player)
            .battle_area
            .get_mut(option_handle.index as usize)
        else {
            return false;
        };
        perm.option_state = OptionState::OrphanedPlugIn { last_carrier_owner };
        true
    }

    /// Re-link an orphaned Plug-In field Option to a carrier.
    pub fn relink_plug_in(
        &mut self,
        option_handle: PermanentHandle,
        mut new_carrier: PermanentHandle,
        link_index: u8,
    ) -> bool {
        let Some(perm) = self
            .player(option_handle.player)
            .battle_area
            .get(option_handle.index as usize)
        else {
            return false;
        };
        if !matches!(perm.option_state, OptionState::OrphanedPlugIn { .. }) {
            return false;
        }
        if perm.card_sources.len() != 1 {
            return false;
        }
        if option_handle == new_carrier {
            return false;
        }
        if option_handle.player == new_carrier.player && option_handle.index < new_carrier.index {
            new_carrier.index = new_carrier.index.saturating_sub(1);
        }
        if !self.can_insert_plug_in_at(new_carrier, link_index) {
            return false;
        }
        let permanent = self
            .player_mut(option_handle.player)
            .battle_area
            .remove(option_handle.index as usize);
        let card = permanent
            .card_sources
            .into_iter()
            .next()
            .expect("validated orphaned Plug-In must have one source");
        let installed = self.install_field_option_as_plug_in(card, new_carrier, link_index);
        debug_assert!(installed, "validated Plug-In relink must install");
        installed
    }

    /// Trash a standalone field Option and dispatch `OnOptionTrashed`.
    pub fn trash_field_option(
        &mut self,
        option_handle: PermanentHandle,
        cause: OptionTrashCause,
    ) -> bool {
        let Some(last_state) = self.option_field_state(option_handle) else {
            return false;
        };
        let owner = option_handle.player;
        let Some(permanent) = self
            .player(owner)
            .battle_area
            .get(option_handle.index as usize)
        else {
            return false;
        };
        if !permanent
            .top_card()
            .is_option_card_for_search(&self.card_data)
        {
            return false;
        }

        let permanent = self
            .player_mut(owner)
            .battle_area
            .remove(option_handle.index as usize);
        let mut cards = permanent.card_sources;
        let Some(top) = cards.pop() else {
            return false;
        };
        let card = top.handle();
        for source in cards {
            self.player_mut(source.owner).trash.push(source);
        }
        self.player_mut(top.owner).trash.push(top);
        self.enqueue_triggered(
            EffectTiming::OnOptionTrashed,
            TriggerSource::OptionTrashed {
                player: owner,
                card,
                cause,
                last_state,
            },
        );
        self.drain_effect_queue();
        true
    }

    // ── Standard `<Delay>` `[Main]`-phase activation (PUPPETS-G009) ──────
    //
    // A standard `<Delay>` Option (`DelayTrigger::MainPhaseActivated`) is NOT
    // auto-fired by the start/end-of-turn scan. Its controller takes a
    // player-visible `[Main]`-phase `FIELD_EFFECT` action — "By trashing this
    // card after the placing turn, activate the effect below" (RULES_CONTEXT
    // 16-16). The activation is optional (16-16-2) and unavailable on the
    // placing turn (16-16-3).

    /// True when the permanent at `option` is a standard `<Delay>` Option
    /// whose placing turn has passed — i.e. its `<Delay>` body may be
    /// activated by its controller's `[Main]`-phase action this turn.
    ///
    /// Panic-safe on out-of-range player / field indices (returns `false`)
    /// so the decode path can call it before its own bounds checks.
    pub fn delayed_option_main_activation_available(&self, option: PermanentHandle) -> bool {
        let Some(perm) = self
            .players
            .get(option.player as usize)
            .and_then(|player| player.battle_area.get(option.index as usize))
        else {
            return false;
        };
        match perm.option_state {
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                placed_on_turn,
                ..
            } => self.turn_count > placed_on_turn,
            _ => false,
        }
    }

    /// Activate a standard `<Delay>` Option's body via its controller's
    /// `[Main]`-phase action. Fires the stored `DelayEffect` body, then
    /// trashes the Option as the activation cost.
    ///
    /// Returns `true` if the activation was dispatched. Mirrors the
    /// per-permanent block of [`Game::resolve_delayed_options_matching`]: the
    /// `DelayEffect` body runs first, then the Option is trashed via
    /// `delete_permanent_with_cause` (cause = `Cost`) so Phase 7 replacement
    /// windows still get a chance to intercept. If the body installs a
    /// pending selection, the trash is deferred behind a
    /// `MainPhaseActivation` lifecycle resume; if the *trash itself* parks a
    /// replacement-window selection, the lifecycle is likewise re-parked so
    /// the resume path re-drives the deletion's continuation — keeping this
    /// path identical to the sibling turn-scan's post-delete handling.
    pub fn activate_delayed_option_main(&mut self, option: PermanentHandle) -> bool {
        if !self.delayed_option_main_activation_available(option) {
            return false;
        }

        // Stable key `(owner, bottom_card_index)` survives the index shifts a
        // `DelayEffect` body might cause. An Option has no digivolution stack,
        // so the bottom card is the Option itself.
        let key = {
            let perm = &self.player(option.player).battle_area[option.index as usize];
            (option.player, perm.card_sources[0].card_index)
        };

        self.enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(option));
        self.drain_effect_queue();

        // The body installed a selection (e.g. the digivolve pick on P-105 /
        // LM-054). Defer the trash until the selection resolves.
        if self.pending_selection.is_some() {
            self.park_delayed_option_lifecycle(DelayedOptionLifecycleResume {
                turn: u16::MAX,
                kind: DelayedOptionLifecycleResumeKind::MainPhaseActivation,
                pending_delete_key: Some(key),
                skip_key: None,
            });
            return true;
        }

        // Re-find by stable key — the body may have shifted indices — then
        // trash the Option as the activation cost.
        if let Some(handle) =
            self.find_delayed_permanent_by_key(key, u16::MAX, &[DelayTrigger::MainPhaseActivated])
        {
            self.delete_permanent_with_cause(handle, crate::replacement::ReplacementCause::Cost);
        }

        // The cost trash can itself park a `pending_selection` at a
        // `WhenWouldLeaveBattleArea` / `WhenWouldBeDeleted` replacement window
        // and early-return (`delete_permanent_with_cause`'s own doc: "caller
        // re-drives"). Mirror the sibling turn-scan path
        // (`resolve_delayed_options_matching`, `game_phases.rs`): re-park the
        // lifecycle with `pending_delete_key: None, skip_key: Some(key)` so
        // `resume_pending_delayed_option_lifecycle` re-drives the deletion's
        // continuation rather than silently completing.
        //
        // For the `MainPhaseActivation` resume kind the re-driven continuation
        // is currently a no-op cleanup — exactly ONE Option is activated by
        // this action, so there are no sibling Delays to scan on resume, and
        // the deferred-delete (`pending_delete_key`) is `None` because the
        // delete was already attempted above. The re-park therefore costs
        // nothing observable today, but it keeps the two post-delete fire
        // sites (turn-scan and `[Main]`-phase activation) handling the
        // parked-selection case IDENTICALLY, so a future change to the
        // post-delete continuation (e.g. a faithful follow-up effect) cannot
        // silently drop it from only the `[Main]`-phase path. None of the 5
        // PUPPETS-G009 migrated Delay Options carry a trash-time replacement
        // window, so this branch is latent — but the divergence from the
        // engine's own established defensive pattern is closed regardless.
        if self.pending_selection.is_some() {
            self.park_delayed_option_lifecycle(DelayedOptionLifecycleResume {
                turn: u16::MAX,
                kind: DelayedOptionLifecycleResumeKind::MainPhaseActivation,
                pending_delete_key: None,
                skip_key: Some(key),
            });
        }
        true
    }
}
