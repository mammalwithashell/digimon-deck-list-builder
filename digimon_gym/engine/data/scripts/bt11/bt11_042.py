from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_042(CardScript):
    """BT11-042 Angewomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may search your security stack, reveal 1 card with [Angel], [Archangel], or [Fallen Angel] in its traits from it, and add it to your hand. If you added a card, <Recovery +1 (Deck)>. Then, shuffle your security stack.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT11-042 Add 1 card from security to hand")
        effect0.set_effect_description("[When Digivolving] You may search your security stack, reveal 1 card with [Angel], [Archangel], or [Fallen Angel] in its traits from it, and add it to your hand. If you added a card, <Recovery +1 (Deck)>. Then, shuffle your security stack.")
        effect0.is_optional = True
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Add a matching security card to hand, then recover."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def security_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                return any(
                    'Angel' in trait or 'Archangel' in trait
                    or 'Fallen Angel' in trait or 'FallenAngel' in trait
                    for trait in traits
                )
            def on_add(selected):
                if selected in player.security_cards:
                    player.security_cards.remove(selected)
                    player.hand_cards.append(selected)
                    player.recovery(1)
            game.effect_select_own_security(
                player, security_filter, on_add, is_optional=True,
                prompt="Select a security card to add to your hand.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [Your Turn][Once Per Turn] When you play [LadyDevimon] or [Mirei Mikagura], gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT11-042 Memory +1")
        effect1.set_effect_description("[Your Turn][Once Per Turn] When you play [LadyDevimon] or [Mirei Mikagura], gain 1 memory.")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("Memory+1_BT11_042")
        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if context.get('event_player') is not card.owner:
                return False
            played_card = context.get('played_card')
            if not played_card:
                return False
            return (
                played_card.contains_card_name('LadyDevimon')
                or played_card.contains_card_name('Mirei Mikagura')
            )

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

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-042 Blocker")
        effect2.set_effect_description("Blocker")
        effect2.is_inherited_effect = True
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
