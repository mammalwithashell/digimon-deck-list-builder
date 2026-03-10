from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_084(CardScript):
    """BT24-084 Inori Misono"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # Gain 1 memory
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT24-084 Memory +1")
        effect0.set_effect_description("Gain 1 memory")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = getattr(card, 'owner', None)
            game = getattr(owner, 'game', None)
            if not (owner and owner.is_my_turn and game):
                return False
            if game.memory > 4:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # Suspend, Digivolve
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnLoseSecurity)
        effect1.set_effect_name("BT24-084 Digivolve an [Aegiomon] into an [Aegiochusmon]")
        effect1.set_effect_description("Suspend, Digivolve")
        effect1.is_optional = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not permanent or permanent.is_suspended:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, then let 1 of your [Aegiomon] digivolve for 0."""
            player = ctx.get('player')
            game = ctx.get('game')
            tamer_perm = card.permanent_of_this_card() if card else None
            if not (player and game and tamer_perm):
                return
            tamer_perm.suspend()

            def target_filter(p):
                return p.is_digimon and p.contains_card_name('Aegiomon')

            def on_digivolve(target_perm):
                def digi_filter(c):
                    return any('Aegiochusmon' in _n for _n in getattr(c, 'card_names', []))

                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter, cost_override=0, is_optional=True)

            game.effect_select_own_permanent(
                player, on_digivolve, filter_fn=target_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-084 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
