from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_054(CardScript):
    """EX6-054 Lucemon: Chaos Mode | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-054 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 6
        effect0._alt_digi_cost = 6
        effect0._alt_digi_name = "Lucemon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, trash the top card of your opponents security stack and <Recovery +1>.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX6-054 Opponent deletes 1 Digimon or Tamer/Trash and Recover")
        effect1.set_effect_description("[On Play] Your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, trash the top card of your opponents security stack and <Recovery +1>.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Recovery +1, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, trash the top card of your opponents security stack and <Recovery +1>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX6-054 Opponent deletes 1 Digimon or Tamer/Trash and Recover")
        effect2.set_effect_description("[When Digivolving] Your opponent may delete 1 of their Digimon or Tamers. If this effect didn't delete, trash the top card of your opponents security stack and <Recovery +1>.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Recovery +1, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.recovery(1)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would leave the battle area; by returning 1 [Lucemon] from this Digimon's digivolution cards or from your trash to the bottom of the deck, you may play 1 [Lucemon: Satan Mode] or 1 level 6 Digimon card with the [Seven Great Demon Lords] trait from your trash without paying the cost.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("EX6-054 Play 1 [Lucemon: Satan Mode] or level 6 with [Seven Great Demon Lords] trait")
        effect3.set_effect_description("[All Turns] When this Digimon would leave the battle area; by returning 1 [Lucemon] from this Digimon's digivolution cards or from your trash to the bottom of the deck, you may play 1 [Lucemon: Satan Mode] or 1 level 6 Digimon card with the [Seven Great Demon Lords] trait from your trash without paying the cost.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Return To Deck"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Seven Great Demon Lords' in _t or 'SevenGreatDemonLords' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Seven Great Demon Lords' in t for t in (getattr(p.top_card, 'card_traits', []) or [])) or any('SevenGreatDemonLords' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
