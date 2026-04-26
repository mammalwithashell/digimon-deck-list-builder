from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX5_058(CardScript):
    """EX5-058 Octomon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If there are 4 or more total Digimon, play 1 [Fujitsumon] Token (Digimon/Purple/3000 DP/[All Turns] This Digimon doesn't unsuspend./[On Deletion] Trash 1 card in your hand.) suspended to your battle area. If there are 3 or fewer, play it suspended to your opponent's battle area.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX5-058 Play 1 [Fujitsumon] token")
        effect0.set_effect_description("[On Play] If there are 4 or more total Digimon, play 1 [Fujitsumon] Token (Digimon/Purple/3000 DP/[All Turns] This Digimon doesn't unsuspend./[On Deletion] Trash 1 card in your hand.) suspended to your battle area. If there are 3 or fewer, play it suspended to your opponent's battle area.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Fujitsumon Token — token play not yet supported in engine
            pass  # descriptive-tagged: play_token

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If there are 4 or more total Digimon, play 1 [Fujitsumon] Token (Digimon/Purple/3000 DP/[All Turns] This Digimon doesn't unsuspend./[On Deletion] Trash 1 card in your hand.) suspended to your battle area. If there are 3 or fewer, play it suspended to your opponent's battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX5-058 Play 1 [Fujitsumon] token")
        effect1.set_effect_description("[When Digivolving] If there are 4 or more total Digimon, play 1 [Fujitsumon] Token (Digimon/Purple/3000 DP/[All Turns] This Digimon doesn't unsuspend./[On Deletion] Trash 1 card in your hand.) suspended to your battle area. If there are 3 or fewer, play it suspended to your opponent's battle area.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Play Fujitsumon Token — token play not yet supported in engine
            pass  # descriptive-tagged: play_token

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When an effect plays an opponent's Digimon, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX5-058 Memory +1")
        effect2.set_effect_description("[All Turns] [Once Per Turn] When an effect plays an opponent's Digimon, gain 1 memory.")
        effect2.is_inherited_effect = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("Memory1_EX5_058")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
