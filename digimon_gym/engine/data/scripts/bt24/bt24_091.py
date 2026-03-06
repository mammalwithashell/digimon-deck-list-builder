from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_091(CardScript):
    """BT24-091 Tidal Stream"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-091 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-091 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # Unsuspend, Bounce, Effect Immunity
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("BT24-091 Bounce all opponent's lowest level. Unsuspend a Digimon. Then, you may link this card.")
        effect2.set_effect_description("Unsuspend, Bounce, Effect Immunity")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce all lowest-level opponent Digimon, unsuspend TS Digimon, link"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # 1. Return ALL of opponent's lowest level Digimon to hand
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            bounced_count = 0
            if opp_digimon:
                min_level = min(p.level for p in opp_digimon)
                to_bounce = [p for p in opp_digimon if p.level == min_level]
                for target in to_bounce:
                    if target in enemy.battle_area:
                        enemy.bounce_permanent_to_hand(target)
                        bounced_count += 1

            # 2. If this effect returned at least 1, unsuspend 1 of your [TS]-trait Digimon
            if bounced_count > 0:
                def ts_filter(p):
                    if not p.is_digimon:
                        return False
                    if p.top_card:
                        traits = getattr(p.top_card, 'card_traits', []) or []
                        if any('TS' in t for t in traits):
                            return True
                    return False

                def on_unsuspend(target_perm):
                    target_perm.unsuspend()

                game.effect_select_own_permanent(
                    player, on_unsuspend, filter_fn=ts_filter, is_optional=True)

            # 3. Link this card to 1 of your Digimon
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level Digimon to the hand.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnAllyAttack)
        effect3.set_effect_name("BT24-091 Bounce 1 opponent's lowest level Digimon.")
        effect3.set_effect_description("[When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level Digimon to the hand.")
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WA_BT24-091")
        effect3.is_on_attack = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Bounce 1 opponent's lowest level Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Find the lowest level among opponent's Digimon
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            if not opp_digimon:
                return
            min_level = min(p.level for p in opp_digimon)

            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level == min_level

            def on_bounce(target_perm):
                if enemy:
                    enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
