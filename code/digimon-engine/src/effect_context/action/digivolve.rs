//! Digivolve / DNA-digivolve mutations on `EffectContext` — extracted by mechanic.

#![allow(unused_imports)]
use crate::action::mask::*;
use crate::action::space::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::dsl_cards::bindings::Bindings;
use crate::dsl_cards::step::StepRuntime;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;

impl<'a> EffectContext<'a> {
    pub fn digivolve_replacement_subject_without_cost(
        &mut self,
        subject: ReplacementSubject,
        card: CardHandle,
    ) -> bool {
        let Some(target) = subject.permanent() else {
            return false;
        };
        if (target.index as usize) >= self.game.player(target.player).battle_area.len() {
            return false;
        }

        let Some(hand_index) = self
            .game
            .player(self.player)
            .hand
            .iter()
            .position(|source| source.handle() == card)
        else {
            return false;
        };

        let card = self.game.player_mut(self.player).hand.remove(hand_index);
        self.game.digivolve_permanent_in_place(target, card);

        self.game.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            crate::selection::TriggerSource::Permanent(target),
        );
        self.game.drain_effect_queue();

        for pid in 0..self.game.players.len() {
            self.game.enqueue_triggered(
                EffectTiming::OnDigivolve,
                crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
            );
        }
        self.game.drain_effect_queue();
        true
    }

    /// Pop up to `amount` cards off `target`'s digivolution stack,
    /// trashing each popped source into the target owner's trash.
    ///
    /// Rules:
    ///   * Never pops the base card — `Permanent` must always retain at
    ///     least one `CardSource`.
    ///   * `stop_at_level = Some(L)` — stop early if popping would leave
    ///     a top whose level is strictly less than `L`. For standard
    ///     De-Digivolve N use `Some(3)` (card text: "You can't trash
    ///     past level 3 cards").
    ///   * `stop_at_level = None` — no level floor; pop until the base.
    ///   * `amount = Some(N)` — cap pops at N.
    ///   * `amount = None` — unbounded (equivalent to `Some(u8::MAX)`).
    ///
    /// Returns the actual number of cards popped.
    /// De-digivolve `target` (pop digivolution sources). Thin facade over the
    /// Tier-2 rules machinery in `Game::de_digivolve_core`: applies the
    /// effect-only `can_affect_permanent` guard, then delegates. The pop-loop
    /// and `WhenWouldBeDeDigivolved` replacement window live in Tier 2 (placement
    /// rule §engine-effect-context-layering) — the facade keeps only the guard.
    pub fn de_digivolve(
        &mut self,
        target: PermanentHandle,
        stop_at_level: Option<u8>,
        amount: Option<u8>,
    ) -> u8 {
        if !self.can_affect_permanent(target) {
            return 0;
        }
        // Per-pop immunity recheck (judge-quiz Q15): De-Digivolve is applied
        // one card at a time and a newly-exposed top card's continuous
        // immunity halts the remaining pops. The core also re-ticks
        // declarative effects after each pop so the entry guard of any
        // FOLLOW-UP de_digivolve call sees the refreshed registry.
        self.game.de_digivolve_core(
            target,
            stop_at_level,
            amount,
            Some((self.player, self.source_kind)),
        )
    }

    /// Install a player-scoped one-shot future-digivolve cost reducer
    /// (`G-COST-REDUCE-ALLY-DIGIVOLVE`).
    ///
    /// Used by BT3-103 Hidden Potential Discovered!'s `[Main]` clause:
    /// "For the turn, when one of your green Digimon would next digivolve,
    /// by suspending 1 of your Digimon, reduce the digivolution cost by 5."
    ///
    /// The reducer is pushed onto `Game::player_digivolve_cost_reducers`
    /// and is consulted at the top of each digivolve-from-hand cost path.
    /// `target_color` gates which digivolutions qualify; `single_fire`
    /// consumes the reducer on the first successful application; the
    /// reducer expires at end of the installing player's turn.
    ///
    /// When `suspend_cost` is `true`, applying the reduction prompts the
    /// player to suspend one of their own unsuspended Digimon — an
    /// interactive, player-visible cost surfaced through `pending_selection`
    /// (Working Rule §17). No auto-suspend.
    pub fn arm_player_digivolve_cost_reducer(
        &mut self,
        amount: i32,
        single_fire: bool,
        target_color: Option<crate::enums::CardColor>,
        suspend_cost: bool,
    ) {
        let reducer = crate::player_cost_reducer::PlayerDigivolveCostReducer {
            player: self.player,
            source_card: self.source_card,
            kind: crate::player_cost_reducer::PlayerCostReducerKind::Digivolve,
            expiry: crate::player_cost_reducer::PlayerCostReducerExpiry::EndOfTurn,
            amount,
            single_fire,
            target_color,
            suspend_cost,
        };
        self.game.player_digivolve_cost_reducers.push(reducer);
    }

    /// Digivolve a card from `player`'s hand at `hand_index` onto `target`
    /// by effect. Bypasses the Main-phase check; optionally ignores color
    /// requirements (`ignore_color=true`); pays memory via `cost_delta`.
    ///
    /// Returns `true` on success. See `Game::effect_initiated_digivolve`.
    pub fn effect_initiated_digivolve(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> bool {
        self.game.effect_initiated_digivolve(
            player,
            hand_index,
            target,
            cost_delta,
            ignore_color,
            PlaySource::ByEffect,
        )
    }

    pub fn effect_initiated_digivolve_ignore_requirements(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
    ) -> bool {
        self.game.effect_initiated_digivolve_ignore_requirements(
            player,
            hand_index,
            target,
            cost_delta,
            PlaySource::ByEffect,
        )
    }

    pub fn effect_initiated_digivolve_with_provenance(
        &mut self,
        player: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> Option<(PermanentHandle, crate::trigger_context::ProvenanceToken)> {
        let card = self.game.player(player).hand.get(hand_index)?.handle();
        let token = self.game.provenance_token_for_card(card);
        if self.effect_initiated_digivolve(player, hand_index, target, cost_delta, ignore_color) {
            Some((target, token))
        } else {
            None
        }
    }

    /// Digivolve a card from any supported source zone onto `target` by
    /// effect. See `Game::effect_initiated_digivolve_from_source`.
    pub fn effect_initiated_digivolve_from_source(
        &mut self,
        player: PlayerId,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
    ) -> bool {
        self.game.effect_initiated_digivolve_from_source(
            player,
            source,
            target,
            cost_delta,
            ignore_color,
            PlaySource::ByEffect,
        )
    }

    pub fn effect_initiated_digivolve_from_source_ignore_requirements(
        &mut self,
        player: PlayerId,
        source: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
    ) -> bool {
        self.game
            .effect_initiated_digivolve_from_source_ignore_requirements(
                player,
                source,
                target,
                cost_delta,
                PlaySource::ByEffect,
            )
    }

    /// Merge two existing battle-area permanents into a single permanent
    /// topped with a card from hand. Effect-initiated DNA digivolve.
    ///
    /// Delegates to `Game::dna_digivolve_inner` for the merge + triggers.
    /// This wrapper handles the IR's two-knob shape (`cost: i32` separate
    /// from `ignore_requirements: bool`) and the pay-memory-bypass branch
    /// that fires when `ignore_requirements` is set and the printed cost
    /// would otherwise dip below the memory floor.
    ///
    /// ## Stacking order
    ///
    /// `target_a.card_sources ++ target_b.card_sources ++ [from_hand]`.
    /// `target_a` corresponds to `DnaCost::requirement1`. See
    /// `Game::dna_digivolve_inner` for the canonical contract.
    ///
    /// ## Triggers
    ///
    /// `WhenDigivolving` → `OnDnaDigivolve` → `OnDigivolve` (global),
    /// each followed by a queue drain. See
    /// `Game::dna_digivolve_inner` for the firing sequence.
    ///
    /// ## Semantics of `ignore_requirements`
    ///
    /// `ignore_requirements: true` skips the affordability floor — i.e. the
    /// merge runs even when subtracting `cost` from memory would dip below
    /// `rules.memory_range.0`. The `cost` argument is still subtracted —
    /// `ignore_requirements` is not the same as "free". For
    /// `cost: 0, ignore_requirements: true`, no memory mutation occurs.
    ///
    /// ## Defensive validation
    ///
    /// Returns `None` if:
    /// - `target_a == target_b`
    /// - either target's index is out of range on its player's battle area
    /// - `from_hand` is not present in any player's hand
    /// - `cost > 0` and `!ignore_requirements` and the controller cannot
    ///   pay the memory cost (early-out before any state mutation)
    pub fn effect_initiated_dna_digivolve(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<PermanentHandle> {
        if target_a == target_b {
            return None;
        }
        if (target_a.index as usize) >= self.game.player(target_a.player).battle_area.len() {
            return None;
        }
        if (target_b.index as usize) >= self.game.player(target_b.player).battle_area.len() {
            return None;
        }

        // Locate the from_hand card across all players' hands.
        let mut hand_owner: Option<PlayerId> = None;
        let mut hand_index: Option<usize> = None;
        for pid in 0..self.game.players.len() {
            if let Some(idx) = self.game.players[pid]
                .hand
                .iter()
                .position(|c| c.handle() == from_hand)
            {
                hand_owner = Some(pid as PlayerId);
                hand_index = Some(idx);
                break;
            }
        }
        let (hand_owner, hand_index) = (hand_owner?, hand_index?);
        if self
            .game
            .modifiers
            .player_has(hand_owner, ModifierType::CannotDigivolveDigimonByEffect)
        {
            return None;
        }

        // G-ENGINE-DNA-RECIPE-ENFORCEMENT (gap 2) — commit-time backstop. Unless
        // the caller explicitly ignores requirements (DCGO cards that skip the
        // jogressCondition), the {target_a, target_b} pair MUST satisfy the
        // result's printed DNA recipe. An illegal pairing is rejected here with
        // no state mutation. Mirrors DCGO `CanJogressFromTargetPermanents`
        // guarding `PlayCardClass.PlayCard`.
        if !ignore_requirements
            && !self.dna_pair_satisfies_recipe(target_a, target_b, hand_owner, hand_index)
        {
            return None;
        }

        let effective_cost: u16 = cost.max(0) as u16;

        // Memory: under ignore_requirements bypass the floor; otherwise let
        // dna_digivolve_inner pay normally.
        if ignore_requirements && effective_cost > 0 {
            self.game.pay_memory_unchecked(effective_cost);
            // Pass cost=0 to the inner so it doesn't double-pay.
            self.game
                .dna_digivolve_inner(target_a, target_b, hand_owner, hand_index, 0, false, true)
        } else {
            self.game.dna_digivolve_inner(
                target_a,
                target_b,
                hand_owner,
                hand_index,
                effective_cost,
                false,
                true,
            )
        }
    }

    /// Recipe oracle for the both-on-field DNA verb: do `target_a` and
    /// `target_b` (battle-area permanents) satisfy the printed DNA recipe of
    /// the result at `hand_owner`'s `hand_index`? DCGO
    /// `CanJogressFromTargetPermanents`.
    fn dna_pair_satisfies_recipe(
        &self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        hand_owner: PlayerId,
        hand_index: usize,
    ) -> bool {
        let Some(result) = self.game.player(hand_owner).hand.get(hand_index) else {
            return false;
        };
        let Some(meta) = self.game.card_data.get(result.data_index) else {
            return false;
        };
        let Some(perm_a) = self
            .game
            .player(target_a.player)
            .battle_area
            .get(target_a.index as usize)
        else {
            return false;
        };
        let Some(perm_b) = self
            .game
            .player(target_b.player)
            .battle_area
            .get(target_b.index as usize)
        else {
            return false;
        };
        crate::dna_digivolve::matching_dna_cost(meta, perm_a, perm_b, &self.game.card_data).is_some()
    }

    /// The printed DNA-digivolve memory cost the both-on-field pair `{target_a,
    /// target_b}` would pay to DNA-digivolve into the hand card `from_hand`, or
    /// `None` if the pair does not satisfy any of that card's printed DNA
    /// requirements. This is what the DSL `cost: printed` lowering computes.
    /// DCGO `condition.cost`.
    pub fn printed_dna_cost_for_pair(
        &self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        from_hand: CardHandle,
    ) -> Option<i32> {
        // Locate the result card + its hand owner.
        for pid in 0..self.game.players.len() {
            let player_id = pid as PlayerId;
            if let Some(result) = self
                .game
                .player(player_id)
                .hand
                .iter()
                .find(|c| c.handle() == from_hand)
            {
                let meta = self.game.card_data.get(result.data_index)?;
                let perm_a = self
                    .game
                    .player(target_a.player)
                    .battle_area
                    .get(target_a.index as usize)?;
                let perm_b = self
                    .game
                    .player(target_b.player)
                    .battle_area
                    .get(target_b.index as usize)?;
                return crate::dna_digivolve::matching_dna_cost(
                    meta,
                    perm_a,
                    perm_b,
                    &self.game.card_data,
                )
                .map(|c| c.memory_cost as i32);
            }
        }
        None
    }

    /// Effect-initiated DNA digivolve where ONE material is a battle-area
    /// permanent (`target`) and the OTHER material is a card in hand
    /// (`hand_partner`). The merged permanent is topped with `result_from_hand`
    /// (also a hand card — the Omnimon-name result).
    ///
    /// This is the BT17-095 Clause B shape: "That Digimon and a card in the
    /// hand may DNA digivolve into a Digimon card with [Omnimon] in its name
    /// in the hand." `effect_initiated_dna_digivolve` cannot express it — that
    /// verb requires BOTH DNA materials to be on-field permanents. See
    /// G-DSL-DNA-FROM-HAND-PARTNER.
    ///
    /// ## Stacking order
    ///
    /// `target.card_sources ++ [hand_partner] ++ [result_from_hand]`.
    ///
    /// ## Triggers
    ///
    /// `WhenDigivolving` → `OnDnaDigivolve` → `OnDigivolve` (global), each
    /// followed by a queue drain, all carrying the `dna_origin` marker — the
    /// same firing sequence as `effect_initiated_dna_digivolve`.
    ///
    /// ## Defensive validation
    ///
    /// Returns `None` if:
    /// - `target`'s index is out of range on its player's battle area,
    /// - `hand_partner` and `result_from_hand` are not both in the SAME
    ///   player's hand (they must share a hand owner),
    /// - `hand_partner == result_from_hand`,
    /// - the hand owner has `CannotDigivolveDigimonByEffect`,
    /// - `cost > 0` and `!ignore_requirements` and the controller cannot pay
    ///   the memory cost.
    ///
    /// `ignore_requirements` bypasses the memory affordability floor exactly
    /// as in `effect_initiated_dna_digivolve` (the cost is still subtracted).
    pub fn effect_initiated_dna_digivolve_with_hand_partner(
        &mut self,
        target: PermanentHandle,
        hand_partner: CardHandle,
        result_from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<PermanentHandle> {
        if hand_partner == result_from_hand {
            return None;
        }
        // The leaving subject may be parked in the DigiXros leaving/limbo slot
        // (G-DIGIXROS-REDIRECT-EXTRACTION). Re-materialize it into `battle_area`
        // first so the merge operates on a real permanent — this is the EXTRACT
        // that pulls the material out of the in-flight DigiXros transaction.
        let target = if crate::digixros::is_limbo_index(target.index) {
            self.game.rematerialize_digixros_limbo(target)?
        } else {
            target
        };
        if (target.index as usize) >= self.game.player(target.player).battle_area.len() {
            return None;
        }

        // Both hand cards must live in the SAME player's hand; locate it.
        let mut hand_owner: Option<PlayerId> = None;
        let mut partner_index: Option<usize> = None;
        let mut result_index: Option<usize> = None;
        for pid in 0..self.game.players.len() {
            let hand = &self.game.players[pid].hand;
            let p = hand.iter().position(|c| c.handle() == hand_partner);
            let r = hand.iter().position(|c| c.handle() == result_from_hand);
            if let (Some(p), Some(r)) = (p, r) {
                hand_owner = Some(pid as PlayerId);
                partner_index = Some(p);
                result_index = Some(r);
                break;
            }
        }
        let (hand_owner, partner_index, result_index) =
            (hand_owner?, partner_index?, result_index?);

        if self
            .game
            .modifiers
            .player_has(hand_owner, ModifierType::CannotDigivolveDigimonByEffect)
        {
            return None;
        }

        // G-ENGINE-DNA-RECIPE-ENFORCEMENT (gap 2) — commit-time backstop for the
        // hand-partner shape. Unless requirements are explicitly ignored, the
        // {field target, hand partner} pair MUST satisfy the result's printed
        // DNA recipe. DCGO `CanJogressFromTargetPermanent` after the partner is
        // materialised.
        if !ignore_requirements
            && !self.field_and_card_satisfy_recipe(target, hand_owner, partner_index, result_index)
        {
            return None;
        }

        let effective_cost: u16 = cost.max(0) as u16;

        if ignore_requirements && effective_cost > 0 {
            self.game.pay_memory_unchecked(effective_cost);
            self.game.dna_digivolve_hand_partner_inner(
                target,
                hand_owner,
                partner_index,
                result_index,
                0,
                true,
            )
        } else {
            self.game.dna_digivolve_hand_partner_inner(
                target,
                hand_owner,
                partner_index,
                result_index,
                effective_cost,
                true,
            )
        }
    }

    /// Recipe oracle for the field+card (hand or trash) DNA shape: does the
    /// battle-area permanent `field` plus the card at `card_owner`'s
    /// `card_index` (in `zone`) satisfy the printed DNA recipe of the result at
    /// `card_owner`'s `result_index` (a hand card)? DCGO
    /// `CanJogressFromTargetPermanent`.
    fn field_and_card_satisfy_recipe(
        &self,
        field: PermanentHandle,
        card_owner: PlayerId,
        partner_hand_index: usize,
        result_hand_index: usize,
    ) -> bool {
        let hand = &self.game.player(card_owner).hand;
        let Some(result) = hand.get(result_hand_index) else {
            return false;
        };
        let Some(partner) = hand.get(partner_hand_index) else {
            return false;
        };
        let Some(result_meta) = self.game.card_data.get(result.data_index) else {
            return false;
        };
        let Some(partner_meta) = self.game.card_data.get(partner.data_index) else {
            return false;
        };
        let Some(field_perm) = self
            .game
            .player(field.player)
            .battle_area
            .get(field.index as usize)
        else {
            return false;
        };
        crate::dna_digivolve::matching_dna_cost_perm_and_card(
            result_meta,
            field_perm,
            partner_meta,
            &self.game.card_data,
        )
        .is_some()
    }

    /// The printed DNA cost the {field target, hand partner} pair would pay to
    /// DNA-digivolve into `result_from_hand`. What the DSL `cost: printed`
    /// lowering computes for the hand-partner verb. DCGO `condition.cost`.
    pub fn printed_dna_cost_for_hand_partner(
        &self,
        target: PermanentHandle,
        hand_partner: CardHandle,
        result_from_hand: CardHandle,
    ) -> Option<i32> {
        // Both cards live in the SAME player's hand; find that player.
        for pid in 0..self.game.players.len() {
            let player_id = pid as PlayerId;
            let hand = &self.game.player(player_id).hand;
            let partner = hand.iter().find(|c| c.handle() == hand_partner);
            let result = hand.iter().find(|c| c.handle() == result_from_hand);
            if let (Some(partner), Some(result)) = (partner, result) {
                let result_meta = self.game.card_data.get(result.data_index)?;
                let partner_meta = self.game.card_data.get(partner.data_index)?;
                let field_perm = self
                    .game
                    .player(target.player)
                    .battle_area
                    .get(target.index as usize)?;
                return crate::dna_digivolve::matching_dna_cost_perm_and_card(
                    result_meta,
                    field_perm,
                    partner_meta,
                    &self.game.card_data,
                )
                .map(|c| c.memory_cost as i32);
            }
        }
        None
    }

    pub fn effect_initiated_dna_digivolve_with_provenance(
        &mut self,
        target_a: PermanentHandle,
        target_b: PermanentHandle,
        from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<(PermanentHandle, crate::trigger_context::ProvenanceToken)> {
        let token = self.game.provenance_token_for_card(from_hand);
        let permanent = self.effect_initiated_dna_digivolve(
            target_a,
            target_b,
            from_hand,
            cost,
            ignore_requirements,
        )?;
        Some((permanent, token))
    }

    /// G-ENGINE-DNA-TRASH-MATERIAL (gap 3) — effect-initiated DNA digivolve
    /// where ONE material is a battle-area permanent (`field_partner`), the
    /// OTHER material lives in the controller's **trash** (`trash_partner`),
    /// and the merged permanent is topped with `result_from_hand` (a **hand**
    /// card). BT18-015 / BT18-073 `[On Deletion]` shape.
    ///
    /// DCGO materialises the trash material via `CreateNewPermanent` (a pure
    /// placement that fires NO `[On Play]` / `OnEnterField`) then jogress-merges
    /// with `payCost: true`. We reproduce that observer-firing surface exactly:
    /// the trash material moves straight into the merged stack (never an
    /// independently-played permanent), so only the merged TOP's DNA triggers
    /// fire — see `Game::dna_digivolve_trash_partner_inner`.
    ///
    /// Composes with gaps 1+2: unless `ignore_requirements`, the {field, trash}
    /// pair must satisfy the result's printed DNA recipe (else `None`), and the
    /// caller passes the result's printed DNA cost (via
    /// `printed_dna_cost_for_field_trash_pair`).
    ///
    /// ## Defensive validation
    ///
    /// Returns `None` if the field target is out of range, `trash_partner` /
    /// `result_from_hand` are not both in the SAME player's zones (trash / hand
    /// respectively), the owner has `CannotDigivolveDigimonByEffect`, the recipe
    /// is unsatisfied (and requirements not ignored), or `cost > 0` and the
    /// controller cannot pay.
    pub fn effect_initiated_dna_digivolve_trash_partner(
        &mut self,
        field_partner: PermanentHandle,
        trash_partner: CardHandle,
        result_from_hand: CardHandle,
        cost: i32,
        ignore_requirements: bool,
    ) -> Option<PermanentHandle> {
        if (field_partner.index as usize)
            >= self.game.player(field_partner.player).battle_area.len()
        {
            return None;
        }

        // The trash material and the hand result must belong to the SAME player.
        let mut owner: Option<PlayerId> = None;
        let mut trash_index: Option<usize> = None;
        let mut result_index: Option<usize> = None;
        for pid in 0..self.game.players.len() {
            let player = &self.game.players[pid];
            let t = player
                .trash
                .iter()
                .position(|c| c.handle() == trash_partner);
            let r = player
                .hand
                .iter()
                .position(|c| c.handle() == result_from_hand);
            if let (Some(t), Some(r)) = (t, r) {
                owner = Some(pid as PlayerId);
                trash_index = Some(t);
                result_index = Some(r);
                break;
            }
        }
        let (owner, trash_index, result_index) = (owner?, trash_index?, result_index?);

        if self
            .game
            .modifiers
            .player_has(owner, ModifierType::CannotDigivolveDigimonByEffect)
        {
            return None;
        }

        // Recipe backstop (gap 2): the {field, trash} pair must satisfy the
        // result's printed DNA recipe unless requirements are ignored.
        if !ignore_requirements
            && !self.field_and_trash_satisfy_recipe(
                field_partner,
                owner,
                trash_index,
                result_index,
            )
        {
            return None;
        }

        let effective_cost: u16 = cost.max(0) as u16;

        if ignore_requirements && effective_cost > 0 {
            self.game.pay_memory_unchecked(effective_cost);
            self.game.dna_digivolve_trash_partner_inner(
                field_partner,
                owner,
                trash_index,
                result_index,
                0,
                true,
            )
        } else {
            self.game.dna_digivolve_trash_partner_inner(
                field_partner,
                owner,
                trash_index,
                result_index,
                effective_cost,
                true,
            )
        }
    }

    /// Recipe oracle for the field+trash DNA shape (gap 3): does `field` plus
    /// the trash card at `owner`'s `trash_index` satisfy the printed DNA recipe
    /// of the result at `owner`'s `result_index` (a hand card)?
    fn field_and_trash_satisfy_recipe(
        &self,
        field: PermanentHandle,
        owner: PlayerId,
        trash_index: usize,
        result_hand_index: usize,
    ) -> bool {
        let player = self.game.player(owner);
        let Some(result) = player.hand.get(result_hand_index) else {
            return false;
        };
        let Some(trash_material) = player.trash.get(trash_index) else {
            return false;
        };
        let Some(result_meta) = self.game.card_data.get(result.data_index) else {
            return false;
        };
        let Some(trash_meta) = self.game.card_data.get(trash_material.data_index) else {
            return false;
        };
        let Some(field_perm) = self
            .game
            .player(field.player)
            .battle_area
            .get(field.index as usize)
        else {
            return false;
        };
        crate::dna_digivolve::matching_dna_cost_perm_and_card(
            result_meta,
            field_perm,
            trash_meta,
            &self.game.card_data,
        )
        .is_some()
    }

    /// The printed DNA cost the {field target, trash partner} pair would pay to
    /// DNA-digivolve into `result_from_hand`. What the DSL `cost: printed`
    /// lowering computes for the trash-partner verb. DCGO `condition.cost`.
    pub fn printed_dna_cost_for_field_trash_pair(
        &self,
        field: PermanentHandle,
        trash_partner: CardHandle,
        result_from_hand: CardHandle,
    ) -> Option<i32> {
        for pid in 0..self.game.players.len() {
            let player_id = pid as PlayerId;
            let player = self.game.player(player_id);
            let trash_material = player
                .trash
                .iter()
                .find(|c| c.handle() == trash_partner);
            let result = player.hand.iter().find(|c| c.handle() == result_from_hand);
            if let (Some(trash_material), Some(result)) = (trash_material, result) {
                let result_meta = self.game.card_data.get(result.data_index)?;
                let trash_meta = self.game.card_data.get(trash_material.data_index)?;
                let field_perm = self
                    .game
                    .player(field.player)
                    .battle_area
                    .get(field.index as usize)?;
                return crate::dna_digivolve::matching_dna_cost_perm_and_card(
                    result_meta,
                    field_perm,
                    trash_meta,
                    &self.game.card_data,
                )
                .map(|c| c.memory_cost as i32);
            }
        }
        None
    }

    /// G-DSL-EOT-DNA-INLINE — surface an inline DNA digivolve choice at
    /// trigger fire time. Orchestrates the three-stage selection chain:
    /// (1) partner permanent from own field (anchor excluded), (2) target
    /// Digimon card from controller's hand, (3) call to the existing
    /// `effect_initiated_dna_digivolve` primitive.
    ///
    /// `anchor` is the source DNA material (typically the trigger source).
    /// The partner filter is re-wrapped internally to exclude the anchor
    /// handle, so callers need not encode that exclusion in the predicate.
    ///
    /// `optional` here is the eligibility "skip silently" gate — when
    /// either no eligible partner exists on own field OR no eligible target
    /// exists in hand, the step is a clean no-op regardless of this flag.
    /// The outer triggered clause's own `optional: true` provides the
    /// player-visible "may" via the trigger-order bundle. When `optional`
    /// is true, the partner selection prompt allows decline (the player
    /// can back out at the partner-pick stage).
    ///
    /// Backed by `effect_initiated_dna_digivolve`; carries identical
    /// trigger semantics (`WhenDigivolving → OnDnaDigivolve → OnDigivolve`
    /// with per-trigger drains).
    pub fn may_dna_digivolve_now(
        &mut self,
        anchor: PermanentHandle,
        partner_filter: std::sync::Arc<dyn Fn(&Game, PermanentHandle) -> bool + Send + Sync>,
        target_filter: std::sync::Arc<dyn Fn(&Game, usize) -> bool + Send + Sync>,
        cost: u16,
        ignore_requirements: bool,
        optional: bool,
        partner_prompt: Option<&str>,
        target_prompt: Option<&str>,
    ) {
        // Defensive: anchor must still be on its player's battle area.
        if (anchor.index as usize) >= self.game.player(anchor.player).battle_area.len() {
            return;
        }

        // Quick install-time eligibility checks. If either side has zero
        // candidates the step is a silent no-op (matches DCGO's
        // `CanActivateCondition` returning false).
        let controller = self.player;
        let has_partner = {
            let battle_len = self.game.player(controller).battle_area.len();
            (0..battle_len).any(|i| {
                let h = PermanentHandle {
                    player: controller,
                    index: i as u8,
                };
                h != anchor && partner_filter(self.game, h)
            })
        };
        if !has_partner {
            return;
        }
        let has_target = {
            let hand_len = self.game.player(controller).hand.len();
            (0..hand_len).any(|i| target_filter(self.game, i))
        };
        if !has_target {
            return;
        }
        let target_candidate_actions: Vec<u16> = {
            let hand_len = self.game.player(controller).hand.len();
            (0..hand_len)
                .filter(|&i| target_filter(self.game, i))
                .map(|i| PLAY_HAND_START + i as u16)
                .collect()
        };

        // Snapshot the partner/target predicates for the chained closures.
        let partner_filter_for_install = std::sync::Arc::clone(&partner_filter);
        let target_filter_for_inner = std::sync::Arc::clone(&target_filter);

        let partner_prompt = partner_prompt
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Choose a DNA digivolve partner".to_string());
        let target_prompt = target_prompt
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Choose a Digimon card from hand to DNA digivolve into".to_string());
        let target_prompt_for_resume = target_prompt.clone();
        let prov = crate::resume::ResumeProvenance {
            source_card: self.source_card,
            source_permanent: self.source_permanent,
            source_kind: self.source_kind,
            controller: self.player,
            override_pin: self.override_selecting_player(),
        };

        // Install partner selection. The anchor exclusion is enforced inline.
        self.select_own_permanent(
            &partner_prompt,
            optional,
            move |game, h| h != anchor && partner_filter_for_install(game, h),
            move |ctx, partner| {
                // Inner stage: install target hand selection.
                let target_filter_for_inner = std::sync::Arc::clone(&target_filter_for_inner);
                ctx.select_hand(
                    controller,
                    &target_prompt,
                    optional,
                    move |g, i| {
                        if !target_filter_for_inner(g, i) {
                            return false;
                        }
                        // Faithful path (DCGO `CanJogressFromTargetPermanent`,
                        // PayCost=true): the chosen hand card must be a LEGAL
                        // DNA-digivolve target for the {anchor, partner} pair —
                        // one of its printed DNA requirements satisfied by the two
                        // materials. Only the explicit `ignore_requirements: true`
                        // escape hatch skips this gate.
                        if ignore_requirements {
                            return true;
                        }
                        dna_pair_can_reach_hand_card(g, controller, anchor, partner, i)
                    },
                    move |ctx, hand_idx| {
                        // Final stage: resolve hand_idx to a CardHandle and
                        // delegate to the existing engine primitive.
                        let card = match ctx
                            .game
                            .player(controller)
                            .hand
                            .get(hand_idx)
                            .map(|c| c.handle())
                        {
                            Some(c) => c,
                            None => return,
                        };
                        // Faithful path pays the TARGET's printed DNA cost (DCGO
                        // `payCost: true` → `condition.cost`); `ignore_requirements`
                        // keeps the authored fixed `cost`.
                        let charge = if ignore_requirements {
                            cost as i32
                        } else {
                            dna_pair_cost_for_hand_card(
                                ctx.game, controller, anchor, partner, hand_idx,
                            )
                            .unwrap_or(cost as i32)
                        };
                        ctx.effect_initiated_dna_digivolve(
                            anchor,
                            partner,
                            card,
                            charge,
                            ignore_requirements,
                        );
                    },
                );
            },
        );
        if self.game.pending_selection.is_some() {
            self.game.pending_selection_resume = Some(crate::resume::ResumeStack {
                frames: vec![crate::resume::ResumeFrame::MayDnaPartnerSelection(
                    crate::resume::MayDnaPartnerSelectionState {
                        prov,
                        controller,
                        anchor,
                        cost,
                        ignore_requirements,
                        optional,
                        target_prompt: target_prompt_for_resume,
                        target_candidate_actions,
                        outer_conts: Vec::new(),
                    },
                )],
            });
        }
    }
}
