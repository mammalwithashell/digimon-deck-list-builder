from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_065(CardScript):
    """EX6-065 Mythical Arms of Salvation!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-065 Ignore color requirements")
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
        # [Main] You may place 1 Digimon card with the [Legend-Arms] trait from your trash as 1 of your Digimon’s bottom digivolution card. Then, place this card in the battle area.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-065 Effect")
        effect1.set_effect_description("[Main] You may place 1 Digimon card with the [Legend-Arms] trait from your trash as 1 of your Digimon’s bottom digivolution card. Then, place this card in the battle area.")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: delay
        # Delay
        effect2 = ICardEffect()
        effect2.set_effect_name("EX6-065 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When one of your Digimon would leave the battle area other than by one of your effects,[Delay] • You may play 1 card with the [Legend-Arms] trait from that Digimon's digivolution cards without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX6-065 Play 1 card with the [Legend-Arms] trait from the removed Digimon's digivolution cards without paying the cost")
        effect3.set_effect_description("[All Turns] When one of your Digimon would leave the battle area other than by one of your effects,[Delay] • You may play 1 card with the [Legend-Arms] trait from that Digimon's digivolution cards without paying the cost.")
        effect3.is_optional = True
        effect3.set_hash_string("PlayDigivolutionCard_EX6_065")

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
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: security_play
        # Security: Play this card
        effect4 = ICardEffect()
        effect4.set_effect_name("EX6-065 Security: Play this card")
        effect4.set_effect_description("Security: Play this card")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
