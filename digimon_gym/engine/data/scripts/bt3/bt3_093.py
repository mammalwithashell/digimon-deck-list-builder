from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT3_093(CardScript):
    """BT3-093 Davis Motomiya | Tamer (Blue, Cost 4)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [On Play] Reveal the top 3 cards of your deck. Add 1 blue and 1 green
        Digimon card among them to your hand. Place the remaining cards at
        the bottom of your deck in any order.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT3-093 Set memory to 3")
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
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Reveal top 3, add 1 blue + 1 green Digimon ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT3-093 Reveal top 3, add blue+green Digimon")
        effect1.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 blue and "
            "1 green Digimon card among them to your hand. Place the remaining "
            "cards at the bottom of your deck in any order."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Reveal top 3
            revealed = []
            for _ in range(min(3, len(player.library_cards))):
                revealed.append(player.library_cards.pop(0))

            added = []
            # Add 1 blue Digimon
            for c in revealed:
                if c not in added and c.is_digimon:
                    colors = [col.name for col in (c.card_colors or [])]
                    if 'Blue' in colors:
                        added.append(c)
                        break
            # Add 1 green Digimon
            for c in revealed:
                if c not in added and c.is_digimon:
                    colors = [col.name for col in (c.card_colors or [])]
                    if 'Green' in colors:
                        added.append(c)
                        break

            for c in added:
                player.hand_cards.append(c)
            # Remaining go to bottom of deck
            remaining = [c for c in revealed if c not in added]
            for c in remaining:
                player.library_cards.append(c)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play without paying cost ---
        effect2 = ICardEffect()
        effect2.set_effect_name("BT3-093 Security Effect")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                game.effect_play_card_from_security(player, card)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
