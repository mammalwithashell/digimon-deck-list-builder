from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_05(CardScript):
    """ST18-05 Muchomon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [All Turns] [Once Per Turn] When this Digimon is suspended by an effect,
        # 1 of your Digimon with [Bird] in any of its traits or with the
        # [Vortex Warriors] trait gets +3000 DP for the turn.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnTappedAnyone)
        effect0.set_effect_name("ST18-05 When suspended by effect, +3000 DP to Bird/Vortex Warriors")
        effect0.set_effect_description(
            "[All Turns] [Once Per Turn] When this Digimon is suspended by an effect, "
            "1 of your Digimon with [Bird] in any of its traits or with the "
            "[Vortex Warriors] trait gets +3000 DP for the turn."
        )
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("ST18_05_OnSuspend")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def target_filter(p):
                if not p.is_digimon:
                    return False
                traits = []
                for cs in p.digivolution_cards + ([p.top_card] if p.top_card else []):
                    traits.extend(getattr(cs, 'card_traits', []) or [])
                return any('Bird' in t for t in traits) or any('Vortex Warriors' in t for t in traits)

            def on_select(target_perm):
                target_perm.change_dp(3000)

            game.effect_select_own_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Inherited: <Piercing>
        effect1 = ICardEffect()
        effect1.set_effect_name("ST18-05 Piercing")
        effect1.set_effect_description("Piercing")
        effect1._is_piercing = True
        effect1.is_inherited_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        return effects
