from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_095(CardScript):
    """BT22-095 Akemi Suedou"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-095 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn] When any of your Digimon with the [Eater] trait are played, by suspending this Tamer, gain 1 memory. Then, if you have 7 or fewer cards in your hand, <Draw 1> (Draw 1 card from your deck.)
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-095 +1 memory, then if you have 7 or less in hand, draw 1")
        effect1.set_effect_description("[Your Turn] When any of your Digimon with the [Eater] trait are played, by suspending this Tamer, gain 1 memory. Then, if you have 7 or fewer cards in your hand, <Draw 1> (Draw 1 card from your deck.)")
        effect1.is_optional = True
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory, Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] Place this Tamer as any of your [Mother Eater]s' bottom digivolution card.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("BT22-095 Place underneath a [Mother Eater]")
        effect2.set_effect_description("[Main] Place this Tamer as any of your [Mother Eater]s' bottom digivolution card.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Mother Eater'))):
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: rush
        # Rush
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-095 Rush")
        effect3.set_effect_description("Rush")
        effect3.is_inherited_effect = True
        effect3._is_rush = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: alliance
        # Alliance
        effect4 = ICardEffect()
        effect4.set_effect_name("BT22-095 Alliance")
        effect4.set_effect_description("Alliance")
        effect4.is_inherited_effect = True
        effect4.is_on_attack = True
        effect4._is_alliance = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: scapegoat
        # Scapegoat
        effect5 = ICardEffect()
        effect5.set_effect_name("BT22-095 Scapegoat")
        effect5.set_effect_description("Scapegoat")
        effect5.is_inherited_effect = True
        effect5._is_scapegoat = True

        def condition5(context: Dict[str, Any]) -> bool:
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        return effects
