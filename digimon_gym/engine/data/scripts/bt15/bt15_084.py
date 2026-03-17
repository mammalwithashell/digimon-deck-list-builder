from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_084(CardScript):
    """BT15-084 Kari Kamiya"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDiscardSecurity
        # When an effect trashes THIS card from the security stack,
        # 1 of your opponent's Digimon gains Security A. -1 until the end of their turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDiscardSecurity)
        effect0.set_effect_name("BT15-084 Opponent Digimon Security A. -1 on security discard")
        effect0.set_effect_description("When an effect trashes this card from the security stack, 1 of your opponent's Digimon gains Security A. -1 until the end of their turn.")

        def condition0(context: Dict[str, Any]) -> bool:
            # Only fires when THIS specific card is the one being discarded from security
            discarded = context.get('card') or context.get('source')
            if discarded is not None and discarded is not card:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Select 1 opponent Digimon, apply Security A. -1 until end of their turn."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....interfaces.modifiers import ModifierType

            def on_select(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_SECURITY_ATTACK,
                    value_fn=lambda: -1,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to give Security A. -1.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Turn] If you have 2 or less memory, set it to 3.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT15-084 Set memory to 3")
        effect1.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] When an effect removes cards from your security stack,
        # by suspending this Tamer, 1 of your opponent's Digimon gains Security A. -1 until the end of their turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnLoseSecurity)
        effect2.set_effect_name("BT15-084 Suspend for opponent Digimon Security A. -1")
        effect2.set_effect_description("[All Turns] When an effect removes cards from your security stack, by suspending this Tamer, 1 of your opponent's Digimon gains Security A. -1 until the end of their turn.")
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # This Tamer must not already be suspended (suspend is the cost)
            this_perm = card.permanent_of_this_card()
            if this_perm and this_perm.is_suspended:
                return False
            # The security must be removed from THIS card's owner (your security)
            player = card.owner if card else None
            if player is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer (cost), then select opponent Digimon for Security A. -1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....interfaces.modifiers import ModifierType

            # Cost: suspend this Tamer
            this_perm = card.permanent_of_this_card() if card else None
            if not this_perm:
                return
            this_perm.suspend()

            # Effect: select 1 opponent Digimon, apply Security A. -1 until end of their turn
            def on_select(target_perm):
                game.register_modifier(
                    target_perm,
                    ModifierType.CHANGE_SECURITY_ATTACK,
                    value_fn=lambda: -1,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to give Security A. -1.",
            )

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
