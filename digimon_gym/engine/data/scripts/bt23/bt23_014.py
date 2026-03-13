from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....interfaces.modifiers import ModifierType
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_014(CardScript):
    """BT23-014 Gallantmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-014 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT23-014 Your opponent effects cannot play digimon or tamers from trash until their turn ends")
        effect1.set_effect_description("[On Play] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Until opponent's turn ends, their effects can't play Digimon/Tamers from trash."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return
            # CANNOT_PLAY_CARD modifier that blocks Digimon and Tamer cards
            # coming from the opponent's trash. Expires end of opponent's next turn.
            # The condition receives (target_permanent, ctx) where ctx['card'] is the card
            # being played. We gate on the card being a Digimon/Tamer in the opponent's trash.
            def _from_trash_digimon_or_tamer(target_perm, c):
                being_played = c.get('card')
                if not being_played:
                    return False
                if not (getattr(being_played, 'is_digimon', False) or
                        getattr(being_played, 'is_tamer', False)):
                    return False
                # Only block plays originating from enemy trash
                return being_played in enemy.trash_cards

            game.register_modifier(
                perm,
                ModifierType.CANNOT_PLAY_CARD,
                condition=_from_trash_digimon_or_tamer,
                source_effect=effect1,
                expiry='end_of_opponent_turn',
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT23-014 Your opponent effects cannot play digimon or tamers from trash until their turn ends")
        effect2.set_effect_description("[When Digivolving] Until your opponent's turn ends, their effects can't play Digimon or Tamers from the trash.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Until opponent's turn ends, their effects can't play Digimon/Tamers from trash."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy
            if not enemy:
                return

            def _from_trash_digimon_or_tamer(target_perm, c):
                being_played = c.get('card')
                if not being_played:
                    return False
                if not (getattr(being_played, 'is_digimon', False) or
                        getattr(being_played, 'is_tamer', False)):
                    return False
                return being_played in enemy.trash_cards

            game.register_modifier(
                perm,
                ModifierType.CANNOT_PLAY_CARD,
                condition=_from_trash_digimon_or_tamer,
                source_effect=effect2,
                expiry='end_of_opponent_turn',
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-014 Delete")
        effect3.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete — 8000 DP base + 2000 per opponent's Digimon/Tamer"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Calculate max DP: 8000 base + 2000 for each opponent Digimon and Tamer
            max_dp = 8000 + 2000 * len([p for p in enemy.battle_area if p.is_digimon or p.is_tamer])
            def target_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= max_dp
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-014 Delete")
        effect4.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.")
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete — 8000 DP base + 2000 per opponent's Digimon/Tamer"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Calculate max DP: 8000 base + 2000 for each opponent Digimon and Tamer
            max_dp = 8000 + 2000 * len([p for p in enemy.battle_area if p.is_digimon or p.is_tamer])
            def target_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= max_dp
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("BT23-014 Delete")
        effect5.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 8000 DP or less. For each of their Digimon and Tamers, add 2000 to this DP deletion effect's maximum.")
        effect5.is_on_attack = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete — 8000 DP base + 2000 per opponent's Digimon/Tamer"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            # Calculate max DP: 8000 base + 2000 for each opponent Digimon and Tamer
            max_dp = 8000 + 2000 * len([p for p in enemy.battle_area if p.is_digimon or p.is_tamer])
            def target_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= max_dp
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
