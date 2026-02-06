from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_087(CardScript):
    """Auto-transpiled from DCGO BT24_087.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # Gain 1 memory (Tamer)
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-087 Gain 1 memory (Tamer)")
        effect0.set_effect_description("Gain 1 memory (Tamer)")
        # [Start of Main] Gain 1 memory if opponent has Digimon
        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.WhenLinked
        # [Your Turn] When any of your Digimon get linked, by suspending this Tamer, <Draw 1> and trash 1 card in your hand. Then, 1 of your Digimon may app fuse into a Digimon card with the [System], [Life] or [Transmutation] trait in the trash.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-087 Draw 1, Trash 1, then you may app fuse")
        effect1.set_effect_description("[Your Turn] When any of your Digimon get linked, by suspending this Tamer, <Draw 1> and trash 1 card in your hand. Then, 1 of your Digimon may app fuse into a Digimon card with the [System], [Life] or [Transmutation] trait in the trash.")
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1, Suspend, Play Card, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.draw_cards(1)
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()
            # Play a card (from hand/trash/reveal)
            pass  # TODO: target selection for play_card
            # Trash from hand (cost/effect)
            if player and player.hand_cards:
                player.trash_from_hand([player.hand_cards[-1]])

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-087 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True
        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
