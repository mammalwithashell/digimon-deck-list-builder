from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_009(CardScript):
    """EX10-009 Creepymon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete all of your opponent's lowest DP Digimon. If this effect didn't delete, trash their deck's top 5 cards.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX10-009 Delete all lowest DP Digimon. if this didnt happen, trash their top 5 deck cards")
        effect0.set_effect_description("[When Digivolving] Delete all of your opponent's lowest DP Digimon. If this effect didn't delete, trash their deck's top 5 cards.")
        effect0.is_when_digivolving = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 5 cards from opponent's deck
            enemy = player.enemy if player else None
            if enemy and enemy.library_cards:
                mill_count = min(5, len(enemy.library_cards))
                trashed = enemy.library_cards[:mill_count]
                enemy.library_cards = enemy.library_cards[mill_count:]
                enemy.trash_cards.extend(trashed)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Delete all of your opponent's lowest DP Digimon. If this effect didn't delete, trash their deck's top 5 cards.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX10-009 Delete all lowest DP Digimon. if this didnt happen, trash their top 5 deck cards")
        effect1.set_effect_description("[On Deletion] Delete all of your opponent's lowest DP Digimon. If this effect didn't delete, trash their deck's top 5 cards.")
        effect1.is_on_deletion = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Mill 5 cards from opponent's deck
            enemy = player.enemy if player else None
            if enemy and enemy.library_cards:
                mill_count = min(5, len(enemy.library_cards))
                trashed = enemy.library_cards[:mill_count]
                enemy.library_cards = enemy.library_cards[mill_count:]
                enemy.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] If your opponent has 10 or more cards in their trash, you may play 1 red or purple 5000 DP or lower Digimon card from your trash to your empty breeding area without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("EX10-009 Play 1 red/purple digimon with 5k DP to breeding area")
        effect2.set_effect_description("[When Attacking] If your opponent has 10 or more cards in their trash, you may play 1 red or purple 5000 DP or lower Digimon card from your trash to your empty breeding area without paying the cost.")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Red' in [col.name for col in getattr(c, 'card_colors', [])] or 'Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] This Digimon may attack.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndTurn)
        effect3.set_effect_name("EX10-009 Attack with this digimon")
        effect3.set_effect_description("[End of Your Turn] This Digimon may attack.")
        effect3.is_optional = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
