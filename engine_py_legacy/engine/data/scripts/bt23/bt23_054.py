from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_054(CardScript):
    """BT23-054 Magnamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-054 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.3 from [Veemon] for cost 3
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 3
        effect0._alt_digi_name = "Veemon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Veemon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-054 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: armor_purge
        # Armor Purge
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-054 Armor Purge")
        effect2.set_effect_description("Armor Purge")
        effect2._is_armor_purge = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        def _rk_cs_filter(p):
            """Filter for Digimon with [Royal Knight] or [CS] trait."""
            if not p.is_digimon:
                return False
            if p.top_card:
                for t in getattr(p.top_card, 'card_traits', []):
                    if 'Royal Knight' in t or t == 'CS':
                        return True
            return False

        def _draw_and_grant_bounce_immunity(ctx):
            """Draw 1, then select 1 of your Digimon with Royal Knight or CS trait to grant bounce immunity."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.draw_cards(1)
            if not (player and game):
                return
            # Check if any valid targets exist before requesting selection
            candidates = [p for p in player.battle_area if _rk_cs_filter(p)]
            if not candidates:
                return

            def on_select(selected_perm):
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    selected_perm, ModifierType.CANNOT_RETURN_TO_HAND,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
                game.register_modifier(
                    selected_perm, ModifierType.CANNOT_RETURN_TO_DECK,
                    value_fn=lambda: True, expiry='end_of_opponent_turn')
            game.effect_select_own_permanent(
                player, on_select, filter_fn=_rk_cs_filter, is_optional=False,
                prompt="Select a Digimon with [Royal Knight] or [CS] trait to protect from bouncing.")

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Draw 1. Then, 1 of your Digimon with [Royal Knight] or [CS] trait can't be returned
        # to hands or decks by opponent's effects until their turn ends.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT23-054 Draw 1, then give can't be returned to hand/deck")
        effect3.set_effect_description("[On Play] <Draw 1> Then, 1 of your Digimon with the [Royal Knight] or [CS] trait can't be returned to hands or decks by your opponent's effects until their turn ends.")
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Draw 1, then grant bounce immunity to a selected Royal Knight/CS Digimon"""
            _draw_and_grant_bounce_immunity(ctx)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Draw 1. Then, 1 of your Digimon with [Royal Knight] or [CS] trait can't be returned
        # to hands or decks by opponent's effects until their turn ends.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("BT23-054 Draw 1, then give can't be returned to hand/deck")
        effect4.set_effect_description("[When Digivolving] <Draw 1> Then, 1 of your Digimon with the [Royal Knight] or [CS] trait can't be returned to hands or decks by your opponent's effects until their turn ends.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Draw 1, then grant bounce immunity to a selected Royal Knight/CS Digimon"""
            _draw_and_grant_bounce_immunity(ctx)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
