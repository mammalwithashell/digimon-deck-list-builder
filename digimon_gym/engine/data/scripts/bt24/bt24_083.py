from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_083(CardScript):
    """BT24-083 Hiroko Sagisaka | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Turn] If you have 4 or less memory, by returning this Tamer to the
        # bottom of the deck, you may play 1 [Hiroko Sagisaka] or 1 [TS] trait Digimon card
        # with 5000 DP or less from your hand without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT24-083 Return to deck to play Hiroko or a 5k- [TS] from hand")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 4 or less memory, by returning this Tamer to the "
            "bottom of the deck, you may play 1 [Hiroko Sagisaka] or 1 [TS] trait Digimon card "
            "with 5000 DP or less from your hand without paying the cost."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Condition: 4 or less memory (check from context game object)
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
            # Effect: play 1 [Hiroko Sagisaka] or 1 [TS] Digimon with DP <= 5000 from hand
            def play_filter(c):
                # Hiroko Sagisaka (any card type)
                if any('Hiroko Sagisaka' in _n for _n in getattr(c, 'card_names', [])):
                    return True
                # [TS] trait Digimon with DP <= 5000
                if getattr(c, 'is_digimon', False):
                    if any('TS' in _t for _t in (getattr(c, 'card_traits', []) or [])):
                        dp = getattr(c, 'base_dp', None)
                        if dp is not None and dp <= 5000:
                            return True
                return False

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [TS] trait among
        # them to the hand. Return the rest to the bottom of the deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-083 Reveal 3, Add 1 [TS] to hand, rest to deck bottom")
        effect1.set_effect_description(
            "[On Play] Reveal the top 3 cards of your deck. Add 1 card with the [TS] trait "
            "among them to the hand. Return the rest to the bottom of the deck."
        )
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

            def ts_filter(c):
                return any('TS' in _t for _t in (getattr(c, 'card_traits', []) or []))

            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for r in reversed(remaining):
                    player.library_cards.append(r)

            game.effect_reveal_and_select(
                player, 3, ts_filter,
                on_selected=on_revealed,
                is_optional=False
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-083 Security: Play this card")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
