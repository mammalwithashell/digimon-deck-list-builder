from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_033(CardScript):
    """EX5-033 Mitamamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] [Once Per Turn] By trashing the top card of your security stack, you may play 1 yellow level 4 or lower card from your hand without paying the cost. Any Digimon played by this effect gain <Rush> (This Digimon can attack the turn it comes into play), for the turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX5-033 Trash the top card of your security to play 1 Digimon from hand")
        effect0.set_effect_description("[When Digivolving] [Once Per Turn] By trashing the top card of your security stack, you may play 1 yellow level 4 or lower card from your hand without paying the cost. Any Digimon played by this effect gain <Rush> (This Digimon can attack the turn it comes into play), for the turn.")
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("PlayDigimon_EX5_033")
        effect0.is_when_digivolving = True
        effect0._is_rush = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Card, Destroy Security, Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not ('Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
            if perm:
                perm.grant_keyword('_is_rush')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] [Once Per Turn] By trashing the top card of your security stack, you may play 1 yellow level 4 or lower card from your hand without paying the cost. Any Digimon played by this effect gain <Rush> (This Digimon can attack the turn it comes into play), for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("EX5-033 Trash the top card of your security to play 1 Digimon from hand")
        effect1.set_effect_description("[When Attacking] [Once Per Turn] By trashing the top card of your security stack, you may play 1 yellow level 4 or lower card from your hand without paying the cost. Any Digimon played by this effect gain <Rush> (This Digimon can attack the turn it comes into play), for the turn.")
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("PlayDigimon_EX5_033")
        effect1.is_on_attack = True
        effect1._is_rush = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Destroy Security, Gain Keyword Rush"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if getattr(c, 'level', None) is None or c.level > 4:
                    return False
                if not ('Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
            if perm:
                perm.grant_keyword('_is_rush')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: barrier
        # Barrier
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-033 Barrier")
        effect2.set_effect_description("Barrier")
        effect2._is_barrier = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Grant Skill
        effect3 = ICardEffect()
        effect3.set_effect_name("EX5-033 Your yellow Digimons gain Barrier")
        effect3.set_effect_description("Grant Skill")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
