from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT16_055(CardScript):
    """BT16-055 Namakemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT16-055 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Pulsemon"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have 3 or more security cards, 1 of your Digimon can't have its DP reduced by your opponent's effects, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn. If you have 3 or fewer security cards, 1 of your Digimon gains <Blocker> and <Reboot> until the end of your opponent's turn.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT16-055 Digimon can't be DP reduced or De-Digivolved, Digimon gains <Rush> and <Blocker>")
        effect1.set_effect_description("[On Play] If you have 3 or more security cards, 1 of your Digimon can't have its DP reduced by your opponent's effects, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn. If you have 3 or fewer security cards, 1 of your Digimon gains <Blocker> and <Reboot> until the end of your opponent's turn.")
        effect1.is_on_play = True
        effect1._is_immune_dp_minus = True
        effect1._is_reboot = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Conditional grant based on security count."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            security_count = len(player.security_cards)
            if security_count >= 3:
                def on_grant(target_perm):
                    target_perm.grant_keyword('_is_immune_dp_minus')
                    target_perm.grant_keyword('_is_immune_de_digivolve')
            else:
                def on_grant(target_perm):
                    target_perm.grant_keyword('_is_blocker')
                    target_perm.grant_keyword('_is_reboot')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] If you have 3 or more security cards, 1 of your Digimon can't have its DP reduced by your opponent's effects, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn. If you have 3 or fewer security cards, 1 of your Digimon gains <Blocker> and <Reboot> until the end of your opponent's turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT16-055 Digimon can't be DP reduced or De-Digivolved, Digimon gains <Rush> and <Blocker>")
        effect2.set_effect_description("[When Digivolving] If you have 3 or more security cards, 1 of your Digimon can't have its DP reduced by your opponent's effects, and isn't affected by <De-Digivolve> effects until the end of your opponent's turn. If you have 3 or fewer security cards, 1 of your Digimon gains <Blocker> and <Reboot> until the end of your opponent's turn.")
        effect2.is_when_digivolving = True
        effect2._is_immune_dp_minus = True
        effect2._is_reboot = True
        effect2._is_blocker = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Conditional grant based on security count."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            security_count = len(player.security_cards)
            if security_count >= 3:
                def on_grant(target_perm):
                    target_perm.grant_keyword('_is_immune_dp_minus')
                    target_perm.grant_keyword('_is_immune_de_digivolve')
            else:
                def on_grant(target_perm):
                    target_perm.grant_keyword('_is_blocker')
                    target_perm.grant_keyword('_is_reboot')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: dp_modifier
        # DP modifier
        effect3 = ICardEffect()
        effect3.set_effect_name("BT16-055 DP modifier")
        effect3.set_effect_description("DP modifier")
        effect3.is_inherited_effect = True
        effect3.dp_modifier = 0

        def condition3(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Pulsemon' in text):
                    return False
            else:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
