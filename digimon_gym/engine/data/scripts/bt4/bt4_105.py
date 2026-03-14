from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT4_105(CardScript):
    """BT4-105 Tactical Retreat! | Option (Yellow, Cost 1)

    [Main] Place 1 of your Digimon on top of your security stack face down.
    [Security] <Recovery +1 (Deck)> (Place the top card of your deck on
        top of your security stack.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 1: [Main] Place 1 of your Digimon on top of security ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT4-105 Place Digimon on security")
        effect1.set_effect_description(
            "[Main] Place 1 of your Digimon on top of your security stack face down."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            own_digimon = [p for p in player.battle_area if p.is_digimon]
            if not own_digimon:
                return

            def on_selected(target_perm):
                if target_perm is None:
                    return
                # Remove the permanent from battle area
                if target_perm in player.battle_area:
                    player.battle_area.remove(target_perm)
                # Place all card sources on top of security stack face down
                for cs in target_perm.card_sources:
                    player.security_cards.append(cs)

            game.effect_select_own_permanent(
                player, on_selected,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Place 1 of your Digimon on top of your security stack."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Recovery +1 (Deck) ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT4-105 Security: Recovery +1")
        effect2.set_effect_description(
            "[Security] <Recovery +1 (Deck)> (Place the top card of your deck "
            "on top of your security stack.)"
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
