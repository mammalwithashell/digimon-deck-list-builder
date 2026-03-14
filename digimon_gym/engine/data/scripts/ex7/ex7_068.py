from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX7_068(CardScript):
    """EX7-068 Wonder Stomp | Option (Yellow, Cost 2, LIBERATOR)

    [Main] <Draw 1> (Draw 1 card from your deck.) Then, you may play 1
        level 3 Digimon card with the [Puppet] trait from your hand without
        paying the cost.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _main_process(ctx: Dict[str, Any]):
            """Draw 1, then play Lv.3 Puppet from hand free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Draw 1
            player.draw(1)

            # Then, you may play 1 level 3 Digimon with [Puppet] trait from hand free
            def puppet_lv3_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) != 3:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Puppet' in t for t in traits)

            game.effect_play_from_zone(
                player, 'hand', puppet_lv3_filter, free=True, is_optional=True)

        # --- Effect 1: [Main] Draw 1, play Lv.3 Puppet from hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("EX7-068 Draw 1, play Lv.3 Puppet from hand")
        effect1.set_effect_description(
            "[Main] <Draw 1> Then, you may play 1 level 3 Digimon card with "
            "the [Puppet] trait from your hand without paying the cost."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_main_process)
        effects.append(effect1)

        # --- Effect 2: [Security] Activate this card's [Main] effects ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX7-068 Security: Activate Main effects")
        effect2.set_effect_description(
            "[Security] Activate this card's [Main] effects."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_main_process)
        effects.append(effect2)

        return effects
