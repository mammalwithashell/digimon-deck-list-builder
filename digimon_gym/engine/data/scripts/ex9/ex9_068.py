from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_068(CardScript):
    """EX9-068 Analogman | Tamer | Black | Cost 4 | DM

    [Start of Your Turn] If you have 2 or less memory, set it to 3.
    [Your Turn] When any of your play cost 7 or higher [Cyborg], [Machine] or
        [DM] trait Digimon are played, by suspending this Tamer, Draw 1 and
        gain 1 memory. Then, you may place 1 card from your hand face down as
        any of those Digimon's bottom digivolution card.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Set memory to 3 if <= 2 ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("EX9-068 Start of Your Turn: Set memory to 3")
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
            """Set memory to 3 if currently 2 or less."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            if player.memory <= 2:
                player.set_memory(3)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] When cost 7+ Cyborg/Machine/DM Digimon played ---
        # Suspend this Tamer -> Draw 1 + gain 1 memory, then optionally place
        # 1 card from hand as bottom digi card of that Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-068 Your Turn: Draw + memory on Cyborg/Machine/DM play")
        effect1.set_effect_description(
            "[Your Turn] When any of your play cost 7 or higher [Cyborg], "
            "[Machine] or [DM] trait Digimon are played, by suspending this "
            "Tamer, Draw 1 and gain 1 memory. Then, you may place 1 card from "
            "your hand face down as any of those Digimon's bottom digivolution card."
        )
        effect1.is_optional = True

        def _is_qualifying_digimon_perm(perm) -> bool:
            """Check if a permanent is a cost 7+ Cyborg/Machine/DM Digimon."""
            if not perm.is_digimon:
                return False
            top = perm.top_card
            if not top:
                return False
            play_cost = top.get_cost_itself if top else 0
            if play_cost < 7:
                return False
            traits = getattr(top, 'card_traits', []) or []
            return any(
                t in ('Cyborg', 'Machine', 'DM') or
                'Cyborg' in t or 'Machine' in t or 'DM' in t
                for t in traits
            )

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Only on your turn
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check that the played permanent qualifies
            trigger_perm = context.get('permanent')
            if not trigger_perm:
                return False
            # Must be our Digimon, not opponent's
            if trigger_perm.owner != card.owner:
                return False
            if not _is_qualifying_digimon_perm(trigger_perm):
                return False
            # This Tamer must be unsuspended
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm and my_perm.is_suspended:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Suspend this Tamer, draw 1, gain 1 memory, optionally place hand card as bottom digi."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            my_perm = card.permanent_of_this_card() if card else None
            if not my_perm:
                return

            # Suspend this Tamer
            my_perm.suspend()

            # Draw 1
            if player.library_cards:
                drawn = player.library_cards.pop(0)
                player.hand_cards.append(drawn)

            # Gain 1 memory
            player.add_memory(1)

            # Optionally place 1 card from hand as bottom digivolution card
            # of the played Digimon
            trigger_perm = ctx.get('permanent')
            if not trigger_perm or trigger_perm not in player.battle_area:
                return
            if not player.hand_cards:
                return

            # Place the first card from hand (simplified selection)
            # In the full engine, this would be a selection prompt
            hand_card = player.hand_cards[0]
            player.hand_cards.remove(hand_card)
            trigger_perm.add_card_source_bottom(hand_card)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX9-068 Security: Play this card free")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this Tamer from security without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game and card):
                return
            player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
