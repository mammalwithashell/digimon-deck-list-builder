from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_014(CardScript):
    """BT22-014 Gaiomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['Greymon']

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-014 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.5 for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_level = 5

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Also Treated As
        effect1 = ICardEffect()
        effect1.set_effect_name("BT22-014 Also treated as having [Greymon]")
        effect1.set_effect_description("Also Treated As")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: raid
        # Raid
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-014 Raid")
        effect2.set_effect_description("Raid")
        effect2.is_on_attack = True
        effect2._is_raid = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: reboot
        # Reboot
        effect3 = ICardEffect()
        effect3.set_effect_name("BT22-014 Reboot")
        effect3.set_effect_description("Reboot")
        effect3._is_reboot = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] You may unsuspend 1 of your opponent's Digimon. Then, this Digimon may attack.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT22-014 Unsuspend 1, then attack")
        effect4.set_effect_description("[When Digivolving] You may unsuspend 1 of your opponent's Digimon. Then, this Digimon may attack.")
        effect4.is_optional = True
        effect4.is_when_digivolving = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Unsuspend, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return True
            def on_unsuspend(target_perm):
                target_perm.unsuspend()
            game.effect_select_own_permanent(
                player, on_unsuspend, filter_fn=target_filter, is_optional=True)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [All Turns] [Once Per Turn] When attack targets change, this Digimon gains <Piercing> (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would.) and +5000 DP for the turn.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnAttackTargetChanged)
        effect5.set_effect_name("BT22-014 Gain Piercing, +5000 DP")
        effect5.set_effect_description("[All Turns] [Once Per Turn] When attack targets change, this Digimon gains <Piercing> (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would.) and +5000 DP for the turn.")
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("AllTurns_BT22-014")
        effect5._is_piercing = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: DP +5000, Gain Keyword Piercing"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(5000)
            if perm:
                perm.grant_keyword('_is_piercing')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
