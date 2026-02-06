from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_044(CardScript):
    """Auto-transpiled from DCGO BT24_044.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may suspend 1 level 6 or lower Digimon. If this effect suspended your Digimon, reveal the top 3 cards of your deck. Add 1 [Shoto Kazama] and 1 card with [Avian] or [Bird] in any of its traits or the [Vortex Warriors] trait among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-044 Suspend 1 Digimon, then maybe search top 3.")
        effect0.set_effect_description("[On Play] You may suspend 1 level 6 or lower Digimon. If this effect suspended your Digimon, reveal the top 3 cards of your deck. Add 1 [Shoto Kazama] and 1 card with [Avian] or [Bird] in any of its traits or the [Vortex Warriors] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Suspend, Add To Hand, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            # Suspend opponent's digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                target = enemy.battle_area[-1]
                target.suspend()
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            # Reveal top cards and select
            pass  # TODO: reveal_and_select needs UI/agent choice

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-044 Memory +1")
        effect1.set_effect_description("[All Turns] [Once Per Turn] When this Digimon deletes your opponent's Digimon in battle, gain 1 memory.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("BT24_044_Inherited")

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
