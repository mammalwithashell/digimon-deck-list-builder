from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_027(CardScript):
    """BT14-027 MarineDevimon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Return all level 3 Digimon to their owner's hands.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-027 Return all level 3 Digimons to hand")
        effect0.set_effect_description("[On Play] Return all level 3 Digimon to their owner's hands.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return

            players = [player]
            if getattr(player, 'enemy', None):
                players.append(player.enemy)

            for owner in players:
                board = getattr(owner, 'battle_area', None)
                if not board:
                    continue
                for perm in list(board):
                    if getattr(perm, 'card_level', None) == 3:
                        owner.bounce_permanent_to_hand(perm)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Return all level 3 Digimon to their owner's hands.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-027 Return all level 3 Digimons to hand")
        effect1.set_effect_description("[When Digivolving] Return all level 3 Digimon to their owner's hands.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return

            players = [player]
            if getattr(player, 'enemy', None):
                players.append(player.enemy)

            for owner in players:
                board = getattr(owner, 'battle_area', None)
                if not board:
                    continue
                for perm in list(board):
                    if getattr(perm, 'card_level', None) == 3:
                        owner.bounce_permanent_to_hand(perm)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
