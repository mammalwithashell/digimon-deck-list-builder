from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_101(CardScript):
    """BT14-101 WarGreymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # While you have a Tamer with [Tai Kamiya] in its name and your opponent has
        # a Digimon with 10000 DP or more, 1 of your [Agumon] may digivolve into this
        # card in your hand for a cost of 4, ignoring its digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-101 Alternate digivolution requirement")
        effect0.set_effect_description("While you have a Tamer with [Tai Kamiya] in its name and your opponent has a Digimon with 10000 DP or more, 1 of your [Agumon] may digivolve into this card in your hand for a cost of 4, ignoring its digivolution requirements.")
        effect0._alt_digi_cost = 4

        def condition0(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            opponent = context.get('opponent')
            permanent = context.get('permanent')

            if permanent is None:
                return False

            # Source must be Agumon
            try:
                source_name = permanent.get_name()
            except Exception:
                source_name = ""
            if source_name != 'Agumon':
                return False

            # You must have a Tamer with Tai Kamiya in name
            has_tai = False
            if player is not None:
                try:
                    for p in player.get_tamers():
                        try:
                            name = p.get_name()
                        except Exception:
                            name = ''
                        if 'Tai Kamiya' in name:
                            has_tai = True
                            break
                except Exception:
                    has_tai = False
            if not has_tai:
                return False

            # Opponent must have a Digimon with 10000+ DP
            has_10k = False
            if opponent is not None:
                try:
                    for p in opponent.get_battle_area():
                        try:
                            dp = p.get_dp()
                        except Exception:
                            dp = 0
                        if dp >= 10000:
                            has_10k = True
                            break
                except Exception:
                    has_10k = False

            return has_10k

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] This Digimon gains <Raid> for the turn. Then, it may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-101 This Digimon gains Raid and can attack")
        effect2.set_effect_description("[When Digivolving] This Digimon gains <Raid> for the turn. Then, it may attack.")
        effect2.is_when_digivolving = True
        effect2._is_raid = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Raid, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_raid')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If you have a Tamer, this Digimon gains Security A. +1 and <Piercing> for the turn.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-101 Gain Keyword Piercing")
        effect3.set_effect_description("[When Attacking] If you have a Tamer, this Digimon gains Security A. +1 and <Piercing> for the turn.")
        effect3.is_on_attack = True
        effect3._is_piercing = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain Keyword Piercing"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_piercing')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
