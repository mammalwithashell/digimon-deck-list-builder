from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_111(CardScript):
    """BT11-111 Galacticmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-111 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 9
        effect0._alt_digi_cost = 9
        effect0._alt_digi_name = "Snatchmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may place up to 4 [Vemmon] from your trash under this Digimon as its bottom digivolution cards. Then, if there are 8 or more [Vemmon] in this Digimon's digivolution cards, delete 1 of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-111 Place up to 4 [Vemmon] from trash to digivolution cards and delete 1 Digimon")
        effect1.set_effect_description("[When Digivolving] You may place up to 4 [Vemmon] from your trash under this Digimon as its bottom digivolution cards. Then, if there are 8 or more [Vemmon] in this Digimon's digivolution cards, delete 1 of your opponent's Digimon.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
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

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area, by placing 4 [Vemmon] from this Digimon's digivolution cards at the bottom of their owners' decks, prevent it from leaving play.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenRemoveField)
        effect2.set_effect_name("BT11-111 Prevent this Digimon from leaving play")
        effect2.set_effect_description("[All Turns] When this Digimon would leave the battle area, by placing 4 [Vemmon] from this Digimon's digivolution cards at the bottom of their owners' decks, prevent it from leaving play.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] Trash the top card of your opponent's security stack.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnStartMainPhase)
        effect3.set_effect_name("BT11-111 Trash the top card of opponent's security")
        effect3.set_effect_description("[Start of Your Main Phase] Trash the top card of your opponent's security stack.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
