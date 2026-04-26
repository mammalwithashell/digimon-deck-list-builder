from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST19_08(CardScript):
    """ST19-08 ShoeShoemon | Lv.4 (Yellow, Puppet/LIBERATOR)

    [Security] You may play 1 card with the [LIBERATOR] trait and a play cost
    of 4 or less from your hand or trash without paying the cost.

    <Overclock ([Puppet] Trait)>

    Inherited Effect:
    [Your Turn] All of your opponent's security Digimon get -3000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] Play 1 LIBERATOR with play cost <= 4 from hand or trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("ST19-08 Security: Play LIBERATOR cost <= 4")
        effect0.set_effect_description(
            "[Security] You may play 1 card with the [LIBERATOR] trait and a "
            "play cost of 4 or less from your hand or trash without paying the cost."
        )
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Play 1 LIBERATOR card with play cost <= 4 from hand or trash without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def liberator_filter(c):
                if getattr(c, 'is_digi_egg', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                if not any('LIBERATOR' in t or 'Liberator' in t for t in traits):
                    return False
                if not getattr(c, 'has_play_cost', True):
                    return False
                cost = getattr(c, 'get_cost_itself', None)
                if cost is None:
                    return False
                return cost <= 4

            game.effect_play_from_zone(
                player, 'hand_or_trash', liberator_filter,
                free=True, is_optional=True,
                prompt="You may play 1 [LIBERATOR] card with play cost 4 or less from hand or trash.",
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: <Overclock ([Puppet] Trait)> ---
        effect1 = ICardEffect()
        effect1.set_effect_name("ST19-08 Overclock")
        effect1.set_effect_description(
            "Overclock ([Puppet] Trait)"
        )
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2 (Inherited): [Your Turn] Opponent's security Digimon get -3000 DP ---
        # This is a continuous/declarative static effect (EffectTiming.NoTiming).
        # The engine applies dp_modifier from active inherited effects during
        # security battle DP calculation.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.NoTiming)
        effect2.set_effect_name("ST19-08 Opponent's Security Digimon -3000 DP")
        effect2.set_effect_description(
            "[Your Turn] All of your opponent's security Digimon get -3000 DP."
        )
        effect2.is_inherited_effect = True
        effect2.dp_modifier = -3000
        effect2._applies_to_opponent_security_digimon = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            return owner.is_my_turn

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            pass  # Declarative DP modifier for security Digimon — applied by engine

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
