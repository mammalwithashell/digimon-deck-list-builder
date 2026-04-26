from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_100(CardScript):
    """BT13-100 Yoshino Fujieda"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: set_memory_3
        # [Start of Your Turn] Set memory to 3 if <= 2
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT13-100 Set memory to 3")
        effect0.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ---------------------------------------------------------------
        # [Your Turn] When one of your Digimon digivolves into a Digimon
        # with [Vegetation], [Plant], or [Fairy] trait, by suspending this
        # Tamer, gain 1 memory.
        # ---------------------------------------------------------------
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT13-100 Memory +1")
        effect1.set_effect_description("[Your Turn] When one of your Digimon digivolves into a Digimon with [Vegetation], [Plant], or [Fairy] in one of its traits, by suspending this Tamer, gain 1 memory.")
        effect1.is_optional = True
        # This triggers on when digivolving, NOT on play
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the digivolving Digimon has Vegetation/Plant/Fairy trait
            # The engine passes the played_permanent in context for OnEnterFieldAnyone
            ctx_perm = context.get('played_permanent')
            if ctx_perm and ctx_perm.top_card:
                traits = getattr(ctx_perm.top_card, 'card_traits', []) or []
                has_trait = any(
                    t in tr for tr in traits
                    for t in ('Vegetation', 'Plant', 'Fairy')
                )
                if not has_trait:
                    return False
            # The tamer itself must not be suspended (can activate suspend cost)
            this_perm = card.permanent_of_this_card() if card else None
            if this_perm and this_perm.is_suspended:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, gain 1 memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Suspend THIS Tamer (not an opponent permanent)
            this_perm = card.permanent_of_this_card() if card else None
            if this_perm:
                this_perm.suspend()
            # Gain 1 memory
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT13-100 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
