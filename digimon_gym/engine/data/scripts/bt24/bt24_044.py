from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_044(CardScript):
    """BT24-044 Muchomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] You may suspend 1 level 6 or lower Digimon. If this effect suspended your Digimon, reveal the top 3 cards of your deck. Add 1 [Shoto Kazama] and 1 card with [Avian] or [Bird] in any of its traits or the [Vortex Warriors] trait among them to the hand. Return the rest to the bottom of the deck.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-044 Suspend 1 Digimon, then maybe search top 3.")
        effect0.set_effect_description("[On Play] You may suspend 1 level 6 or lower Digimon. If this effect suspended your Digimon, reveal the top 3 cards of your deck. Add 1 [Shoto Kazama] and 1 card with [Avian] or [Bird] in any of its traits or the [Vortex Warriors] trait among them to the hand. Return the rest to the bottom of the deck.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
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
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if p.level is None or p.level > 6:
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def reveal_filter(c):
                if not (any('Shoto Kazama' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                if getattr(c, 'level', None) is None or c.level > 6:
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

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

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
