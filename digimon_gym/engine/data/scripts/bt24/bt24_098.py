from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_098(CardScript):
    """BT24-098 Invasion of the Titans"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ---------------------------------------------------------------
        # [Main] Draw 2, trash 2 from hand, then place this card in battle area.
        # ---------------------------------------------------------------
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT24-098 Draw 2, Trash 2")
        effect0.set_effect_description("[Main] <Draw 2> and trash 2 cards in your hand. Then, place this card in the battle area.")

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 2, Trash 2 from hand"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player:
                player.draw_cards(2)
            if not (player and game):
                return

            # Trash 2 cards from hand (not 1)
            def hand_filter(c):
                return True

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)

            hand_count = len(player.hand_cards)
            trash_count = min(2, hand_count)
            for _ in range(trash_count):
                game.effect_select_hand_card(
                    player, hand_filter, on_trashed, is_optional=False,
                    prompt="Select a card from your hand to trash.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-098 Delay")
        effect1.set_effect_description("Delay")
        effect1.is_on_play = True
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ---------------------------------------------------------------
        # [Your Turn] Delay: When any of your [Titan] trait Digimon are played,
        # if your opponent has 5 or more memory, play 1 Lv.5- [Titan] Digimon
        # from trash without paying the cost.
        # ---------------------------------------------------------------
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-098 If Opp has 5+ memory, Play a 5- [Titan] from trash")
        effect2.set_effect_description("[Your Turn] When any of your [Titan] trait Digimon are played, <Delay> If your opponent has 5 or more memory, you may play 1 level 5 or lower Digimon card with the [Titan] trait from your trash without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delay: Play 1 Lv.5- Titan from trash if opponent has 5+ memory."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 5:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Titan' in tr for tr in traits)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # ---------------------------------------------------------------
        # Security: Play 1 Lv.4- [Titan] Digimon from hand or trash free.
        # Then, add THIS card to hand.
        # ---------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT24-098 Play a 4- [Titan] and add this to hand")
        effect3.set_effect_description("[Security] You may play 1 level 4 or lower [Titan] trait Digimon card from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Security: Play Lv.4- Titan from hand/trash, then add THIS card to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Titan' in tr for tr in traits)

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

            # Add THIS card (the Option card itself) to hand
            if card:
                # Move card from wherever it is (security/trash) to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                    player.hand_cards.append(card)
                elif card in player.security_cards:
                    player.security_cards.remove(card)
                    player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
