from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_007(CardScript):
    """BT17-007 Agumon | Lv.3 Red Rookie

    Alt digivolve: from [Koromon] for 0.
    [Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] in its
        name, return 1 card with [Garurumon]/[Greymon]/[Omnimon] in its name
        from your trash to the hand.
    Inherited: [End of Your Turn] This Digimon and another of your Digimon
        may DNA digivolve into a Digimon card in the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt digivolve from [Koromon] for 0 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("BT17-007 Alt digivolve from Koromon")
        effect0.set_effect_description("Alt digivolve: from [Koromon] for 0")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Koromon"
        effect0._alt_digi_exact = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [Start of Your Main Phase] Return Greymon/Garurumon/Omnimon from trash ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnStartMainPhase)
        effect1.set_effect_name("BT17-007 Return Greymon/Garurumon/Omnimon from trash")
        effect1.set_effect_description(
            "[Start of Your Main Phase] If you have a Tamer with [Tai Kamiya] "
            "in its name, return 1 card with [Garurumon]/[Greymon]/[Omnimon] "
            "in its name from your trash to the hand."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Must have a Tamer with [Tai Kamiya]
            has_tai = any(
                p.is_tamer and (p.top_card.contains_card_name('Tai Kamiya') or
                                p.top_card.contains_card_name('TaiKamiya'))
                for p in card.owner.battle_area
                if p.top_card
            )
            if not has_tai:
                return False
            # Must have matching card in trash (C# has no IsDigimon check)
            def _match(c):
                return (c.contains_card_name('Garurumon') or
                        c.contains_card_name('Greymon') or
                        c.contains_card_name('Omnimon'))
            return any(_match(c) for c in card.owner.trash_cards)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def trash_filter(c):
                return (c.contains_card_name('Garurumon') or
                        c.contains_card_name('Greymon') or
                        c.contains_card_name('Omnimon'))

            matching = [c for c in player.trash_cards if trash_filter(c)]
            if not matching:
                return

            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = []
            for i, c in enumerate(player.trash_cards):
                if trash_filter(c):
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
                is_optional=False,
                prompt="Select 1 card with [Garurumon]/[Greymon]/[Omnimon] from trash to return to hand."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Inherited [End of Your Turn] DNA digivolve from hand ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndTurn)
        effect2.set_effect_name("BT17-007 Inherited: End of Turn DNA digivolve")
        effect2.set_effect_description(
            "[End of Your Turn] This Digimon and another of your Digimon may "
            "DNA digivolve into a Digimon card in the hand."
        )
        effect2.is_inherited_effect = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Need at least 1 hand card and 1 other Digimon
            if len(card.owner.hand_cards) < 1:
                return False
            other_digimon = [
                p for p in card.owner.battle_area
                if p.is_digimon and p is not perm
            ]
            if not other_digimon:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (c.c_entity_base and c.c_entity_base.dna_costs):
                    return False
                return True

            game.effect_dna_digivolve_from_hand(
                player, dna_filter, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
