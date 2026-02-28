from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_093(CardScript):
    """BT21-093 Raging Serpentine"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be used, if your opponent has 3 or fewer security cards, reduce the use cost by 
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-093 Reduce Play Cost -4")
        effect0.set_effect_description("When this card would be used, if your opponent has 3 or fewer security cards, reduce the use cost by ")
        effect0.set_hash_string("PlayCost-4_BT21_093")
        effect0.cost_reduction = 4

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -4"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Cost -4
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-093 Play Cost -4")
        effect1.set_effect_description("Cost -4")
        effect1.cost_reduction = 4

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Cost -4"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 4 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's highest DP Digimon. Then, place this card in the battle area.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-093 Delete 1 of your opponent's Digimon with the highest DP")
        effect2.set_effect_description("[Main] Delete 1 of your opponent's highest DP Digimon. Then, place this card in the battle area.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: delay
        # Delay
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-093 Delay")
        effect3.set_effect_description("Delay")
        effect3._is_delay = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Reptile' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Dragonkin' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] When your opponent's security stack is removed from, <Delay> (After this card is placed, by trashing it the next turn or later, activate the effect below).\r\n・1 of your [Reptile]/[Dragonkin] trait Digimon may digivolve into a [Reptile]/[Dragonkin] trait Digimon card in the hand without paying the cost.\r\n
        effect4 = ICardEffect()
        effect4.set_effect_name("BT21-093 1 of your [Reptile]/[Dragonkin] trait Digimon may digivolve")
        effect4.set_effect_description("[All Turns] When your opponent's security stack is removed from, <Delay> (After this card is placed, by trashing it the next turn or later, activate the effect below).\\r\\n・1 of your [Reptile]/[Dragonkin] trait Digimon may digivolve into a [Reptile]/[Dragonkin] trait Digimon card in the hand without paying the cost.\\r\\n")
        effect4.is_optional = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Digivolve"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not (any('Reptile' in _t or 'Dragonkin' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Delete 1 of your opponent's Digimon with the highest DP.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT21-093 Delete 1 of your opponent's Digimon with the highest DP")
        effect5.set_effect_description("[Security] Delete 1 of your opponent's Digimon with the highest DP.")
        effect5.is_security_effect = True
        effect5.is_security_effect = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
