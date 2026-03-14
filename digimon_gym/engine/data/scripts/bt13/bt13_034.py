from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_034(CardScript):
    """BT13-034 Kudamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Reveal the top 3 cards of your deck. Add 1 yellow Digimon card with the [Vaccine] trait and 1 yellow Tamer card among them to the hand. Place the rest at the bottom of the deck in any order.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT13-034 Reveal the top 3 cards of deck")
        effect0.set_effect_description("[On Play] Reveal the top 3 cards of your deck. Add 1 yellow Digimon card with the [Vaccine] trait and 1 yellow Tamer card among them to the hand. Place the rest at the bottom of the deck in any order.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 yellow Vaccine Digimon and 1 yellow Tamer"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter_0(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Vaccine' in t for t in traits)
            def reveal_filter_1(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                if not ('Yellow' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_reveal_and_select_multi(
                player, 3, [(reveal_filter_0, 'hand'), (reveal_filter_1, 'hand')],
                remaining_placement='deck_bottom', is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking][Once per Turn] If there're 6 or fewer total cards in both players' security stacks, 1 of your opponent�f Digimon gets -2000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnUseAttack)
        effect1.set_effect_name("BT13-034 DP -2000")
        effect1.set_effect_description("[When Attacking][Once per Turn] If there're 6 or fewer total cards in both players' security stacks, 1 of your opponent�f Digimon gets -2000 DP for the turn.")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DP-2000_BT13_034")
        effect1.is_on_attack = True
        effect1.dp_modifier = -2000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            enemy = player.enemy
            if not enemy:
                return False
            total_sec = len(player.security_cards) + len(enemy.security_cards)
            if total_sec > 6:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -2000 to 1 opponent Digimon for the turn"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....interfaces.modifiers import ModifierType
            def target_filter(p):
                return p.is_digimon
            def on_target(target_perm):
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda: -2000,
                    expiry='end_of_turn',
                )
            game.effect_select_opponent_permanent(
                player, on_target, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
