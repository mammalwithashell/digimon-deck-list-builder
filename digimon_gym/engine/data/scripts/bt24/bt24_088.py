from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_088(CardScript):
    """BT24-088 Asuna Shiroki | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Turn] If you have 4 or less memory, by returning this Tamer to the
        # bottom of the deck, you may play 1 [Asuna Shiroki] or 1 level 4 or lower Digimon card
        # with the [TS] trait or [Three Musketeers] in any of its traits from your trash
        # without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name(
            "BT24-088 Return to deck to play Asuna or lv4- [TS]/[Three Musketeers] from trash"
        )
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 4 or less memory, by returning this Tamer to the "
            "bottom of the deck, you may play 1 [Asuna Shiroki] or 1 level 4 or lower Digimon "
            "card with the [TS] trait or [Three Musketeers] in any of its traits from your trash "
            "without paying the cost."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Condition: 4 or less memory
            game = context.get('game')
            if game is not None and game.memory > 4:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this tamer to the bottom of the deck
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            player.return_permanent_to_deck_bottom(tamer_perm)

            # Effect: play from trash
            def play_filter(c):
                # 1. Named [Asuna Shiroki] (any card type)
                if any('Asuna Shiroki' in _n for _n in getattr(c, 'card_names', [])):
                    return True
                # 2. Level 4 or lower Digimon with [TS] trait or [Three Musketeers] in text
                if getattr(c, 'is_digimon', False):
                    lv = getattr(c, 'level', None)
                    if lv is not None and lv <= 4:
                        if any('TS' in _t for _t in (getattr(c, 'card_traits', []) or [])):
                            return True
                        if any('Three Musketeers' in _t for _t in (getattr(c, 'card_traits', []) or [])):
                            return True
                return False

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True,
                prompt="Select a card to play from trash without paying its cost.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Play] By trashing 1 card with [Three Musketeers] in its text or the [TS] trait
        # from your hand, <Draw 2>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-088 Trash 1 [TS]/[Three Musketeers] from hand to Draw 2")
        effect1.set_effect_description(
            "[On Play] By trashing 1 card with [Three Musketeers] in its text or the [TS] trait "
            "from your hand, <Draw 2>."
        )
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                traits = getattr(c, 'card_traits', []) or []
                if any('TS' in t for t in traits):
                    return True
                if any('Three Musketeers' in t for t in (getattr(c, 'card_traits', []) or [])):
                    return True
                return False

            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
                    player.draw_cards(2)

            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True,
                prompt="Trash 1 [TS] or [Three Musketeers] card from hand to draw 2.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-088 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
