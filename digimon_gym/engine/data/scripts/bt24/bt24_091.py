from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_091(CardScript):
    """BT24-091 Tidal Stream | Option (Blue, Cost 5)

    While you have an [TS] trait Digimon or Tamer on the field, you can ignore
    this card's color requirements.
    [Security] Activate this card's [Main] effects.
    [Main] Return all of your opponent's lowest level Digimon to the hand.
    If this effect returned at least 1, 1 of your [TS] trait Digimon
    unsuspends. Then, you may link this card to 1 of your Digimon without paying
    the cost.
    [Link] [TS] trait: Cost 3
    [When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level
    Digimon to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        # Bypass color requirement (TS trait condition is dynamic but
        # any deck running this card will have TS trait permanents)
        card._match_color_requirement = False
        effects = []

        def _main_effect_process(ctx: Dict[str, Any]):
            """Shared process for [Main] and [Security] effects."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Return all opponent's lowest level Digimon to hand (C# uses bounce)
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            bounced_count = 0
            if opp_digimon:
                min_level = min(p.level for p in opp_digimon)
                targets = [p for p in opp_digimon if p.level == min_level]
                for target in list(targets):
                    if target in enemy.battle_area:
                        enemy.bounce_permanent_to_hand(target)
                        bounced_count += 1

            # If at least 1 was returned, unsuspend 1 of your [TS] Digimon
            if bounced_count > 0:
                def ts_filter(p):
                    if not p.is_digimon:
                        return False
                    return any('TS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))

                def on_unsuspend(target_perm):
                    target_perm.unsuspend()

                game.effect_select_own_permanent(
                    player, on_unsuspend, filter_fn=ts_filter, is_optional=False
                )

            # Then optionally link this card to 1 of your Digimon
            game.effect_link_to_permanent(player, card, is_optional=True)

        # --- Effect 0: [Main] Return lowest level, unsuspend TS, link ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name(
            "BT24-091 Return all opponent's lowest-level Digimon to hand, "
            "unsuspend a [TS] Digimon, then link"
        )
        effect0.set_effect_description(
            "[Main] Return all of your opponent's lowest level Digimon to the "
            "hand. If this effect returned at least 1, 1 of your [TS] trait Digimon "
            "unsuspends. Then, you may link this card to 1 of your Digimon on the field "
            "without paying the cost."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_main_effect_process)
        effects.append(effect0)

        # --- Effect 1: [Security] Activate [Main] effects ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT24-091 Security: Activate [Main] effects")
        effect1.set_effect_description("[Security] Activate this card's [Main] effects.")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_main_effect_process)
        effects.append(effect1)

        # --- Effect 2: [Link] [TS] trait: Cost 3
        #    [When Attacking] [Once Per Turn] Return 1 opponent's lowest level Digimon to hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnUseAttack)
        effect2.set_effect_name("BT24-091 Bounce 1 opponent's lowest level Digimon")
        effect2.set_effect_description(
            "[When Attacking] [Once Per Turn] Return 1 of your opponent's lowest level "
            "Digimon to the hand."
        )
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("WA_BT24-091")
        effect2.is_linked_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            opp_digimon = [p for p in enemy.battle_area if p.is_digimon and p.level is not None]
            if not opp_digimon:
                return
            min_level = min(p.level for p in opp_digimon)

            def target_filter(p):
                return p.is_digimon and p.level is not None and p.level == min_level

            def on_bounce(target_perm):
                enemy.bounce_permanent_to_hand(target_perm)

            game.effect_select_opponent_permanent(
                player, on_bounce, filter_fn=target_filter, is_optional=False
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
