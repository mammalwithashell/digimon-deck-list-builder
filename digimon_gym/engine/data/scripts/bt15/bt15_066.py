from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_066(CardScript):
    """BT15-066 Machinedramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # De Digivolve
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT15-066 De Digivolve")
        effect0.set_effect_description("De Digivolve")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # De Digivolve
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT15-066 De Digivolve")
        effect1.set_effect_description("De Digivolve")
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(2)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Opponent's Turn] Delete this Digimon. Then you may play 1 Digimon card with the [Dark Masters] trait, other than [Machinedramon] from your hand without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT15-066 Delete this Digimon and play 1 Digimon from hand")
        effect2.set_effect_description("[End of Opponent's Turn] Delete this Digimon. Then you may play 1 Digimon card with the [Dark Masters] trait, other than [Machinedramon] from your hand without paying the cost.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # End of OPPONENT's turn
            if card and card.owner and card.owner.is_my_turn:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete THIS Digimon, then play 1 Dark Masters (not Machinedramon) from hand free"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: Delete THIS Digimon
            if perm and perm in player.battle_area:
                player.delete_permanent(perm)
            # Step 2: Play 1 Digimon with Dark Masters trait (not Machinedramon) from hand free
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('Dark Masters' in t for t in traits):
                    return False
                names = getattr(c, 'card_names', []) or []
                if any('Machinedramon' in n for n in names):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: reboot
        # Reboot
        effect3 = ICardEffect()
        effect3.set_effect_name("BT15-066 Reboot")
        effect3.set_effect_description("Reboot")
        effect3.is_inherited_effect = True
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
