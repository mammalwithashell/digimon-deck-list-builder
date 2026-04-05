from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_025(CardScript):
    """BT21-025 Lamiamon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: progress
        # Progress
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-025 Progress")
        effect0.set_effect_description("Progress")
        effect0._is_progress = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAttackTargetChanged
        # Destroy Security
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAttackTargetChanged)
        effect1.set_effect_name("BT21-025 trash top security")
        effect1.set_effect_description("Destroy Security")
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("YT_BT21-025")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Only trigger if the attacking Digimon has Reptile or Dragonkin trait
            # context['attacker'] is the attacking permanent from OnAttackTargetChanged
            attacker = context.get('attacker')
            if not attacker:
                return False
            # Must be YOUR Digimon (belongs to card owner)
            owner = card.owner if card else None
            if owner:
                if attacker not in owner.battle_area:
                    return False
            top = getattr(attacker, 'top_card', None)
            traits = getattr(top, 'card_traits', []) or []
            if not any('Reptile' in t or 'Dragonkin' in t for t in traits):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card
            enemy = player.enemy if player else None
            if enemy and enemy.security_cards:
                top_sec = enemy.security_cards[-1]  # top = last
                enemy.trash_security_card(top_sec)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnLoseSecurity
        # Play Card
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnLoseSecurity)
        effect2.set_effect_name("BT21-025 Play 1 [Reptile] or [Dragonkin] from hand")
        effect2.set_effect_description("Play Card")
        effect2.is_inherited_effect = True
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("PlayDigimon_BT21_025")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # [All Turns] — no is_my_turn check
            # Must only trigger when OPPONENT's security is removed
            event_player = context.get('event_player') or context.get('player')
            owner = card.owner if card else None
            if event_player is owner:
                return False  # Own security loss — do not trigger
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
                if not (any('Reptile' in _t or 'Dragonkin' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                # Card text: "5000 DP or less"
                dp = getattr(c, 'base_dp', None)
                if dp is not None and dp > 5000:
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
