from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_082(CardScript):
    """Auto-transpiled from DCGO BT24_082.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Owen Dreadnought] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Elizamon] from your trash without paying the cost.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-082 By bottom decking, Play 1 [Owen Dreadnought] from hand. Then if you have no digimon, Play 1 [Elizamon] from trash")
        effect0.set_effect_description("[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Owen Dreadnought] from your hand without paying the cost. Then, if you don't have a Digimon, you may play 1 [Elizamon] from your trash without paying the cost.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] trait Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-082 +3k and may attack")
        effect1.set_effect_description("[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] trait Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack.")
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if perm:
                perm.change_dp(3000)
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-082 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
