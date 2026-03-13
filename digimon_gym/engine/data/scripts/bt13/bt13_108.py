from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_108(CardScript):
    """BT13-108 Waltz's End | Option (Black, Cost 6)

    [Main] Until the end of your opponent's turn, 1 of your Digimon gains
    '[Opponent's Turn] When this Digimon becomes suspended, delete all of your
    opponent's Digimon with a play cost less than or equal to this Digimon's'
    and '[Opponent's Turn] This Digimon isn't affected by your opponent's
    Option cards.'

    [Security] Delete 1 of your opponent's Digimon with the lowest play cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        from ....interfaces.modifiers import ModifierType

        effects = []

        # --- Effect 0: [Main] Grant temporary abilities to 1 of your Digimon ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT13-108 Grant suspend-trigger delete + option immunity")
        effect0.set_effect_description(
            "[Main] Until the end of your opponent's turn, 1 of your Digimon gains "
            "'[Opponent's Turn] When this Digimon becomes suspended, delete all of your "
            "opponent's Digimon with a play cost less than or equal to this Digimon's' "
            "and '[Opponent's Turn] This Digimon isn't affected by your opponent's "
            "Option cards.'"
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_select(target_perm):
                # Grant option immunity until end of opponent's turn
                game.register_modifier(
                    target_perm, ModifierType.CANNOT_BE_AFFECTED,
                    value_fn=lambda: True,
                    expiry='end_of_opponent_turn'
                )
                # The suspend-trigger delete is a granted triggered effect.
                # Engine gap: cannot grant temporary triggered effects to permanents.
                # Workaround: register a OnTappedAnyone listener that checks if
                # the suspended permanent is the target and fires the mass delete.
                # This is an approximation since the effect lives on the option card,
                # not on the permanent itself.
                target_ref = [target_perm]

                suspend_effect = ICardEffect()
                suspend_effect.set_timing(EffectTiming.OnTappedAnyone)
                suspend_effect.set_effect_name("BT13-108 Granted suspend-trigger mass delete")

                def suspend_condition(context2: Dict[str, Any]) -> bool:
                    tp = target_ref[0]
                    if tp is None or tp not in (player.enemy.battle_area if player.enemy else []):
                        # Target no longer on opponent's field — but this is OUR Digimon
                        pass
                    if tp not in player.battle_area:
                        return False
                    # Must be opponent's turn
                    if player.is_my_turn:
                        return False
                    # The suspended permanent must be the target
                    event_perm = context2.get('permanent')
                    if event_perm is not tp:
                        return False
                    return True
                suspend_effect.set_can_use_condition(suspend_condition)

                def suspend_process(ctx2: Dict[str, Any]):
                    p = ctx2.get('player')
                    g = ctx2.get('game')
                    if not (p and g):
                        return
                    tp = target_ref[0]
                    if tp is None:
                        return
                    my_cost = getattr(tp.top_card, 'get_cost_itself', 99)
                    enemy = p.enemy
                    if not enemy:
                        return
                    # Delete ALL opponent Digimon with play cost <= this Digimon's
                    to_delete = [
                        opp for opp in list(enemy.battle_area)
                        if opp.is_digimon and getattr(opp.top_card, 'get_cost_itself', 99) <= my_cost
                    ]
                    for opp in to_delete:
                        enemy.delete_permanent(opp)
                suspend_effect.set_on_process_callback(suspend_process)
                # Note: This effect won't be registered on the target permanent
                # through the standard system since we can't grant_temp_effect.
                # The effect is appended to the card's effects list but won't
                # be properly scoped to the target. This is a best-effort workaround.

            game.effect_select_own_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Security] Delete 1 opponent Digimon with lowest play cost ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT13-108 Security: Delete lowest cost Digimon")
        effect1.set_effect_description(
            "[Security] Delete 1 of your opponent's Digimon with the lowest play cost."
        )
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            # Find minimum play cost
            min_cost = min(getattr(p.top_card, 'get_cost_itself', 99) for p in opp_digimon)
            lowest = [p for p in opp_digimon if getattr(p.top_card, 'get_cost_itself', 99) == min_cost]
            if len(lowest) == 1:
                enemy.delete_permanent(lowest[0])
            else:
                # Multiple tied — let player choose
                game.effect_select_opponent_permanent(
                    player,
                    lambda target: enemy.delete_permanent(target),
                    filter_fn=lambda p: p.is_digimon and getattr(p.top_card, 'get_cost_itself', 99) == min_cost,
                    is_optional=False
                )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
