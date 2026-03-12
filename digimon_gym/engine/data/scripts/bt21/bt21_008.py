from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_008(CardScript):
    """BT21-008 Elizamon | Lv.3"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Add To Hand, Reveal And Select
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT21-008 Reveal top 3, Add Reptile/Dragonkin & 1 Liberator")
        effect0.set_effect_description("Add To Hand, Reveal And Select")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal top 3, add 1 Reptile/Dragonkin and 1 LIBERATOR to hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            revealed = player.library_cards[:3]
            player.library_cards = player.library_cards[3:]
            # First pick: Reptile/Dragonkin
            reptile_cards = [c for c in revealed if any('Reptile' in t or 'Dragonkin' in t for t in (getattr(c, 'card_traits', []) or []))]
            if reptile_cards:
                picked = reptile_cards[0]
                revealed.remove(picked)
                player.hand_cards.append(picked)
            # Second pick: LIBERATOR
            lib_cards = [c for c in revealed if any('LIBERATOR' in t for t in (getattr(c, 'card_traits', []) or []))]
            if lib_cards:
                picked = lib_cards[0]
                revealed.remove(picked)
                player.hand_cards.append(picked)
            # Return rest to bottom
            for c in revealed:
                player.library_cards.append(c)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnLoseSecurity
        # Gain 1 memory
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnLoseSecurity)
        effect1.set_effect_name("BT21-008 Gain 1 memory")
        effect1.set_effect_description("Gain 1 memory")
        effect1.is_inherited_effect = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("GainMemory_BT21_008")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            owner = card.owner if card else ctx.get('player')
            if owner:
                owner.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
