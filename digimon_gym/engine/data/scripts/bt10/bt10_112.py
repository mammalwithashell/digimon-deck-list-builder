from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class _DynamicSAEffect(ICardEffect):
    """ICardEffect subclass with dynamic _security_attack_modifier based on RK count in digi-stack."""
    def __init__(self, card_ref):
        super().__init__()
        self._card_ref = card_ref

    @property
    def _security_attack_modifier(self):
        _card = self._card_ref
        perm = _card.permanent_of_this_card() if _card else None
        if perm is None:
            return 0
        count = 0
        for cs in perm.card_sources:
            if cs is perm.top_card:
                continue
            traits = getattr(cs, 'card_traits', []) or []
            if any('Royal Knight' in tr for tr in traits):
                count += 1
        return count

    @_security_attack_modifier.setter
    def _security_attack_modifier(self, value):
        # Allow setting without error (used during initialization patterns)
        pass


class BT10_112(CardScript):
    """BT10-112 Jesmon GX | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-112 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 5
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 6
        effect0._alt_digi_trait = "Royal Knight"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 1 card with [Royal Knight] in its traits and a play cost of 13 or less from your hand or trash under this Digimon as its bottom digivolution card. Activate 1 of that Digimon's [When Digivolving] effects as an effect of this Digimon. Then, <Blitz>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT10-112 Place 1 card to digivolution cards to activate effects and Blitz")
        effect1.set_effect_description("[When Digivolving] You may place 1 card with [Royal Knight] in its traits and a play cost of 13 or less from your hand or trash under this Digimon as its bottom digivolution card. Activate 1 of that Digimon's [When Digivolving] effects as an effect of this Digimon. Then, <Blitz>.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Place 1 Royal Knight from hand/trash under self, activate WhenDigivolving, Blitz."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return

            from ....data.enums import EffectTiming as _ET

            def rk_filter(c):
                if not any('Royal Knight' in tr for tr in (getattr(c, 'card_traits', []) or [])):
                    return False
                pc = getattr(c, 'play_cost', 0) or 0
                return pc <= 13

            # Build valid action IDs from hand + trash
            from ....game.action_decoder import (
                SEL_HAND_START, SEL_TRASH_START, ACTION_SPACE_SIZE, GamePhase)
            valid = []
            for i, c in enumerate(player.hand_cards):
                if rk_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    valid.append(SEL_HAND_START + i)
            for i, c in enumerate(player.trash_cards):
                if rk_filter(c) and (SEL_TRASH_START + i) < ACTION_SPACE_SIZE:
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                # No valid targets — still grant Blitz
                perm.grant_keyword('_is_rush')
                return

            def on_select(action_id):
                selected_card = None
                if action_id >= SEL_TRASH_START:
                    idx = action_id - SEL_TRASH_START
                    if 0 <= idx < len(player.trash_cards):
                        selected_card = player.trash_cards[idx]
                        player.trash_cards.remove(selected_card)
                else:
                    idx = action_id - SEL_HAND_START
                    if 0 <= idx < len(player.hand_cards):
                        selected_card = player.hand_cards[idx]
                        player.hand_cards.remove(selected_card)
                if selected_card is None:
                    perm.grant_keyword('_is_rush')
                    return
                perm.add_card_source_bottom(selected_card)
                # Activate 1 of that card's [When Digivolving] effects
                wd_effects = []
                for eff in selected_card.effect_list(_ET.NoTiming):
                    if getattr(eff, '_is_when_digivolving', False):
                        wd_effects.append(eff)
                if wd_effects:
                    if len(wd_effects) == 1:
                        # Only one WD effect — activate it directly
                        wd_eff = wd_effects[0]
                        if wd_eff.on_process_callback:
                            wd_eff.on_process_callback({
                                'player': player, 'permanent': perm, 'game': game})
                    else:
                        # Multiple WD effects — let player choose
                        labels = [e.effect_name or e.effect_description for e in wd_effects]
                        def on_branch(branch_idx, _effs=wd_effects):
                            chosen = _effs[branch_idx]
                            if chosen.on_process_callback:
                                chosen.on_process_callback({
                                    'player': player, 'permanent': perm, 'game': game})
                        game.effect_choose_branch(
                            player, len(wd_effects), on_branch,
                            prompt="Activate 1 [When Digivolving] effect.",
                            branch_labels=labels)
                # Then, <Blitz>
                perm.grant_keyword('_is_rush')

            game.request_selection(
                GamePhase.SelectTarget, player, on_select, valid,
                is_optional=True,
                prompt="Select 1 [Royal Knight] card (cost 13 or less) from hand/trash.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT10-112 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: security_attack_plus (dynamic)
        # Security Attack +1 for each [Royal Knight] trait in digivolution cards
        effect3 = _DynamicSAEffect(card)
        effect3.set_effect_name("BT10-112 Security Attack +N (Royal Knight count)")
        effect3.set_effect_description("This Digimon gains <Security Attack +1> for each card with the [Royal Knight] trait in this Digimon's digivolution cards.")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Royal Knight' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
