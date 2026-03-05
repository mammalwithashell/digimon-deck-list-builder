from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_111(CardScript):
    """BT13-111 Gallantmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When you would play this card, if you have no Digimon in play,
        # reduce the play cost by 2 for every 5 cards in both players' trashes.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT13-111 Reduce play cost by 2 per 5 trash cards")
        effect0.set_effect_description("When you would play this card, if you have no Digimon in play, reduce the play cost by 2 for every 5 cards in both players' trashes.")

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            # Must have no Digimon in play
            if any(p.is_digimon for p in owner.battle_area):
                return False
            return True

        effect0.set_can_use_condition(condition0)
        effect0._cost_reduction_value_fn = (
            lambda context: 2 * ((len(getattr(card, 'owner', None).trash_cards if getattr(card, 'owner', None) else []) +
                                   len(getattr(card, 'owner', None).enemy.trash_cards if (getattr(card, 'owner', None) and getattr(card, 'owner', None).enemy) else [])) // 5)
        )
        effects.append(effect0)

        # Factory effect: rush
        # Rush
        effect1 = ICardEffect()
        effect1.set_effect_name("BT13-111 Rush")
        effect1.set_effect_description("Rush")
        effect1._is_rush = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT13-111 Delete 1 Digimon with 6000 DP or less, or delete 1 Digimon with 13000 DP or more")
        effect2.set_effect_description("[On Play] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def _gallantmon_delete(player, game):
            """Delete 1 opponent Digimon with 6000 DP or less. If none deleted, delete 1 with 13000 DP or more."""
            enemy = player.enemy if player else None
            if not enemy:
                return

            # First try: delete 1 with 6000 DP or less
            low_filter = lambda p: p.is_digimon and p.dp is not None and p.dp <= 6000
            has_low = any(low_filter(p) for p in enemy.battle_area)

            if has_low:
                def on_delete_low(target_perm):
                    enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete_low, filter_fn=low_filter, is_optional=False)
            else:
                # Fallback: delete 1 with 13000 DP or more
                high_filter = lambda p: p.is_digimon and p.dp is not None and p.dp >= 13000
                has_high = any(high_filter(p) for p in enemy.battle_area)
                if has_high:
                    def on_delete_high(target_perm):
                        enemy.delete_permanent(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_delete_high, filter_fn=high_filter, is_optional=False)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete 6000 DP or less, else 13000 DP or more"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _gallantmon_delete(player, game)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT13-111 Delete 1 Digimon with 6000 DP or less, or delete 1 Digimon with 13000 DP or more")
        effect3.set_effect_description("[When Digivolving] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete 6000 DP or less, else 13000 DP or more"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _gallantmon_delete(player, game)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnAllyAttack)
        effect4.set_effect_name("BT13-111 Delete 1 Digimon with 6000 DP or less, or delete 1 Digimon with 13000 DP or more")
        effect4.set_effect_description("[When Attacking] Delete 1 of your opponent's Digimon with 6000 DP or less. If no opponent's Digimon was deleted by this effect, delete 1 of their Digimon with 13000 DP or more.")
        effect4.is_on_attack = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Delete 6000 DP or less, else 13000 DP or more"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                _gallantmon_delete(player, game)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
