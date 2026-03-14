from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_077(CardScript):
    """BT22-077 Dianamon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-077 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "Night Claw"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: iceclad
        # Iceclad
        eff_iceclad = ICardEffect()
        eff_iceclad.set_effect_name("BT22-077 Iceclad")
        eff_iceclad.set_effect_description("Iceclad")
        eff_iceclad._is_iceclad = True

        def cond_iceclad(context: Dict[str, Any]) -> bool:
            return True
        eff_iceclad.set_can_use_condition(cond_iceclad)
        effects.append(eff_iceclad)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-077 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If this Digimon's stack has 2 or more same-level cards, trash any 4 digivolution cards from your opponent's Digimon. Then, return 1 of your opponent's Digimon with 1 or fewer digivolution cards to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT22-077 Trash 4 sources, then return 1 digimon.")
        effect2.set_effect_description("[When Digivolving] If this Digimon's stack has 2 or more same-level cards, trash any 4 digivolution cards from your opponent's Digimon. Then, return 1 of your opponent's Digimon with 1 or fewer digivolution cards to the bottom of the deck.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must have 2+ same-level cards in stack
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            from collections import Counter
            levels = [getattr(cs, 'level', None) for cs in perm.card_sources if getattr(cs, 'level', None) is not None]
            level_counts = Counter(levels)
            return any(c >= 2 for c in level_counts.values())

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Trash 4 from opponent sources, return 1 to deck bottom."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            # Trash any 4 digivolution cards from opponent's Digimon
            trashed_count = 0
            for opp_perm in list(enemy.battle_area):
                if trashed_count >= 4:
                    break
                if not opp_perm.is_digimon:
                    continue
                needed = 4 - trashed_count
                removed = opp_perm.trash_digivolution_cards(needed)
                enemy.trash_cards.extend(removed)
                trashed_count += len(removed)
            # Return 1 opponent Digimon with 1 or fewer evo cards to deck bottom
            def target_filter(p):
                return p.is_digimon and len(p.card_sources) <= 2
            def on_return(target_perm):
                enemy2 = player.enemy if player else None
                if enemy2 and target_perm in enemy2.battle_area:
                    enemy2.battle_area.remove(target_perm)
                    for cs in target_perm.card_sources:
                        enemy2.library_cards.append(cs)
            targets = [p for p in enemy.battle_area if target_filter(p)]
            if targets:
                game.effect_select_opponent_permanent(
                    player, on_return, filter_fn=target_filter, is_optional=False,
                    prompt="Return 1 opponent Digimon with 1 or fewer evo cards to deck bottom.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] 1 of your Digimon may unsuspend.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndTurn)
        effect3.set_effect_name("BT22-077 Unsuspend")
        effect3.set_effect_description("[End of Your Turn] [Once Per Turn] 1 of your Digimon may unsuspend.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Unsuspend_BT22-077")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] [Once Per Turn] 1 of your Digimon may unsuspend.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEndTurn)
        effect4.set_effect_name("BT22-077 Unsuspend")
        effect4.set_effect_description("[End of Your Turn] [Once Per Turn] 1 of your Digimon may unsuspend.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("UnsuspendESS_BT22-077")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Unsuspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
