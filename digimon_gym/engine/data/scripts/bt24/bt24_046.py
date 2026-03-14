from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_046(CardScript):
    """BT24-046 Garurumon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: named [Gabumon] OR Lv.3 with [TS] trait, cost 2
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-046 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Gabumon"
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent is None:
                return False
            top = getattr(permanent, 'top_card', None)
            if top is None:
                return False
            has_name = any('Gabumon' in n for n in getattr(top, 'card_names', []))
            has_ts = any('TS' in t for t in getattr(top, 'card_traits', []))
            is_lv3 = getattr(top, 'level', 0) == 3
            return has_name or (is_lv3 and has_ts)
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: jamming
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-046 Jamming")
        effect1.set_effect_description("Jamming")
        effect1._is_jamming = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Shared process for On Play / When Digivolving: Suspend 1 opponent's Digimon
        def make_shared_process():
            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def target_filter(p):
                    return (getattr(p, 'is_digimon', False) and
                            p in player.enemy.battle_area)

                def on_suspend(target_perm):
                    target_perm.suspend()

                game.effect_select_opponent_permanent(
                    player, on_suspend, filter_fn=target_filter, is_optional=False
                )
            return process

        # Timing: EffectTiming.OnEnterFieldAnyone — On Play
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-046 Suspend 1 opponent's Digimon")
        effect2.set_effect_description("[On Play] Suspend 1 of your opponent's Digimon.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(make_shared_process())
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone — When Digivolving
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-046 Suspend 1 opponent's Digimon")
        effect3.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(make_shared_process())
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack — Inherited [When Attacking] [Once Per Turn]
        # Suspend 1 of your opponent's Digimon.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT24-046 Suspend 1 opponent's Digimon")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon.")
        effect4.is_inherited_effect = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_046_Inherited")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Suspend 1 opponent's Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                return (getattr(p, 'is_digimon', False) and
                        p in player.enemy.battle_area)

            def on_suspend(target_perm):
                target_perm.suspend()

            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=False
            )

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
