from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_008(CardScript):
    """EX10-008 MetalGreymon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-008 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 3
        effect0._alt_digi_cost = 3

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: reboot
        # Reboot
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-008 Reboot")
        effect1.set_effect_description("Reboot")
        effect1._is_reboot = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Give 1 of your opponent's Digimon <Collision> (During this Digimon's attack, all of your opponent's Digimon gain <Blocker>, and must block if possible.) and '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-008 Give 1 opponent digimon <Collision> and [Start of Your Main Phase] This Digimon attacks")
        effect2.set_effect_description("[On Play] Give 1 of your opponent's Digimon <Collision> (During this Digimon's attack, all of your opponent's Digimon gain <Blocker>, and must block if possible.) and '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Give 1 of your opponent's Digimon <Collision> (During this Digimon's attack, all of your opponent's Digimon gain <Blocker>, and must block if possible.) and '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-008 Give 1 opponent digimon <Collision> and [Start of Your Main Phase] This Digimon attacks")
        effect3.set_effect_description("[When Digivolving] Give 1 of your opponent's Digimon <Collision> (During this Digimon's attack, all of your opponent's Digimon gain <Blocker>, and must block if possible.) and '[Start of Your Main Phase] This Digimon attacks.' until their turn ends.")
        effect3.is_when_digivolving = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnAttackTargetChanged
        # [Opponent's Turn] [Once Per Turn] When attack targets change, if this Digimon has [Greymon] in its name, trash your opponent's top security card.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-008 Trash opponent top security")
        effect4.set_effect_description("[Opponent's Turn] [Once Per Turn] When attack targets change, if this Digimon has [Greymon] in its name, trash your opponent's top security card.")
        effect4.is_inherited_effect = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("EX10_008_AttackTargetChanged")

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Destroy Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop()
                        enemy.trash_cards.append(trashed)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
