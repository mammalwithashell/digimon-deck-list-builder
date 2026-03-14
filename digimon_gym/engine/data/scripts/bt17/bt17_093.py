from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_093(CardScript):
    """BT17-093 Kari Kamiya | Tamer White

    [All Turns] When your breeding area is hatched in, by suspending this
        Tamer, gain 1 memory.
    [End of Your Turn] By returning this Tamer to the bottom of the deck,
        <Draw 1>. Then, you may play 1 Tamer card with [Tai Kamiya] or
        [Kari Kamiya] in its name from your hand without paying the cost.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [All Turns] When hatching, suspend -> gain 1 memory ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT17-093 Suspend when hatch to gain 1 memory")
        effect0.set_effect_description(
            "[All Turns] When your breeding area is hatched in, by suspending "
            "this Tamer, gain 1 memory."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if perm.is_suspended:
                return False
            if not (card and card.owner):
                return False
            # Trigger: a Digi-Egg was hatched into breeding area
            # The hatched perm is in played_permanent (event context),
            # not permanent (which is the iterating perm during execute_effects)
            if not context.get('is_hatch'):
                return False
            hatched_perm = context.get('played_permanent')
            if hatched_perm is None:
                return False
            # Must be our breeding area
            if card.owner.breeding_area is not hatched_perm:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            # Pay cost: suspend this Tamer
            perm.suspend()
            # Gain 1 memory
            player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [End of Your Turn] Return to deck bottom, draw 1,
        #     then play Tai Kamiya/Kari Kamiya tamer from hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("BT17-093 End of Turn: return, draw, play tamer")
        effect1.set_effect_description(
            "[End of Your Turn] By returning this Tamer to the bottom of the "
            "deck, <Draw 1>. Then, play 1 [Tai Kamiya]/[Kari Kamiya] Tamer "
            "from hand without paying cost."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            # Cost: return this Tamer to bottom of deck
            player.return_permanent_to_deck_bottom(perm)

            # Draw 1
            player.draw()

            # Then, play 1 Tamer with [Tai Kamiya] or [Kari Kamiya] from hand
            def tamer_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return (c.contains_card_name('Tai Kamiya') or
                        c.contains_card_name('Kari Kamiya'))

            has_tamer = any(tamer_filter(c) for c in player.hand_cards)
            if has_tamer:
                game.effect_play_from_zone(
                    player, 'hand', tamer_filter,
                    free=True, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT17-093 Security: Play this Tamer")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
