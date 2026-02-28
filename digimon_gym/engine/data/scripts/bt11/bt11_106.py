from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_106(CardScript):
    """BT11-106 Cooties Kick"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Cost -1
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-106 Cost -1")
        effect0.set_effect_description("Cost -1")
        effect0.cost_reduction = 1

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Cost -1"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction by 1 — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # Gain Keyword Cannot Be Blocked
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-106 Gain Keyword Cannot Be Blocked")
        effect1.set_effect_description("Gain Keyword Cannot Be Blocked")
        effect1._is_cannot_be_blocked = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain Keyword Cannot Be Blocked"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Numemon') or p.contains_card_name('Sukamon') or p.contains_card_name('Nanimon') or p.contains_card_name('Etemon')):
                    return False
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_cannot_be_blocked')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OptionSkill
        # [On Deletion] Gain 3 memory.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-106 Memory +3")
        effect2.set_effect_description("[On Deletion] Gain 3 memory.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 3 memory, Add Temp Effect, Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(3)
            # Grant temporary effect to target permanent
            pass  # descriptive-tagged: add_temp_effect
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Reveal the top 3 cards of your deck. You may play 1 black Digimon card with a play cost of 3 or less among them without paying the cost. Trash the rest.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT11-106 Play Card, Reveal And Select")
        effect3.set_effect_description("[Security] Reveal the top 3 cards of your deck. You may play 1 black Digimon card with a play cost of 3 or less among them without paying the cost. Trash the rest.")
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card, Reveal And Select"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 3:
                    return False
                if not ('Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            if not (player and game):
                return
            def reveal_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not getattr(c, 'has_play_cost', False):
                    return False
                if getattr(c, 'get_cost_itself', 0) > 3:
                    return False
                if not ('Black' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            def on_revealed(selected, remaining):
                player.hand_cards.append(selected)
                for c in remaining:
                    player.library_cards.append(c)
            game.effect_reveal_and_select(
                player, 3, reveal_filter, on_revealed, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
