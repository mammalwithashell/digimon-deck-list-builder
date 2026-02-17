from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_045(CardScript):
    """P-045 Kurisarimon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: decoy
        # Decoy
        effect0 = ICardEffect()
        effect0.set_effect_name("P-045 Decoy")
        effect0.set_effect_description("Decoy")
        effect0.is_inherited_effect = True
        effect0._is_decoy = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # <Decoy (Black/White)> (When one of your other black or white Digimon would be deleted by an opponent's effect, you may delete this Digimon to prevent that deletion.)
        effect1 = ICardEffect()
        effect1.set_effect_name("P-045 Your Digimons gain Decoy")
        effect1.set_effect_description("<Decoy (Black/White)> (When one of your other black or white Digimon would be deleted by an opponent's effect, you may delete this Digimon to prevent that deletion.)")
        effect1.is_inherited_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Grant Skill"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Grant keyword to other permanents (AddSkillClass) — not yet in engine
            pass  # descriptive-tagged: grant_skill

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
