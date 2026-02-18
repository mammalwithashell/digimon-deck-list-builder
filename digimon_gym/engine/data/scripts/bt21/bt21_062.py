from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_062(CardScript):
    """BT21-062 Galacticmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-062 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Snatchmon] for cost 9
        effect0._alt_digi_cost = 9
        effect0._alt_digi_name = "Snatchmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Snatchmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By placing 4 cards with [Vemmon] in their texts from your trash as this Digimon's bottom digivolution cards, you may use 1 [Ragnarok Cannon] from your hand or trash without paying the cost.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-062 Place 4 cards from trash to digivolution cards to use [Ragnarok Cannon]")
        effect1.set_effect_description("[When Digivolving] By placing 4 cards with [Vemmon] in their texts from your trash as this Digimon's bottom digivolution cards, you may use 1 [Ragnarok Cannon] from your hand or trash without paying the cost.")
        effect1.is_optional = True
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Vemmon' in text):
                    return False
            else:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Delete 1 of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-062 Delete 1 opponent Digimon")
        effect2.set_effect_description("[Start of Your Main Phase] Delete 1 of your opponent's Digimon.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, by placing 4 [Vemmon] from this Digimon's digivolution cards at the bottom of their owners' decks, prevent it from leaving play.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-062 Prevent this Digimon from leaving play")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area, by placing 4 [Vemmon] from this Digimon's digivolution cards at the bottom of their owners' decks, prevent it from leaving play.")
        effect3.is_optional = True
        effect3.set_hash_string("Substitute_BT21_062")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
