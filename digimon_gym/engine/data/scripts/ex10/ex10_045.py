from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_045(CardScript):
    """EX10-045 Tuwarmon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-045 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Damemon] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "Damemon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Damemon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-045 Effect")
        effect1.set_effect_description("Effect")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: collision
        # Collision
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-045 Collision")
        effect2.set_effect_description("Collision")
        effect2._is_collision = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Factory effect: rush
        # Rush
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-045 Rush")
        effect3.set_effect_description("Rush")
        effect3._is_rush = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX10-045 By trashing 1 source, 1 digimon gains Blocker, Retaliation")
        effect4.set_effect_description("Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation")
        effect4.set_hash_string("OPWDWA_EX10_045")
        effect4.is_on_play = True
        effect4._is_blocker = True
        effect4._is_retaliation = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_blocker')
                perm.grant_keyword('_is_retaliation')

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect5.set_effect_name("EX10-045 By trashing 1 source, 1 digimon gains Blocker, Retaliation")
        effect5.set_effect_description("Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation")
        effect5.set_hash_string("OPWDWA_EX10_045")
        effect5.is_when_digivolving = True
        effect5._is_blocker = True
        effect5._is_retaliation = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_blocker')
                perm.grant_keyword('_is_retaliation')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.OnAllyAttack
        # Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnAllyAttack)
        effect6.set_effect_name("EX10-045 By trashing 1 source, 1 digimon gains Blocker, Retaliation")
        effect6.set_effect_description("Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation")
        effect6.set_hash_string("OPWDWA_EX10_045")
        effect6.is_on_attack = True
        effect6._is_blocker = True
        effect6._is_retaliation = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on attack — validated by engine timing
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Trash Digivolution Cards, Gain Keyword Blocker, Gain Keyword Retaliation"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash digivolution cards from this permanent
            if perm and not perm.has_no_digivolution_cards:
                trashed = perm.trash_digivolution_cards(1)
                if player:
                    player.trash_cards.extend(trashed)
            if perm:
                perm.grant_keyword('_is_blocker')
                perm.grant_keyword('_is_retaliation')

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # Factory effect: save
        # Save
        effect7 = ICardEffect()
        effect7.set_effect_name("EX10-045 Save")
        effect7.set_effect_description("Save")
        effect7.is_on_deletion = True
        effect7._is_save = True

        def condition7(context: Dict[str, Any]) -> bool:
            return True
        effect7.set_can_use_condition(condition7)
        effects.append(effect7)

        # Timing: EffectTiming.OnDigivolutionCardDiscarded
        # When effects trash this card from a [Bagra Army] trait Digimon's digivolution cards, <Draw 1>
        effect8 = ICardEffect()
        effect8.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect8.set_effect_name("EX10-045 <Draw 1>")
        effect8.set_effect_description("When effects trash this card from a [Bagra Army] trait Digimon's digivolution cards, <Draw 1>")
        effect8.is_inherited_effect = True

        effect = effect8  # alias for condition closure
        def condition8(context: Dict[str, Any]) -> bool:
            return True

        effect8.set_can_use_condition(condition8)

        def process8(ctx: Dict[str, Any]):
            """Action: Draw 1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)

        effect8.set_on_process_callback(process8)
        effects.append(effect8)

        return effects
