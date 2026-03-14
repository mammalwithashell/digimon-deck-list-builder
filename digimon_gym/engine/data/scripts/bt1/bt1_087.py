from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_087(CardScript):
    """BT1-087 T.K. Takaishi | Tamer (Yellow, Cost 4)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [On Play] Search your security stack, reveal 1 card among it and add it
        to your hand. If it's a yellow card, Recovery +1 (Deck). Then, shuffle
        your security stack.
    Security Effect [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT1-087 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Search security, add 1 to hand, recovery if yellow ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT1-087 On Play: search security")
        effect1.set_effect_description(
            "[On Play] Search your security stack, reveal 1 card among it "
            "and add it to your hand. If it's a yellow card, Recovery +1 "
            "(Deck). Then, shuffle your security stack."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Search security, add 1 to hand, recovery if yellow, shuffle."""
            import random
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.security_cards:
                return

            def on_security_selected(selected):
                if selected in player.security_cards:
                    player.security_cards.remove(selected)
                    player.hand_cards.append(selected)

                    # If it's yellow, recovery +1
                    colors = getattr(selected, 'card_colors', []) or []
                    color_names = [col.name for col in colors]
                    if 'Yellow' in color_names:
                        player.recovery(1)

                # Shuffle remaining security
                random.shuffle(player.security_cards)

            game.effect_select_own_security(
                player, lambda c: True, on_security_selected, is_optional=False,
                prompt="Select 1 card from your security stack to add to your hand."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT1-087 Security: Play free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this tamer from security without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
