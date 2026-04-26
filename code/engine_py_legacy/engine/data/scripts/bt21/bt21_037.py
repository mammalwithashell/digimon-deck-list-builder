from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_037(CardScript):
    """BT21-037 Lighdramon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-037 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Veemon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Veemon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Veemon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: piercing
        # Piercing
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-037 Piercing")
        effect1.set_effect_description("<Piercing>")
        effect1._is_piercing = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: armor_purge
        # Armor Purge
        effect1b = ICardEffect()
        effect1b.set_effect_name("BT21-037 Armor Purge")
        effect1b.set_effect_description("Armor Purge")
        effect1b._is_armor_purge = True

        def condition1b(context: Dict[str, Any]) -> bool:
            return True
        effect1b.set_can_use_condition(condition1b)
        effects.append(effect1b)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Suspend 1 of your opponent's Digimon. Then, this Digimon gets +2000 DP until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT21-037 Suspend 1 of your opponent's Digimon and get DP +2000")
        effect2.set_effect_description("[When Digivolving] Suspend 1 of your opponent's Digimon. Then, this Digimon gets +2000 DP until your opponent's turn ends.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Suspend 1 opponent Digimon, then DP +2000 (C# order)."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Suspend 1 of opponent's Digimon (must be able to suspend)
            def target_filter(p):
                return p.is_digimon and not p.is_suspended

            has_targets = any(
                target_filter(p) for p in (player.enemy.battle_area if player.enemy else [])
            )
            if has_targets:
                def on_suspend(target_perm):
                    target_perm.suspend()

                game.effect_select_opponent_permanent(
                    player, on_suspend, filter_fn=target_filter, is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to suspend.")

            # Step 2: This Digimon gets +2000 DP until opponent's turn ends
            if perm:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CHANGE_DP,
                    value_fn=lambda current, target, ctx: current + 2000,
                    expiry='end_of_opponent_turn',
                )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
