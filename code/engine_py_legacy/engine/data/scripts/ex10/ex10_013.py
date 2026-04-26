from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_013(CardScript):
    """EX10-013 Lucemon | Lv.3

    <Blocker>
    [Breeding] [When Digivolving] This Digimon may move.
    [End of Your Turn] By returning 5 cards with [Lucemon] in their texts
        from your trash to the bottom of the deck, this Digimon may digivolve
        into [Lucemon: Chaos Mode] in the trash without paying the cost.
    Inherited: <Blocker>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Alt digi from Cupimon for cost 5
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-013 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 5
        effect0._alt_digi_level = 2
        effect0._alt_digi_name = "Cupimon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.contains_card_name('Cupimon')):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-013 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [Breeding] [When Digivolving] This Digimon may move.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-013 Breeding: may move when digivolving")
        effect2.set_effect_description("[Breeding] [When Digivolving] This Digimon may move.")
        effect2.is_optional = True
        effect2.is_when_digivolving = True
        effect2._breeding_move = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and player.breeding_area:
                perm = card.permanent_of_this_card() if card else None
                if perm and player.breeding_area is perm:
                    player.move_from_breeding()

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [End of Your Turn] Return 5 Lucemon-text cards from trash to deck bottom,
        # digivolve into Lucemon: Chaos Mode from trash
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.EndOfTurn)
        effect3.set_effect_name("EX10-013 End Turn: return 5, digivolve into Chaos Mode")
        effect3.set_effect_description(
            "[End of Your Turn] By returning 5 cards with [Lucemon] in their "
            "texts from your trash to the bottom of the deck, this Digimon may "
            "digivolve into [Lucemon: Chaos Mode] in the trash without paying the cost."
        )
        effect3.is_optional = True

        def _has_lucemon_text(c):
            text = getattr(c, 'card_text', '') or ''
            names = getattr(c, 'card_names', []) or []
            return 'Lucemon' in text or any('Lucemon' in n for n in names)

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner
            # Need 5+ cards with Lucemon in text in trash
            lucemon_trash = [c for c in player.trash_cards if _has_lucemon_text(c)]
            if len(lucemon_trash) < 5:
                return False
            # Need Lucemon: Chaos Mode in trash
            has_chaos = any(
                any('Lucemon: Chaos Mode' in n or 'Lucemon Chaos Mode' in n
                    for n in (getattr(c, 'card_names', []) or []))
                for c in player.trash_cards
            )
            return has_chaos
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Cost: return 5 cards with [Lucemon] in text from trash to deck bottom
            lucemon_trash = [c for c in player.trash_cards if _has_lucemon_text(c)]
            returned = 0
            for c in lucemon_trash[:5]:
                if c in player.trash_cards:
                    player.trash_cards.remove(c)
                    player.library_cards.append(c)
                    returned += 1
            if returned < 5:
                return

            # Digivolve into Lucemon: Chaos Mode from trash
            def chaos_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Lucemon: Chaos Mode' in n or 'Lucemon Chaos Mode' in n for n in names)

            chaos_cards = [c for c in player.trash_cards if chaos_filter(c)]
            if chaos_cards:
                chosen = chaos_cards[0]
                player.trash_cards.remove(chosen)
                perm.card_sources.append(chosen)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Inherited: Blocker
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-013 Inherited Blocker")
        effect4.set_effect_description("Blocker")
        effect4.is_inherited_effect = True
        effect4._is_blocker = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
