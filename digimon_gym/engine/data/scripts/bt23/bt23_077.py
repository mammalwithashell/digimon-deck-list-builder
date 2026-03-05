from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_077(CardScript):
    """BT23-077 Sistermon Ciel | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-077 Also treated as [Sistermon Noir]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-077 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-077 Delete 1 Digimon with a play cost of 4 or less")
        effect2.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon and p.top_card and p.top_card.get_cost_itself <= 4
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnTappedAnyone
        # [All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("BT23-077 <De-Digivolve 1> 1 of your opponent's Digimon")
        effect3.set_effect_description("[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when THIS Digimon suspends
            ctx_perm = context.get('permanent')
            owner_perm = card.permanent_of_this_card() if card else None
            if owner_perm and ctx_perm and ctx_perm is not owner_perm:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: De Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon and len(p.card_sources) > 1,
                is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Inherited effects ---
        # All three main effects (Blocker, On Play delete, When suspends de-digivolve)
        # are also inherited effects per card text.

        # Inherited: Blocker
        effect_inh_blocker = ICardEffect()
        effect_inh_blocker.set_effect_name("BT23-077 Blocker (Inherited)")
        effect_inh_blocker.set_effect_description("Blocker")
        effect_inh_blocker._is_blocker = True
        effect_inh_blocker.is_inherited_effect = True

        def condition_inh_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_inh_blocker.set_can_use_condition(condition_inh_blocker)
        effects.append(effect_inh_blocker)

        # Inherited: [On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.
        effect_inh_delete = ICardEffect()
        effect_inh_delete.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_inh_delete.set_effect_name("BT23-077 Delete 1 Digimon with a play cost of 4 or less (Inherited)")
        effect_inh_delete.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with a play cost of 4 or less.")
        effect_inh_delete.is_on_play = True
        effect_inh_delete.is_inherited_effect = True

        def condition_inh_delete(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_inh_delete.set_can_use_condition(condition_inh_delete)

        def process_inh_delete(ctx: Dict[str, Any]):
            """Action: Delete (Inherited)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon and p.top_card and p.top_card.get_cost_itself <= 4
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect_inh_delete.set_on_process_callback(process_inh_delete)
        effects.append(effect_inh_delete)

        # Inherited: [All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect_inh_dedigivolve = ICardEffect()
        effect_inh_dedigivolve.set_timing(EffectTiming.OnTappedAnyone)
        effect_inh_dedigivolve.set_effect_name("BT23-077 <De-Digivolve 1> (Inherited)")
        effect_inh_dedigivolve.set_effect_description("[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon.")
        effect_inh_dedigivolve.is_inherited_effect = True

        def condition_inh_dedigivolve(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only trigger when THIS Digimon suspends
            ctx_perm = context.get('permanent')
            owner_perm = card.permanent_of_this_card() if card else None
            if owner_perm and ctx_perm and ctx_perm is not owner_perm:
                return False
            return True
        effect_inh_dedigivolve.set_can_use_condition(condition_inh_dedigivolve)

        def process_inh_dedigivolve(ctx: Dict[str, Any]):
            """Action: De Digivolve (Inherited)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon and len(p.card_sources) > 1,
                is_optional=False)

        effect_inh_dedigivolve.set_on_process_callback(process_inh_dedigivolve)
        effects.append(effect_inh_dedigivolve)

        return effects
