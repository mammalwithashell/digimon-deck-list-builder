from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_102(CardScript):
    """BT14-102 Angemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-102 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: alternate source for cost 2
        effect0._alt_digi_cost = 2

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [When Attacking] By deleting this Digimon, activate 1 of the effects below: - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack. - 1 of your opponent's Digimon gets -5000 DP for the turn.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-102 Delete this Digimon to select effects")
        effect1.set_effect_description("[When Attacking] By deleting this Digimon, activate 1 of the effects below: - Place 1 of your opponent's Digimon with the [Virus] trait at the bottom of their security stack. - 1 of your opponent's Digimon gets -5000 DP for the turn.")
        effect1.is_optional = True
        effect1.is_on_attack = True
        effect1.dp_modifier = -5000

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP -5000"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # DP change targets opponent digimon
            enemy = player.enemy if player else None
            if enemy and enemy.battle_area:
                dp_targets = [p for p in enemy.battle_area if p.is_digimon and p.dp is not None]
                if dp_targets:
                    target = min(dp_targets, key=lambda p: p.dp)
                    target.change_dp(-5000)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place this card at the bottom of your security stack. Then, if you have a Tamer, you may hatch in your breeding area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-102 Place this card at the bottom of security and hatch")
        effect2.set_effect_description("[On Deletion] Place this card at the bottom of your security stack. Then, if you have a Tamer, you may hatch in your breeding area.")
        effect2.is_on_deletion = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack. 
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-102 Place 1 card from hand at the bottom security")
        effect3.set_effect_description("[On Deletion] Place 1 yellow card with the [Vaccine] trait from your hand at the bottom of your security stack. ")
        effect3.is_inherited_effect = True
        effect3.is_optional = True
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Trash From Hand, Add To Security"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Vaccine' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)
            # Add top card of deck to security
            if player:
                player.recovery(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
