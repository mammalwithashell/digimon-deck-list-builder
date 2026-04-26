from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming
from ....interfaces.modifiers import ModifierType

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_054(CardScript):
    """EX10-054 VenomMyotismon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnDeclaration
        # [Trash] [Main] By deleting 1 of your level 5 Digimon with [Myotismon] in its text, play this card with the play cost reduced by 7.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnDeclaration)
        effect0._is_trash_main = True
        effect0.set_effect_name("EX10-054 Play for reduced cost of 7")
        effect0.set_effect_description("[Trash] [Main] By deleting 1 of your level 5 Digimon with [Myotismon] in its text, play this card with the play cost reduced by 7.")
        effect0.is_optional = True
        effect0.cost_reduction = 7

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect0.effect_source_permanent if hasattr(effect0, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if 'Myotismon' not in text:
                    return False
            else:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -7, Play Card"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                return True

            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ---------------------------------------------------------------
        # Shared On Play / When Digivolving:
        # Suspend up to 2 opponent Digimon or Tamers.
        # Then, up to 2 opponent Digimon or Tamers gain cannot-unsuspend
        # until their turn ends.
        # ---------------------------------------------------------------

        def _suspend_and_lock_process(ctx: Dict[str, Any]):
            """Suspend up to 2 opp Digimon/Tamers, then 2 cannot-unsuspend."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def opp_digi_or_tamer(p):
                return p.is_digimon or p.is_tamer

            # Suspend up to 2
            opp_targets = [p for p in enemy.battle_area if opp_digi_or_tamer(p)]
            for _ in range(min(2, len(opp_targets))):
                def on_suspend(target_perm):
                    target_perm.suspend()
                game.effect_select_opponent_permanent(
                    player, on_suspend, filter_fn=opp_digi_or_tamer,
                    is_optional=True,
                    prompt="Select an opponent's Digimon or Tamer to suspend.")

            # Then, 2 of their Digimon or Tamers cannot unsuspend until their turn ends
            for _ in range(min(2, len(opp_targets))):
                def on_lock(target_perm):
                    game.register_modifier(target_perm, ModifierType.CANNOT_UNSUSPEND, {
                        'condition': lambda p, ctx: True,
                        'source_effect': effect1,
                        'expiry': 'end_of_opponent_turn',
                        'granting_player': player,
                    })
                game.effect_select_opponent_permanent(
                    player, on_lock, filter_fn=opp_digi_or_tamer,
                    is_optional=False,
                    prompt="Select an opponent's Digimon or Tamer that cannot unsuspend.")

        # On Play
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX10-054 Suspend 2 opponent Digimon or Tamers")
        effect1.set_effect_description("[On Play] You may suspend 2 of your opponent's Digimon or Tamers. Then, 2 of their Digimon or Tamers can't unsuspend until their turn ends.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_suspend_and_lock_process)
        effects.append(effect1)

        # When Digivolving
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX10-054 Suspend 2 opponent Digimon or Tamers")
        effect2.set_effect_description("[When Digivolving] You may suspend 2 of your opponent's Digimon or Tamers. Then, 2 of their Digimon or Tamers can't unsuspend until their turn ends.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_suspend_and_lock_process)
        effects.append(effect2)

        # ---------------------------------------------------------------
        # On Deletion: Delete 1 opponent's SUSPENDED Digimon
        # ---------------------------------------------------------------
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("EX10-054 Delete 1 suspended Digimon")
        effect3.set_effect_description("[On Deletion] Delete 1 of your opponent's suspended Digimon.")
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete 1 opponent suspended Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return p.is_digimon and p.is_suspended

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False,
                prompt="Select 1 of your opponent's suspended Digimon to delete.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
