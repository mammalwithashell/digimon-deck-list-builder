from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT6_082(CardScript):
    """BT6-082 Sistermon Blanc | Lv.3

    [All Turns] While you have a Digimon with [Huckmon] in its name or
        [Royal Knight] trait in play, all of your Digimon with [Sistermon]
        in their name gain <Blocker>.
    [On Play] <Draw 1> (Draw 1 card from your deck.)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] Grant Blocker to this Sistermon while Huckmon/RK exists ---
        # Note: The card text says "all of your Digimon with [Sistermon]" but since each
        # Sistermon carries its own copy of this effect via its own script, each will
        # independently gain Blocker when the condition is met. This effect applies Blocker
        # to THIS permanent only, gated on Huckmon/Royal Knight existing on the field.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT6-082 Blocker (conditional)")
        effect0.set_effect_description(
            "[All Turns] While you have a Digimon with [Huckmon] in its name "
            "or [Royal Knight] trait in play, all of your Digimon with "
            "[Sistermon] in their name gain <Blocker>."
        )
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Check if this permanent is a Sistermon
            perm = card.permanent_of_this_card() if card else None
            if perm:
                names = getattr(perm.top_card, 'card_names', []) or []
                name = names[0] if names else ''
                if 'Sistermon' not in name:
                    return False
            # Check for Huckmon-name or Royal Knight trait ally
            has_huckmon_or_rk = False
            for p in player.battle_area:
                if not p.is_digimon:
                    continue
                names = getattr(p.top_card, 'card_names', []) or []
                name = names[0] if names else ''
                if 'Huckmon' in name:
                    has_huckmon_or_rk = True
                    break
                traits = getattr(p.top_card, 'card_traits', []) or []
                if any('Royal Knight' in t for t in traits):
                    has_huckmon_or_rk = True
                    break
            return has_huckmon_or_rk
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Draw 1 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT6-082 Draw 1")
        effect1.set_effect_description("[On Play] <Draw 1>")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
