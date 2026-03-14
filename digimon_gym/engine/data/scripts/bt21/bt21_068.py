from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_068(CardScript):
    """BT21-068 Growlmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-068 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Guilmon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with 4000 DP or less. If this effect didn't delete, trash the top 2 cards of your deck.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT21-068 Delete 4k or mill 2")
        effect1.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with 4000 DP or less. If this effect didn't delete, trash the top 2 cards of your deck.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def _delete_or_mill(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon 4000 DP or less. If didn't delete, mill 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            targets = [p for p in enemy.battle_area
                      if p.is_digimon and p.dp <= 4000]
            if targets:
                def target_filter(p):
                    return p.is_digimon and p.dp <= 4000

                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=False)
            else:
                # No valid targets, mill 2
                if player.library_cards:
                    mill_count = min(2, len(player.library_cards))
                    trashed = player.library_cards[:mill_count]
                    player.library_cards = player.library_cards[mill_count:]
                    player.trash_cards.extend(trashed)

        effect1.set_on_process_callback(_delete_or_mill)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Same effect
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-068 Delete 4k or mill 2")
        effect2.set_effect_description("[When Digivolving] Delete opponent 4000 DP or less, else mill 2.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_delete_or_mill)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT21-068 Memory +1")
        effect3.set_effect_description("[On Deletion] Gain 1 memory.")
        effect3.is_inherited_effect = True
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
