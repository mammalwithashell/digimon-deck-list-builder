from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_016(CardScript):
    """BT24-016 Lamiamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0.set_effect_name("BT24-016 Place 1 [Dimetromon] from trash under 1 [Elizamon], to digivolve for 3")
        effect0.set_effect_description("[Hand] [Main] If you have [Owen Dreadnought], by placing 1 [Dimetromon] from your trash as any of your [Elizamon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Owen Dreadnought') or permanent.contains_card_name('Elizamon'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # Add To Security, Destroy Security
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAllyAttack)
        effect1.set_effect_name("BT24-016 Opponent places 1 card from hand in security bottom. Trash their security top")
        effect1.set_effect_description("Add To Security, Destroy Security")
        effect1.set_hash_string("WAWD_BT24-016")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            player = card.owner
            if player:
                has_qualifying = False
                for p in player.battle_area:
                    if p is perm:
                        continue
                    traits = getattr(p.top_card, 'card_traits', []) or []
                    if any('Reptile' in t or 'Dragonkin' in t for t in traits):
                        has_qualifying = True
                        break
                if not has_qualifying:
                    return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Add To Security, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Place 1 card from opponent's hand on top of their security stack
            enemy = player.enemy if player else None
            if enemy and enemy.hand_cards:
                card_to_place = enemy.hand_cards.pop(0)
                enemy.security_cards.insert(0, card_to_place)
            # Trash opponent's top security card(s)
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Add To Security, Destroy Security
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-016 Opponent places 1 card from hand in security bottom. Trash their security top")
        effect2.set_effect_description("Add To Security, Destroy Security")
        effect2.set_hash_string("WAWD_BT24-016")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            player = card.owner
            if player:
                has_qualifying = False
                for p in player.battle_area:
                    if p is perm:
                        continue
                    traits = getattr(p.top_card, 'card_traits', []) or []
                    if any('Reptile' in t or 'Dragonkin' in t for t in traits):
                        has_qualifying = True
                        break
                if not has_qualifying:
                    return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add To Security, Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Place 1 card from opponent's hand on top of their security stack
            enemy = player.enemy if player else None
            if enemy and enemy.hand_cards:
                card_to_place = enemy.hand_cards.pop(0)
                enemy.security_cards.insert(0, card_to_place)
            # Trash opponent's top security card(s)
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnLoseSecurity
        # Play Card
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnLoseSecurity)
        effect3.set_effect_name("BT24-016 Play 1 [Reptile] or [Dragonkin] from hand")
        effect3.set_effect_description("Play Card")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("PlayDigimon_BT24_016")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Reptile' in _t or 'Dragonkin' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
