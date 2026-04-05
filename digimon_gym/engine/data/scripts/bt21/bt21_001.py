from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_001(CardScript):
    """BT21-001 Gigimon | Lv.2"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnLoseSecurity
        # Digivolve
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnLoseSecurity)
        effect0.set_effect_name("BT21-001 Digivolve a digimon")
        effect0.set_effect_description("Digivolve")
        effect0.is_inherited_effect = True
        effect0.is_optional = True
        effect0.set_max_count_per_turn(1)
        effect0.set_hash_string("Digivolve_BT21_001")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # "When your opponent's security stack is removed from" —
            # OnLoseSecurity fires with event_player = the player who lost security.
            # Must verify it's the opponent, not the card owner.
            event_player = context.get('event_player')
            owner = context.get('player')
            if event_player and owner and event_player is not owner.enemy:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: 1 of your Digimon may digivolve into a Reptile/Dragonkin from hand with cost -1."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def digi_filter(c):
                if not (any('Reptile' in _t or 'Dragonkin' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            # "1 of your Digimon" — player selects which Digimon to digivolve (not just self)
            def own_filter(p):
                return p.is_digimon
            def on_selected(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter, cost_reduction=1, is_optional=True,
                    prompt="Select a [Reptile] or [Dragonkin] Digimon from hand to digivolve into (cost -1).")
            game.effect_select_own_permanent(
                player, on_selected, filter_fn=own_filter, is_optional=True,
                prompt="Select 1 of your Digimon to digivolve.")

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        return effects
