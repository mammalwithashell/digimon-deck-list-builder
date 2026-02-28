from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT10_102(CardScript):
    """BT10-102 Pyon Dump"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your Digimon gains <Piercing> for the turn. (When this Digimon attacks and deletes an opponent's Digimon, it performs any security checks it normally would.) Then, if you have a Digimon in play with [Angoramon] in its name or digivolution cards, suspend 1 of your opponent's Digimon.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT10-102 Gain Keyword Piercing, Suspend")
        effect0.set_effect_description("[Main] 1 of your Digimon gains <Piercing> for the turn. (When this Digimon attacks and deletes an opponent's Digimon, it performs any security checks it normally would.) Then, if you have a Digimon in play with [Angoramon] in its name or digivolution cards, suspend 1 of your opponent's Digimon.")
        effect0._is_piercing = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Piercing, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_piercing')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Angoramon')):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Suspend 1 of your opponent's Digimon. Then, add this card to your hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT10-102 Add To Hand, Suspend")
        effect1.set_effect_description("[Security] Suspend 1 of your opponent's Digimon. Then, add this card to your hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Hand, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
