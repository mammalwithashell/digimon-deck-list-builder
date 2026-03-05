from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_065(CardScript):
    """BT24-065 Diaboromon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-065 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Diaboromon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Diaboromon"

        def condition0(context: Dict[str, Any]) -> bool:
            # Alt-digi name check handled by _alt_digi_name attribute
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: overclock
        # Overclock
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-065 Overclock")
        effect1.set_effect_description("Overclock")
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: blocker
        # Blocker
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-065 Blocker")
        effect2.set_effect_description("Blocker")
        effect2._is_blocker = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Delete, De Digivolve
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-065 De-digivolve 1 opponent's Digimon. Delete all their highest play cost Digimon.")
        effect3.set_effect_description("Delete, De Digivolve")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: De-Digivolve 1 per own Digimon to 1 target, then delete all highest cost"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Count own Digimon for de-digivolve scaling
            own_digimon_count = sum(1 for p in player.battle_area if p.is_digimon)

            def on_de_digivolve(target_perm):
                # De-Digivolve 1 for each of your Digimon
                removed = target_perm.de_digivolve(own_digimon_count)
                enemy.trash_cards.extend(removed)

                # Then, delete all opponent's Digimon with the highest play cost
                enemy_digimon = [p for p in enemy.battle_area if p.is_digimon]
                if not enemy_digimon:
                    return
                max_cost = max(
                    (getattr(p.top_card, 'get_cost_itself', 0) for p in enemy_digimon),
                    default=0
                )
                to_delete = [p for p in enemy_digimon
                             if getattr(p.top_card, 'get_cost_itself', 0) == max_cost]
                for p in to_delete:
                    if p in enemy.battle_area:
                        enemy.delete_permanent(p)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] [Once Per Turn] When any of your Digimon with [Diaboromon] in their names would leave the battle area, you may play 1 [Diaboromon] from your hand or this Digimon's digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.WhenRemoveField)
        effect4.set_effect_name("BT24-065 Play a Diaboromon")
        effect4.set_effect_description("[All Turns] [Once Per Turn] When any of your Digimon with [Diaboromon] in their names would leave the battle area, you may play 1 [Diaboromon] from your hand or this Digimon's digivolution cards without paying the cost.")
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_65_AT_Play_Diaboromon")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play [Diaboromon] from hand or this Digimon's evo cards free"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                return any('Diaboromon' in _n for _n in getattr(c, 'card_names', []))
            # Try hand first, then evo cards
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
