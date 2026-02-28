from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_076(CardScript):
    """BT21-076 WarGrowlmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-076 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Trash the top 2 cards of your deck. Then, this Digimon gains <Raid> and <Retaliation> until your opponent's turn ends.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-076 trash 2, get raid retal")
        effect1.set_effect_description("[On Play] Trash the top 2 cards of your deck. Then, this Digimon gains <Raid> and <Retaliation> until your opponent's turn ends.")
        effect1.is_on_play = True
        effect1._is_raid = True
        effect1._is_retaliation = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Raid, Gain Keyword Retaliation, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_raid')
                perm.grant_keyword('_is_retaliation')
            # Mill 2 cards from own deck
            if player and player.library_cards:
                mill_count = min(2, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Trash the top 2 cards of your deck. Then, this Digimon gains <Raid> and <Retaliation> until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-076 trash 2, get raid retal")
        effect2.set_effect_description("[When Digivolving] Trash the top 2 cards of your deck. Then, this Digimon gains <Raid> and <Retaliation> until your opponent's turn ends.")
        effect2.is_when_digivolving = True
        effect2._is_raid = True
        effect2._is_retaliation = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Raid, Gain Keyword Retaliation, Mill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_raid')
                perm.grant_keyword('_is_retaliation')
            # Mill 2 cards from own deck
            if player and player.library_cards:
                mill_count = min(2, len(player.library_cards))
                trashed = player.library_cards[:mill_count]
                player.library_cards = player.library_cards[mill_count:]
                player.trash_cards.extend(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking][Once Per Turn] This Digimon may digivolve into a Digimon card with [Megidramon] or [ChaosGallantmon] in its name in the hand. For every 10 total cards in both players' trashes, reduce this effect's digivolution cost by 1.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-076 Digivolve to Megidra or ChaosGallant")
        effect3.set_effect_description("[When Attacking][Once Per Turn] This Digimon may digivolve into a Digimon card with [Megidramon] or [ChaosGallantmon] in its name in the hand. For every 10 total cards in both players' trashes, reduce this effect's digivolution cost by 1.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("BT21_076 when attacking")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Megidramon' in _n or 'ChaosGallantmon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Trash the top card of your opponent's security stack.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-076 Trash the top card of your opponent's security stack.")
        effect4.set_effect_description("[On Deletion] Trash the top card of your opponent's security stack.")
        effect4.is_inherited_effect = True
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
