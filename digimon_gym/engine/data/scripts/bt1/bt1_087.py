from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_087(CardScript):
    """BT1-087 T.K. Takaishi | Tamer (Yellow, Cost 4)

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [On Play] Search your security stack, reveal 1 card among it and add it
        to your hand. If it's a yellow card, <Recovery +1 (Deck)>. Then,
        shuffle your security stack.
    Security Effect [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 ---
        # C# uses EffectTiming.OnStartTurn + SetMemoryTo3TamerEffect
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT1-087 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Set memory to 3 if <= 2."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Search security, add 1 to hand, recovery if yellow ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT1-087 On Play: search security")
        effect1.set_effect_description(
            "[On Play] Search your security stack, reveal 1 card among it "
            "and add it to your hand. If it's a yellow card, Recovery +1 "
            "(Deck). Then, shuffle your security stack."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must belong to the player whose turn it is (On Play always does)
            owner = card.owner if card else None
            if not owner:
                return False
            # C# CanActivateCondition requires at least 1 security card
            if not owner.security_cards:
                # Still allow the effect to "fire" for the shuffle step but
                # there is nothing to select — let process0 no-op gracefully.
                # Return True so the effect still runs (matches C# which enters
                # the coroutine and short-circuits on empty security).
                return True
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Search security, reveal 1, add to hand, recovery if yellow, shuffle."""
            import random
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if not player.security_cards:
                return

            def on_security_selected(selected):
                # Move the chosen security card to hand
                if selected in player.security_cards:
                    player.security_cards.remove(selected)
                    player.hand_cards.append(selected)

                    # If it's a yellow card, Recovery +1 (Deck)
                    colors = getattr(selected, 'card_colors', []) or []
                    if CardColor.Yellow in colors:
                        player.recovery(1)

                # Then, shuffle your security stack
                random.shuffle(player.security_cards)

            # "Search your security stack" — player browses all security cards
            # and selects 1 to reveal and add to hand. No filter: any card is valid.
            game.effect_select_own_security(
                player,
                lambda c: True,
                on_security_selected,
                is_optional=False,
                prompt="Search your security stack and reveal 1 card to add to your hand.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        # C# uses CardEffectFactory.PlaySelfTamerSecurityEffect(card)
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT1-087 Security: Play free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this tamer from security without paying the cost.

            Must use game.effect_play_from_security so _security_played is set
            and security_attack() does not also trash the card after play.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            security_card = ctx.get('card')
            if not (player and game and security_card):
                return
            game.effect_play_from_security(player, security_card)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
