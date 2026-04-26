from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_114(CardScript):
    """P-114 Diaboromon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blast_digivolve
        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("P-114 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may play 1 [Diaboromon] Token without paying its memory cost (Diaboromon Tokens are level 6 white Digimon with a memory cost of 14, 3000 DP, and are Mega form, Unidentified type, and Unknown attribute).
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-114 Play 1 Diaboromon Token")
        effect1.set_effect_description("[When Digivolving] You may play 1 [Diaboromon] Token without paying its memory cost (Diaboromon Tokens are level 6 white Digimon with a memory cost of 14, 3000 DP, and are Mega form, Unidentified type, and Unknown attribute).")
        effect1.is_optional = True
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
            game = ctx.get('game')
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] You may play 1 [Diaboromon] Token without paying its memory cost (Diaboromon Tokens are level 6 white Digimon with a memory cost of 14, 3000 DP, and are Mega form, Unidentified type, and Unknown attribute).
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("P-114 Play 1 Diaboromon Token")
        effect2.set_effect_description("[When Attacking] You may play 1 [Diaboromon] Token without paying its memory cost (Diaboromon Tokens are level 6 white Digimon with a memory cost of 14, 3000 DP, and are Mega form, Unidentified type, and Unknown attribute).")
        effect2.is_optional = True
        effect2.is_on_attack = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] [Once Per Turn] When another Digimon is played by an effect, you may delete 1 of your opponent's Digimon with a play cost of 3 or less. For each [Diaboromon] you have in play, increase the maximum cost of the Digimon card you can delete with this effect by 2.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("P-114 Delete 1 Digimon")
        effect3.set_effect_description("[All Turns] [Once Per Turn] When another Digimon is played by an effect, you may delete 1 of your opponent's Digimon with a play cost of 3 or less. For each [Diaboromon] you have in play, increase the maximum cost of the Digimon card you can delete with this effect by 2.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Delete_P_114")
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggers when another Digimon is played (by an effect)
            trigger_perm = context.get('played_permanent') or context.get('event_permanent')
            own_perm = card.permanent_of_this_card() if card else None
            if trigger_perm is None or trigger_perm is own_perm:
                return False
            if not (trigger_perm.is_digimon):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Delete opponent Digimon with play cost <= 3 + 2 per Diaboromon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            # Calculate max play cost: 3 + 2 per Diaboromon
            diaboromon_count = sum(
                1 for p in player.battle_area
                if p.is_digimon and p.contains_card_name('Diaboromon')
            )
            max_cost = 3 + (diaboromon_count * 2)

            def target_filter(p):
                if not p.is_digimon:
                    return False
                top = p.top_card
                if top is None:
                    return False
                return (top.get_cost_itself or 0) <= max_cost

            def on_delete(target_perm):
                enemy.delete_permanent(target_perm)

            has_target = any(target_filter(p) for p in enemy.battle_area)
            if has_target:
                game.effect_select_opponent_permanent(
                    player, on_delete, filter_fn=target_filter, is_optional=True)
            else:
                game.logger.log(
                    f"[P-114] No opponent Digimon with play cost <= {max_cost} to delete."
                )

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
