from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_091(CardScript):
    """Auto-transpiled from DCGO BT24_091.cs"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Effect
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-091 Ignore color requirements")
        effect0.set_effect_description("Effect")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-091 Bounce all opponent's lowest level. Unsuspend a Digimon. Then, you may link this card.")
        effect1.set_effect_description("Effect")

        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Bounce all opponent's lowest level Digimon. Unsuspend 1 [TS]. Then link."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                # Find lowest level among opponent's Digimon
                digimons = [p for p in enemy.battle_area if p.is_digimon]
                if digimons:
                    min_level = min(p.level for p in digimons)
                    targets = [p for p in digimons if p.level == min_level]
                    for t in targets:
                        player.bounce_permanent_to_hand(t)
                        game.logger.log(f"[Effect] Bounced {t.top_card.card_names[0] if t.top_card else 'Unknown'}")
            # Unsuspend 1 of your [TS] Digimon
            ts_suspended = [p for p in player.battle_area
                            if p.is_digimon and p.is_suspended and p.has_trait('TS')]
            if ts_suspended:
                ts_suspended[0].unsuspend()
                game.logger.log(f"[Effect] Unsuspended {ts_suspended[0].top_card.card_names[0] if ts_suspended[0].top_card else 'Unknown'}")
            # Then, may link this card
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level Digimon to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-091 Bounce 1 opponent's lowest level Digimon.")
        effect2.set_effect_description("[When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level Digimon to the hand.")
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WA_BT24-091")
        effect2.is_on_attack = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on attack — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Bounce 1 opponent's lowest level Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                digimons = [p for p in enemy.battle_area if p.is_digimon]
                if digimons:
                    min_level = min(p.level for p in digimons)

                    def on_selected(target_perm):
                        player.bounce_permanent_to_hand(target_perm)
                        game.logger.log(f"[Effect] Bounced {target_perm.top_card.card_names[0] if target_perm.top_card else 'Unknown'}")

                    game.effect_select_opponent_permanent(
                        player, on_selected,
                        filter_fn=lambda p: p.is_digimon and p.level == min_level)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
