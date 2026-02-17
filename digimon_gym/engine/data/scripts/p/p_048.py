from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_048(CardScript):
    """P-048 UlforceVeedramon Zero | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place 3 non-Digi-Egg cards from your trash at the bottom of your deck in any order to unsuspend this Digimon and 1 of your Tamers.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-048 Return cards from trash to unsuspend this Digimon and your 1 Tamer")
        effect0.set_effect_description("[When Digivolving] You may place 3 non-Digi-Egg cards from your trash at the bottom of your deck in any order to unsuspend this Digimon and 1 of your Tamers.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Unsuspend, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=False)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnReturnCardsToLibraryFromTrash
        # [Your Turn][Once Per Turn] When a card is returned from your trash to your deck, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-048 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When a card is returned from your trash to your deck, gain 1 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory_P_048")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
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
