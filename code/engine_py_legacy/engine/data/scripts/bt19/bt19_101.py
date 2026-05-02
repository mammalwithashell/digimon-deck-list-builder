from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT19_101(CardScript):
    """BT19-101 ZeedMillenniummon | Lv.7"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT19-101 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [MoonMillenniummon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "MoonMillenniummon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('MoonMillenniummon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: overclock
        # Overclock
        effect1 = ICardEffect()
        effect1.set_effect_name("BT19-101 Overclock")
        effect1.set_effect_description("Overclock")
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared process for On Play / When Digivolving / When Attacking ---
        # C#: Select 1 Digimon card from opponent's trash -> place on top of opponent's deck ->
        #     then select 1 opponent's Digimon -> return to bottom of deck
        def _trash_to_deck_top_then_bounce(ctx: Dict[str, Any]):
            """By returning 1 Digimon card from opponent's trash to deck top,
            return 1 of their Digimon to the bottom of the deck."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: Select 1 Digimon card from opponent's trash to return to deck top
            from ....game.constants import SEL_TRASH_START
            valid_trash = []
            for i, c in enumerate(enemy.trash_cards):
                if getattr(c, 'is_digimon', False):
                    valid_trash.append(SEL_TRASH_START + i)
            if not valid_trash:
                return

            def on_trash_selected(action_id):
                idx = action_id - SEL_TRASH_START
                if idx < len(enemy.trash_cards):
                    selected = enemy.trash_cards[idx]
                    enemy.trash_cards.remove(selected)
                    # Place on top of opponent's deck
                    enemy.library_cards.insert(0, selected)

                    # Step 2: Select 1 opponent's Digimon to return to deck bottom
                    def target_filter(p):
                        return p.is_digimon
                    def on_bounce(target_perm):
                        if target_perm:
                            enemy.return_permanent_to_deck_bottom(target_perm)
                    game.effect_select_opponent_permanent(
                        player, on_bounce, filter_fn=target_filter, is_optional=False)

            game.request_selection(
                GamePhase.SelectTrash, player, on_trash_selected,
                valid_trash, is_optional=True,
                prompt="Select 1 Digimon card from opponent's trash to return to deck top.")

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] By returning 1 Digimon card from your opponent's trash to the top of the deck, return 1 of their Digimon to the bottom of the deck.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT19-101 Return 1 card from opponent's trash to deck top return 1 digimon to bottom of the deck")
        effect2.set_effect_description("[On Play] By returning 1 Digimon card from your opponent's trash to the top of the deck, return 1 of their Digimon to the bottom of the deck.")
        effect2.is_optional = True
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_trash_to_deck_top_then_bounce)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of the deck, return 1 of their Digimon to the bottom of the deck.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT19-101 Return 1 card from opponent's trash to deck top return 1 digimon to bottom of the deck")
        effect3.set_effect_description("[When Digivolving] By returning 1 Digimon card from your opponent's trash to the top of the deck, return 1 of their Digimon to the bottom of the deck.")
        effect3.is_optional = True
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_trash_to_deck_top_then_bounce)
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack
        # [When Attacking] By returning 1 Digimon card from your opponent's trash to the top of the deck, return 1 of their Digimon to the bottom of the deck.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT19-101 Return 1 card from opponent's trash to deck top return 1 digimon to bottom of the deck")
        effect4.set_effect_description("[When Attacking] By returning 1 Digimon card from your opponent's trash to the top of the deck, return 1 of their Digimon to the bottom of the deck.")
        effect4.is_optional = True
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_trash_to_deck_top_then_bounce)
        effects.append(effect4)

        # Timing: EffectTiming.NoTiming
        # [All Turns] This Digimon with no digivolution cards isn't affected by opponent's effects.
        # C#: CanNotAffectedClass with ImmunityAndNoSuspendCondition checking DigivolutionCards.Count == 0
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.NoTiming)
        effect5.set_effect_name("BT19-101 Isn't affected by opponent's effects")
        effect5.set_effect_description("[All Turns] This Digimon with no digivolution cards isn't affected by opponent's effects.")

        def condition5(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            # C#: DigivolutionCards.Count == 0 means no digi cards (top card only)
            if len(perm.card_sources) > 1:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Register CANNOT_BE_AFFECTED modifier for opponent's effects."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CANNOT_BE_AFFECTED,
                    value_fn=lambda: True, expiry='permanent')

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Timing: EffectTiming.NoTiming
        # [All Turns] This Digimon with no digivolution cards can't be suspended.
        # C#: CanNotSuspendClass with ImmunityAndNoSuspendCondition
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.NoTiming)
        effect6.set_effect_name("BT19-101 Can't be suspended")
        effect6.set_effect_description("[All Turns] This Digimon with no digivolution cards can't be suspended.")

        def condition6(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            # C#: DigivolutionCards.Count == 0 means no digi cards (top card only)
            if len(perm.card_sources) > 1:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Register CANNOT_SUSPEND modifier."""
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm and game:
                from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    perm, ModifierType.CANNOT_SUSPEND,
                    value_fn=lambda: True, expiry='permanent')

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
