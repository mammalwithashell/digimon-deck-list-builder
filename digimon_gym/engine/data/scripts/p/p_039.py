from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_039(CardScript):
    """P-039 Black Memory Boost!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] Reveal the top 4 cards of your deck. Add 1 black Digimon card among them to your hand. Place the remaining cards at the bottom of your deck in any order. Then, place this card in your battle area.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("P-039 Add To Hand, Reveal And Select")
        effect0.set_effect_description("[Main] Reveal the top 4 cards of your deck. Add 1 black Digimon card among them to your hand. Place the remaining cards at the bottom of your deck in any order. Then, place this card in your battle area.")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Reveal And Select"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not ('Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 4, reveal_filter, on_revealed, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: delay
        # Delay
        effect1 = ICardEffect()
        effect1.set_effect_name("P-039 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - Gain 2 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDeclaration)
        effect2._is_field_main = True
        effect2.set_effect_name("P-039 Memory +2")
        effect2.set_effect_description("[Main] <Delay> (Trash this card in your battle area to activate the effect below. You can't activate this effect the turn this card enters play.) - Gain 2 memory.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 2 memory"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(2)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Place this card in the battle area ---
        # C# ref: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("P-039 Security: Place in battle area")
        effect3.set_effect_description(
            "[Security] Place this card in the battle area."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Place this card in the battle area from security."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if card is None:
                return
            # Mark as security_played so the engine won't trash it after the
            # security check resolves.
            card._security_played = True

            # Create a permanent for the option and place it in the battle area.
            from ....core.permanent import Permanent
            FIELD_SLOTS = 15
            if len(player.battle_area) >= FIELD_SLOTS:
                return
            perm = Permanent([card])
            perm.turn_played = game.turn_count
            perm._owner_game = game
            player.battle_area.append(perm)
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
