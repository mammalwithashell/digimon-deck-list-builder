//! DigiLink Shape-B link handling on `EffectContext` — the host-side reducer
//! window readers, the in-flight link-cost mutation, the leave-replacement
//! link-card-trash cost, and the facet-#9 chosen-card link entrypoint.
//!
//! Extracted as its own mechanic submodule (see `action/mod.rs`); all of the
//! `Game`-side machinery these call lives in `game_actions/` + `game/`.

#![allow(unused_imports)]
use crate::card_source::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::permanent::*;

impl<'a> EffectReadContext<'a> {
    /// The host a card is about to link ONTO during the active `WhenWouldLink`
    /// replacement window, or `None` outside that window. A host-side reducer
    /// effect (Gap 5 — BT25-004 / BT25-045) compares this against its own
    /// `source_permanent` to fire only when the linking card is attaching to
    /// THIS Digimon ("...link to this Digimon").
    pub fn pending_link_host(&self) -> Option<PermanentHandle> {
        self.game.pending_link_host
    }

    /// The card about to link (the `WhenWouldLink` replacement subject) during
    /// the active standing-Digimon link window, or `None` outside it. Lets a
    /// host-side `condition` / DSL `active_when` predicate inspect the linking
    /// card's traits ("when a [Social]/[Tool]/[Game] card would link...").
    pub fn would_link_subject_card(&self) -> Option<CardHandle> {
        self.game.pending_digimon_link.as_ref().map(|p| p.card)
    }
}

impl<'a> EffectContext<'a> {
    /// See [`EffectReadContext::pending_link_host`].
    pub fn pending_link_host(&self) -> Option<PermanentHandle> {
        self.game.pending_link_host
    }

    /// See [`EffectReadContext::would_link_subject_card`].
    pub fn would_link_subject_card(&self) -> Option<CardHandle> {
        self.game.pending_digimon_link.as_ref().map(|p| p.card)
    }

    /// Reduce the cost of the link about to resolve in the active
    /// `WhenWouldLink` window by `n` (saturating at 0). Called from a host-side
    /// reducer effect's `replacement_process` accept-branch (Gap 5 — BT25-004 /
    /// BT25-045: "you may reduce the cost by 1"). `commit_digimon_link` then
    /// pays the reduced `pending_digimon_link.cost`.
    ///
    /// This is a one-shot mutation of the in-flight pending link only — it does
    /// NOT install a persistent `ChangeLinkCost` modifier, so it cannot leak to
    /// any other link this turn. Returns `true` if a pending link was present.
    pub fn reduce_pending_link_cost(&mut self, n: u16) -> bool {
        if let Some(p) = self.game.pending_digimon_link.as_mut() {
            p.cost = p.cost.saturating_sub(n);
            true
        } else {
            false
        }
    }

    /// Gap 3a — pay the link-card-trash cost of a `WhenWouldLeaveBattleArea`
    /// replacement ("by trashing 1 of its link cards, it doesn't leave").
    ///
    /// Installs a selection over `host`'s link cards (one option per card, so
    /// the choice of WHICH link card to trash is exposed to the RL action
    /// space — never auto-selected). On pick, the chosen link card is trashed
    /// via `Game::trash_specific_link_card` (firing `OnLinkedCardTrashed`) and
    /// the parked leave is cancelled (`cancel_leave`).
    ///
    /// Caller MUST gate on `host.linked_cards.len() >= 1` via the replacement's
    /// `replacement_condition`/preflight so this is never reached with no cost
    /// to pay. With exactly one link card a single-option selection still
    /// installs (faithful to DCGO's `TrashLinkedCards` UI flow — the choice is
    /// surfaced even when forced).
    ///
    /// DCGO ref: `OnTrashLinkCard.cs` (CanUse: has ≥1 link card) +
    /// `TrashLinkedCards.cs` (the per-card trash + observer dispatch).
    pub fn trash_own_link_card_and_cancel_leave(&mut self, host: PermanentHandle) {
        // Snapshot the link-card handles + labels at install time.
        let Some(perm) = self
            .game
            .player(host.player)
            .battle_area
            .get(host.index as usize)
        else {
            return;
        };
        if perm.linked_cards.is_empty() {
            return;
        }
        let cards: Vec<crate::card_source::CardHandle> =
            perm.linked_cards.iter().map(|c| c.handle()).collect();
        let labels: Vec<String> = cards
            .iter()
            .map(|h| {
                self.game
                    .card_data_for_handle(*h)
                    .map(|d| d.card_name.clone())
                    .unwrap_or_else(|| "Link card".to_string())
            })
            .collect();

        self.select_effect_choice(
            "Choose 1 link card to trash (it doesn't leave)",
            labels,
            move |cb_ctx, idx| {
                let Some(card) = cards.get(idx).copied() else {
                    return;
                };
                if cb_ctx.game.trash_specific_link_card(host, card) {
                    cb_ctx.cancel_leave();
                }
            },
        );
    }

    /// EX11-027 Maquinamon — pay a `WhenWouldLeaveBattleArea` replacement by
    /// placing 1 of `host`'s link cards as its BOTTOM digivolution card ("by
    /// placing 1 of its link cards as its bottom digivolution card, it doesn't
    /// leave"). Sibling of [`Self::trash_own_link_card_and_cancel_leave`]: same
    /// player-chosen-which-link-card selection (exposed to the RL action space,
    /// never auto-selected), but the chosen card is MOVED under the carrier via
    /// `Game::place_specific_link_card_as_bottom_source` (no trash, no
    /// `OnLinkedCardTrashed`) before the parked leave is cancelled.
    ///
    /// Caller MUST gate on `host.linked_cards.len() >= 1` (the replacement
    /// preflight does). With exactly one link card a single-option selection
    /// still installs (faithful to DCGO's `SelectCardEffect` flow).
    pub fn place_link_card_as_bottom_source_and_cancel_leave(&mut self, host: PermanentHandle) {
        let Some(perm) = self
            .game
            .player(host.player)
            .battle_area
            .get(host.index as usize)
        else {
            return;
        };
        if perm.linked_cards.is_empty() {
            return;
        }
        let cards: Vec<crate::card_source::CardHandle> =
            perm.linked_cards.iter().map(|c| c.handle()).collect();
        let labels: Vec<String> = cards
            .iter()
            .map(|h| {
                self.game
                    .card_data_for_handle(*h)
                    .map(|d| d.card_name.clone())
                    .unwrap_or_else(|| "Link card".to_string())
            })
            .collect();

        self.select_effect_choice(
            "Choose 1 link card to place as the bottom digivolution card (it doesn't leave)",
            labels,
            move |cb_ctx, idx| {
                let Some(card) = cards.get(idx).copied() else {
                    return;
                };
                if cb_ctx.game.place_specific_link_card_as_bottom_source(host, card) {
                    cb_ctx.cancel_leave();
                }
            },
        );
    }

    /// Facet #9 — link a chosen card from a non-battle-area zone (hand /
    /// trash / a permanent's digivolution sources) onto `host`'s linked cards
    /// (DCGO `ILinkCard.LinkCard` with `root != None`). The card is lifted out
    /// of `from`, attached sideways onto the host, and `OnLink` fires globally.
    /// Cost payment + `WhenWouldLink` cost reduction are the calling effect's
    /// responsibility (register via the cost step before invoking). Returns
    /// `true` when the card was found and attached.
    pub fn link_chosen_card_into_host(
        &mut self,
        host: PermanentHandle,
        card: CardHandle,
        from: crate::enums::LinkCardSource,
    ) -> bool {
        self.game.link_chosen_card_into_host(host, card, from)
    }
}
