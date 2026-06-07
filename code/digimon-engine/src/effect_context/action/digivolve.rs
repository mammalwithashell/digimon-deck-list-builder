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

        let turn = self.game.turn_count;
        let card = self.game.player_mut(self.player).hand.remove(hand_index);
        self.game.player_mut(target.player).battle_area[target.index as usize]
            .digivolve(card, turn);

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
        self.game.de_digivolve_core(target, stop_at_level, amount)
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

        // Snapshot the partner/target predicates for the chained closures.
        let partner_filter_for_install = std::sync::Arc::clone(&partner_filter);
        let target_filter_for_inner = std::sync::Arc::clone(&target_filter);

        let partner_prompt = partner_prompt
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Choose a DNA digivolve partner".to_string());
        let target_prompt = target_prompt
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Choose a Digimon card from hand to DNA digivolve into".to_string());

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
                    move |g, i| target_filter_for_inner(g, i),
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
                        ctx.effect_initiated_dna_digivolve(
                            anchor,
                            partner,
                            card,
                            cost as i32,
                            ignore_requirements,
                        );
                    },
                );
            },
        );
    }
}
