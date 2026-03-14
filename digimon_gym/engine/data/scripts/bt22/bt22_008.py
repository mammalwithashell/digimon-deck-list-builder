from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_008(CardScript):
    """BT22-008 Agumon | Lv.3

    Digivolve: from [Koromon] or Lv.2 w/ CS trait for 0.
    [On Play] You may return 1 Digimon card with [Greymon], [Garurumon],
        or [Omnimon] in its name from your trash to the hand.
    Inherited: [End of Your Turn] This Digimon and any of your other Digimon
        may DNA digivolve into a Digimon card in the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Koromon] for cost 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT22-008 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_level = 2
        effect0._alt_digi_name = "Koromon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Koromon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Return 1 Digimon with Greymon/Garurumon/Omnimon from trash to hand ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT22-008 Return a card from trash")
        effect1.set_effect_description(
            "[On Play] You may return 1 Digimon card with [Greymon], "
            "[Garurumon], or [Omnimon] in its name from your trash to the hand."
        )
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def _trash_name_filter(c) -> bool:
            if not getattr(c, 'is_digimon', False):
                return False
            return (c.contains_card_name('Greymon') or
                    c.contains_card_name('Garurumon') or
                    c.contains_card_name('Omnimon'))

        def process1(ctx: Dict[str, Any]):
            """Return 1 qualifying Digimon from trash to hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            matching = [c for c in player.trash_cards if _trash_name_filter(c)]
            if not matching:
                return

            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = []
            for i, c in enumerate(player.trash_cards):
                if _trash_name_filter(c):
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_select(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                chosen = player.trash_cards[idx]
                player.trash_cards.remove(chosen)
                player.hand_cards.append(chosen)

            game.request_selection(
                GamePhase.SelectTrash, player, on_select, valid,
                is_optional=True,
                prompt="Select 1 Digimon with [Greymon], [Garurumon], or [Omnimon] from trash to return to hand."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [End of Your Turn] DNA digivolve ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT22-008 DNA digivolve this Digimon")
        effect2.set_effect_description(
            "[End of Your Turn] This Digimon and any of your other Digimon "
            "may DNA digivolve into a Digimon card in the hand."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """DNA digivolve from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_dna_digivolve_from_hand(
                player,
                filter_fn=lambda c: getattr(c, 'is_digimon', False),
                is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
