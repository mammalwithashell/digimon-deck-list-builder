from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_070(CardScript):
    """EX11-070 Unchained"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-070 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: set_memory_3
        # [Start of Your Turn] Set memory to 3 if <= 2
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("EX11-070 Set memory to 3")
        effect1.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Set memory to 3 if <= 2"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [End of Your Turn] 2 of your Digimon may DNA digivolve into [ExMaquinamon] in the hand. Then, this Tamer may <Mind Link> with 1 of your Digimon with [Maquinamon] in its text.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("EX11-070 DNA digivolve into [ExMaquinamon]. Mind Link to [Maquinamon] in text.")
        effect2.set_effect_description("[End of Your Turn] 2 of your Digimon may DNA digivolve into [ExMaquinamon] in the hand. Then, this Tamer may <Mind Link> with 1 of your Digimon with [Maquinamon] in its text.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: DNA digivolve into ExMaquinamon, then Mind Link"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            # Step 1: DNA digivolve into [ExMaquinamon] from hand
            def dna_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('ExMaquinamon' in n for n in names)
            game.effect_dna_digivolve_from_hand(player, dna_filter, is_optional=True)
            # Step 2: Mind Link with a Digimon that has [Maquinamon] in its text
            def link_filter(p):
                if not p.top_card:
                    return False
                text = getattr(p.top_card, 'card_text', '') or ''
                return 'Maquinamon' in text
            game.effect_link_to_permanent(player, card, filter_fn=link_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # DP Floor — this Digimon's DP can't be reduced below 1000
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-070 Can't have less than 1000 DP")
        effect3.set_effect_description("This Digimon's DP can't become less than 1000.")
        effect3.is_inherited_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = getattr(permanent.top_card, 'card_text', '') or ''
                if 'Maquinamon' not in text:
                    return False
            else:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: DP Floor — register modifier so DP can't go below 1000.
            Note: permanent.dp does not yet query CHANGE_DP from ModifierRegistry;
            this registers the correct semantic modifier for when the engine integrates it.
            """
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                def dp_floor_fn(current_dp, target, context):
                    return max(1000, current_dp)
                game.register_modifier(
                    perm, ModifierType.CHANGE_DP,
                    value_fn=dp_floor_fn, expiry='permanent')

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
