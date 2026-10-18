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
    /// the shared `link_host_candidates` eligibility (Digimon, Standard state;
    /// a host at its link limit included, §4-9-5). Cost is the printed cost
    /// before `ChangeLinkCost` modifiers, which the initiation applies at pay
    /// time via `link_cost_delta_for_player`.
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

    /// DigiLink Shape-B, hand origin: if the card at `player`'s hand slot
    /// `hand_index` is a Digimon carrying a `LinkCondition` self-effect, return
    /// its link cost and the legal hosts it may link onto right now. `None`
    /// when the card has no self link-condition (or is not a Digimon — the
    /// Plug-In *Option* hand link is `OptionPlayMode::Link`, a play mode).
    ///
    /// Mirrors DCGO `LinkEffect`'s `CanUseCondition` for the
    /// `IsExistOnHand(card)` branch: every standing Digimon of the owner that
    /// passes the printed link filter and `link_host_candidates`' shared
    /// eligibility (Digimon, Standard state; at the limit the link replaces,
    /// §4-9-5). No self-exclusion is needed — the card is not on the field.
    /// Cost is the printed cost before
    /// `ChangeLinkCost` modifiers, applied at pay time like the field origin.
    pub fn hand_digimon_link_condition_targets(
        &self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<(u16, Vec<PermanentHandle>)> {
        let card = self.player(player).hand.get(hand_index)?;
        if card.card_kind(&self.card_data) != CardKind::Digimon {
            return None;
        }
        let source_card = card.handle();
        let effects = self.effects_for_card(card.card_id(&self.card_data), source_card)?;
        let cost = effects
            .iter()
            .find(|e| e.timing == EffectTiming::LinkCondition && e.link_cost.is_some())
            .and_then(|e| e.link_cost)?;
        let hosts = self.link_host_candidates(player, source_card, &effects);
        Some((cost, hosts))
    }

    /// Whether the `HAND_EFFECT` bit for `hand_index` should carry a hand
    /// link: a self link-condition with at least one legal host and an
    /// affordable cost. Shared by the mask and the decoder so the bit is never
    /// offered for a link the decoder would then refuse.
    pub fn hand_digimon_link_available(&self, player: PlayerId, hand_index: usize) -> bool {
        match self.hand_digimon_link_condition_targets(player, hand_index) {
            Some((cost, hosts)) => !hosts.is_empty() && self.can_afford_link_cost(cost),
            None => false,
        }
    }

    /// The Plug-In **Option** half of the from-hand link declaration: if the
    /// card at `player`'s hand slot `hand_index` is an Option (or a Dual card
    /// read as one) carrying a `LinkCondition` effect, return its link cost
    /// and the legal hosts it may plug into right now.
    ///
    /// `general_rule.pdf` (Ver.4.3) §10-1-1: "A card from the hand or battle
    /// area can be linked to a Digimon in the battle area by paying the cost
    /// **as part of the main phase actions**" — and §6-5-1 lists "use an
    /// Option card from the hand" (§6-5-1-3) and "link a card from the hand or
    /// battle area" (§6-5-1-4) as two SEPARATE main-phase actions. So plugging
    /// a Plug-In Option in from hand is its own declaration, not a mode of
    /// using it; the `HAND_EFFECT` bit is where that declaration lives, exactly
    /// as it does for a Link Digimon (`hand_digimon_link_available`).
    ///
    /// DCGO agrees and needs no special case for it: `CardEffectFactory
    /// .LinkEffect` (`Link.cs:19-24`) accepts ANY `CardSource` with a
    /// `linkCondition` that `IsExistOnHand`, so the Option's link declaration
    /// sits in its `CanDeclareSkillList` beside a Digimon's and the harness's
    /// `InputDriver.FindHandDeclarableSkillIndex` finds it the same way.
    ///
    /// No Option use-requirement / colour check is applied: those gate §9-1
    /// "Using Cards", and `LinkEffect`'s own `CanUseCondition` (`Link.cs:49`)
    /// checks only the origin zone and the host set. Cost is the printed link
    /// cost; the `ChangeLinkCost` delta is applied by the availability check
    /// and again at pay time inside `play_option_core`.
    pub fn hand_option_link_condition_targets(
        &self,
        player: PlayerId,
        hand_index: usize,
    ) -> Option<(u16, Vec<PermanentHandle>)> {
        let card = self.player(player).hand.get(hand_index)?;
        if !matches!(
            card.card_kind(&self.card_data),
            CardKind::Option | CardKind::Dual
        ) {
            return None;
        }
        let source_card = card.handle();
        let effects = self.effects_for_card(card.card_id(&self.card_data), source_card)?;
        let cost = effects.iter().find_map(|e| e.link_cost)?;
        let hosts = self.link_host_candidates(player, source_card, &effects);
        Some((cost, hosts))
    }

    /// Whether the `HAND_EFFECT` bit for `hand_index` should carry a Plug-In
    /// Option's from-hand link: a link condition with at least one legal host
    /// and an affordable cost. The affordability mirrors
    /// `option_legal_play_modes`' Link arm (the `ChangeLinkCost` player delta
    /// applies), so the bit is never offered for a link the decoder's
    /// `play_option_core` re-entry would then refuse.
    pub fn hand_option_link_available(&self, player: PlayerId, hand_index: usize) -> bool {
        match self.hand_option_link_condition_targets(player, hand_index) {
            Some((cost, hosts)) => {
                !hosts.is_empty()
                    && self.can_afford_link_cost(self.effective_link_cost(player, cost))
            }
            None => false,
        }
    }

    /// The printed link cost after this player's `ChangeLinkCost` modifiers —
    /// the number `play_option_core`'s Link arm actually pays.
    pub(crate) fn effective_link_cost(&self, player: PlayerId, cost: u16) -> u16 {
        (cost as i32 + self.modifiers.link_cost_delta_for_player(player)).max(0) as u16
    }

    /// Either half of the §6-5-1-4 from-hand link declaration: the Shape-B
    /// Link **Digimon** or the Plug-In **Option**. One card is never both (the
    /// two helpers gate on disjoint `CardKind`s).
    pub fn hand_link_available(&self, player: PlayerId, hand_index: usize) -> bool {
        self.hand_digimon_link_available(player, hand_index)
            || self.hand_option_link_available(player, hand_index)
    }

    /// What the `HAND_EFFECT` bit for `hand_index` MEANS right now: `true`
    /// when it is the from-hand `<Link>` declaration, `false` when it is a
    /// `[Hand] [Main]` activation (or nothing). The decoder's order is `[Main]`
    /// first, link second, so a card carrying both exposes only its `[Main]`
    /// on this bit — the one-declarable-per-hand-slot limit DCGO's recorder
    /// shares. Published so `explain_action` and the exam's lowering agree
    /// with the decoder on which decision the bit is.
    pub fn hand_effect_slot_is_link(&self, player: PlayerId, hand_index: usize) -> bool {
        crate::action::main_effect_select::hand_main_match(self, player, hand_index).is_none()
            && self.hand_link_available(player, hand_index)
    }

    /// Decode entry for a `HAND_EFFECT` bit that carries a hand link (no
    /// `[Hand] [Main]` effect fired for the slot): the Digimon card at
    /// `hand_index` declares its `<Link>` from hand. Installs the same
    /// host-selection prompt the field origin uses; the pick drives
    /// `begin_digimon_link` with a `Hand` origin. Returns `false` when the
    /// slot carries no legal hand link (the mask already guards this).
    pub(crate) fn activate_hand_link(&mut self, player: PlayerId, hand_index: usize) -> bool {
        // A Plug-In OPTION takes the Option lifecycle, not `begin_digimon_link`:
        // the card is used out of the hand through `play_option_core` in
        // `OptionPlayMode::Link`, which pays the link cost, fires
        // `OnUseOption` without the `[Main]` body, and disposes by plugging the
        // card into the chosen host. Same §6-5-1-4 declaration, different
        // resolution machinery — see `hand_option_link_condition_targets`.
        //
        // `G-ENGINE-OPTION-HAND-LINK-COST-TIMING`: the HOST PICK COMES FIRST.
        // `general_rule.pdf` (Ver.4.3) §10-1-3, p.21, orders the link
        // procedure 10-1-3-1 "the player chooses 1 of their Digimon that meets
        // the requirement" -> 10-1-3-2 "The specified link cost is paid." ->
        // 10-1-3-3 "The card to link is plugged in sideways into the chosen
        // Digimon" -- payment is step 2 of 3. §9-1-8 adds that revealing a card
        // as part of a use procedure "isn't considered removal from an area",
        // so the declared card is still in hand while the host is picked.
        // DCGO matches (`LinkEffect.ActivateCoroutine`, `Link.cs:70-88`: the
        // `SelectPermanentEffect` host pick runs before the payment). So the
        // declaration installs the host prompt and pays NOTHING; the resolved
        // pick pins the host and re-enters `play_option_core`, whose `Link`
        // arm pays (§10-1-3-2) and whose `dispose_option` plugs the card into
        // the pinned host (§10-1-3-3) without asking again.
        if self.hand_option_link_available(player, hand_index) {
            let Some((cost, hosts)) = self.hand_option_link_condition_targets(player, hand_index)
            else {
                return false;
            };
            if hosts.is_empty() {
                return false;
            }
            let Some(card) = self.player(player).hand.get(hand_index).map(|c| c.handle()) else {
                return false;
            };
            self.install_option_hand_link_host_selection(player, hand_index, card, cost, hosts);
            return true;
        }
        let Some((cost, hosts)) = self.hand_digimon_link_condition_targets(player, hand_index)
        else {
            return false;
        };
        if hosts.is_empty() || !self.can_afford_link_cost(cost) {
            return false;
        }
        let Some(card) = self.player(player).hand.get(hand_index).map(|c| c.handle()) else {
            return false;
        };
        self.install_digimon_link_host_selection(
            player,
            crate::game::DigimonLinkOrigin::Hand(player),
            card,
            cost,
            hosts,
        );
        true
    }

    /// Install the §6-5-1-4 host prompt for a from-hand Plug-In **Option**
    /// link, BEFORE any cost is paid and before the card leaves the hand
    /// (`general_rule.pdf` §10-1-3-1/-2, §9-1-8 — see `activate_hand_link`).
    /// Prompt shape is identical to `install_link_host_selection`'s (same
    /// `OwnField` kind, same attack-id encoding, same wording) so the wire
    /// meaning of the pick is unchanged; only its position in the procedure
    /// moved.
    pub(crate) fn install_option_hand_link_host_selection(
        &mut self,
        owner: PlayerId,
        hand_index: usize,
        card: crate::card_source::CardHandle,
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
            prompt: "Choose a Digimon to link this Option to".to_string(),
            effect_choices: None,
            source_card: card,
            source_permanent: None,
            source_kind: EffectSourceKind::Option,
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
                game.finish_option_hand_link(owner, hand_index, card, cost, picked);
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::OptionHandLinkHostSelection(
                OptionHandLinkHostSelectionState {
                    owner,
                    hand_index,
                    card,
                    cost,
                    candidates,
                },
            )],
        });
    }

    pub(crate) fn run_option_hand_link_host_selection_step(
        &mut self,
        state: OptionHandLinkHostSelectionState,
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
        self.finish_option_hand_link(
            state.owner,
            state.hand_index,
            state.card,
            state.cost,
            picked,
        );
    }

    /// §10-1-3-2 + §10-1-3-3 for a from-hand Plug-In Option link: pin the
    /// chosen host and run the Option lifecycle, which pays the link cost,
    /// fires the link-mode body, and disposes by plugging the card into the
    /// pinned host. The hand slot is re-validated first (an interposing effect
    /// could have moved the card while the prompt was parked); a stale slot
    /// aborts without paying, which §9-1-8 explicitly allows ("the memory
    /// doesn't move when a card can't be used because its cost can't be
    /// paid").
    fn finish_option_hand_link(
        &mut self,
        owner: PlayerId,
        hand_index: usize,
        card: crate::card_source::CardHandle,
        cost: u16,
        host: PermanentHandle,
    ) {
        let still_there = self
            .player(owner)
            .hand
            .get(hand_index)
            .map(|c| c.handle() == card)
            .unwrap_or(false);
        if !still_there {
            return;
        }
        self.pending_option_link_host = Some(host);
        let result = self.play_option_core(
            owner,
            OptionSource::Hand(hand_index),
            Some(OptionPlayMode::Link { cost }),
            OptionCostPolicy::Pay,
        );
        if matches!(result, OptionPlayResult::Invalid) {
            // Never leave a stale pin behind for the next link.
            self.pending_option_link_host = None;
        }
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
        self.install_digimon_link_host_selection(
            player,
            crate::game::DigimonLinkOrigin::Standing(source),
            source_card,
            cost,
            hosts,
        );
    }

    /// Install the host-selection prompt for a Digimon-link from either
    /// origin. Reuses the attack-id encoding convention shared with
    /// `install_link_host_selection`; the resolved pick drives
    /// `begin_digimon_link`.
    pub(crate) fn install_digimon_link_host_selection(
        &mut self,
        owner: PlayerId,
        origin: crate::game::DigimonLinkOrigin,
        source_card: crate::card_source::CardHandle,
        cost: u16,
        candidates: Vec<PermanentHandle>,
    ) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};
        use crate::game::DigimonLinkOrigin;
        use crate::selection::SelectionKind;

        let valid_action_ids: Vec<u16> = candidates
            .iter()
            .map(|h| encode_attack(0, h.index as u16))
            .collect();
        let candidate_snapshot = candidates.clone();
        let source_permanent = match origin {
            DigimonLinkOrigin::Standing(handle) => Some(handle),
            DigimonLinkOrigin::Hand(_) => None,
        };

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
            source_permanent,
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
                game.begin_digimon_link(origin, source_card, picked, cost);
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DigimonLinkHostSelection(
                DigimonLinkHostSelectionState {
                    owner,
                    origin,
                    card: source_card,
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
        self.begin_digimon_link(state.origin, state.card, picked, state.cost);
    }

    /// Whether the linking card is still where its origin says it is: a
    /// standing, un-linked permanent whose top card is `card`, or a card in
    /// the owner's hand. Checked before the `WhenWouldLink` window opens and
    /// again before the cost is paid (an interactive replacement may have
    /// moved things mid-window).
    fn digimon_link_source_live(
        &self,
        origin: crate::game::DigimonLinkOrigin,
        card: crate::card_source::CardHandle,
    ) -> bool {
        use crate::game::DigimonLinkOrigin;
        use crate::permanent::OptionState;
        match origin {
            DigimonLinkOrigin::Standing(source) => self
                .player(source.player)
                .battle_area
                .get(source.index as usize)
                .map(|perm| {
                    perm.top_card().handle() == card
                        && matches!(perm.option_state, OptionState::Standard)
                })
                .unwrap_or(false),
            DigimonLinkOrigin::Hand(owner) => {
                self.player(owner).hand.iter().any(|c| c.handle() == card)
            }
        }
    }

    /// Begin the link attach: fire the `WhenWouldLink` replacement window on the
    /// linking card, then commit (or park if the replacement installs an
    /// interactive selection, resumed via `commit_digimon_link`).
    ///
    /// `card` is the linking card — for a `Standing` origin the top card of
    /// the source permanent, for a `Hand` origin the hand card. Mirrors DCGO
    /// `ILinkCard.LinkCard`: `WhenWouldLink` → pay `GetChangedLinkCost` →
    /// `IPlacePermanentToLinkCards` (root `None`) / `Permanent.AddLinkCard`
    /// (root `Hand`).
    pub(crate) fn begin_digimon_link(
        &mut self,
        origin: crate::game::DigimonLinkOrigin,
        card: crate::card_source::CardHandle,
        host: PermanentHandle,
        cost: u16,
    ) {
        use crate::enums::Zone;
        use crate::game::DigimonLinkOrigin;
        use crate::replacement::{ReplacementCause, ReplacementSubject};

        if !self.digimon_link_source_live(origin, card) {
            return;
        }
        let source_card = card;
        let subject_zone = match origin {
            DigimonLinkOrigin::Standing(_) => Zone::BattleArea,
            DigimonLinkOrigin::Hand(_) => Zone::Hand,
        };
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
            origin,
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
            ReplacementSubject::Card(source_card, subject_zone),
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
        use crate::game::DigimonLinkOrigin;
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
        // Re-validate the source and the host are still live (an interactive
        // replacement may have moved things mid-window).
        let source_live = self.digimon_link_source_live(p.origin, p.card);
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
        let owner = match p.origin {
            DigimonLinkOrigin::Standing(source) => source.player,
            DigimonLinkOrigin::Hand(owner) => owner,
        };
        let effective =
            (p.cost as i32 + self.modifiers.link_cost_delta_for_player(owner)).max(0) as u16;
        if !self.pay_memory(effective) {
            self.check_turn_end();
            return;
        }
        // §4-9-5 / §10-1-3-3: at the link limit the new card replaces existing
        // link cards; with a choice, the linking player picks them first.
        let purpose = LinkTrashPurpose::DigimonLink {
            origin: p.origin,
            host: p.host,
            card: p.card,
        };
        if self.park_link_replacement_choice(owner, p.host, purpose) {
            return;
        }
        self.finish_digimon_link_attach(p.origin, p.host, p.card);
    }

    /// §10-1-3-3: plug the paid-for linking card into `host` (the attach
    /// primitives trim a forced single replaced link card, §4-9-5).
    pub(crate) fn finish_digimon_link_attach(
        &mut self,
        origin: crate::game::DigimonLinkOrigin,
        host: PermanentHandle,
        card: crate::card_source::CardHandle,
    ) {
        use crate::game::DigimonLinkOrigin;
        match origin {
            DigimonLinkOrigin::Standing(source) => {
                self.absorb_standing_digimon_as_link(source, host);
            }
            // Hand origin — DCGO `Permanent.AddLinkCard`: the card is lifted
            // out of the hand and attached; `OnLink` fires inside.
            DigimonLinkOrigin::Hand(owner) => {
                let _ = self.link_chosen_card_into_host(
                    host,
                    card,
                    crate::enums::LinkCardSource::Hand(owner),
                );
            }
        }
        self.check_turn_end();
    }

    /// An effect links its own standing Digimon `source` onto `host`
    /// (`relink_self_to_own_digimon`, EX11-027; DCGO
    /// `IPlacePermanentToLinkCards`). At the link limit with a choice, the
    /// linking player `chooser` first picks the replaced link cards (§4-9-5,
    /// §10-1-3-1), and the absorb runs after the picks; otherwise it absorbs
    /// now. Returns `true` when the choice was parked.
    pub(crate) fn link_standing_digimon_by_effect(
        &mut self,
        chooser: PlayerId,
        source: PermanentHandle,
        host: PermanentHandle,
    ) -> bool {
        let purpose = LinkTrashPurpose::RelinkStanding { source, host };
        if self.park_link_replacement_choice(chooser, host, purpose) {
            return true;
        }
        self.absorb_standing_digimon_as_link(source, host);
        false
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
        self.shift_effect_queue_after_battle_area_remove(source.player, source.index);

        let mut sources = perm.card_sources;
        let Some(top) = sources.pop() else {
            return;
        };
        let top_handle = top.handle();
        let top_owner = top.owner;
        // Digivolution sources under the top → owner's trash + trigger, all
        // at the same time: ONE event (§15-5-2), enqueued without draining —
        // nothing activates before the link has finished (§15-8-3-2).
        let mut trashed = Vec::new();
        for card in sources {
            let scard = card.handle();
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
            trashed.push(super::TrashedSource {
                player: source.player,
                host: source,
                host_card: top_handle,
                card: scard,
            });
        }
        self.enqueue_digivolution_cards_trashed(
            trashed,
            crate::trigger_context::EventCause::OwnEffect,
        );
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
        }
        // Fix the host handle after the Vec shift, then attach the top card.
        let host_adj = Game::shift_handle_after_soft_remove(source, host);
        // §4-9-5: every caller parks the replacement choice first
        // (`park_link_replacement_choice`); only a forced trim is left here.
        debug_assert!(
            !self.link_replacement_is_a_choice(host_adj),
            "§4-9-5: park_link_replacement_choice before absorbing onto a host at its limit"
        );
        self.trash_forced_link_replacement(host_adj);
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
        // The link has finished: its trash and link observers activate now,
        // unless an effect is still resolving around it (§15-8-3-2).
        self.maybe_drain_effect_queue();
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
        self.trash_forced_link_replacement(host);
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
        let host_card = perm.top_card().handle();
        let removed = perm.linked_cards.remove(pos);
        let owner = removed.owner;
        self.player_mut(owner).trash.push(removed);

        // Fire OnLinkedCardTrashed globally (every battle area + event-gated
        // Delays) carrying the host, the trashed card and the cause — an
        // effect (or an effect's cost) is what trashes a link card off a host
        // that stays. G-DSL-ON-LINK-CARD-TRASHED-DELAY.
        let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(host.player));
        self.enqueue_triggered(
            EffectTiming::OnLinkedCardTrashed,
            TriggerSource::LinkCardTrashed {
                player: host.player,
                host,
                host_card,
                card,
                cause,
            },
        );
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
    ///
    /// `cause` is the placing effect (the replacement-cost body): DCGO
    /// `AddDigivolutionCardsBottom(..., cardEffect)` on a linked card unlinks
    /// it and fires `OnAddDigivolutionCards` for the host, so the host's batch
    /// window is noted here. G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS.
    pub fn place_specific_link_card_as_bottom_source(
        &mut self,
        host: PermanentHandle,
        card: crate::card_source::CardHandle,
        cause: crate::trigger_context::EffectAttribution,
    ) -> bool {
        if self.is_token_source_destination(host) { return false; }
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
        self.note_effect_added_sources(host, vec![card], cause);
        true
    }
}

// ─────────── §4-9-5 link limit, link-time replacement, §17-1-3-2-5..7 ───────────

/// A §17-1-3-2-5..7 rule-check target on one link host, collected by
/// [`Game::collect_link_rule_targets`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkRuleTarget {
    /// §17-1-3-2-5: the host holds this many link cards over its link limit,
    /// counted after its `Unqualified` cards leave (DCGO runs
    /// `DigimonLackLinkConditionProcess` before `DigimonLackLinkMaxCountProcess`).
    /// The host's controller chooses which (Q6370).
    Excess(usize),
    /// §17-1-3-2-6 / -7: link cards whose link requirement the host doesn't
    /// meet, or with no `[Link]` at all (no `<Link>` note names their
    /// category). Trashed with no choice.
    Unqualified(Vec<CardHandle>),
}

/// What runs after a link-card trash choice's picks are trashed. Every link
/// entry point that can replace link cards (§4-9-5) parks the same
/// `LinkCardTrashChoice` frame through [`Game::park_link_replacement_choice`]
/// and finishes its own attach here.
#[derive(Debug, Clone)]
pub(crate) enum LinkTrashPurpose {
    /// §17-1-3-2-5 rule-check trim; nothing else follows.
    RuleExcess,
    /// §4-9-5 replacement in a declared Digimon link (cost already paid):
    /// attach `card` from `origin` onto `host`.
    DigimonLink {
        origin: crate::game::DigimonLinkOrigin,
        host: PermanentHandle,
        card: CardHandle,
    },
    /// §4-9-5 replacement in a Plug-In Option link: resume
    /// `commit_linked_card_no_replace`.
    OptionLink(crate::game::PendingWouldLinkResume),
    /// §4-9-5 replacement in an effect's `link_cards` pick (cost already
    /// paid): attach the chosen card, then continue the step's pick loop.
    EffectLink(Box<crate::dsl_cards::step::link_cards::EffectLinkResume>),
    /// §4-9-5 replacement in an effect relinking its own standing Digimon
    /// (`relink_self_to_own_digimon`): absorb `source` onto `host`.
    RelinkStanding {
        source: PermanentHandle,
        host: PermanentHandle,
    },
    /// §4-9-5 replacement in an orphaned Plug-In's relink onto `carrier`:
    /// resume [`Game::relink_plug_in`].
    RelinkPlugIn {
        option: PermanentHandle,
        carrier: PermanentHandle,
        link_index: u8,
    },
}

/// Resumable state of a link-card trash choice (rule 28: plain data). The
/// `chooser` picks one of `remaining` per decision until `picks_left` is 0;
/// the picked cards are trashed together after the last pick.
#[derive(Debug, Clone)]
pub(crate) struct LinkCardTrashChoiceState {
    pub(crate) chooser: PlayerId,
    pub(crate) host: PermanentHandle,
    pub(crate) remaining: Vec<CardHandle>,
    pub(crate) picks_left: usize,
    pub(crate) chosen: Vec<CardHandle>,
    pub(crate) purpose: LinkTrashPurpose,
    pub(crate) outer_conts: Vec<crate::resume::OuterContinuation>,
}

impl Game {
    /// §4-9-5 "1 card can have a maximum of 1 link card", plus every
    /// `<Link +N>` instance (§16-40-1, -3; the `ChangeLinkMax` modifiers).
    /// DCGO `Permanent.LinkedMax` starts at 1 likewise.
    pub fn link_limit(&self, host: PermanentHandle) -> usize {
        (1 + self.modifiers.link_max_delta(host)).clamp(0, u8::MAX as i32) as usize
    }

    /// §4-9-5: how many of `host`'s existing link cards one more link card
    /// replaces ("the same number of the existing link cards are trashed at
    /// the same time as the newly linked cards"); 0 below the limit.
    pub(crate) fn link_replacement_excess(&self, host: PermanentHandle) -> usize {
        let len = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map_or(0, |p| p.linked_cards.len());
        (len + 1).saturating_sub(self.link_limit(host)).min(len)
    }

    /// §4-9-5 / §10-1-3-1: whether one more link card onto `host` replaces
    /// some but not all of its link cards, so the linking player chooses
    /// which (2+ existing link cards at the limit).
    pub(crate) fn link_replacement_is_a_choice(&self, host: PermanentHandle) -> bool {
        let len = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map_or(0, |p| p.linked_cards.len());
        let excess = self.link_replacement_excess(host);
        excess > 0 && excess < len
    }

    /// Whether `card`, linked to `host`, still qualifies (§17-1-3-2-6 / -7):
    /// the host is a Digimon and meets one of the card's link requirements
    /// (a `link_cost` effect's `link_filter`, the predicate
    /// `link_host_candidates` offers hosts by). A card with no `[Link]` has no
    /// requirement to meet and no `<Link>` note naming its category. DCGO
    /// `AutoProcessing.IsDigimonLackLinkCondition` (`CanLinkToTargetPermanent`).
    pub(crate) fn link_card_meets_requirements(
        &self,
        host: PermanentHandle,
        card: &CardSource,
    ) -> bool {
        self.link_card_qualifies_for(host, card.handle())
    }

    /// [`Self::link_card_meets_requirements`] by handle, for a card not yet
    /// linked (Q4533: an effect can't link a card without `[Link]`; DCGO
    /// `CanLinkToTargetPermanent`). Shared by the rule check, the declared
    /// link's host candidates and the effect-driven link gates.
    pub fn link_card_qualifies_for(&self, host: PermanentHandle, card: CardHandle) -> bool {
        if !self.permanent_is_digimon_for_rules(host) {
            return false;
        }
        let Some(card_id) = self.card_data_for_handle(card).map(|d| d.card_id.clone()) else {
            return false;
        };
        let effects = self.effects_for_card(&card_id, card).unwrap_or_default();
        self.link_requirement_met_by(card, &effects[..], host, host.player)
    }

    /// Whether `host` meets one of `card`'s link requirements (`effects` are
    /// its effects; `reader` the controller the filters read as). A card
    /// whose printed Link box exists (`CardData::link_text`) but whose
    /// requirement isn't lowered in the engine counts as meeting it (unknown
    /// isn't unmet; `G-DATA-LINK-REQUIREMENT-NOT-LOWERED`); a card with no
    /// `[Link]` at all meets none (§17-1-3-2-7).
    pub(crate) fn link_requirement_met_by(
        &self,
        card: CardHandle,
        effects: &[crate::effect::Effect],
        host: PermanentHandle,
        reader: PlayerId,
    ) -> bool {
        let mut requirements = effects.iter().filter(|e| e.link_cost.is_some()).peekable();
        if requirements.peek().is_none() {
            return self
                .card_data_for_handle(card)
                .is_some_and(|d| !d.link_text.trim().is_empty());
        }
        let read_ctx = EffectReadContext::new(self, card, None, reader);
        // `any`, not the chosen requirement: a card with two link requirements
        // at different costs could be offered a host only the second admits
        // while the declared link pays the first one's cost (§10-1-3-1 has the
        // player choose 1 requirement). No lowered card has two today.
        requirements.any(|e| e.link_filter.as_ref().map_or(true, |f| f(&read_ctx, host)))
    }

    /// The §17-1-3-2-5..7 targets on every player's battle area, in
    /// (player, index) order. Read-only; `run_state_based_rules_check`
    /// applies them.
    pub fn collect_link_rule_targets(&self) -> Vec<(PermanentHandle, LinkRuleTarget)> {
        let mut out = Vec::new();
        for (pid, player) in self.players.iter().enumerate() {
            for (idx, perm) in player.battle_area.iter().enumerate() {
                if perm.linked_cards.is_empty() || perm.card_sources.is_empty() {
                    continue;
                }
                let host = PermanentHandle {
                    player: pid as PlayerId,
                    index: idx as u8,
                };
                let unqualified: Vec<CardHandle> = perm
                    .linked_cards
                    .iter()
                    .filter(|c| !self.link_card_meets_requirements(host, c))
                    .map(|c| c.handle())
                    .collect();
                let kept = perm.linked_cards.len() - unqualified.len();
                if !unqualified.is_empty() {
                    out.push((host, LinkRuleTarget::Unqualified(unqualified)));
                }
                let limit = self.link_limit(host);
                if kept > limit {
                    out.push((host, LinkRuleTarget::Excess(kept - limit)));
                }
            }
        }
        out
    }

    /// Trash `cards` off `host`'s link cards together, by rule (§4-9-5
    /// replacement, §17-1-3-2-5..7): each goes to its owner's trash and an
    /// `OnLinkedCardTrashed` (cause `Rule`) is enqueued. Does not drain, so the
    /// triggers resolve with whatever the caller enqueues next (§4-9-5 "at the
    /// same time as the newly linked cards").
    pub(crate) fn trash_link_cards_by_rule(&mut self, host: PermanentHandle, cards: &[CardHandle]) {
        let Some(perm) = self
            .player_mut(host.player)
            .battle_area
            .get_mut(host.index as usize)
        else {
            return;
        };
        if perm.card_sources.is_empty() {
            return;
        }
        let host_card = perm.top_card().handle();
        let mut removed = Vec::new();
        for card in cards {
            if let Some(pos) = perm.linked_cards.iter().position(|c| c.handle() == *card) {
                removed.push(perm.linked_cards.remove(pos));
            }
        }
        for card in removed {
            let handle = card.handle();
            let owner = card.owner;
            self.player_mut(owner).trash.push(card);
            self.enqueue_triggered(
                EffectTiming::OnLinkedCardTrashed,
                TriggerSource::LinkCardTrashed {
                    player: host.player,
                    host,
                    host_card,
                    card: handle,
                    cause: EventCause::Rule,
                },
            );
        }
    }

    /// §4-9-5 with nothing to choose: one more link card replaces every one
    /// of `host`'s link cards (a lone card at the limit, DCGO
    /// `Permanent.AddLinkCard`'s `LinkedMax == 1` branch; or every card when
    /// the excess covers them all). Called by every attach primitive just
    /// before the push. Where a choice exists, every link entry point (the
    /// declared Digimon and Option links, the effect `link_cards` pick, the
    /// effect relinks) parks [`Self::park_link_replacement_choice`] before it
    /// reaches the primitive, so a choice is never left here.
    pub(crate) fn trash_forced_link_replacement(&mut self, host: PermanentHandle) {
        let cards: Vec<CardHandle> = match self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
        {
            Some(p) => p.linked_cards.iter().map(|c| c.handle()).collect(),
            None => return,
        };
        let excess = self.link_replacement_excess(host);
        if !cards.is_empty() && excess >= cards.len() {
            self.trash_link_cards_by_rule(host, &cards);
        }
    }

    /// §4-9-5 / §10-1-3-1 for every link (declared, Plug-In Option, or made
    /// by an effect): when the new card replaces existing link cards and there
    /// is a choice (fewer to trash than there are), park the linking player's
    /// choice of exactly that many; `purpose` finishes the link after the
    /// picks are trashed. Returns `false` when nothing needs choosing: every
    /// existing card goes (trashed here, e.g. a host already over its limit),
    /// or a lone card the attach primitive trims, or nothing could be parked;
    /// the caller then attaches directly.
    pub(crate) fn park_link_replacement_choice(
        &mut self,
        chooser: PlayerId,
        host: PermanentHandle,
        purpose: LinkTrashPurpose,
    ) -> bool {
        let excess = self.link_replacement_excess(host);
        let cards: Vec<CardHandle> = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map(|p| p.linked_cards.iter().map(|c| c.handle()).collect())
            .unwrap_or_default();
        if excess == 0 || cards.len() <= 1 {
            // Nothing, or a lone card the attach primitive trims.
            return false;
        }
        if !self.link_replacement_is_a_choice(host) {
            // No choice: every existing link card is replaced.
            self.trash_link_cards_by_rule(host, &cards);
            return false;
        }
        self.install_link_card_trash_choice(LinkCardTrashChoiceState {
            chooser,
            host,
            remaining: cards,
            picks_left: excess,
            chosen: Vec::new(),
            purpose,
            outer_conts: Vec::new(),
        })
    }

    /// §17-1-3-2-5 (Q6370): the first host over its link limit has its
    /// controller choose exactly the excess (never a tail trim). When every
    /// removable card is excess (a lone card, or a limit of 0) they are
    /// trashed directly: there is nothing to choose. Returns `true` if it
    /// acted.
    pub(crate) fn park_first_link_excess_trim(&mut self) -> bool {
        let targets = self.collect_link_rule_targets();
        let Some((host, excess)) = targets.iter().find_map(|(h, t)| match t {
            LinkRuleTarget::Excess(n) => Some((*h, *n)),
            LinkRuleTarget::Unqualified(_) => None,
        }) else {
            return false;
        };
        let unqualified: Vec<CardHandle> = targets
            .iter()
            .filter(|(h, _)| *h == host)
            .flat_map(|(_, t)| match t {
                LinkRuleTarget::Unqualified(c) => c.clone(),
                LinkRuleTarget::Excess(_) => Vec::new(),
            })
            .collect();
        let remaining: Vec<CardHandle> = self.player(host.player).battle_area[host.index as usize]
            .linked_cards
            .iter()
            .map(|c| c.handle())
            .filter(|c| !unqualified.contains(c))
            .collect();
        if excess >= remaining.len() {
            self.trash_link_cards_by_rule(host, &remaining);
            return !remaining.is_empty();
        }
        self.install_link_card_trash_choice(LinkCardTrashChoiceState {
            chooser: host.player,
            host,
            remaining,
            picks_left: excess,
            chosen: Vec::new(),
            purpose: LinkTrashPurpose::RuleExcess,
            outer_conts: Vec::new(),
        })
    }

    /// Park one pick of a link-card trash choice: an `EffectChoice` over
    /// `remaining` (each entry carries its card, so the choice reaches the RL
    /// action space), resumed by the `LinkCardTrashChoice` frame. Returns
    /// `false` (nothing installed) when the host is gone or has no stack.
    fn install_link_card_trash_choice(&mut self, state: LinkCardTrashChoiceState) -> bool {
        use crate::action::space::{HAND_EFFECT_START, HAND_MAIN_LIMIT};
        let Some(host_card) = self
            .player(state.host.player)
            .battle_area
            .get(state.host.index as usize)
            .filter(|p| !p.card_sources.is_empty())
            .map(|p| p.top_card().handle())
        else {
            return false;
        };
        let mut valid_action_ids = Vec::new();
        let mut choices = Vec::new();
        for (i, card) in state.remaining.iter().take(HAND_MAIN_LIMIT).enumerate() {
            let action_id = HAND_EFFECT_START + i as u16;
            valid_action_ids.push(action_id);
            choices.push(EffectChoiceEntry {
                count_choice: None,
                label: self
                    .card_data_for_handle(*card)
                    .map(|d| d.card_name.clone())
                    .unwrap_or_else(|| "Link card".to_string()),
                action_id,
                source_card: Some(*card),
                source_kind: Some(EffectSourceKind::Rule),
                timing: None,
                is_optional: false,
                keyword: None,
                observation_metadata: Default::default(),
            });
        }
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: state.chooser,
            previous_phase,
            valid_action_ids,
            is_optional: false,
            prompt: format!("Choose a link card to trash ({} more)", state.picks_left),
            effect_choices: Some(choices),
            source_card: host_card,
            source_permanent: Some(state.host),
            source_kind: EffectSourceKind::Rule,
            callback: Box::new(|_, _| {
                panic!("link-card trash choice requires its typed resume frame")
            }),
            on_decline: None,
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::LinkCardTrashChoice(state)],
        });
        true
    }

    /// Resume a link-card trash choice: record the pick, ask again until the
    /// count is met (when every remaining card must go there is no choice),
    /// then trash every pick together and finish `purpose`.
    pub(crate) fn run_link_card_trash_choice_step(
        &mut self,
        mut state: LinkCardTrashChoiceState,
        action_id: u16,
    ) {
        let idx = action_id.saturating_sub(crate::action::space::HAND_EFFECT_START) as usize;
        if idx < state.remaining.len() {
            state.chosen.push(state.remaining.remove(idx));
            state.picks_left = state.picks_left.saturating_sub(1);
        }
        if state.picks_left > 0 && !state.remaining.is_empty() {
            if state.picks_left < state.remaining.len()
                && self.install_link_card_trash_choice(state.clone())
            {
                return;
            }
            state.chosen.append(&mut state.remaining);
        }
        let outer_conts = std::mem::take(&mut state.outer_conts);
        self.trash_link_cards_by_rule(state.host, &state.chosen);
        match state.purpose {
            LinkTrashPurpose::RuleExcess => {}
            LinkTrashPurpose::DigimonLink { origin, host, card } => {
                self.finish_digimon_link_attach(origin, host, card);
            }
            LinkTrashPurpose::OptionLink(resume) => self.commit_linked_card_no_replace(resume),
            LinkTrashPurpose::EffectLink(resume) => {
                crate::dsl_cards::step::link_cards::resume_effect_link(self, *resume);
            }
            LinkTrashPurpose::RelinkStanding { source, host } => {
                self.absorb_standing_digimon_as_link(source, host);
            }
            LinkTrashPurpose::RelinkPlugIn {
                option,
                carrier,
                link_index,
            } => {
                // The picks shortened the carrier's link cards.
                let len = self
                    .player(carrier.player)
                    .battle_area
                    .get(carrier.index as usize)
                    .map_or(0, |p| p.linked_cards.len());
                let _ = self.relink_plug_in(option, carrier, link_index.min(len as u8));
            }
        }
        crate::dsl_cards::step::selections::run_outer_conts(self, outer_conts);
    }
}
