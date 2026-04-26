from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_073(CardScript):
    """BT19-073 LordKnightmon (X Antibody) | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-073 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [LordKnightmon] for cost 1
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "LordKnightmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('LordKnightmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: collision
        # Collision
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-073 Collision")
        effect1.set_effect_description("Collision")
        effect1._is_collision = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon for each of your Digimon. Then, 1 of your opponent's Digimon can't digivolve until the end of their turn.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT19-073 <De-Digivolve 1> 1 of your opponent's Digimon for each of your Digimon")
        effect2.set_effect_description("[When Digivolving] <De-Digivolve 1> 1 of your opponent's Digimon for each of your Digimon. Then, 1 of your opponent's Digimon can't digivolve until the end of their turn.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: De Digivolve, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve, filter_fn=lambda p: p.is_digimon, is_optional=False)
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: alliance
        # Alliance
        effect3 = ICardEffect()
        effect3.set_effect_name("BT19-073 Alliance")
        effect3.set_effect_description("Alliance")
        effect3._is_alliance = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: dp_modifier_all
        # All your Digimon DP modifier
        effect4 = ICardEffect()
        effect4.set_effect_name("BT19-073 All your Digimon DP modifier")
        effect4.set_effect_description("All your Digimon DP modifier")
        effect4.dp_modifier = 3000
        effect4._applies_to_all_own_digimon = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.None
        # Grant Skill
        effect5 = ICardEffect()
        effect5.set_effect_name("BT19-073 Your Digimons gain Alliance")
        effect5.set_effect_description("Grant Skill")

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
