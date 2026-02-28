from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_084(CardScript):
    """BT15-084 Kari Kamiya"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardSecurity
        # When an effect trashes this card from the security stack, 1 of your opponent's Digimon gains [Security A -1] until the end of your opponent's turn.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-084 Opponent's 1 Digimon gains Security Attack -1")
        effect0.set_effect_description("When an effect trashes this card from the security stack, 1 of your opponent's Digimon gains [Security A -1] until the end of your opponent's turn.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: set_memory_3
        # Set memory to 3
        effect1 = ICardEffect()
        effect1.set_effect_name("BT15-084 Set memory to 3")
        effect1.set_effect_description("Set memory to 3")
        # [Start of Your Turn] Set memory to 3 if <= 2

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] When an effect removes a card from your security stack, by suspending this Tamer, 1 of your opponent's Digimon gains [Security A-1] until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT15-084 Opponent's 1 Digimon gains Security Attack -1")
        effect2.set_effect_description("[All Turns] When an effect removes a card from your security stack, by suspending this Tamer, 1 of your opponent's Digimon gains [Security A-1] until the end of your opponent's turn.")
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("AllTurns_BT15_084")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend, Change Security Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            # Grant Security Attack modifier to target permanent
            pass  # descriptive-tagged: change_security_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("BT15-084 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
