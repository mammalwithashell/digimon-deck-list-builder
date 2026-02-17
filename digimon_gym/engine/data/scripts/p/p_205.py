from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_205(CardScript):
    """P-205 Insane Synthetic Monster"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("P-205 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] <Draw 2> and trash 2 cards in your hand. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("P-205 Draw 2, Trash 2.")
        effect1.set_effect_description("[Main] <Draw 2> and trash 2 cards in your hand. Then, place this card in the battle area.")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("P-205 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay>. By deleting 1 of your play cost 7 or lower Digimon, you may play 1 Digimon card with [Kimeramon] or [Millenniummon] in its name from your trash with the play cost reduced by 3.
        effect3 = ICardEffect()
        effect3.set_effect_name("P-205 By delete 1 7 play cost or less digimon, play 1 [Kimeramon]/[Millenniummon] in its name digimon from trash for 3 reduced play cost")
        effect3.set_effect_description("[Main] <Delay>. By deleting 1 of your play cost 7 or lower Digimon, you may play 1 Digimon card with [Kimeramon] or [Millenniummon] in its name from your trash with the play cost reduced by 3.")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Kimeramon' in _n or 'Millenniummon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.SecuritySkill
        # [Security] <Draw 2> and trash 2 cards in your hand. Then, place this card in the battle area.
        effect4 = ICardEffect()
        effect4.set_effect_name("P-205 Draw 2, Trash 2.")
        effect4.set_effect_description("[Security] <Draw 2> and trash 2 cards in your hand. Then, place this card in the battle area.")
        effect4.is_security_effect = True
        effect4.is_security_effect = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
