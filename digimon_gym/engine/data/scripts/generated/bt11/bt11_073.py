from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_073(CardScript):
    """BT11-073 Justimon: Accel Arm | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 1
        effect0._alt_digi_cost = 1

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By returning 1 level 6 Digimon card without [Justimon: Accel Arm] in its name from this Digimon's digivolution cards to its owner's hand, this Digimon gains <Security Attack +1> and <Piercing> for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-073 Return 1 digivolution card to get effects")
        effect1.set_effect_description("[When Digivolving] By returning 1 level 6 Digimon card without [Justimon: Accel Arm] in its name from this Digimon's digivolution cards to its owner's hand, this Digimon gains <Security Attack +1> and <Piercing> for the turn.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True
        effect1._is_piercing = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Hand, Gain Keyword Piercing"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if perm:
                perm.grant_keyword('_is_piercing')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] If you have a Tamer in play, this Digimon can digivolve into a Digimon card with [Justimon] in its name other than [Justimon: Accel Arm] in your hand for a digivolution cost of 2, ignoring its digivolution requirements.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-073 This Digimon digivolves into a card with [Justimon] in its name")
        effect2.set_effect_description("[When Attacking] If you have a Tamer in play, this Digimon can digivolve into a Digimon card with [Justimon] in its name other than [Justimon: Accel Arm] in your hand for a digivolution cost of 2, ignoring its digivolution requirements.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Justimon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
