from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_044(CardScript):
    """BT20-044 Breakdramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-044 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Groundramon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_name = "Groundramon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Groundramon') or permanent.contains_card_name('Wingdramon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-044 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Suspend 2 of your opponent's Digimon or Tamers. Then, 1 of your Digimon may attack.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT20-044 Suspend 2 Digimon or Tamers and 1 of your Digimon can attack")
        effect2.set_effect_description("[On Play] Suspend 2 of your opponent's Digimon or Tamers. Then, 1 of your Digimon may attack.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 2 of your opponent's Digimon or Tamers. Then, 1 of your Digimon may attack.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT20-044 Suspend 2 Digimon or Tamers and 1 of your Digimon can attack")
        effect3.set_effect_description("[When Digivolving] Suspend 2 of your opponent's Digimon or Tamers. Then, 1 of your Digimon may attack.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Suspend"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] (Once Per Turn) When any of your Digimon with [Dracomon]/[Examon] in their texts delete 1 your opponent's Digimon in battle, delete 1 of their suspended Digimon or Tamers.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT20-044 Delete 1 of your opponent's suspended Digimon or Tamers")
        effect4.set_effect_description("[All Turns] (Once Per Turn) When any of your Digimon with [Dracomon]/[Examon] in their texts delete 1 your opponent's Digimon in battle, delete 1 of their suspended Digimon or Tamers.")
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("Delete_BT20_044")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Dracomon' in text or 'Examon' in text):
                    return False
            else:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndBattle
        # [All Turns] (Once Per Turn) When any of your Digimon with [Dracomon]/[Examon] in their texts delete 1 your opponent's Digimon in battle, delete 1 of their suspended Digimon or Tamers.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT20-044 Delete 1 of your opponent's suspended Digimon or Tamers")
        effect5.set_effect_description("[All Turns] (Once Per Turn) When any of your Digimon with [Dracomon]/[Examon] in their texts delete 1 your opponent's Digimon in battle, delete 1 of their suspended Digimon or Tamers.")
        effect5.is_inherited_effect = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("Delete_ESS_BT20_044")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Dracomon' in text or 'Examon' in text):
                    return False
            else:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
